package cmd

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestGenerateTaiDocsFromTaiFile(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "main.tai")
	output := filepath.Join(tempDir, "docs")

	content := ".版本 3\n.程序集 auth\n.说明 \"认证流程\"\n\n.子程序 login\n.参数 email, 任意型\n.参数 password, 任意型\n.说明 \"邮箱密码登录\"\n.校验 \"邮箱不能为空\"\n\n.代码 python\nprint('hello')\n.代码结束\n\n.待定 rule, \"缺少密码复杂度规则\"\n"
	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("write input: %v", err)
	}

	indexPath, count, err := generateTaiDocs(input, output, "markdown")
	if err != nil {
		t.Fatalf("generateTaiDocs returned error: %v", err)
	}
	if count != 1 {
		t.Fatalf("expected 1 doc file, got %d", count)
	}
	if _, err := os.Stat(indexPath); err != nil {
		t.Fatalf("stat index: %v", err)
	}

	page := filepath.Join(output, "modules", "main.md")
	pageContent, err := os.ReadFile(page)
	if err != nil {
		t.Fatalf("read page: %v", err)
	}

	text := string(pageContent)
	if !strings.Contains(text, "### auth") {
		t.Fatalf("expected module section in page, got %s", text)
	}
	if !strings.Contains(text, "#### login") {
		t.Fatalf("expected function section in page, got %s", text)
	}
	if !strings.Contains(text, "## Code Blocks") {
		t.Fatalf("expected code blocks section in page, got %s", text)
	}
}

func TestGenerateTaiDocsFromDirectoryPrefersTai(t *testing.T) {
	tempDir := t.TempDir()
	srcDir := filepath.Join(tempDir, "src")
	output := filepath.Join(tempDir, "docs")

	if err := os.MkdirAll(srcDir, 0755); err != nil {
		t.Fatalf("mkdir src: %v", err)
	}

	mengPath := filepath.Join(srcDir, "main.meng")
	taiPath := filepath.Join(srcDir, "main.tai")
	testPath := filepath.Join(srcDir, "main_test.meng")

	if err := os.WriteFile(mengPath, []byte("这不是可直接解析的 tai"), 0644); err != nil {
		t.Fatalf("write meng: %v", err)
	}
	if err := os.WriteFile(testPath, []byte("测试"), 0644); err != nil {
		t.Fatalf("write test meng: %v", err)
	}

	content := ".版本 3\n.程序集 main\n.说明 \"主模块\"\n"
	if err := os.WriteFile(taiPath, []byte(content), 0644); err != nil {
		t.Fatalf("write tai: %v", err)
	}

	_, count, err := generateTaiDocs(tempDir, output, "markdown")
	if err != nil {
		t.Fatalf("generateTaiDocs returned error: %v", err)
	}
	if count != 1 {
		t.Fatalf("expected 1 doc file, got %d", count)
	}
}
