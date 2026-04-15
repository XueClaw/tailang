package cmd

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

var validateTaiCmd = &cobra.Command{
	Use:   "validate-tai [file.tai]",
	Short: "Validate a .tai file against the shared schema",
	Long: `Validate a .tai JSON file against Tailang's shared schema document.

This command loads docs/spec/tai.schema.json and checks the input .tai file
against the current CLI schema rules.`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		inputFile := args[0]

		content, err := os.ReadFile(inputFile)
		if err != nil {
			return fmt.Errorf("failed to read file: %w", err)
		}

		var doc taiSchema
		if err := json.Unmarshal(content, &doc); err != nil {
			return fmt.Errorf("invalid .tai JSON: %w", err)
		}

		if doc.Modules == nil {
			doc.Modules = []taiModule{}
		}
		if doc.CodeBlocks == nil {
			doc.CodeBlocks = []taiCodeBlock{}
		}
		if doc.UnresolvedItems == nil {
			doc.UnresolvedItems = []taiUnresolvedItem{}
		}

		if err := validateTaiAgainstSchema(&doc); err != nil {
			return err
		}

		fmt.Printf("✓ Valid .tai: %s\n", inputFile)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(validateTaiCmd)
}
