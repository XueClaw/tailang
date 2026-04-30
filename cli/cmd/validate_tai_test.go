package cmd

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/spf13/cobra"
)

func newValidateTaiCommandForTest() *cobra.Command {
	cmd := &cobra.Command{Use: validateTaiCmd.Use}
	cmd.RunE = validateTaiCmd.RunE
	cmd.Flags().Bool("json", false, "Emit machine-readable validation diagnostics as JSON")
	return cmd
}

func TestValidateTaiCommandSuccess(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "valid.tai")
	content := `{
	  "version": "0.1.0",
	  "source": {
	    "provider": "dashscope",
	    "model": "qwen-plus",
	    "temperature": "0"
	  },
	  "modules": [],
	  "code_blocks": [],
	  "unresolved_items": []
	}`

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write .tai file: %v", err)
	}

	if err := validateTaiCmd.RunE(validateTaiCmd, []string{input}); err != nil {
		t.Fatalf("validateTaiCmd returned error: %v", err)
	}
}

func TestValidateTaiCommandFailure(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "invalid.tai")
	content := `{
	  "version": "0.1.0",
	  "source": {
	    "provider": "dashscope",
	    "model": "qwen-plus",
	    "temperature": "0"
	  },
	  "modules": [
	    {
	      "name": "",
	      "description": "bad",
	      "functions": []
	    }
	  ],
	  "code_blocks": [],
	  "unresolved_items": []
	}`

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write .tai file: %v", err)
	}

	if err := validateTaiCmd.RunE(validateTaiCmd, []string{input}); err == nil {
		t.Fatal("expected validateTaiCmd to fail for invalid .tai")
	}
}

func TestValidateTaiCommandTextualSourceSuccess(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "valid-textual.tai")
	content := `.版本 3
.目标平台 视窗

.程序集 认证
.说明 "认证流程"

.子程序 登录(邮箱: 文本型, 密码: 文本型) -> 文本型, , ,
.校验 "邮箱不能为空"
.如果 邮箱 等于 ""
    .返回 "邮箱不能为空"
.如果结束
结果: 文本型 = 邮箱
.返回 结果
.代码 Rust
println!("hello");
.代码结束

.待定 规则, "缺少密码复杂度规则"
`

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write textual .tai file: %v", err)
	}

	if err := validateTaiCmd.RunE(validateTaiCmd, []string{input}); err != nil {
		t.Fatalf("validateTaiCmd returned error for textual .tai: %v", err)
	}
}

func TestValidateTaiCommandTextualSourceFailure(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "invalid-textual.tai")
	content := `.程序集 认证
.子程序 登录() -> 文本型, , ,
.如果 邮箱 等于 ""
`

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write invalid textual .tai file: %v", err)
	}

	if err := validateTaiCmd.RunE(validateTaiCmd, []string{input}); err == nil {
		t.Fatal("expected validateTaiCmd to fail for invalid textual .tai")
	}
}

func TestValidateTaiCommandRejectsUnclosedBlocks(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "unclosed.tai")
	content := `.版本 3
.程序集 认证
.子程序 登录() -> 文本型, , ,
.如果 真
    .返回 "ok"
`

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write invalid textual .tai file: %v", err)
	}

	if err := validateTaiCmd.RunE(validateTaiCmd, []string{input}); err == nil {
		t.Fatal("expected validateTaiCmd to fail for unclosed .如果 block")
	}
}

func TestValidateTaiCommandSupportsMatchAndLoop(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "match-loop.tai")
	content := `.版本 3
.程序集 认证
.子程序 登录() -> 文本型, , ,
.判断开始 状态
.判断 "成功"
    .返回 "ok"
.默认
    .循环判断首 true
        .跳出循环
    .end
    .返回 "unknown"
.end
`

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write textual .tai file: %v", err)
	}

	if err := validateTaiCmd.RunE(validateTaiCmd, []string{input}); err != nil {
		t.Fatalf("validateTaiCmd returned error for match/loop .tai: %v", err)
	}
}

func TestValidateTaiCommandJSONDiagnosticsForTextualFailure(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "invalid-json-output.tai")
	content := `.版本 3
.程序集 认证
.子程序 登录() -> 文本型, , ,
.如果 真
`

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write invalid textual .tai file: %v", err)
	}

	cmd := newValidateTaiCommandForTest()
	var out bytes.Buffer
	cmd.SetOut(&out)
	if err := cmd.Flags().Set("json", "true"); err != nil {
		t.Fatalf("set json flag: %v", err)
	}

	err := cmd.RunE(cmd, []string{input})
	if err == nil {
		t.Fatal("expected validateTaiCmd to fail for invalid textual .tai")
	}

	var result validateOutcome
	if decodeErr := json.Unmarshal(out.Bytes(), &result); decodeErr != nil {
		t.Fatalf("expected JSON validation output, got decode error: %v\nraw: %s", decodeErr, out.String())
	}
	if result.Ok {
		t.Fatalf("expected validation failure outcome, got %+v", result)
	}
	if len(result.Diagnostics) != 1 {
		t.Fatalf("expected 1 diagnostic, got %+v", result.Diagnostics)
	}
	if result.Diagnostics[0].Code != "TAI-TEXT-UNCLOSED-BLOCK" {
		t.Fatalf("unexpected diagnostic code: %+v", result.Diagnostics[0])
	}
	if result.Diagnostics[0].File == "" || result.Diagnostics[0].Span == nil || result.Diagnostics[0].Span.Line == 0 {
		t.Fatalf("expected diagnostic to include file and span, got %+v", result.Diagnostics[0])
	}
}

