package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "meng",
	Short: "Tailang CLI - Zero syntax programming",
	Long: `Tailang (太语言) - 道法自然，码由心生

A programming language where you code in natural language.
Zero syntax, supports 50+ programming languages, compiles to executable in one step.`,
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
