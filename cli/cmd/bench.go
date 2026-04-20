package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"github.com/spf13/cobra"
)

var executeBuildFunc = executeBuild
var runBenchmarkCommand = func(name string, args ...string) ([]byte, time.Duration, error) {
	start := time.Now()
	cmd := exec.Command(name, args...)
	output, err := cmd.CombinedOutput()
	return output, time.Since(start), err
}

type benchmarkSummary struct {
	Name       string  `json:"name"`
	Command    string  `json:"command"`
	Iterations int     `json:"iterations"`
	BestMillis float64 `json:"bestMillis"`
	AvgMillis  float64 `json:"avgMillis"`
	LastStdout string  `json:"lastStdout"`
}

type benchmarkReport struct {
	Target        string           `json:"target"`
	Output        string           `json:"output"`
	Backend       string           `json:"backend"`
	OptLevel      string           `json:"optLevel"`
	PythonCommand string           `json:"pythonCommand"`
	PythonScript  string           `json:"pythonScript"`
	Native        benchmarkSummary `json:"native"`
	Python        benchmarkSummary `json:"python"`
	Speedup       float64          `json:"speedup"`
	GeneratedAt   string           `json:"generatedAt"`
}

var benchCmd = &cobra.Command{
	Use:   "bench [file.tai]",
	Short: "Build and benchmark a Tailang target",
	Long: `Build and benchmark a Tailang target.

This command is the first step for native-vs-Python performance baselines.
It builds the requested .tai benchmark target, runs the native artifact,
runs a comparable Python baseline, and prints a timing summary.`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		target := defaultBenchTarget()
		if len(args) > 0 {
			target = args[0]
		}

		resolvedTarget, err := resolveBenchTarget(target)
		if err != nil {
			return err
		}
		request, err := newBuildRequest(
			resolvedTarget,
			commandOutputName(cmd),
			"windows",
			commandBackend(cmd),
			commandOptLevel(cmd),
		)
		if err != nil {
			return err
		}
		iterations, _ := cmd.Flags().GetInt("iterations")
		pythonCmd, _ := cmd.Flags().GetString("python-cmd")
		pythonBaseline, _ := cmd.Flags().GetString("python-baseline")
		reportPath, _ := cmd.Flags().GetString("report")

		if iterations <= 0 {
			return fmt.Errorf("iterations must be greater than 0")
		}

		pythonScript, err := resolvePythonBenchmarkTarget(resolvedTarget, pythonBaseline)
		if err != nil {
			return err
		}

		fmt.Printf("📈 Building benchmark target %s -> %s\n", request.inputFile, request.outputName)
		if err := executeBuildFunc(request); err != nil {
			return err
		}

		nativeSummary, err := runBenchmarkSuite("tailang-native", request.outputName, nil, iterations)
		if err != nil {
			return err
		}
		pythonSummary, err := runBenchmarkSuite("python-baseline", pythonCmd, []string{pythonScript}, iterations)
		if err != nil {
			return err
		}

		report := buildBenchmarkReport(request, pythonCmd, pythonScript, nativeSummary, pythonSummary)
		printBenchmarkReport(report)
		if reportPath != "" {
			if err := writeBenchmarkReport(reportPath, report); err != nil {
				return err
			}
			fmt.Printf("\n📝 Benchmark report written to: %s\n", reportPath)
		}
		return nil
	},
}

func init() {
	rootCmd.AddCommand(benchCmd)
	addBenchFlags(benchCmd)
}

func addBenchFlags(cmd *cobra.Command) {
	cmd.Flags().StringP("output", "o", "", "Output filename")
	cmd.Flags().String("backend", "self-native", "Compiler backend (self-native, llvm)")
	cmd.Flags().String("opt-level", "1", "Optimization level (0, 1, 2)")
	cmd.Flags().Int("iterations", 3, "Number of timing runs for each benchmark")
	cmd.Flags().String("python-cmd", "python", "Python interpreter command for the baseline run")
	cmd.Flags().String("python-baseline", "", "Optional explicit Python baseline script path")
	cmd.Flags().String("report", "", "Optional JSON report output path")
}

func defaultBenchTarget() string {
	return "bench_numeric.tai"
}

func resolveBenchTarget(target string) (string, error) {
	if _, err := os.Stat(target); err == nil {
		return target, nil
	}

	fallback := filepath.Join("cli", target)
	if _, err := os.Stat(fallback); err == nil {
		return fallback, nil
	}

	return "", fmt.Errorf("benchmark target not found: %s", target)
}

