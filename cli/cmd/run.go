package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/spf13/cobra"
)

var runCmd = &cobra.Command{
	Use:   "run [file.meng]",
	Short: "Compile and run .meng or .tai file",
	Long: `Compile a Tailang .meng or .tai file and execute it immediately.

This is a convenience command that combines:
  meng build file
  execute the generated artifact

Examples:
  meng run src/main.meng
  meng run src/main.tai
  meng run src/main.meng --args "arg1 arg2"`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		inputFile := args[0]

		buildRequest, err := newBuildRequestFromCommand(cmd, inputFile)
		if err != nil {
			return err
		}

		// Get additional arguments
		runArgs, _ := cmd.Flags().GetString("args")

		fmt.Printf("🚀 Running %s...\n\n", inputFile)

		// Step 1: Build
		fmt.Println("Step 1/2: Building...")
		if err := executeBuild(buildRequest); err != nil {
			return fmt.Errorf("build failed: %w", err)
		}
		fmt.Println()

		// Step 2: Execute
		fmt.Println("Step 2/2: Executing...")
		fmt.Println(strings.Repeat("─", 40))

		// Prepare command
		var execCmd *exec.Cmd
		if runtime.GOOS == "windows" {
			execCmd = exec.Command(buildRequest.outputName)
		} else {
			execCmd = exec.Command("./" + buildRequest.outputName)
		}

		// Add additional arguments
		if runArgs != "" {
			execCmd.Args = append(execCmd.Args, strings.Fields(runArgs)...)
		}

		// Set up I/O
		execCmd.Stdin = os.Stdin
		execCmd.Stdout = os.Stdout
		execCmd.Stderr = os.Stderr

		// Execute
		if err := execCmd.Run(); err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				fmt.Println()
				return fmt.Errorf("program exited with code: %d", exitErr.ExitCode())
			}
			return fmt.Errorf("execution failed: %w", err)
		}

		fmt.Println(strings.Repeat("─", 40))
		fmt.Println("\n✅ Execution complete!")

		return nil
	},
}

func init() {
	rootCmd.AddCommand(runCmd)
	runCmd.Flags().String("args", "", "Additional arguments to pass to the program")
	runCmd.Flags().StringP("output", "o", "", "Output filename")
	runCmd.Flags().String("target", "", "Target platform (windows, macos, linux)")
	runCmd.Flags().String("backend", "self-native", "Compiler backend (self-native, llvm)")
	runCmd.Flags().String("opt-level", "1", "Optimization level (0, 1, 2)")
}

func defaultOutputName(inputFile string, target string) string {
	base := strings.TrimSuffix(filepath.Base(inputFile), filepath.Ext(inputFile))
	switch target {
	case "windows":
		return base + ".exe"
	case "darwin":
		return base + ".app"
	default:
		return base
	}
}
