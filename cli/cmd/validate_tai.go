package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
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
		jsonOutput, _ := cmd.Flags().GetBool("json")

		outcome, err := validateTaiFile(inputFile)
		if jsonOutput {
			if encodeErr := json.NewEncoder(cmd.OutOrStdout()).Encode(outcome); encodeErr != nil {
				return fmt.Errorf("failed to write JSON validation result: %w", encodeErr)
			}
			return err
		}

		if err != nil {
			return err
		}

		switch outcome.Kind {
		case "legacy-json":
			fmt.Fprintf(cmd.OutOrStdout(), "✓ Valid legacy .tai JSON snapshot: %s\n", inputFile)
		default:
			fmt.Fprintf(cmd.OutOrStdout(), "✓ Valid textual .tai source: %s\n", inputFile)
		}
		return nil
	},
}

func init() {
	rootCmd.AddCommand(validateTaiCmd)
	validateTaiCmd.Flags().Bool("json", false, "Emit machine-readable validation diagnostics as JSON")
}

var (
	versionPattern    = regexp.MustCompile(`^\.(版本|version)\s+\S+$`)
	metaPattern       = regexp.MustCompile(`^\.(元信息|meta)\s+\S+\s*=\s*"[^"]*"$`)
	targetPattern     = regexp.MustCompile(`^\.(目标平台|target)\s+\S+$`)
	importPattern     = regexp.MustCompile(`^\.(导入|import)\s+"[^"]+"$`)
	modulePattern     = regexp.MustCompile(`^\.(程序集|module)\s+\S+$`)
	functionPattern   = regexp.MustCompile(`^\.(子程序|subprogram)\s+\S+\s*\([^)]*\)\s*->\s*[^,]+(\s*,\s*[^,]*){3}$`)
	docPattern        = regexp.MustCompile(`^\.(说明|doc)\s+"[^"]*"$`)
	validatePattern   = regexp.MustCompile(`^\.(校验|validate)\s+"[^"]*"$`)
	codePattern       = regexp.MustCompile(`^\.(代码|code)\s+\S+$`)
	unresolvedPattern = regexp.MustCompile(`^\.(待定|todo)\s+\S+(\s*,\s*"[^"]+"|\s+"[^"]+")$`)
	typedLocalPattern = regexp.MustCompile(`^[A-Za-z_\p{Han}][A-Za-z0-9_\p{Han}]*\s*:\s*[^=]+(\s*=\s*.+)?$`)
)

type textualTaiBlock struct {
	kind string
	line int
}

type validateSpan struct {
	Line   int `json:"line,omitempty"`
	Column int `json:"column,omitempty"`
}

type validateDiagnostic struct {
	Code     string        `json:"code"`
	Severity string        `json:"severity"`
	Stage    string        `json:"stage"`
	Message  string        `json:"message"`
	File     string        `json:"file,omitempty"`
	Span     *validateSpan `json:"span,omitempty"`
	Hint     string        `json:"hint,omitempty"`
}

type validateOutcome struct {
	Ok          bool                 `json:"ok"`
	Kind        string               `json:"kind,omitempty"`
	File        string               `json:"file,omitempty"`
	Diagnostics []validateDiagnostic `json:"diagnostics,omitempty"`
}

type validationError struct {
	diagnostic validateDiagnostic
}

func (e validationError) Error() string {
	return e.diagnostic.Message
}

type textualTaiAnalysis struct {
	imports []textualTaiImport
	modules []string
}

type textualTaiImport struct {
	path string
	line int
}

type validateWorkspaceState struct {
	visited     map[string]struct{}
	visiting    map[string]struct{}
	moduleOwner map[string]string
}

