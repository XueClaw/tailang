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
	Short: "Compile and run .meng file",
	Long: `Compile a Tailang .meng file and execute it immediately.

This is a convenience command that combines:
  meng build file.meng
  ./file.exe

Examples:
  meng run src/main.meng
  meng run src/main.meng --args "arg1 arg2"`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		inputFile := args[0]
		
		// Validate input file exists
		if _, err := os.Stat(inputFile); os.IsNotExist(err) {
			return fmt.Errorf("file not found: %s", inputFile)
		}
		
		// Get additional arguments
		runArgs, _ := cmd.Flags().GetString("args")
		
		fmt.Printf("🚀 Running %s...\n\n", inputFile)
		
		// Step 1: Build
		fmt.Println("Step 1/2: Building...")
		outputName := strings.TrimSuffix(filepath.Base(inputFile), ".meng")
		if runtime.GOOS == "windows" {
			outputName = outputName + ".exe"
		}
		
		// Call build command
		buildCmd.SetArgs([]string{inputFile, "-o", outputName})
		if err := buildCmd.Execute(); err != nil {
			return fmt.Errorf("build failed: %w", err)
		}
		fmt.Println()
		
		// Step 2: Execute
		fmt.Println("Step 2/2: Executing...")
		fmt.Println(strings.Repeat("─", 40))
		
		// Prepare command
		var execCmd *exec.Cmd
		if runtime.GOOS == "windows" {
			execCmd = exec.Command(outputName)
		} else {
			execCmd = exec.Command("./" + outputName)
		}
		
		// Add additional arguments
		if runArgs != "" {
			execCmd.Args = append(execCmd.Args, strings.Split(runArgs, " ")...)
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
}
