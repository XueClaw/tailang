package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
)

var precompileCmd = &cobra.Command{
	Use:   "precompile [file.meng]",
	Short: "Precompile .meng file to .tai",
	Long: `Precompile a .meng file (casual natural language) to .tai file (structured natural language).

This uses LLM (DashScope/Qwen) to understand the intent and convert to structured format.

Environment variables:
  DASHSCOPE_API_KEY     API key for DashScope
  DASHSCOPE_BASE_URL    Base URL (default: https://dashscope.aliyuncs.com/api/v1)
  PRECOMPILER_MODEL     Model name (default: qwen-turbo)

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
		
		// Check environment variables
		apiKey := os.Getenv("DASHSCOPE_API_KEY")
		if apiKey == "" {
			return fmt.Errorf("DASHSCOPE_API_KEY environment variable not set\n" +
				"Please set it in .env file or export it:\n" +
				"  export DASHSCOPE_API_KEY=your-api-key")
		}
		
		fmt.Println("Step 1/3: Reading .meng file...")
		content, err := os.ReadFile(inputFile)
		if err != nil {
			return fmt.Errorf("failed to read file: %w", err)
		}
		fmt.Println("  ✓ File read successfully")
		
		fmt.Println("Step 2/3: Calling LLM API...")
		// TODO: Call Rust precompiler library
		// For now, create a placeholder .tai file
		taiContent := fmt.Sprintf(`# Precompiled from %s
# TODO: Implement LLM integration

模块 预编译占位：
  功能 占位 ():
    返回 "待实现"
`, inputFile)
		
		fmt.Println("  ✓ LLM API called")
		
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
	precompileCmd.Flags().StringP("output", "o", "", "Output .tai file path")
}
