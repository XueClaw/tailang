package cmd

import (
	"os"
	"path/filepath"
	"testing"
)

func TestRunMengTestFilePasses(t *testing.T) {
	t.Setenv("TAILANG_LLM_PROVIDER", "custom")
	t.Setenv("TAILANG_LLM_BASE_URL", "http://127.0.0.1:0")

	tempDir := t.TempDir()
	srcDir := filepath.Join(tempDir, "src")
	testsDir := filepath.Join(tempDir, "tests")

	if err := os.MkdirAll(srcDir, 0755); err != nil {
		t.Fatalf("mkdir src: %v", err)
	}
	if err := os.MkdirAll(testsDir, 0755); err != nil {
		t.Fatalf("mkdir tests: %v", err)
	}

	target := filepath.Join(srcDir, "main.meng")
	testFile := filepath.Join(testsDir, "main_test.meng")

	targetContent := ".版本 3\n.程序集 main\n.说明 \"Hello\"\n"
	testContent := "测试 打印功能:\n  期望 输出 \"Hello\"\n"

	if err := os.WriteFile(target, []byte(targetContent), 0644); err != nil {
		t.Fatalf("write target: %v", err)
	}
	if err := os.WriteFile(testFile, []byte(testContent), 0644); err != nil {
		t.Fatalf("write test: %v", err)
	}

	if err := runMengTestFile(testFile); err != nil {
		t.Fatalf("runMengTestFile returned error: %v", err)
	}
}

func TestRunMengTestFileFailsWhenOutputMissing(t *testing.T) {
	t.Setenv("TAILANG_LLM_PROVIDER", "custom")
	t.Setenv("TAILANG_LLM_BASE_URL", "http://127.0.0.1:0")

	tempDir := t.TempDir()
	srcDir := filepath.Join(tempDir, "src")
	testsDir := filepath.Join(tempDir, "tests")

	if err := os.MkdirAll(srcDir, 0755); err != nil {
		t.Fatalf("mkdir src: %v", err)
	}
	if err := os.MkdirAll(testsDir, 0755); err != nil {
		t.Fatalf("mkdir tests: %v", err)
	}

	target := filepath.Join(srcDir, "main.meng")
	testFile := filepath.Join(testsDir, "main_test.meng")

	targetContent := ".版本 3\n.程序集 main\n.说明 \"Hello\"\n"
	testContent := "测试 打印功能:\n  期望 输出 \"World\"\n"

	if err := os.WriteFile(target, []byte(targetContent), 0644); err != nil {
		t.Fatalf("write target: %v", err)
	}
	if err := os.WriteFile(testFile, []byte(testContent), 0644); err != nil {
		t.Fatalf("write test: %v", err)
	}

	if err := runMengTestFile(testFile); err == nil {
		t.Fatal("expected runMengTestFile to fail")
	}
}
