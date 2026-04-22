package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"regexp"
	"strings"

	"github.com/spf13/cobra"
)

var validateTaiCmd = &cobra.Command{
	Use:   "validate-tai [file.tai]",
	Short: "Validate a .tai file",
	Long: `Validate a .tai file.

If the file is a legacy JSON snapshot, this command validates it against the
current shared schema rules.

If the file is textual .tai source, this command validates it against the
current .tai v0.3 Chinese-keyword source rules.`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		inputFile := args[0]

		content, err := os.ReadFile(inputFile)
		if err != nil {
			return fmt.Errorf("failed to read file: %w", err)
		}
		decoded, err := decodeUTF8Source(content)
		if err != nil {
			return err
		}

		trimmed := strings.TrimSpace(decoded)
		if looksLikeLegacyTaiJSON(trimmed) {
			var doc taiSchema
			if err := json.Unmarshal([]byte(decoded), &doc); err != nil {
				return fmt.Errorf("invalid .tai JSON snapshot: %w", err)
			}

			if doc.Modules == nil {
				doc.Modules = []taiModule{}
			}
			if doc.CodeBlocks == nil {
				doc.CodeBlocks = []taiCodeBlock{}
			}
			if doc.UnresolvedItems == nil {
				doc.UnresolvedItems = []taiUnresolvedItem{}
			}

			if err := validateTaiAgainstSchema(&doc); err != nil {
				return err
			}

			fmt.Printf("✓ Valid legacy .tai JSON snapshot: %s\n", inputFile)
			return nil
		}

		if err := validateTextualTaiSource(trimmed); err != nil {
			return err
		}
		fmt.Printf("✓ Valid textual .tai source: %s\n", inputFile)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(validateTaiCmd)
}

var (
	versionPattern      = regexp.MustCompile(`^\.(版本|version)\s+\S+$`)
	metaPattern         = regexp.MustCompile(`^\.(元信息|meta)\s+\S+\s*=\s*"[^"]*"$`)
	targetPattern       = regexp.MustCompile(`^\.(目标平台|target)\s+\S+$`)
	modulePattern       = regexp.MustCompile(`^\.(程序集|module)\s+\S+$`)
	functionPattern     = regexp.MustCompile(`^\.(子程序|subprogram)\s+\S+\s*\([^)]*\)\s*->\s*[^,]+(\s*,\s*[^,]*){3}$`)
	docPattern          = regexp.MustCompile(`^\.(说明|doc)\s+"[^"]*"$`)
	validatePattern     = regexp.MustCompile(`^\.(校验|validate)\s+"[^"]*"$`)
	codePattern         = regexp.MustCompile(`^\.(代码|code)\s+\S+$`)
	unresolvedPattern   = regexp.MustCompile(`^\.(待定|todo)\s+\S+(\s*,\s*"[^"]+"|\s+"[^"]+")$`)
	typedLocalPattern   = regexp.MustCompile(`^[A-Za-z_\p{Han}][A-Za-z0-9_\p{Han}]*\s*:\s*[^=]+(\s*=\s*.+)?$`)
)

type textualTaiBlock struct {
	kind string
	line int
}

