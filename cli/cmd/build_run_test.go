package cmd

import (
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/spf13/cobra"
)

func newCommandForTest(base *cobra.Command) *cobra.Command {
	cmd := &cobra.Command{Use: base.Use}
	cmd.Flags().StringP("output", "o", "", "Output filename")
	cmd.Flags().String("target", "", "Target platform (windows, macos, linux)")
	cmd.Flags().String("backend", "self-native", "Compiler backend (self-native, llvm)")
	cmd.Flags().String("opt-level", "1", "Optimization level (0, 1, 2)")
	return cmd
}

func newBenchCommandForTest() *cobra.Command {
	cmd := &cobra.Command{Use: benchCmd.Use}
	addBenchFlags(cmd)
	return cmd
}

func newRunCommandForTest() *cobra.Command {
	cmd := newCommandForTest(runCmd)
	cmd.Flags().String("args", "", "Additional arguments to pass to the program")
	cmd.RunE = runCmd.RunE
	return cmd
}

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

 .子程序 登录(邮箱: 文本型) -> 文本型, , ,
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

func TestNewBuildRequestFromCommandUsesDefaults(t *testing.T) {
	tempDir := t.TempDir()
	inputPath := filepath.Join(tempDir, "main.tai")
	if err := os.WriteFile(inputPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write input: %v", err)
	}

	cmd := newCommandForTest(buildCmd)
	request, err := newBuildRequestFromCommand(cmd, inputPath)
	if err != nil {
		t.Fatalf("newBuildRequestFromCommand returned error: %v", err)
	}

	if request.inputFile != inputPath {
		t.Fatalf("unexpected input file: %s", request.inputFile)
	}
	if request.outputName != "main.exe" {
		t.Fatalf("unexpected default output: %s", request.outputName)
	}
	if request.backend != "self-native" {
		t.Fatalf("unexpected default backend: %s", request.backend)
	}
	if request.optLevel != "1" {
		t.Fatalf("unexpected default opt level: %s", request.optLevel)
	}
}

func TestNewBuildRequestFromCommandHonorsRunFlags(t *testing.T) {
	tempDir := t.TempDir()
	inputPath := filepath.Join(tempDir, "main.tai")
	if err := os.WriteFile(inputPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write input: %v", err)
	}

	cmd := newCommandForTest(runCmd)
	if err := cmd.Flags().Set("output", "custom.exe"); err != nil {
		t.Fatalf("set output flag: %v", err)
	}
	if err := cmd.Flags().Set("target", "windows"); err != nil {
		t.Fatalf("set target flag: %v", err)
	}
	if err := cmd.Flags().Set("backend", "llvm"); err != nil {
		t.Fatalf("set backend flag: %v", err)
	}
	if err := cmd.Flags().Set("opt-level", "2"); err != nil {
		t.Fatalf("set opt-level flag: %v", err)
	}

	request, err := newBuildRequestFromCommand(cmd, inputPath)
	if err != nil {
		t.Fatalf("newBuildRequestFromCommand returned error: %v", err)
	}

	if request.outputName != "custom.exe" {
		t.Fatalf("expected output flag to be preserved, got %s", request.outputName)
	}
	if request.target != "windows" {
		t.Fatalf("expected target flag to be preserved, got %s", request.target)
	}
	if request.backend != "llvm" {
		t.Fatalf("expected backend flag to be preserved, got %s", request.backend)
	}
	if request.optLevel != "2" {
		t.Fatalf("expected opt-level flag to be preserved, got %s", request.optLevel)
	}
}