func validateTaiFile(inputFile string) (validateOutcome, error) {
	content, err := os.ReadFile(inputFile)
	if err != nil {
		diag := validateDiagnostic{
			Code:     "TAI-INPUT-READ",
			Severity: "error",
			Stage:    "input",
			Message:  fmt.Sprintf("failed to read file: %v", err),
			File:     inputFile,
		}
		return validateOutcome{Ok: false, File: inputFile, Diagnostics: []validateDiagnostic{diag}}, validationError{diagnostic: diag}
	}
	decoded, err := decodeUTF8Source(content)
	if err != nil {
		diag := validateDiagnostic{
			Code:     "TAI-INPUT-UTF8",
			Severity: "error",
			Stage:    "input",
			Message:  err.Error(),
			File:     inputFile,
			Hint:     "确保 .tai/.meng 文件统一使用 UTF-8 编码",
		}
		return validateOutcome{Ok: false, File: inputFile, Diagnostics: []validateDiagnostic{diag}}, validationError{diagnostic: diag}
	}

	trimmed := strings.TrimSpace(decoded)
	if looksLikeLegacyTaiJSON(trimmed) {
		var doc taiSchema
		if err := json.Unmarshal([]byte(decoded), &doc); err != nil {
			diag := validateDiagnostic{
				Code:     "TAI-JSON-PARSE",
				Severity: "error",
				Stage:    "schema",
				Message:  fmt.Sprintf("invalid .tai JSON snapshot: %v", err),
				File:     inputFile,
			}
			return validateOutcome{Ok: false, File: inputFile, Diagnostics: []validateDiagnostic{diag}}, validationError{diagnostic: diag}
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
			diag := validateDiagnostic{
				Code:     "TAI-SCHEMA-INVALID",
				Severity: "error",
				Stage:    "schema",
				Message:  err.Error(),
				File:     inputFile,
			}
			return validateOutcome{Ok: false, File: inputFile, Diagnostics: []validateDiagnostic{diag}}, validationError{diagnostic: diag}
		}

		return validateOutcome{Ok: true, Kind: "legacy-json", File: inputFile}, nil
	}

	state := &validateWorkspaceState{
		visited:     map[string]struct{}{},
		visiting:    map[string]struct{}{},
		moduleOwner: map[string]string{},
	}
	if err := validateTextualTaiFile(inputFile, state); err != nil {
		if ve, ok := err.(validationError); ok {
			return validateOutcome{Ok: false, File: inputFile, Diagnostics: []validateDiagnostic{ve.diagnostic}}, err
		}
		diag := validateDiagnostic{
			Code:     "TAI-TEXT-UNKNOWN",
			Severity: "error",
			Stage:    "parser",
			Message:  err.Error(),
			File:     inputFile,
		}
		return validateOutcome{Ok: false, File: inputFile, Diagnostics: []validateDiagnostic{diag}}, validationError{diagnostic: diag}
	}

	return validateOutcome{Ok: true, Kind: "textual", File: inputFile}, nil
}

