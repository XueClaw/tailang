package cmd

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/spf13/cobra"
)

var buildCmd = &cobra.Command{
	Use:   "build [file.meng]",
	Short: "Compile .meng file to executable",
	Long: `Compile a Tailang .meng or .tai file to a build artifact.

The compiler will:
1. Read the input source (.meng or .tai)
2. Normalize it to stable .tai source
3. Extract and validate code supplements from .tai
4. Generate intermediate representation
5. Hand off to the compiler backend

Examples:
  meng build src/main.meng
  meng build src/main.tai
  meng build src/main.meng -o myapp
  meng build src/main.tai --target windows`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		inputFile := args[0]
		
		// Validate input file exists
		if _, err := os.Stat(inputFile); os.IsNotExist(err) {
			return fmt.Errorf("file not found: %s", inputFile)
		}
		
		// Get output name
		outputName, _ := cmd.Flags().GetString("output")
		target, _ := cmd.Flags().GetString("target")
		backend, _ := cmd.Flags().GetString("backend")
		optLevel, _ := cmd.Flags().GetString("opt-level")
		if target == "" {
			target = runtime.GOOS
		}
		
		// Load .env file
		envPath := findEnvFile(inputFile)
		if envPath != "" {
			loadEnvFile(envPath)
			fmt.Printf("   📝 Loaded .env from: %s\n\n", envPath)
		} else {
			fmt.Println()
		}
		
		if outputName == "" {
			outputName = defaultOutputName(inputFile, target)
		}
		
		fmt.Printf("🔨 Building %s...\n", inputFile)
		fmt.Printf("   Output: %s\n", outputName)
		fmt.Printf("   Target: %s\n\n", target)
		
		// Step 1: Read source file
		fmt.Println("Step 1/5: Reading source file...")
		content, err := os.ReadFile(inputFile)
		if err != nil {
			return fmt.Errorf("failed to read file: %w", err)
		}
		decoded, err := decodeUTF8Source(content)
		if err != nil {
			return err
		}
		fmt.Println("  ✓ File read successfully")
		
		// Step 2: Normalize to .tai source
		fmt.Println("Step 2/5: Normalizing source to .tai...")
		precompiled, err := loadNormalizedTai(inputFile, decoded)
		if err != nil {
			return err
		}
		fmt.Println("  ✓ .tai normalized")
		
		// Step 3: Extract code supplements
		fmt.Println("Step 3/5: Extracting code supplements from .tai...")
		codeBlocks, err := extractCodeBlocksFromTai(precompiled)
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
		err = compileToExecutable(ir, outputName, target, backend, optLevel)
		if err != nil {
			return fmt.Errorf("compilation failed: %w", err)
		}
		fmt.Println("  ✓ Compilation successful")
		
		// Success message
		fmt.Printf("\n✅ Build complete!\n\n")
		fmt.Printf("📦 Output: %s\n", outputName)
		fmt.Printf("📊 Size: %s\n", formatFileSize(outputName))
		fmt.Printf("\n🚀 Run with:\n")
		fmt.Printf("   ./%s\n", outputName)
		fmt.Printf("\nOr use:\n")
		fmt.Printf("   meng run %s\n", inputFile)
		
		return nil
	},
}

func init() {
	rootCmd.AddCommand(buildCmd)
	buildCmd.Flags().StringP("output", "o", "", "Output filename")
	buildCmd.Flags().String("target", "", "Target platform (windows, macos, linux)")
	buildCmd.Flags().String("backend", "self-native", "Compiler backend (self-native, llvm)")
	buildCmd.Flags().String("opt-level", "1", "Optimization level (0, 1, 2)")
}

// loadEnvFile loads environment variables from .env file
func loadEnvFile(path string) error {
	file, err := os.Open(path)
	if err != nil {
		return err
	}
	defer file.Close()
	
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		// Skip comments and empty lines
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		
		parts := strings.SplitN(line, "=", 2)
		if len(parts) == 2 {
			key := strings.TrimSpace(parts[0])
			value := strings.TrimSpace(parts[1])
			os.Setenv(key, value)
		}
	}
	
	return scanner.Err()
}

// findEnvFile searches for .env file in project directory
func findEnvFile(inputFile string) string {
	// Start from input file's directory and go up
	dir := filepath.Dir(inputFile)
	
	for {
		envPath := filepath.Join(dir, ".env")
		if _, err := os.Stat(envPath); err == nil {
			return envPath
		}
		
		parent := filepath.Dir(dir)
		if parent == dir {
			// Reached root
			break
		}
		dir = parent
	}
	
	return ""
}