func TestNewBuildRequestAllowsExplicitWindowsTarget(t *testing.T) {
	tempDir := t.TempDir()
	inputPath := filepath.Join(tempDir, "bench_numeric.tai")
	if err := os.WriteFile(inputPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write input: %v", err)
	}

	request, err := newBuildRequest(inputPath, "", "windows", "llvm", "2")
	if err != nil {
		t.Fatalf("newBuildRequest returned error: %v", err)
	}

	if request.outputName != "bench_numeric.exe" {
		t.Fatalf("unexpected default output for windows target: %s", request.outputName)
	}
	if request.target != "windows" {
		t.Fatalf("unexpected target: %s", request.target)
	}
	if request.backend != "llvm" {
		t.Fatalf("unexpected backend: %s", request.backend)
	}
	if request.optLevel != "2" {
		t.Fatalf("unexpected opt level: %s", request.optLevel)
	}
}

func TestResolveBenchTargetFallsBackToCliDirectory(t *testing.T) {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("get working directory: %v", err)
	}
	tempDir := t.TempDir()
	if err := os.Chdir(tempDir); err != nil {
		t.Fatalf("chdir temp dir: %v", err)
	}
	t.Cleanup(func() {
		_ = os.Chdir(wd)
	})

	cliDir := filepath.Join(tempDir, "cli")
	if err := os.MkdirAll(cliDir, 0755); err != nil {
		t.Fatalf("mkdir cli: %v", err)
	}
	expectedPath := filepath.Join(cliDir, defaultBenchTarget())
	if err := os.WriteFile(expectedPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write benchmark target: %v", err)
	}

	resolved, err := resolveBenchTarget(defaultBenchTarget())
	if err != nil {
		t.Fatalf("resolveBenchTarget returned error: %v", err)
	}
	resolvedAbs, err := filepath.Abs(resolved)
	if err != nil {
		t.Fatalf("resolve resolved path: %v", err)
	}
	if resolvedAbs != expectedPath {
		t.Fatalf("expected fallback path %s, got %s", expectedPath, resolvedAbs)
	}
}

func TestBenchFlagsFlowIntoSharedBuildRequest(t *testing.T) {
	tempDir := t.TempDir()
	inputPath := filepath.Join(tempDir, "bench_numeric.tai")
	if err := os.WriteFile(inputPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write input: %v", err)
	}

	cmd := newBenchCommandForTest()
	if err := cmd.Flags().Set("output", "bench-custom.exe"); err != nil {
		t.Fatalf("set output flag: %v", err)
	}
	if err := cmd.Flags().Set("backend", "llvm"); err != nil {
		t.Fatalf("set backend flag: %v", err)
	}
	if err := cmd.Flags().Set("opt-level", "2"); err != nil {
		t.Fatalf("set opt-level flag: %v", err)
	}

	request, err := newBuildRequest(
		inputPath,
		commandOutputName(cmd),
		"windows",
		commandBackend(cmd),
		commandOptLevel(cmd),
	)
	if err != nil {
		t.Fatalf("newBuildRequest returned error: %v", err)
	}

	if request.inputFile != inputPath {
		t.Fatalf("unexpected input file: %s", request.inputFile)
	}
	if request.outputName != "bench-custom.exe" {
		t.Fatalf("expected explicit output to be preserved, got %s", request.outputName)
	}
	if request.backend != "llvm" {
		t.Fatalf("expected backend to be preserved, got %s", request.backend)
	}
	if request.optLevel != "2" {
		t.Fatalf("expected opt-level to be preserved, got %s", request.optLevel)
	}
	if request.target != "windows" {
		t.Fatalf("expected benchmark target platform to remain windows, got %s", request.target)
	}
}