func validateTextualTaiFile(inputFile string, state *validateWorkspaceState) error {
	absPath, err := filepath.Abs(inputFile)
	if err != nil {
		return validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-IMPORT-PATH",
			Severity: "error",
			Stage:    "workspace",
			Message:  fmt.Sprintf("failed to resolve path for %s: %v", inputFile, err),
			File:     inputFile,
		}}
	}
	if _, seen := state.visited[absPath]; seen {
		return nil
	}
	if _, active := state.visiting[absPath]; active {
		return validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-IMPORT-CYCLE",
			Severity: "error",
			Stage:    "workspace",
			Message:  fmt.Sprintf("detected import cycle at %s", absPath),
			File:     absPath,
			Hint:     "移除循环 .导入/.import 依赖，保持工作区依赖图有向无环",
		}}
	}

	raw, err := os.ReadFile(absPath)
	if err != nil {
		return validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-INPUT-READ",
			Severity: "error",
			Stage:    "input",
			Message:  fmt.Sprintf("failed to read file: %v", err),
			File:     absPath,
		}}
	}
	decoded, err := decodeUTF8Source(raw)
	if err != nil {
		return validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-INPUT-UTF8",
			Severity: "error",
			Stage:    "input",
			Message:  err.Error(),
			File:     absPath,
			Hint:     "确保工作区中的 .tai 文件统一使用 UTF-8 编码",
		}}
	}
	if looksLikeLegacyTaiJSON(strings.TrimSpace(decoded)) {
		return validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-IMPORT-TEXTUAL-ONLY",
			Severity: "error",
			Stage:    "workspace",
			Message:  fmt.Sprintf("imported file %s must use textual .tai, not legacy JSON snapshot", absPath),
			File:     absPath,
			Hint:     "先将兼容快照归一化为文本 .tai，再纳入 import/workspace 验证图",
		}}
	}

	state.visiting[absPath] = struct{}{}
	analysis, err := validateTextualTaiSourceDetailed(strings.TrimSpace(decoded), absPath)
	if err != nil {
		delete(state.visiting, absPath)
		return err
	}

	for _, moduleName := range analysis.modules {
		if owner, exists := state.moduleOwner[moduleName]; exists && owner != absPath {
			delete(state.visiting, absPath)
			return validationError{diagnostic: validateDiagnostic{
				Code:     "TAI-WORKSPACE-DUPLICATE-MODULE",
				Severity: "error",
				Stage:    "workspace",
				Message:  fmt.Sprintf("module %q is defined in both %s and %s", moduleName, owner, absPath),
				File:     absPath,
				Hint:     "确保同一工作区内每个 .程序集/.module 名称唯一",
			}}
		}
		state.moduleOwner[moduleName] = absPath
	}

	for _, imp := range analysis.imports {
		targetPath := filepath.Clean(filepath.Join(filepath.Dir(absPath), imp.path))
		if filepath.Ext(targetPath) == "" {
			targetPath += ".tai"
		}
		if strings.ToLower(filepath.Ext(targetPath)) != ".tai" {
			delete(state.visiting, absPath)
			return validationError{diagnostic: validateDiagnostic{
				Code:     "TAI-IMPORT-EXTENSION",
				Severity: "error",
				Stage:    "workspace",
				Message:  fmt.Sprintf("import target %q must resolve to a .tai file", imp.path),
				File:     absPath,
				Span:     &validateSpan{Line: imp.line, Column: 1},
				Hint:     "使用相对路径导入文本 .tai 文件，例如 .import \"shared/math.tai\"",
			}}
		}
		if _, err := os.Stat(targetPath); err != nil {
			delete(state.visiting, absPath)
			return validationError{diagnostic: validateDiagnostic{
				Code:     "TAI-IMPORT-MISSING",
				Severity: "error",
				Stage:    "workspace",
				Message:  fmt.Sprintf("import target %q was not found", imp.path),
				File:     absPath,
				Span:     &validateSpan{Line: imp.line, Column: 1},
				Hint:     "确保导入路径相对当前 .tai 文件存在，并已纳入工作区",
			}}
		}
		if err := validateTextualTaiFile(targetPath, state); err != nil {
			delete(state.visiting, absPath)
			return err
		}
	}

	delete(state.visiting, absPath)
	state.visited[absPath] = struct{}{}
	return nil
}

func validateTextualTaiSource(input string) error {
	_, err := validateTextualTaiSourceDetailed(input, "")
	return err
}

