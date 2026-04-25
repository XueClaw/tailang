package cmd

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/spf13/cobra"
)

func TestRunMengTestFileBuildsAndExecutesTaiTarget(t *testing.T) {
	tempDir := t.TempDir()
	srcDir := filepath.Join(tempDir, "src")
	testsDir := filepath.Join(tempDir, "tests")

	if err := os.MkdirAll(srcDir, 0755); err != nil {
		t.Fatalf("mkdir src: %v", err)
	}
	if err := os.MkdirAll(testsDir, 0755); err != nil {
		t.Fatalf("mkdir tests: %v", err)
	}

	target := filepath.Join(srcDir, "main.tai")
	testFile := filepath.Join(testsDir, "main_test.meng")

	targetContent := ".版本 3\n.程序集 main\n.子程序 主程序() -> 整数型, , ,\n.显示 \"Hello\"\n.返回 0\n"
	testContent := "测试 打印功能:\n  期望 输出 \"Hello\"\n  期望 退出码 0\n"

	if err := os.WriteFile(target, []byte(targetContent), 0644); err != nil {
		t.Fatalf("write target: %v", err)
	}
	if err := os.WriteFile(testFile, []byte(testContent), 0644); err != nil {
		t.Fatalf("write test: %v", err)
	}

	originalBuild := executeMengTestBuild
	originalExec := runMengTestExecutable
	t.Cleanup(func() {
		executeMengTestBuild = originalBuild
		runMengTestExecutable = originalExec
	})

	var captured buildRequest
	executeMengTestBuild = func(request buildRequest) error {
		captured = request
		return nil
	}
	runMengTestExecutable = func(path string) (compiledProgramResult, error) {
		return compiledProgramResult{stdout: "Hello\n", exitCode: 0}, nil
	}

	if err := runMengTestFile(nil, testFile); err != nil {
		t.Fatalf("runMengTestFile returned error: %v", err)
	}
	if captured.inputFile != target {
		t.Fatalf("expected input file %s, got %s", target, captured.inputFile)
	}
	if captured.backend != "self-native" {
		t.Fatalf("expected default backend self-native, got %s", captured.backend)
	}
	if captured.optLevel != "1" {
		t.Fatalf("expected default opt-level 1, got %s", captured.optLevel)
	}
	if !strings.HasSuffix(captured.outputName, ".exe") {
		t.Fatalf("expected windows executable output, got %s", captured.outputName)
	}
}

func TestRunMengTestFilePreservesBackendFlags(t *testing.T) {
	tempDir := t.TempDir()
	srcDir := filepath.Join(tempDir, "src")
	testsDir := filepath.Join(tempDir, "tests")

	if err := os.MkdirAll(srcDir, 0755); err != nil {
		t.Fatalf("mkdir src: %v", err)
	}
	if err := os.MkdirAll(testsDir, 0755); err != nil {
		t.Fatalf("mkdir tests: %v", err)
	}

	target := filepath.Join(srcDir, "main.tai")
	testFile := filepath.Join(testsDir, "main_test.meng")

	if err := os.WriteFile(target, []byte(".版本 3\n"), 0644); err != nil {
		t.Fatalf("write target: %v", err)
	}
	if err := os.WriteFile(testFile, []byte("测试 LLVM:\n  期望 退出码 0\n"), 0644); err != nil {
		t.Fatalf("write test: %v", err)
	}

	originalBuild := executeMengTestBuild
	originalExec := runMengTestExecutable
	t.Cleanup(func() {
		executeMengTestBuild = originalBuild
		runMengTestExecutable = originalExec
	})

	var captured buildRequest
	executeMengTestBuild = func(request buildRequest) error {
		captured = request
		return nil
	}
	runMengTestExecutable = func(path string) (compiledProgramResult, error) {
		return compiledProgramResult{stdout: "", exitCode: 0}, nil
	}

	cmd := &cobra.Command{}
	cmd.Flags().String("backend", "self-native", "Compiler backend (self-native, llvm)")
	cmd.Flags().String("opt-level", "1", "Optimization level (0, 1, 2)")
	if err := cmd.Flags().Set("backend", "llvm"); err != nil {
		t.Fatalf("set backend: %v", err)
	}
	if err := cmd.Flags().Set("opt-level", "2"); err != nil {
		t.Fatalf("set opt-level: %v", err)
	}

	if err := runMengTestFile(cmd, testFile); err != nil {
		t.Fatalf("runMengTestFile returned error: %v", err)
	}
	if captured.backend != "llvm" {
		t.Fatalf("expected backend llvm, got %s", captured.backend)
	}
	if captured.optLevel != "2" {
		t.Fatalf("expected opt-level 2, got %s", captured.optLevel)
	}
}