func TestBenchCommandRoutesThroughSharedBuildExecutor(t *testing.T) {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("get working directory: %v", err)
	}
	tempDir := t.TempDir()
	if err := os.Chdir(tempDir); err != nil {
		t.Fatalf("chdir temp dir: %v", err)
	}
	t.Cleanup(func() {
		_ = os.Chdir(wd)
	})

	cliDir := filepath.Join(tempDir, "cli")
	if err := os.MkdirAll(cliDir, 0755); err != nil {
		t.Fatalf("mkdir cli: %v", err)
	}
	targetPath := filepath.Join(cliDir, defaultBenchTarget())
	pythonPath := filepath.Join(cliDir, "bench_numeric.py")
	if err := os.WriteFile(targetPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write target: %v", err)
	}
	if err := os.WriteFile(pythonPath, []byte("print(1000000)\n"), 0644); err != nil {
		t.Fatalf("write python baseline: %v", err)
	}

	original := executeBuildFunc
	originalRun := runBenchmarkCommand
	t.Cleanup(func() {
		executeBuildFunc = original
		runBenchmarkCommand = originalRun
	})

	var captured buildRequest
	executeBuildFunc = func(request buildRequest) error {
		captured = request
		return nil
	}
	runBenchmarkCommand = func(name string, args ...string) ([]byte, time.Duration, error) {
		return []byte("1000000\n"), 2 * time.Millisecond, nil
	}

	cmd := newBenchCommandForTest()
	cmd.RunE = benchCmd.RunE
	if err := cmd.Flags().Set("output", "bench-out.exe"); err != nil {
		t.Fatalf("set output: %v", err)
	}
	if err := cmd.Flags().Set("backend", "llvm"); err != nil {
		t.Fatalf("set backend: %v", err)
	}
	if err := cmd.Flags().Set("opt-level", "2"); err != nil {
		t.Fatalf("set opt-level: %v", err)
	}

	if err := cmd.RunE(cmd, nil); err != nil {
		t.Fatalf("bench command failed: %v", err)
	}
	if captured.inputFile != filepath.Join("cli", defaultBenchTarget()) {
		t.Fatalf("unexpected captured input path: %s", captured.inputFile)
	}
	if captured.outputName != "bench-out.exe" {
		t.Fatalf("unexpected captured output: %s", captured.outputName)
	}
	if captured.backend != "llvm" {
		t.Fatalf("unexpected captured backend: %s", captured.backend)
	}
	if captured.optLevel != "2" {
		t.Fatalf("unexpected captured opt-level: %s", captured.optLevel)
	}
	if captured.target != "windows" {
		t.Fatalf("unexpected captured target: %s", captured.target)
	}
}

func TestResolvePythonBenchmarkTargetFallsBackToSiblingPythonFile(t *testing.T) {
	tempDir := t.TempDir()
	targetPath := filepath.Join(tempDir, "bench_numeric.tai")
	pythonPath := filepath.Join(tempDir, "bench_numeric.py")
	if err := os.WriteFile(targetPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write tai target: %v", err)
	}
	if err := os.WriteFile(pythonPath, []byte("print(1)\n"), 0644); err != nil {
		t.Fatalf("write python target: %v", err)
	}

	resolved, err := resolvePythonBenchmarkTarget(targetPath, "")
	if err != nil {
		t.Fatalf("resolvePythonBenchmarkTarget returned error: %v", err)
	}
	if resolved != pythonPath {
		t.Fatalf("expected %s, got %s", pythonPath, resolved)
	}
}

func TestWriteBenchmarkReportCreatesJsonFile(t *testing.T) {
	tempDir := t.TempDir()
	reportPath := filepath.Join(tempDir, "reports", "bench.json")
	report := benchmarkReport{
		Target:        "cli\\bench_numeric.tai",
		Output:        "bench_numeric.exe",
		Backend:       "llvm",
		OptLevel:      "2",
		PythonCommand: "python",
		PythonScript:  "cli\\bench_numeric.py",
		Native: benchmarkSummary{
			Name:       "tailang-native",
			Command:    "bench_numeric.exe",
			Iterations: 3,
			BestMillis: 1.1,
			AvgMillis:  1.3,
			LastStdout: "1000000",
		},
		Python: benchmarkSummary{
			Name:       "python-baseline",
			Command:    "python cli\\bench_numeric.py",
			Iterations: 3,
			BestMillis: 2.4,
			AvgMillis:  2.8,
			LastStdout: "1000000",
		},
		Speedup:     2.15,
		GeneratedAt: "2026-04-20T12:00:00Z",
	}

	if err := writeBenchmarkReport(reportPath, report); err != nil {
		t.Fatalf("writeBenchmarkReport returned error: %v", err)
	}

	payload, err := os.ReadFile(reportPath)
	if err != nil {
		t.Fatalf("read report: %v", err)
	}

	var decoded benchmarkReport
	if err := json.Unmarshal(payload, &decoded); err != nil {
		t.Fatalf("unmarshal report: %v", err)
	}
	if decoded.Speedup != report.Speedup {
		t.Fatalf("expected speedup %.2f, got %.2f", report.Speedup, decoded.Speedup)
	}
	if decoded.Native.LastStdout != "1000000" {
		t.Fatalf("unexpected native stdout: %s", decoded.Native.LastStdout)
	}
}