func resolvePythonBenchmarkTarget(target string, override string) (string, error) {
	if override != "" {
		if _, err := os.Stat(override); err == nil {
			return override, nil
		}
		return "", fmt.Errorf("python baseline not found: %s", override)
	}

	base := strings.TrimSuffix(filepath.Base(target), filepath.Ext(target)) + ".py"
	candidates := []string{
		filepath.Join(filepath.Dir(target), base),
		filepath.Join("cli", base),
		base,
	}
	for _, candidate := range candidates {
		if _, err := os.Stat(candidate); err == nil {
			return candidate, nil
		}
	}

	return "", fmt.Errorf("python baseline not found for benchmark target: %s", target)
}

func runBenchmarkSuite(name string, command string, args []string, iterations int) (benchmarkSummary, error) {
	total := 0.0
	best := -1.0
	lastStdout := ""
	resolvedCommand := command
	if len(args) == 0 {
		resolvedCommand = executableCommandPath(command)
	}
	for i := 0; i < iterations; i++ {
		output, duration, err := runBenchmarkCommand(resolvedCommand, args...)
		if err != nil {
			return benchmarkSummary{}, fmt.Errorf("%s run failed: %w", name, err)
		}
		millis := float64(duration.Microseconds()) / 1000.0
		total += millis
		if best < 0 || millis < best {
			best = millis
		}
		lastStdout = strings.ReplaceAll(strings.TrimSpace(string(output)), "\r\n", "\n")
	}

	return benchmarkSummary{
		Name:       name,
		Command:    strings.TrimSpace(strings.Join(append([]string{resolvedCommand}, args...), " ")),
		Iterations: iterations,
		BestMillis: best,
		AvgMillis:  total / float64(iterations),
		LastStdout: lastStdout,
	}, nil
}

func executableCommandPath(command string) string {
	if filepath.IsAbs(command) {
		return command
	}
	if strings.Contains(command, string(os.PathSeparator)) {
		return command
	}
	if runtime.GOOS == "windows" {
		return ".\\" + command
	}
	return "./" + command
}

func buildBenchmarkReport(
	request buildRequest,
	pythonCmd string,
	pythonScript string,
	native benchmarkSummary,
	python benchmarkSummary,
) benchmarkReport {
	speedup := 0.0
	if native.AvgMillis > 0 {
		speedup = python.AvgMillis / native.AvgMillis
	}

	return benchmarkReport{
		Target:        request.inputFile,
		Output:        request.outputName,
		Backend:       request.backend,
		OptLevel:      request.optLevel,
		PythonCommand: pythonCmd,
		PythonScript:  pythonScript,
		Native:        native,
		Python:        python,
		Speedup:       speedup,
		GeneratedAt:   time.Now().UTC().Format(time.RFC3339),
	}
}

func defaultBenchmarkReportPath(target string) string {
	base := strings.TrimSuffix(filepath.Base(target), filepath.Ext(target))
	return base + ".bench.json"
}

func printBenchmarkReport(report benchmarkReport) {
	fmt.Printf("\n📊 Benchmark summary\n")
	fmt.Printf("   Native avg: %.3f ms (best %.3f ms)\n", report.Native.AvgMillis, report.Native.BestMillis)
	fmt.Printf("   Python avg: %.3f ms (best %.3f ms)\n", report.Python.AvgMillis, report.Python.BestMillis)
	if report.Speedup > 0 {
		fmt.Printf("   Speedup: %.2fx (Python avg / native avg)\n", report.Speedup)
	}
	if report.Native.LastStdout != "" || report.Python.LastStdout != "" {
		fmt.Printf("   Native stdout: %s\n", report.Native.LastStdout)
		fmt.Printf("   Python stdout: %s\n", report.Python.LastStdout)
	}
}

func writeBenchmarkReport(path string, report benchmarkReport) error {
	reportPath, err := filepath.Abs(path)
	if err != nil {
		return fmt.Errorf("resolve report path failed: %w", err)
	}
	if err := os.MkdirAll(filepath.Dir(reportPath), 0755); err != nil {
		return fmt.Errorf("create report directory failed: %w", err)
	}
	payload, err := json.MarshalIndent(report, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal benchmark report failed: %w", err)
	}
	if err := os.WriteFile(reportPath, payload, 0644); err != nil {
		return fmt.Errorf("write benchmark report failed: %w", err)
	}
	return nil
}