func TestRunMengTestFileFailsWhenOutputMissing(t *testing.T) {
	tempDir := t.TempDir()
	srcDir := filepath.Join(tempDir, "src")
	testsDir := filepath.Join(tempDir, "tests")

	if err := os.MkdirAll(srcDir, 0755); err != nil {
		t.Fatalf("mkdir src: %v", err)
	}
	if err := os.MkdirAll(testsDir, 0755); err != nil {
		t.Fatalf("mkdir tests: %v", err)
	}

	target := filepath.Join(srcDir, "main.tai")
	testFile := filepath.Join(testsDir, "main_test.meng")

	targetContent := ".版本 3\n.程序集 main\n.子程序 主程序() -> 整数型, , ,\n.显示 \"Hello\"\n.返回 0\n"
	testContent := "测试 打印功能:\n  期望 输出 \"World\"\n"

	if err := os.WriteFile(target, []byte(targetContent), 0644); err != nil {
		t.Fatalf("write target: %v", err)
	}
	if err := os.WriteFile(testFile, []byte(testContent), 0644); err != nil {
		t.Fatalf("write test: %v", err)
	}

	originalBuild := executeMengTestBuild
	originalExec := runMengTestExecutable
	t.Cleanup(func() {
		executeMengTestBuild = originalBuild
		runMengTestExecutable = originalExec
	})
	executeMengTestBuild = func(request buildRequest) error { return nil }
	runMengTestExecutable = func(path string) (compiledProgramResult, error) {
		return compiledProgramResult{stdout: "Hello\n", exitCode: 0}, nil
	}

	if err := runMengTestFile(nil, testFile); err == nil {
		t.Fatal("expected runMengTestFile to fail")
	}
}

func TestRunMengTestFileFailsWhenExitCodeDiffers(t *testing.T) {
	tempDir := t.TempDir()
	srcDir := filepath.Join(tempDir, "src")
	testsDir := filepath.Join(tempDir, "tests")

	if err := os.MkdirAll(srcDir, 0755); err != nil {
		t.Fatalf("mkdir src: %v", err)
	}
	if err := os.MkdirAll(testsDir, 0755); err != nil {
		t.Fatalf("mkdir tests: %v", err)
	}

	target := filepath.Join(srcDir, "main.tai")
	testFile := filepath.Join(testsDir, "main_test.meng")

	if err := os.WriteFile(target, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write target: %v", err)
	}
	if err := os.WriteFile(testFile, []byte("测试 退出码:\n  期望 退出码 3\n"), 0644); err != nil {
		t.Fatalf("write test: %v", err)
	}

	originalBuild := executeMengTestBuild
	originalExec := runMengTestExecutable
	t.Cleanup(func() {
		executeMengTestBuild = originalBuild
		runMengTestExecutable = originalExec
	})
	executeMengTestBuild = func(request buildRequest) error { return nil }
	runMengTestExecutable = func(path string) (compiledProgramResult, error) {
		return compiledProgramResult{stdout: "", exitCode: 1}, nil
	}

	err := runMengTestFile(nil, testFile)
	if err == nil || !strings.Contains(err.Error(), "expected exit code 3, got 1") {
		t.Fatalf("expected exit-code mismatch, got %v", err)
	}
}

func TestResolveTargetSourceFilePrefersTai(t *testing.T) {
	tempDir := t.TempDir()
	srcDir := filepath.Join(tempDir, "src")
	testsDir := filepath.Join(tempDir, "tests")

	if err := os.MkdirAll(srcDir, 0755); err != nil {
		t.Fatalf("mkdir src: %v", err)
	}
	if err := os.MkdirAll(testsDir, 0755); err != nil {
		t.Fatalf("mkdir tests: %v", err)
	}

	taiTarget := filepath.Join(srcDir, "main.tai")
	mengTarget := filepath.Join(srcDir, "main.meng")
	testFile := filepath.Join(testsDir, "main_test.meng")
	if err := os.WriteFile(taiTarget, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write tai target: %v", err)
	}
	if err := os.WriteFile(mengTarget, []byte("需求"), 0644); err != nil {
		t.Fatalf("write meng target: %v", err)
	}
	if err := os.WriteFile(testFile, []byte("测试"), 0644); err != nil {
		t.Fatalf("write test file: %v", err)
	}

	resolved, err := resolveTargetSourceFile(testFile)
	if err != nil {
		t.Fatalf("resolveTargetSourceFile returned error: %v", err)
	}
	if resolved != taiTarget {
		t.Fatalf("expected .tai target %s, got %s", taiTarget, resolved)
	}
}

func TestExecuteCompiledProgramForTestPreservesNonZeroExitCode(t *testing.T) {
	tempDir := t.TempDir()
	scriptPath := filepath.Join(tempDir, "fail.cmd")
	if err := os.WriteFile(scriptPath, []byte("@echo off\r\necho boom\r\nexit /b 7\r\n"), 0644); err != nil {
		t.Fatalf("write script: %v", err)
	}

	result, err := executeCompiledProgramForTest(scriptPath)
	if err != nil {
		t.Fatalf("executeCompiledProgramForTest returned error: %v", err)
	}
	if result.exitCode != 7 {
		t.Fatalf("expected exit code 7, got %d", result.exitCode)
	}
	if result.stdout != "boom\n" {
		t.Fatalf("expected stdout boom, got %q", result.stdout)
	}
}

func TestExecuteCompiledProgramForTestReturnsLaunchErrors(t *testing.T) {
	_, err := executeCompiledProgramForTest(filepath.Join(t.TempDir(), "missing.exe"))
	if err == nil {
		t.Fatal("expected launch error for missing executable")
	}
	var pathErr *os.PathError
	if !errors.As(err, &pathErr) {
		t.Fatalf("expected path error, got %T", err)
	}
}
