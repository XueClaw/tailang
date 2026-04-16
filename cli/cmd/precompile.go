package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
)

var precompileCmd = &cobra.Command{
	Use:   "precompile [file.meng]",
	Short: "Precompile .meng file to .tai source",
	Long: `Precompile a .meng file (casual natural language) to normalized .tai source.

This uses the configured Tailang LLM provider to understand the intent and convert it into normalized Tailang .tai v0.3 source.

Environment variables:
  TAILANG_LLM_PROVIDER  Provider name (dashscope, ollama, custom)
  TAILANG_LLM_MODEL     Model name override
  TAILANG_LLM_BASE_URL  Custom/OpenAI-compatible base URL
  TAILANG_LLM_API_KEY   Shared API key override
  DASHSCOPE_API_KEY     DashScope API key
  DASHSCOPE_BASE_URL    DashScope base URL
  OLLAMA_BASE_URL       Ollama base URL

Examples:
  meng precompile src/main.meng
  meng precompile src/main.meng -o src/main.tai`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		inputFile := args[0]
		
		// Get output file
		outputFile, _ := cmd.Flags().GetString("output")
		if outputFile == "" {
			outputFile = filepath.Join(filepath.Dir(inputFile), 
				filepath.Base(inputFile[:len(filepath.Base(inputFile))-5])+".tai")
		}
		
		fmt.Printf("🔄 Precompiling %s...\n", inputFile)
		fmt.Printf("   Output: %s\n\n", outputFile)
		
		fmt.Println("Step 1/3: Reading .meng file...")
		content, err := os.ReadFile(inputFile)
		if err != nil {
			return fmt.Errorf("failed to read file: %w", err)
		}
		fmt.Println("  ✓ File read successfully")
		
		fmt.Println("Step 2/3: Calling configured provider...")
		taiContent, err := precompileMeng(string(content))
		if err != nil {
			return fmt.Errorf("precompilation failed: %w", err)
		}
		fmt.Println("  ✓ Provider returned normalized .tai source")
		
		fmt.Println("Step 3/3: Writing .tai file...")
		err = os.WriteFile(outputFile, []byte(taiContent), 0644)
		if err != nil {
			return fmt.Errorf("failed to write file: %w", err)
		}
		fmt.Println("  ✓ .tai file written")
		
		fmt.Printf("\n✅ Precompilation complete!\n\n")
		fmt.Printf("📄 Output: %s\n", outputFile)
		fmt.Printf("\n🚀 Next step:\n")
		fmt.Printf("   meng build %s\n", outputFile)
		
		return nil
	},
}

func init() {
	rootCmd.AddCommand(precompileCmd)
	precompileCmd.Flags().StringP("output", "o", "", "Output .tai file path")
}
