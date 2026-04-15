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

	targetContent := "{\n  \"version\": \"0.1.0\",\n  \"source\": {\n    \"provider\": \"custom\",\n    \"model\": \"test\",\n    \"temperature\": \"0\"\n  },\n  \"modules\": [\n    {\n      \"name\": \"main\",\n      \"description\": \"打印 \\\"Hello\\\"\",\n      \"functions\": []\n    }\n  ],\n  \"code_blocks\": [],\n  \"unresolved_items\": []\n}\n"
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

	targetContent := "{\n  \"version\": \"0.1.0\",\n  \"source\": {\n    \"provider\": \"custom\",\n    \"model\": \"test\",\n    \"temperature\": \"0\"\n  },\n  \"modules\": [\n    {\n      \"name\": \"main\",\n      \"description\": \"打印 \\\"Hello\\\"\",\n      \"functions\": []\n    }\n  ],\n  \"code_blocks\": [],\n  \"unresolved_items\": []\n}\n"
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