func TestBenchCommandBuildsRunsAndWritesReport(t *testing.T) {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("get working directory: %v", err)
	}
	tempDir := t.TempDir()
	if err := os.Chdir(tempDir); err != nil {
		t.Fatalf("chdir temp dir: %v", err)
	}
	t.Cleanup(func() {
		_ = os.Chdir(wd)
	})

	cliDir := filepath.Join(tempDir, "cli")
	if err := os.MkdirAll(cliDir, 0755); err != nil {
		t.Fatalf("mkdir cli: %v", err)
	}
	taiPath := filepath.Join(cliDir, defaultBenchTarget())
	pythonPath := filepath.Join(cliDir, "bench_numeric.py")
	if err := os.WriteFile(taiPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write tai target: %v", err)
	}
	if err := os.WriteFile(pythonPath, []byte("print(1000000)\n"), 0644); err != nil {
		t.Fatalf("write python target: %v", err)
	}

	originalBuild := executeBuildFunc
	originalRun := runBenchmarkCommand
	t.Cleanup(func() {
		executeBuildFunc = originalBuild
		runBenchmarkCommand = originalRun
	})

	var buildCaptured buildRequest
	executeBuildFunc = func(request buildRequest) error {
		buildCaptured = request
		return nil
	}

	var commands []string
	runBenchmarkCommand = func(name string, args ...string) ([]byte, time.Duration, error) {
		commands = append(commands, strings.Join(append([]string{name}, args...), " "))
		if len(args) == 0 {
			return []byte("1000000\n"), 2 * time.Millisecond, nil
		}
		return []byte("1000000\n"), 4 * time.Millisecond, nil
	}

	reportPath := filepath.Join(tempDir, "out", "bench-report.json")
	cmd := newBenchCommandForTest()
	cmd.RunE = benchCmd.RunE
	if err := cmd.Flags().Set("output", "bench-out.exe"); err != nil {
		t.Fatalf("set output: %v", err)
	}
	if err := cmd.Flags().Set("backend", "llvm"); err != nil {
		t.Fatalf("set backend: %v", err)
	}
	if err := cmd.Flags().Set("opt-level", "2"); err != nil {
		t.Fatalf("set opt-level: %v", err)
	}
	if err := cmd.Flags().Set("iterations", "2"); err != nil {
		t.Fatalf("set iterations: %v", err)
	}
	if err := cmd.Flags().Set("report", reportPath); err != nil {
		t.Fatalf("set report: %v", err)
	}

	if err := cmd.RunE(cmd, nil); err != nil {
		t.Fatalf("bench command failed: %v", err)
	}

	if buildCaptured.outputName != "bench-out.exe" {
		t.Fatalf("unexpected build output: %s", buildCaptured.outputName)
	}
	if len(commands) != 4 {
		t.Fatalf("expected 4 benchmark runs, got %d", len(commands))
	}
	expectedNative := executableCommandPath("bench-out.exe")
	if commands[0] != expectedNative {
		t.Fatalf("unexpected native command: %s", commands[0])
	}
	if !strings.HasPrefix(commands[2], "python ") {
		t.Fatalf("unexpected python command: %s", commands[2])
	}
	if _, err := os.Stat(reportPath); err != nil {
		t.Fatalf("expected report file to be written: %v", err)
	}
}