func validateTextualTaiSourceDetailed(input string, sourcePath string) (textualTaiAnalysis, error) {
	if strings.TrimSpace(input) == "" {
		return textualTaiAnalysis{}, validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-TEXT-EMPTY",
			Severity: "error",
			Stage:    "parser",
			Message:  "invalid textual .tai source: empty file",
			File:     sourcePath,
		}}
	}

	lines := strings.Split(input, "\n")
	var stack []textualTaiBlock
	hasTopLevelDecl := false
	analysis := textualTaiAnalysis{
		imports: []textualTaiImport{},
		modules: []string{},
	}

	for idx, raw := range lines {
		lineNo := idx + 1
		line := strings.TrimSpace(raw)
		if line == "" || strings.HasPrefix(line, "//") || strings.HasPrefix(line, "#") {
			continue
		}

		switch {
		case importPattern.MatchString(line):
			importPath := extractQuotedValue(line)
			analysis.imports = append(analysis.imports, textualTaiImport{path: importPath, line: lineNo})
			continue

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
			if modulePattern.MatchString(line) {
				if name := extractDirectiveValue(line, []string{".程序集 ", ".module "}); name != "" {
					analysis.modules = append(analysis.modules, name)
				}
			}
			continue

		case codePattern.MatchString(line):
			stack = append(stack, textualTaiBlock{kind: "代码", line: lineNo})
			continue

		case matchesAnyKeyword(line, ".代码结束", ".endcode"):
			if err := popExpectedBlock(&stack, "代码", sourcePath, lineNo); err != nil {
				return textualTaiAnalysis{}, err
			}
			continue

		case startsWithAnyKeyword(line, ".如果 ", ".if "):
			stack = append(stack, textualTaiBlock{kind: "如果", line: lineNo})
			continue

		case startsWithAnyKeyword(line, ".否则如果 "):
			if err := requireOpenBlock(stack, "如果", sourcePath, lineNo, ".否则如果"); err != nil {
				return textualTaiAnalysis{}, err
			}
			continue

		case matchesAnyKeyword(line, ".否则", ".else"):
			if err := requireOpenBlock(stack, "如果", sourcePath, lineNo, ".否则"); err != nil {
				return textualTaiAnalysis{}, err
			}
			continue

		case matchesAnyKeyword(line, ".如果结束"):
			if err := popExpectedBlock(&stack, "如果", sourcePath, lineNo); err != nil {
				return textualTaiAnalysis{}, err
			}
			continue

		case startsWithAnyKeyword(line, ".判断开始 ", ".match "):
			stack = append(stack, textualTaiBlock{kind: "判断", line: lineNo})
			continue

		case startsWithAnyKeyword(line, ".判断 ", ".case "):
			if err := requireOpenBlock(stack, "判断", sourcePath, lineNo, ".判断"); err != nil {
				return textualTaiAnalysis{}, err
			}
			continue

		case matchesAnyKeyword(line, ".默认", ".default"):
			if err := requireOpenBlock(stack, "判断", sourcePath, lineNo, ".默认"); err != nil {
				return textualTaiAnalysis{}, err
			}
			continue

		case matchesAnyKeyword(line, ".判断结束"):
			if err := popExpectedBlock(&stack, "判断", sourcePath, lineNo); err != nil {
				return textualTaiAnalysis{}, err
			}
			continue

		case startsWithAnyKeyword(line, ".循环判断首 ", ".while "):
			stack = append(stack, textualTaiBlock{kind: "循环", line: lineNo})
			continue

		case matchesAnyKeyword(line, ".循环判断尾"):
			if err := popExpectedBlock(&stack, "循环", sourcePath, lineNo); err != nil {
				return textualTaiAnalysis{}, err
			}
			continue

		case matchesAnyKeyword(line, ".end"):
			if len(stack) == 0 {
				return textualTaiAnalysis{}, textualDiagnostic(sourcePath, lineNo, "TAI-TEXT-UNEXPECTED-END", "invalid textual .tai source: unexpected .end", "检查 .end 是否在 .if/.while/.match 块内部")
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
				return textualTaiAnalysis{}, textualDiagnostic(sourcePath, lineNo, "TAI-TEXT-DEPRECATED-DECL", fmt.Sprintf("invalid textual .tai source: deprecated declaration syntax %q", line), "改用新函数头和标准单行变量声明")
			}
			if strings.HasPrefix(line, ".") {
				return textualTaiAnalysis{}, textualDiagnostic(sourcePath, lineNo, "TAI-TEXT-UNKNOWN-DIRECTIVE", fmt.Sprintf("invalid textual .tai source: unknown or malformed directive %q", line), "检查当前指令是否属于正式 .tai v0.3 语法")
			}
			continue
		}
	}

	if !hasTopLevelDecl {
		return textualTaiAnalysis{}, validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-TEXT-MISSING-TOPLEVEL",
			Severity: "error",
			Stage:    "parser",
			Message:  "invalid textual .tai source: expected at least one '.版本' or '.程序集' declaration",
			File:     sourcePath,
			Hint:     "至少声明一个 .版本 或 .程序集/.module 顶层结构",
		}}
	}

	if len(stack) > 0 {
		last := stack[len(stack)-1]
		return textualTaiAnalysis{}, validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-TEXT-UNCLOSED-BLOCK",
			Severity: "error",
			Stage:    "parser",
			Message:  fmt.Sprintf("invalid textual .tai source: block '%s' opened at line %d was not closed", last.kind, last.line),
			File:     sourcePath,
			Span:     &validateSpan{Line: last.line, Column: 1},
			Hint:     "补齐对应的结束指令，例如 .如果结束 / .判断结束 / .循环判断尾 / .end",
		}}
	}

	return analysis, nil
}

