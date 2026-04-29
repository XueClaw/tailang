package cmd

import (
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strconv"
	"strings"

	"github.com/spf13/cobra"
)

var testCmd = &cobra.Command{
	Use:   "test [path]",
	Short: "Run tests",
	Long: `Run Tailang tests.

Examples:
  meng test              # Run all tests
  meng test tests/       # Run tests in directory
  meng test tests/foo.meng  # Run specific test file
  meng test tests/ --backend llvm --opt-level 2  # Run tests through LLVM`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		testPath := "."
		if len(args) > 0 {
			testPath = args[0]
		}

		fmt.Println("🧪 Running tests...")
		fmt.Println()

		var testFiles []string
		filepath.Walk(testPath, func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if strings.HasSuffix(path, "_test.meng") || strings.HasSuffix(path, ".test.meng") {
				testFiles = append(testFiles, path)
			}
			return nil
		})

		if len(testFiles) == 0 {
			fmt.Println("No test files found")
			return nil
		}

		sort.Strings(testFiles)
		fmt.Printf("Found %d test file(s):\n\n", len(testFiles))

		passed := 0
		failed := 0
		skipped := 0

		for _, testFile := range testFiles {
			fmt.Printf("  Running %s... ", testFile)

			if err := runMengTestFile(cmd, testFile); err != nil {
				var skipErr mengTestSkipError
				if errors.As(err, &skipErr) {
					fmt.Printf("↷ SKIPPED (%s)\n", skipErr.reason)
					skipped++
					continue
				}
				fmt.Printf("✗ FAILED (%s)\n", err)
				failed++
			} else {
				fmt.Println("✓ PASSED")
				passed++
			}
		}

		fmt.Printf("\n✅ Tests complete: %d passed, %d failed, %d skipped\n", passed, failed, skipped)

		if failed > 0 {
			os.Exit(1)
		}

		return nil
	},
}

var executeMengTestBuild = executeBuild
var runMengTestExecutable = executeCompiledProgramForTest
var executeMengTestBuildForExpectedFailure = func(request buildRequest) error {
	originalCompile := compileToExecutableForBuild
	compileToExecutableForBuild = func(ir *IR, outputName string, target string, backend string, optLevel string) error {
		return compileToExecutableWithWriters(ir, outputName, target, backend, optLevel, os.Stdout, io.Discard)
	}
	defer func() {
		compileToExecutableForBuild = originalCompile
	}()
	return executeBuild(request)
}

var expectedOutputPattern = regexp.MustCompile(`期望\s+输出\s+"([^"]*)"`)
var expectedExitCodePattern = regexp.MustCompile(`期望\s+退出码\s+(-?\d+)`)
var expectedBuildFailurePattern = regexp.MustCompile(`期望\s+构建失败\s+"([^"]+)"`)
var requiredBackendPattern = regexp.MustCompile(`需要\s+后端\s+([A-Za-z0-9_-]+)`)

type mengTestExpectations struct {
	expectedOutputs      []string
	expectedExit         *int
	expectedBuildFailure string
	requiredBackend      string
}

type mengTestSkipError struct {
	reason string
}

func (e mengTestSkipError) Error() string {
	return e.reason
}

type compiledProgramResult struct {
	stdout   string
	stderr   string
	exitCode int
}

func runMengTestFile(cmd *cobra.Command, testFile string) error {
	content, err := os.ReadFile(testFile)
	if err != nil {
		return fmt.Errorf("read test file: %w", err)
	}

	expectations, err := parseMengTestExpectations(string(content))
	if err != nil {
		return err
	}
	if len(expectations.expectedOutputs) == 0 && expectations.expectedExit == nil && expectations.expectedBuildFailure == "" {
		return fmt.Errorf("no supported assertions found")
	}
	if expectations.requiredBackend != "" && expectations.requiredBackend != commandBackendForTests(cmd) {
		return mengTestSkipError{reason: fmt.Sprintf("requires backend %s", expectations.requiredBackend)}
	}

	targetFile, err := resolveTargetSourceFile(testFile)
	if err != nil {
		return err
	}

	tempDir, err := os.MkdirTemp("", "tailang-test-*")
	if err != nil {
		return fmt.Errorf("create temp test directory: %w", err)
	}
	defer os.RemoveAll(tempDir)

	outputPath := filepath.Join(tempDir, defaultOutputName(targetFile, "windows"))
	request, err := newBuildRequest(
		targetFile,
		outputPath,
		commandTargetForTests(),
		commandBackendForTests(cmd),
		commandOptLevelForTests(cmd),
	)
	if err != nil {
		return err
	}

	if expectations.expectedBuildFailure != "" {
		buildErr := executeMengTestBuildForExpectedFailure(request)
		if buildErr == nil {
			return fmt.Errorf("expected build failure containing %q, got success", expectations.expectedBuildFailure)
		}
		if !strings.Contains(buildErr.Error(), expectations.expectedBuildFailure) {
			return fmt.Errorf("expected build failure containing %q, got %q", expectations.expectedBuildFailure, buildErr.Error())
		}
		return nil
	}
	buildErr := executeMengTestBuild(request)
	if buildErr != nil {
		return fmt.Errorf("build target: %w", buildErr)
	}

	result, err := runMengTestExecutable(request.outputName)
	if err != nil {
		return err
	}

	if err := assertExpectedOutputs(expectations.expectedOutputs, result.stdout); err != nil {
		return err
	}
	if expectations.expectedExit != nil && result.exitCode != *expectations.expectedExit {
		return fmt.Errorf("expected exit code %d, got %d", *expectations.expectedExit, result.exitCode)
	}

	return nil
}