func TestValidateTaiCommandImportWorkspaceSuccess(t *testing.T) {
	tempDir := t.TempDir()
	sharedDir := filepath.Join(tempDir, "shared")
	if err := os.MkdirAll(sharedDir, 0755); err != nil {
		t.Fatalf("mkdir shared: %v", err)
	}

	mainTai := filepath.Join(tempDir, "main.tai")
	sharedTai := filepath.Join(sharedDir, "math.tai")

	mainContent := `.版本 3
.导入 "shared/math.tai"
.程序集 主程序
.子程序 入口() -> 整数型, , ,
.返回 0
`
	sharedContent := `.版本 3
.程序集 数学
.子程序 加一(值: 整数型) -> 整数型, , ,
.返回 值
`

	if err := os.WriteFile(mainTai, []byte(mainContent), 0644); err != nil {
		t.Fatalf("write main.tai: %v", err)
	}
	if err := os.WriteFile(sharedTai, []byte(sharedContent), 0644); err != nil {
		t.Fatalf("write math.tai: %v", err)
	}

	if err := validateTextualTaiFile(mainTai, &validateWorkspaceState{
		visited:     map[string]struct{}{},
		visiting:    map[string]struct{}{},
		moduleOwner: map[string]string{},
	}); err != nil {
		t.Fatalf("expected workspace validation to succeed, got %v", err)
	}
}

func TestValidateTaiCommandImportWorkspaceMissingFile(t *testing.T) {
	tempDir := t.TempDir()
	mainTai := filepath.Join(tempDir, "main.tai")
	mainContent := `.版本 3
.import "shared/missing.tai"
.module main
.subprogram main() -> int, , ,
.return 0
`

	if err := os.WriteFile(mainTai, []byte(mainContent), 0644); err != nil {
		t.Fatalf("write main.tai: %v", err)
	}

	err := validateTextualTaiFile(mainTai, &validateWorkspaceState{
		visited:     map[string]struct{}{},
		visiting:    map[string]struct{}{},
		moduleOwner: map[string]string{},
	})
	if err == nil {
		t.Fatal("expected missing import validation to fail")
	}
	ve, ok := err.(validationError)
	if !ok {
		t.Fatalf("expected validationError, got %T", err)
	}
	if ve.diagnostic.Code != "TAI-IMPORT-MISSING" {
		t.Fatalf("unexpected diagnostic: %+v", ve.diagnostic)
	}
}

func TestValidateTaiCommandImportWorkspaceDuplicateModule(t *testing.T) {
	tempDir := t.TempDir()
	sharedDir := filepath.Join(tempDir, "shared")
	if err := os.MkdirAll(sharedDir, 0755); err != nil {
		t.Fatalf("mkdir shared: %v", err)
	}

	mainTai := filepath.Join(tempDir, "main.tai")
	sharedTai := filepath.Join(sharedDir, "dup.tai")

	mainContent := `.版本 3
.导入 "shared/dup.tai"
.程序集 重复
.子程序 入口() -> 整数型, , ,
.返回 0
`
	sharedContent := `.版本 3
.程序集 重复
.子程序 帮助() -> 整数型, , ,
.返回 0
`

	if err := os.WriteFile(mainTai, []byte(mainContent), 0644); err != nil {
		t.Fatalf("write main.tai: %v", err)
	}
	if err := os.WriteFile(sharedTai, []byte(sharedContent), 0644); err != nil {
		t.Fatalf("write dup.tai: %v", err)
	}

	err := validateTextualTaiFile(mainTai, &validateWorkspaceState{
		visited:     map[string]struct{}{},
		visiting:    map[string]struct{}{},
		moduleOwner: map[string]string{},
	})
	if err == nil {
		t.Fatal("expected duplicate module validation to fail")
	}
	ve, ok := err.(validationError)
	if !ok {
		t.Fatalf("expected validationError, got %T", err)
	}
	if ve.diagnostic.Code != "TAI-WORKSPACE-DUPLICATE-MODULE" {
		t.Fatalf("unexpected diagnostic: %+v", ve.diagnostic)
	}
}