func validateTextualTaiSource(input string) error {
	if strings.TrimSpace(input) == "" {
		return fmt.Errorf("invalid textual .tai source: empty file")
	}

	lines := strings.Split(input, "\n")
	var stack []textualTaiBlock
	hasTopLevelDecl := false

	for idx, raw := range lines {
		lineNo := idx + 1
		line := strings.TrimSpace(raw)
		if line == "" || strings.HasPrefix(line, "//") || strings.HasPrefix(line, "#") {
			continue
		}

		switch {
		case versionPattern.MatchString(line),
			metaPattern.MatchString(line),
			targetPattern.MatchString(line),
			modulePattern.MatchString(line),
			functionPattern.MatchString(line),
			docPattern.MatchString(line),
			validatePattern.MatchString(line),
			unresolvedPattern.MatchString(line):
			if startsWithAnyKeyword(line, ".版本 ", ".version ", ".程序集 ", ".module ") {
				hasTopLevelDecl = true
			}
			continue

		case codePattern.MatchString(line):
			stack = append(stack, textualTaiBlock{kind: "代码", line: lineNo})
			continue

		case matchesAnyKeyword(line, ".代码结束", ".endcode"):
			if err := popExpectedBlock(&stack, "代码", lineNo); err != nil {
				return err
			}
			continue

		case startsWithAnyKeyword(line, ".如果 ", ".if "):
			stack = append(stack, textualTaiBlock{kind: "如果", line: lineNo})
			continue

		case startsWithAnyKeyword(line, ".否则如果 "):
			if err := requireOpenBlock(stack, "如果", lineNo, ".否则如果"); err != nil {
				return err
			}
			continue

		case matchesAnyKeyword(line, ".否则", ".else"):
			if err := requireOpenBlock(stack, "如果", lineNo, ".否则"); err != nil {
				return err
			}
			continue

		case matchesAnyKeyword(line, ".如果结束"):
			if err := popExpectedBlock(&stack, "如果", lineNo); err != nil {
				return err
			}
			continue

		case startsWithAnyKeyword(line, ".判断开始 ", ".match "):
			stack = append(stack, textualTaiBlock{kind: "判断", line: lineNo})
			continue

		case startsWithAnyKeyword(line, ".判断 ", ".case "):
			if err := requireOpenBlock(stack, "判断", lineNo, ".判断"); err != nil {
				return err
			}
			continue

		case matchesAnyKeyword(line, ".默认", ".default"):
			if err := requireOpenBlock(stack, "判断", lineNo, ".默认"); err != nil {
				return err
			}
			continue

		case matchesAnyKeyword(line, ".判断结束"):
			if err := popExpectedBlock(&stack, "判断", lineNo); err != nil {
				return err
			}
			continue

		case startsWithAnyKeyword(line, ".循环判断首 ", ".while "):
			stack = append(stack, textualTaiBlock{kind: "循环", line: lineNo})
			continue

		case matchesAnyKeyword(line, ".循环判断尾"):
			if err := popExpectedBlock(&stack, "循环", lineNo); err != nil {
				return err
			}
			continue

		case matchesAnyKeyword(line, ".end"):
			if len(stack) == 0 {
				return fmt.Errorf("invalid textual .tai source at line %d: unexpected .end", lineNo)
			}
			stack = stack[:len(stack)-1]
			continue

		case matchesAnyKeyword(line, ".跳出循环", ".break", ".到循环尾", ".continue"):
			continue

		case startsWithAnyKeyword(line, ".返回", ".return"):
			continue

		case startsWithAnyKeyword(line, ".显示 ", ".print "):
			continue

		case typedLocalPattern.MatchString(line):
			continue

		case matchesAnyKeyword(line, "真", "假", "空", "true", "false", "null"), startsWithAnyKeyword(line, "true ", "false ", "null "):
			continue

		default:
			if startsWithAnyKeyword(line, ".参数 ", ".局部变量 ", ".常量 ", ".param ", ".local ", ".const ") {
				return fmt.Errorf("invalid textual .tai source at line %d: deprecated declaration syntax %q", lineNo, line)
			}
			if strings.HasPrefix(line, ".") {
				return fmt.Errorf("invalid textual .tai source at line %d: unknown or malformed directive %q", lineNo, line)
			}
			continue
		}
	}

	if !hasTopLevelDecl {
		return fmt.Errorf("invalid textual .tai source: expected at least one '.版本' or '.程序集' declaration")
	}

	if len(stack) > 0 {
		last := stack[len(stack)-1]
		return fmt.Errorf("invalid textual .tai source: block '%s' opened at line %d was not closed", last.kind, last.line)
	}

	return nil
}

func looksLikeLegacyTaiJSON(input string) bool {
	return strings.HasPrefix(input, "{") &&
		(strings.Contains(input, `"modules"`) || strings.Contains(input, `"code_blocks"`))
}

func requireOpenBlock(stack []textualTaiBlock, kind string, lineNo int, keyword string) error {
	if len(stack) == 0 || stack[len(stack)-1].kind != kind {
		return fmt.Errorf("invalid textual .tai source at line %d: %s must appear inside %s block", lineNo, keyword, kind)
	}
	return nil
}

func popExpectedBlock(stack *[]textualTaiBlock, kind string, lineNo int) error {
	if len(*stack) == 0 {
		return fmt.Errorf("invalid textual .tai source at line %d: unexpected %s结束", lineNo, kind)
	}
	last := (*stack)[len(*stack)-1]
	if last.kind != kind {
		return fmt.Errorf("invalid textual .tai source at line %d: tried to close %s block but current block is %s", lineNo, kind, last.kind)
	}
	*stack = (*stack)[:len(*stack)-1]
	return nil
}

func startsWithAnyKeyword(line string, prefixes ...string) bool {
	for _, prefix := range prefixes {
		if strings.HasPrefix(line, prefix) {
			return true
		}
	}
	return false
}

func matchesAnyKeyword(line string, keywords ...string) bool {
	for _, keyword := range keywords {
		if line == keyword {
			return true
		}
	}
	return false
}
