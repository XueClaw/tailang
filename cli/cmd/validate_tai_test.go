package cmd

import (
	"os"
	"path/filepath"
	"testing"
)

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
