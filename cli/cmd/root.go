package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "meng",
	Short: "Tailang CLI - .meng to .tai to native build orchestration",
	Long: `Tailang (太语言) - 道法自然，码由心生

A .tai-first programming system where .meng acts as engineering input
and .tai remains the formal reviewable source before native compilation.

Current accepted native target: Windows x64.
Current CLI responsibilities: precompile, validate-tai, build, run, test, bench, and doc.`,
}

func Execute() error {
	return rootCmd.Execute()
}

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Print version",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("meng version 0.1.0")
	},
}

func init() {
	// Commands are registered in their respective files' init() functions
	// This ensures no circular dependencies
	rootCmd.AddCommand(versionCmd)
}
