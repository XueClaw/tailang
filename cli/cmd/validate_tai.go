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

		trimmed := strings.TrimSpace(string(content))
		if looksLikeLegacyTaiJSON(trimmed) {
			var doc taiSchema
			if err := json.Unmarshal(content, &doc); err != nil {
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
	versionPattern    = regexp.MustCompile(`^\.版本\s+\S+$`)
	metaPattern       = regexp.MustCompile(`^\.元信息\s+\S+\s*=\s*"[^"]*"$`)
	targetPattern     = regexp.MustCompile(`^\.目标平台\s+\S+$`)
	modulePattern     = regexp.MustCompile(`^\.程序集\s+\S+$`)
	functionPattern   = regexp.MustCompile(`^\.子程序\s+\S+(\s*,\s*\S+)?$`)
	paramPattern      = regexp.MustCompile(`^\.参数\s+\S+(\s*,\s*\S+)?(\s*=\s*.+)?$`)
	variablePattern   = regexp.MustCompile(`^\.(局部变量|程序集变量|常量)\s+\S+(\s*,\s*[^=]+)?(\s*=\s*.+)?$`)
	docPattern        = regexp.MustCompile(`^\.说明\s+"[^"]*"$`)
	validatePattern   = regexp.MustCompile(`^\.校验\s+"[^"]*"$`)
	codePattern       = regexp.MustCompile(`^\.代码\s+\S+$`)
	unresolvedPattern = regexp.MustCompile(`^\.待定\s+\S+(\s*,\s*"[^"]+"|\s+"[^"]+")$`)
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
			paramPattern.MatchString(line),
			variablePattern.MatchString(line),
			docPattern.MatchString(line),
			validatePattern.MatchString(line),
			unresolvedPattern.MatchString(line):
			if strings.HasPrefix(line, ".版本 ") || strings.HasPrefix(line, ".程序集 ") {
				hasTopLevelDecl = true
			}
			continue

		case codePattern.MatchString(line):
			stack = append(stack, textualTaiBlock{kind: "代码", line: lineNo})
			continue

		case line == ".代码结束":
			if err := popExpectedBlock(&stack, "代码", lineNo); err != nil {
				return err
			}
			continue

		case strings.HasPrefix(line, ".如果 "):
			stack = append(stack, textualTaiBlock{kind: "如果", line: lineNo})
			continue

		case strings.HasPrefix(line, ".否则如果 "):
			if err := requireOpenBlock(stack, "如果", lineNo, ".否则如果"); err != nil {
				return err
			}
			continue

		case line == ".否则":
			if err := requireOpenBlock(stack, "如果", lineNo, ".否则"); err != nil {
				return err
			}
			continue

		case line == ".如果结束":
			if err := popExpectedBlock(&stack, "如果", lineNo); err != nil {
				return err
			}
			continue

		case strings.HasPrefix(line, ".判断开始 "):
			stack = append(stack, textualTaiBlock{kind: "判断", line: lineNo})
			continue

		case strings.HasPrefix(line, ".判断 "):
			if err := requireOpenBlock(stack, "判断", lineNo, ".判断"); err != nil {
				return err
			}
			continue

		case line == ".默认":
			if err := requireOpenBlock(stack, "判断", lineNo, ".默认"); err != nil {
				return err
			}
			continue

		case line == ".判断结束":
			if err := popExpectedBlock(&stack, "判断", lineNo); err != nil {
				return err
			}
			continue

		case strings.HasPrefix(line, ".循环判断首 "), strings.HasPrefix(line, ".循环当 "), strings.HasPrefix(line, ".计次循环首 "), strings.HasPrefix(line, ".变量循环首 "):
			stack = append(stack, textualTaiBlock{kind: "循环", line: lineNo})
			continue

		case line == ".循环判断尾", line == ".变量循环尾":
			if err := popExpectedBlock(&stack, "循环", lineNo); err != nil {
				return err
			}
			continue

		case line == ".跳出循环", line == ".到循环尾":
			continue

		case strings.HasPrefix(line, ".返回"):
			continue

		case strings.HasPrefix(line, ".令 "):
			continue

		case strings.HasPrefix(line, ".真"), strings.HasPrefix(line, ".假"), strings.HasPrefix(line, ".空"):
			continue

		default:
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