func TestRunCommandRoutesThroughSharedBuildAndExec(t *testing.T) {
	tempDir := t.TempDir()
	inputPath := filepath.Join(tempDir, "main.tai")
	if err := os.WriteFile(inputPath, []byte(".版本 3"), 0644); err != nil {
		t.Fatalf("write input: %v", err)
	}

	originalBuild := executeBuildFunc
	originalExec := execCommand
	originalRunExec := runExecCommand
	t.Cleanup(func() {
		executeBuildFunc = originalBuild
		execCommand = originalExec
		runExecCommand = originalRunExec
	})

	var captured buildRequest
	executeBuildFunc = func(request buildRequest) error {
		captured = request
		return nil
	}

	var execName string
	var executedArgs []string
	execCommand = func(name string, args ...string) *exec.Cmd {
		execName = name
		if runtime.GOOS == "windows" {
			return exec.Command("cmd", "/c", "exit", "0")
		}
		return exec.Command("sh", "-c", "exit 0")
	}
	runExecCommand = func(cmd *exec.Cmd) error {
		executedArgs = append([]string{}, cmd.Args[1:]...)
		return nil
	}

	cmd := newRunCommandForTest()
	if err := cmd.Flags().Set("output", "run-out.exe"); err != nil {
		t.Fatalf("set output: %v", err)
	}
	if err := cmd.Flags().Set("target", "windows"); err != nil {
		t.Fatalf("set target: %v", err)
	}
	if err := cmd.Flags().Set("backend", "llvm"); err != nil {
		t.Fatalf("set backend: %v", err)
	}
	if err := cmd.Flags().Set("opt-level", "2"); err != nil {
		t.Fatalf("set opt-level: %v", err)
	}
	if err := cmd.Flags().Set("args", "alpha beta"); err != nil {
		t.Fatalf("set args: %v", err)
	}

	if err := cmd.RunE(cmd, []string{inputPath}); err != nil {
		t.Fatalf("run command failed: %v", err)
	}
	if captured.inputFile != inputPath {
		t.Fatalf("unexpected build input: %s", captured.inputFile)
	}
	if captured.outputName != "run-out.exe" {
		t.Fatalf("unexpected build output: %s", captured.outputName)
	}
	if captured.backend != "llvm" {
		t.Fatalf("unexpected build backend: %s", captured.backend)
	}
	if captured.optLevel != "2" {
		t.Fatalf("unexpected build opt-level: %s", captured.optLevel)
	}
	if execName != "run-out.exe" {
		t.Fatalf("unexpected exec name: %s", execName)
	}
	if len(executedArgs) < 2 {
		t.Fatalf("unexpected exec args: %v", executedArgs)
	}
	if got := strings.Join(executedArgs[len(executedArgs)-2:], " "); got != "alpha beta" {
		t.Fatalf("unexpected exec args: %s", got)
	}
}

func TestExecutableCommandPathPrefixesCurrentDirectory(t *testing.T) {
	got := executableCommandPath("bench_numeric.exe")
	if runtime.GOOS == "windows" {
		if got != ".\\bench_numeric.exe" {
			t.Fatalf("unexpected windows exec path: %s", got)
		}
		return
	}
	if got != "./bench_numeric.exe" {
		t.Fatalf("unexpected posix exec path: %s", got)
	}
}