func loadNormalizedTai(inputFile string, content string) (string, error) {
	switch strings.ToLower(filepath.Ext(inputFile)) {
	case ".tai":
		trimmed := strings.TrimSpace(content)
		if looksLikeLegacyTaiJSON(trimmed) {
			var doc taiSchema
			if err := json.Unmarshal([]byte(content), &doc); err != nil {
				return "", fmt.Errorf("invalid .tai JSON snapshot: %w", err)
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
				return "", err
			}

			normalized, err := json.MarshalIndent(doc, "", "  ")
			if err != nil {
				return "", fmt.Errorf("serialize normalized .tai snapshot failed: %w", err)
			}
			return string(normalized), nil
		}

		if err := validateTextualTaiSource(trimmed); err != nil {
			return "", err
		}
		return trimmed, nil
	case ".meng":
		precompiled, err := precompileMeng(content)
		if err != nil {
			return "", fmt.Errorf("precompilation failed: %w", err)
		}
		return precompiled, nil
	default:
		return "", fmt.Errorf("unsupported input file: %s (expected .meng or .tai)", inputFile)
	}
}

// extractCodeBlocksFromTai extracts code blocks from normalized .tai.
func extractCodeBlocksFromTai(content string) ([]CodeBlock, error) {
	trimmed := strings.TrimSpace(content)
	if looksLikeLegacyTaiJSON(trimmed) {
		var doc taiSchema
		if err := json.Unmarshal([]byte(content), &doc); err != nil {
			return nil, fmt.Errorf("invalid .tai JSON: %w", err)
		}

		blocks := make([]CodeBlock, 0, len(doc.CodeBlocks))
		for _, block := range doc.CodeBlocks {
			blocks = append(blocks, CodeBlock{
				Language: block.Language,
				Code:     block.Code,
			})
		}
		return blocks, nil
	}

	lines := strings.Split(trimmed, "\n")
	blocks := make([]CodeBlock, 0)
	var current *CodeBlock
	for _, raw := range lines {
		line := strings.TrimSpace(raw)
		if current != nil {
			if line == ".代码结束" {
				blocks = append(blocks, *current)
				current = nil
				continue
			}
			if current.Code == "" {
				current.Code = raw
			} else {
				current.Code += "\n" + raw
			}
			continue
		}

		if strings.HasPrefix(line, ".代码 ") {
			current = &CodeBlock{Language: strings.TrimSpace(strings.TrimPrefix(line, ".代码 "))}
		}
	}

	if current != nil {
		return nil, fmt.Errorf("invalid textual .tai: .代码 block missing .代码结束")
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
		Source:     content,
		CodeBlocks: codeBlocks,
	}, nil
}

// IR represents intermediate representation
type IR struct {
	Source     string
	CodeBlocks []CodeBlock
}

// compileToExecutable compiles IR to native executable
func compileToExecutable(ir *IR, outputName string, target string, backend string, optLevel string) error {
	if strings.TrimSpace(ir.Source) == "" {
		return fmt.Errorf("empty .tai source")
	}
	outputPath, err := filepath.Abs(outputName)
	if err != nil {
		return fmt.Errorf("resolve output path failed: %w", err)
	}
	tempDir, err := os.MkdirTemp("", "tailang-build-*")
	if err != nil {
		return fmt.Errorf("create temp build directory failed: %w", err)
	}
	defer os.RemoveAll(tempDir)

	inputPath := filepath.Join(tempDir, "input.tai")
	if err := os.WriteFile(inputPath, []byte(ir.Source), 0644); err != nil {
		return fmt.Errorf("write temp .tai source failed: %w", err)
	}

	compilerDir, err := findCompilerDir()
	if err != nil {
		return err
	}

	cargoArgs := []string{
		"run", "--quiet", "--",
		"compile",
		"--input", inputPath,
		"--output", outputPath,
		"--backend", backend,
		"--opt-level", optLevel,
	}
	cmd := exec.Command("cargo", cargoArgs...)
	cmd.Dir = compilerDir
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("Rust compiler invocation failed: %w", err)
	}
	return nil
}

func findCompilerDir() (string, error) {
	wd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("get working directory failed: %w", err)
	}

	dir := wd
	for {
		candidate := filepath.Join(dir, "compiler", "Cargo.toml")
		if _, err := os.Stat(candidate); err == nil {
			return filepath.Dir(candidate), nil
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}

	return "", fmt.Errorf("failed to locate Tailang compiler/Cargo.toml")
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