func parseMengTestExpectations(content string) (mengTestExpectations, error) {
	expectations := mengTestExpectations{
		expectedOutputs: parseExpectedOutputs(content),
	}

	exitMatches := expectedExitCodePattern.FindAllStringSubmatch(content, -1)
	if len(exitMatches) > 0 {
		value := exitMatches[len(exitMatches)-1][1]
		exitCode, err := strconv.Atoi(value)
		if err != nil {
			return mengTestExpectations{}, fmt.Errorf("invalid expected exit code %q: %w", value, err)
		}
		expectations.expectedExit = &exitCode
	}

	buildFailureMatches := expectedBuildFailurePattern.FindAllStringSubmatch(content, -1)
	if len(buildFailureMatches) > 0 {
		expectations.expectedBuildFailure = strings.TrimSpace(buildFailureMatches[len(buildFailureMatches)-1][1])
	}

	requiredBackendMatches := requiredBackendPattern.FindAllStringSubmatch(content, -1)
	if len(requiredBackendMatches) > 0 {
		expectations.requiredBackend = strings.TrimSpace(requiredBackendMatches[len(requiredBackendMatches)-1][1])
	}

	return expectations, nil
}

func commandTargetForTests() string {
	if runtime.GOOS == "windows" {
		return "windows"
	}
	return runtime.GOOS
}

func commandBackendForTests(cmd *cobra.Command) string {
	if cmd == nil {
		return "self-native"
	}
	return commandBackend(cmd)
}

func commandOptLevelForTests(cmd *cobra.Command) string {
	if cmd == nil {
		return "1"
	}
	return commandOptLevel(cmd)
}

func executeCompiledProgramForTest(programPath string) (compiledProgramResult, error) {
	cmd := exec.Command(programPath)
	output, err := cmd.CombinedOutput()
	exitCode := 0
	if cmd.ProcessState != nil {
		exitCode = cmd.ProcessState.ExitCode()
	}
	if err != nil {
		var exitErr *exec.ExitError
		if !errors.As(err, &exitErr) {
			return compiledProgramResult{}, fmt.Errorf("run compiled test target: %w", err)
		}
	}

	return compiledProgramResult{
		stdout:   normalizeOutput(string(output)),
		stderr:   "",
		exitCode: exitCode,
	}, nil
}

func assertExpectedOutputs(expectedOutputs []string, stdout string) error {
	if len(expectedOutputs) == 0 {
		return nil
	}

	actualLines := nonEmptyLines(stdout)
	searchFrom := 0
	for _, expected := range expectedOutputs {
		found := false
		for i := searchFrom; i < len(actualLines); i++ {
			if actualLines[i] == expected {
				searchFrom = i + 1
				found = true
				break
			}
		}
		if !found {
			return fmt.Errorf("expected output line %q not found in program stdout %q", expected, stdout)
		}
	}
	return nil
}

func normalizeOutput(output string) string {
	return strings.ReplaceAll(output, "\r\n", "\n")
}

func nonEmptyLines(output string) []string {
	normalized := normalizeOutput(output)
	lines := strings.Split(normalized, "\n")
	result := make([]string, 0, len(lines))
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" {
			result = append(result, trimmed)
		}
	}
	return result
}

func parseExpectedOutputs(content string) []string {
	matches := expectedOutputPattern.FindAllStringSubmatch(content, -1)
	outputs := make([]string, 0, len(matches))
	for _, match := range matches {
		if len(match) > 1 {
			outputs = append(outputs, match[1])
		}
	}
	return outputs
}

func resolveTargetSourceFile(testFile string) (string, error) {
	base := filepath.Base(testFile)
	dir := filepath.Dir(testFile)

	candidates := []string{
		strings.TrimSuffix(base, "_test.meng") + ".tai",
		strings.TrimSuffix(base, ".test.meng") + ".tai",
		strings.TrimSuffix(base, "_test.meng") + ".meng",
		strings.TrimSuffix(base, ".test.meng") + ".meng",
	}

	searchRoots := []string{
		dir,
		filepath.Join(filepath.Dir(dir), "src"),
		filepath.Dir(dir),
	}

	for _, root := range searchRoots {
		for _, candidate := range candidates {
			path := filepath.Join(root, candidate)
			if _, err := os.Stat(path); err == nil {
				return path, nil
			}
		}
	}

	return "", fmt.Errorf("unable to resolve target .tai or .meng file")
}

func init() {
	rootCmd.AddCommand(testCmd)
	testCmd.Flags().String("backend", "self-native", "Compiler backend for test builds (self-native, llvm)")
	testCmd.Flags().String("opt-level", "1", "Optimization level for test builds (0, 1, 2)")
}