func TestCompileToExecutableFromTaiInputProducesExecutable(t *testing.T) {
	tempDir := t.TempDir()
	inputPath := filepath.Join(tempDir, "main.tai")
	outputPath := filepath.Join(tempDir, "main.exe")

	content := `.版本 3
.程序集 演示

.子程序 主程序() -> 整数型, , ,
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
.子程序 主程序() -> 整数型, , ,
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
.子程序 主程序() -> 整数型, , ,
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
.子程序 打招呼() -> 空, , ,
.显示 "hi"
.返回

.子程序 主程序() -> 整数型, , ,
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

func TestCompileToExecutableSupportsLlvmRuntimeArrayFlow(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "llvm_array.exe")
	ir := &IR{
		Source: `.version 3
.module demo
.subprogram main() -> int, , ,
values: int[] = [3, 5, 8]
.print values[1]
.return values[2]`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "llvm", "1"); err != nil {
		t.Fatalf("expected llvm backend to compile runtime array flow, got %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 8 {
		t.Fatalf("expected exit code 8, got %d", result.exitCode)
	}
	if result.stdout != "5\n" {
		t.Fatalf("expected runtime array output 5, got %q", result.stdout)
	}
}

func TestCompileToExecutableSupportsRuntimeArrayOnSelfNative(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "self_native_array.exe")
	ir := &IR{
		Source: `.version 3
.module demo
.subprogram main() -> int, , ,
values: int[] = [3, 5, 8]
.print values[1]
.return values[2]`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "self-native", "1"); err != nil {
		t.Fatalf("expected self-native runtime array compile to succeed, got %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 8 {
		t.Fatalf("expected exit code 8, got %d", result.exitCode)
	}
	if result.stdout != "5\n" {
		t.Fatalf("expected runtime array output 5, got %q", result.stdout)
	}
}

func TestCompileToExecutableSupportsLlvmRuntimeObjectFlow(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "llvm_object.exe")
	ir := &IR{
		Source: `.version 3
.module demo
.subprogram main() -> int, , ,
data: object = {"name": "Yui", "score": 8}
.print data["name"]
.return data.score`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "llvm", "1"); err != nil {
		t.Fatalf("expected llvm backend to compile runtime object flow, got %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 8 {
		t.Fatalf("expected exit code 8, got %d", result.exitCode)
	}
	if result.stdout != "Yui\n" {
		t.Fatalf("expected runtime object output Yui, got %q", result.stdout)
	}
}

func TestCompileToExecutableSupportsLlvmNestedRuntimeObjectFlow(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "llvm_nested_object.exe")
	ir := &IR{
		Source: `.version 3
.module demo
.subprogram main() -> int, , ,
data: object = {"profile": {"name": "Yui"}, "items": [{"score": 5}, {"score": 8}]}
.print data.profile.name
.return data.items[1].score`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "llvm", "1"); err != nil {
		t.Fatalf("expected llvm backend to compile nested runtime object flow, got %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 8 {
		t.Fatalf("expected exit code 8, got %d", result.exitCode)
	}
	if result.stdout != "Yui\n" {
		t.Fatalf("expected nested runtime object output Yui, got %q", result.stdout)
	}
}

func TestCompileToExecutableSupportsScalarRuntimeObjectOnSelfNative(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "self_native_object.exe")
	ir := &IR{
		Source: `.version 3
.module demo
.subprogram main() -> int, , ,
data: object = {"score": 8}
.print data.score
.return data.score`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "self-native", "1"); err != nil {
		t.Fatalf("expected self-native runtime object compile to succeed, got %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 8 {
		t.Fatalf("expected exit code 8, got %d", result.exitCode)
	}
	if result.stdout != "8\n" {
		t.Fatalf("expected runtime object output 8, got %q", result.stdout)
	}
}

func TestCompileToExecutableSupportsRuntimeObjectOnSelfNative(t *testing.T) {
	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "self_native_object.exe")
	ir := &IR{
		Source: `.version 3
.module demo
.subprogram main() -> int, , ,
data: object = {"name": "Yui", "score": 8}
.print data["name"]
.return data.score`,
	}

	if err := compileToExecutable(ir, outputPath, "windows", "self-native", "1"); err != nil {
		t.Fatalf("expected self-native runtime object compile to succeed, got %v", err)
	}
	result := runExecutable(t, outputPath)
	if result.exitCode != 8 {
		t.Fatalf("expected exit code 8, got %d", result.exitCode)
	}
	if result.stdout != "Yui\n" {
		t.Fatalf("expected runtime object output Yui, got %q", result.stdout)
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
