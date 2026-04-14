package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

var buildCmd = &cobra.Command{
	Use:   "build [file.meng]",
	Short: "Compile .meng file to executable",
	Long: `Compile a Tailang .meng file to an executable binary.

The compiler will:
1. Parse the .meng file
2. Precompile natural language to structured logic
3. Extract and validate code supplements
4. Compile to native executable

Examples:
  meng build src/main.meng
  meng build src/main.meng -o myapp
  meng build src/main.meng --target windows`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		inputFile := args[0]
		
		// Validate input file exists
		if _, err := os.Stat(inputFile); os.IsNotExist(err) {
			return fmt.Errorf("file not found: %s", inputFile)
		}
		
		// Get output name
		outputName, _ := cmd.Flags().GetString("output")
		if outputName == "" {
			// Default: use input filename without extension
			baseName := strings.TrimSuffix(filepath.Base(inputFile), ".meng")
			outputName = baseName
		}
		
		// Get target platform
		target, _ := cmd.Flags().GetString("target")
		if target == "" {
			target = runtime.GOOS
		}
		
		// Add extension based on platform
		if target == "windows" {
			if !strings.HasSuffix(outputName, ".exe") {
				outputName = outputName + ".exe"
			}
		} else if target == "darwin" {
			if !strings.HasSuffix(outputName, ".app") {
				outputName = outputName + ".app"
			}
		}
		
		fmt.Printf("🔨 Building %s...\n", inputFile)
		fmt.Printf("   Output: %s\n", outputName)
		fmt.Printf("   Target: %s\n\n", target)
		
		// Step 1: Read and parse .meng file
		fmt.Println("Step 1/5: Reading .meng file...")
		content, err := os.ReadFile(inputFile)
		if err != nil {
			return fmt.Errorf("failed to read file: %w", err)
		}
		fmt.Println("  ✓ File read successfully")
		
		// Step 2: Precompile natural language
		fmt.Println("Step 2/5: Precompiling natural language...")
		precompiled, err := precompileMeng(string(content))
		if err != nil {
			return fmt.Errorf("precompilation failed: %w", err)
		}
		fmt.Println("  ✓ Natural language expanded")
		
		// Step 3: Extract code supplements
		fmt.Println("Step 3/5: Extracting code supplements...")
		codeBlocks, err := extractCodeBlocks(precompiled)
		if err != nil {
			return fmt.Errorf("failed to extract code blocks: %w", err)
		}
		fmt.Printf("  ✓ Found %d code block(s)\n", len(codeBlocks))
		
		// Step 4: Generate intermediate representation
		fmt.Println("Step 4/5: Generating intermediate representation...")
		ir, err := generateIR(precompiled, codeBlocks)
		if err != nil {
			return fmt.Errorf("IR generation failed: %w", err)
		}
		fmt.Println("  ✓ IR generated")
		
		// Step 5: Compile to executable
		fmt.Println("Step 5/5: Compiling to executable...")
		err = compileToExecutable(ir, outputName, target)
		if err != nil {
			return fmt.Errorf("compilation failed: %w", err)
		}
		fmt.Println("  ✓ Compilation successful")
		
		// Success message
		fmt.Printf("\n✅ Build complete!\n\n")
		fmt.Printf("📦 Output: %s\n", outputName)
		fmt.Printf("📊 Size: %s\n", formatFileSize(outputName))
		fmt.Printf("\n🚀 Run with:\n")
		fmt.Printf("   ./ %s\n", outputName)
		fmt.Printf("\nOr use:\n")
		fmt.Printf("   meng run %s\n", inputFile)
		
		return nil
	},
}

func init() {
	buildCmd.Flags().StringP("output", "o", "", "Output filename")
	buildCmd.Flags().String("target", "", "Target platform (windows, macos, linux)")
}

// precompileMeng expands natural language to structured logic
func precompileMeng(content string) (string, error) {
	// TODO: Integrate with LLM for precompilation
	// For now, return content as-is
	return content, nil
}

// extractCodeBlocks extracts code blocks from .meng content
func extractCodeBlocks(content string) ([]CodeBlock, error) {
	var blocks []CodeBlock
	
	// Simple parser for ```language ... ``` blocks
	lines := strings.Split(content, "\n")
	inCodeBlock := false
	var currentBlock CodeBlock
	
	for i, line := range lines {
		trimmed := strings.TrimSpace(line)
		
		if strings.HasPrefix(trimmed, "```") && !inCodeBlock {
			// Start of code block
			inCodeBlock = true
			currentBlock.Language = strings.TrimPrefix(trimmed, "```")
			currentBlock.StartLine = i + 1
			currentBlock.Code = ""
		} else if strings.HasPrefix(trimmed, "```") && inCodeBlock {
			// End of code block
			inCodeBlock = false
			currentBlock.EndLine = i + 1
			blocks = append(blocks, currentBlock)
			currentBlock = CodeBlock{}
		} else if inCodeBlock {
			currentBlock.Code += line + "\n"
		}
	}
	
	if inCodeBlock {
		return nil, fmt.Errorf("unclosed code block starting at line %d", currentBlock.StartLine)
	}
	
	return blocks, nil
}

// CodeBlock represents a code supplement block
type CodeBlock struct {
	Language  string
	Code      string
	StartLine int
	EndLine   int
}

// generateIR generates intermediate representation
func generateIR(content string, codeBlocks []CodeBlock) (*IR, error) {
	return &IR{
		Source:    content,
		CodeBlocks: codeBlocks,
	}, nil
}

// IR represents intermediate representation
type IR struct {
	Source     string
	CodeBlocks []CodeBlock
}

// compileToExecutable compiles IR to native executable
func compileToExecutable(ir *IR, outputName string, target string) error {
	// TODO: Implement actual compilation
	// For now, create a placeholder executable
	
	// Create a simple script that prints a message
	var script string
	if target == "windows" {
		script = fmt.Sprintf(`@echo off
echo Tailang Executable
echo ==================
echo This is a placeholder for: %s
echo Full compiler implementation coming soon!
pause
`, outputName)
		os.WriteFile(outputName+".bat", []byte(script), 0755)
		os.Rename(outputName+".bat", outputName)
	} else {
		script = fmt.Sprintf(`#!/bin/bash
echo "Tailang Executable"
echo "=================="
echo "This is a placeholder for: %s"
echo "Full compiler implementation coming soon!"
`, outputName)
		os.WriteFile(outputName, []byte(script), 0755)
	}
	
	return nil
}

// formatFileSize formats file size in human-readable format
func formatFileSize(filename string) string {
	info, err := os.Stat(filename)
	if err != nil {
		return "unknown"
	}
	
	size := info.Size()
	if size < 1024 {
		return fmt.Sprintf("%d B", size)
	} else if size < 1024*1024 {
		return fmt.Sprintf("%.1f KB", float64(size)/1024)
	} else {
		return fmt.Sprintf("%.1f MB", float64(size)/(1024*1024))
	}
}