func looksLikeLegacyTaiJSON(input string) bool {
	return strings.HasPrefix(input, "{") &&
		(strings.Contains(input, `"modules"`) || strings.Contains(input, `"code_blocks"`))
}

func requireOpenBlock(stack []textualTaiBlock, kind string, file string, lineNo int, keyword string) error {
	if len(stack) == 0 || stack[len(stack)-1].kind != kind {
		return validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-TEXT-BLOCK-CONTEXT",
			Severity: "error",
			Stage:    "parser",
			Message:  fmt.Sprintf("invalid textual .tai source at line %d: %s must appear inside %s block", lineNo, keyword, kind),
			File:     file,
			Span:     &validateSpan{Line: lineNo, Column: 1},
			Hint:     "检查控制流分支是否位于正确的块结构内",
		}}
	}
	return nil
}

func popExpectedBlock(stack *[]textualTaiBlock, kind string, file string, lineNo int) error {
	if len(*stack) == 0 {
		return validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-TEXT-UNEXPECTED-END",
			Severity: "error",
			Stage:    "parser",
			Message:  fmt.Sprintf("invalid textual .tai source at line %d: unexpected %s结束", lineNo, kind),
			File:     file,
			Span:     &validateSpan{Line: lineNo, Column: 1},
			Hint:     "检查结束指令是否与最近打开的块匹配",
		}}
	}
	last := (*stack)[len(*stack)-1]
	if last.kind != kind {
		return validationError{diagnostic: validateDiagnostic{
			Code:     "TAI-TEXT-MISMATCH-END",
			Severity: "error",
			Stage:    "parser",
			Message:  fmt.Sprintf("invalid textual .tai source at line %d: tried to close %s block but current block is %s", lineNo, kind, last.kind),
			File:     file,
			Span:     &validateSpan{Line: lineNo, Column: 1},
			Hint:     "按照打开顺序关闭块结构，避免 .end/.判断结束/.如果结束 混用错位",
		}}
	}
	*stack = (*stack)[:len(*stack)-1]
	return nil
}

func textualDiagnostic(file string, lineNo int, code string, message string, hint string) error {
	return validationError{diagnostic: validateDiagnostic{
		Code:     code,
		Severity: "error",
		Stage:    "parser",
		Message:  message,
		File:     file,
		Span:     &validateSpan{Line: lineNo, Column: 1},
		Hint:     hint,
	}}
}

func extractQuotedValue(line string) string {
	first := strings.Index(line, `"`)
	last := strings.LastIndex(line, `"`)
	if first < 0 || last <= first {
		return ""
	}
	return line[first+1 : last]
}

func extractDirectiveValue(line string, prefixes []string) string {
	for _, prefix := range prefixes {
		if strings.HasPrefix(line, prefix) {
			return strings.TrimSpace(strings.TrimPrefix(line, prefix))
		}
	}
	return ""
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
