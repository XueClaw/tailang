package cmd

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

type executedBinary struct {
	stdout   string
	exitCode int
}

func runExecutable(t *testing.T, exePath string) executedBinary {
	t.Helper()
	cmd := exec.Command(exePath)
	output, err := cmd.CombinedOutput()
	exitCode := 0
	if cmd.ProcessState != nil {
		exitCode = cmd.ProcessState.ExitCode()
	}
	if err != nil && exitCode == 0 {
		t.Fatalf("run executable failed: %v", err)
	}
	return executedBinary{
		stdout:   strings.ReplaceAll(string(output), "\r\n", "\n"),
		exitCode: exitCode,
	}
}

func TestLoadNormalizedTaiAcceptsTaiInput(t *testing.T) {
	input := `.版本 3
.程序集 认证

.子程序 登录, 文本型
.参数 邮箱, 文本型
.返回 邮箱`

	out, err := loadNormalizedTai("main.tai", input)
	if err != nil {
		t.Fatalf("loadNormalizedTai returned error: %v", err)
	}

	if !strings.Contains(out, ".程序集 认证") {
		t.Fatalf("expected normalized .tai to preserve module content, got %s", out)
	}
}

func TestLoadNormalizedTaiRejectsUnsupportedExtension(t *testing.T) {
	if _, err := loadNormalizedTai("main.txt", "hello"); err == nil {
		t.Fatal("expected unsupported input extension to fail")
	}
}

func TestDefaultOutputNameSupportsMengAndTai(t *testing.T) {
	if got := defaultOutputName(filepath.Join("src", "main.meng"), "windows"); got != "main.exe" {
		t.Fatalf("unexpected output name for windows target: %s", got)
	}

	if got := defaultOutputName(filepath.Join("src", "main.tai"), "linux"); got != "main" {
		t.Fatalf("unexpected output name for linux target: %s", got)
	}
}

func TestCompileToExecutableFromTaiInputProducesExecutable(t *testing.T) {
	tempDir := t.TempDir()
	inputPath := filepath.Join(tempDir, "main.tai")
	outputPath := filepath.Join(tempDir, "main.exe")

	content := `.版本 3
.程序集 演示

.子程序 主程序, 整数型
.返回 0`

	if err := os.WriteFile(inputPath, []byte(content), 0644); err != nil {
		t.Fatalf("write tai input: %v", err)
	}

	source, err := os.ReadFile(inputPath)
	if err != nil {
		t.Fatalf("read tai input: %v", err)
	}

	normalized, err := loadNormalizedTai(inputPath, string(source))
	if err != nil {
		t.Fatalf("loadNormalizedTai returned error: %v", err)
	}

	blocks, err := extractCodeBlocksFromTai(normalized)
	if err != nil {
		t.Fatalf("extractCodeBlocksFromTai returned error: %v", err)
	}

	ir, err := generateIR(normalized, blocks)
	if err != nil {
		t.Fatalf("generateIR returned error: %v", err)
	}

	if err := compileToExecutable(ir, outputPath, "windows", "self-native", "1"); err != nil {
		t.Fatalf("compileToExecutable returned error: %v", err)
	}
	if _, err := os.Stat(outputPath); err != nil {
		t.Fatalf("expected native executable output, got stat error: %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", result.exitCode)
	}
}

func TestCompileToExecutableSupportsLlvmBackend(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "main.exe")
	ir := &IR{
		Source: `.版本 3
.程序集 演示
.子程序 主程序, 整数型
.返回 0`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "llvm", "1"); err != nil {
		t.Fatalf("expected llvm backend to compile, got %v", err)
	}
	if _, err := os.Stat(outputPath); err != nil {
		t.Fatalf("expected llvm executable output, got stat error: %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", result.exitCode)
	}
}

func TestCompileToExecutableSupportsLlvmBackendWithStdout(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "hello.exe")
	ir := &IR{
		Source: `.版本 3
.程序集 演示
.子程序 主程序, 整数型
.显示 "Hello World"
.返回 0`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "llvm", "1"); err != nil {
		t.Fatalf("expected llvm backend to compile hello world, got %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", result.exitCode)
	}
	if result.stdout != "Hello World\n" {
		t.Fatalf("expected hello world output, got %q", result.stdout)
	}
}

func TestCompileToExecutableSupportsLlvmVoidReturnFlow(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "void_flow.exe")
	ir := &IR{
		Source: `.版本 3
.程序集 演示
.子程序 打招呼, 空
.显示 "hi"
.返回

.子程序 主程序, 整数型
打招呼()
.返回 0`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "llvm", "1"); err != nil {
		t.Fatalf("expected llvm backend to compile void-return flow, got %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", result.exitCode)
	}
	if result.stdout != "hi\n" {
		t.Fatalf("expected void-return flow output, got %q", result.stdout)
	}
}

func TestExtractCodeBlocksFromTextualTai(t *testing.T) {
	input := `.版本 3
.程序集 演示

.子程序 主程序
.代码 Rust
println!("hello");
.代码结束`

	blocks, err := extractCodeBlocksFromTai(input)
	if err != nil {
		t.Fatalf("extractCodeBlocksFromTai returned error: %v", err)
	}
	if len(blocks) != 1 {
		t.Fatalf("expected 1 code block, got %d", len(blocks))
	}
	if blocks[0].Language != "Rust" {
		t.Fatalf("unexpected language: %s", blocks[0].Language)
	}
}
