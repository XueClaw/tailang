package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
)

var benchCmd = &cobra.Command{
	Use:   "bench [file.tai]",
	Short: "Build a benchmark target",
	Long: `Build a Tailang benchmark target.

This command is the first step for native-vs-Python performance baselines.
Current implementation only builds the requested .tai benchmark target.`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		target := "bench_numeric.tai"
		if len(args) > 0 {
			target = args[0]
		}

		if _, err := os.Stat(target); os.IsNotExist(err) {
			if _, err := os.Stat(filepath.Join("cli", target)); err == nil {
				target = filepath.Join("cli", target)
			}
		}

		output := defaultOutputName(target, "windows")
		fmt.Printf("📈 Building benchmark target %s -> %s\n", target, output)
		irSource, err := os.ReadFile(target)
		if err != nil {
			return fmt.Errorf("read benchmark target: %w", err)
		}
		decoded, err := decodeUTF8Source(irSource)
		if err != nil {
			return err
		}
		normalized, err := loadNormalizedTai(target, decoded)
		if err != nil {
			return err
		}
		blocks, err := extractCodeBlocksFromTai(normalized)
		if err != nil {
			return err
		}
		ir, err := generateIR(normalized, blocks)
		if err != nil {
			return err
		}
		return compileToExecutable(ir, output, "windows")
	},
}

func init() {
	rootCmd.AddCommand(benchCmd)
}
