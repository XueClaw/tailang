package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/spf13/cobra"
)

var initCmd = &cobra.Command{
	Use:   "init [project-name]",
	Short: "Initialize a new Tailang project",
	Long: `Initialize a new Tailang project with a complete directory structure.

Example:
  meng init my-blog
  meng init todo-api`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		projectName := args[0]
		
		// Validate project name
		if !isValidProjectName(projectName) {
			return fmt.Errorf("invalid project name: %s\nProject name must start with a letter and contain only letters, numbers, hyphens, and underscores", projectName)
		}
		
		fmt.Printf("🚀 Creating Tailang project: %s\n\n", projectName)
		
		// Create directory structure
		dirs := []string{
			"src",
			"tests",
			"docs",
			"assets",
		}
		
		for _, dir := range dirs {
			dirPath := filepath.Join(projectName, dir)
			if err := os.MkdirAll(dirPath, 0755); err != nil {
				return fmt.Errorf("failed to create directory %s: %w", dirPath, err)
			}
			fmt.Printf("  ✓ Created %s/\n", dirPath)
		}
		
		// Create .gitignore
		gitignore := `# Tailang build outputs
*.exe
*.app
*.bin
*.o
*.so
*.dll
target/
dist/
build/

# Cache files
.meng.cache
*.cache

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log
logs/

# Dependencies
vendor/
node_modules/
`
		gitignorePath := filepath.Join(projectName, ".gitignore")
		if err := os.WriteFile(gitignorePath, []byte(gitignore), 0644); err != nil {
			return fmt.Errorf("failed to create .gitignore: %w", err)
		}
		fmt.Printf("  ✓ Created .gitignore\n")
		
		// Create src/main.meng
		mainMeng := `# Welcome to Tailang!
# This is your first .meng file

# Natural language description
打印 "Hello, Tailang!" qwq

# Optional: Code supplement in any language
` + "```python" + `
# Python example
print("Hello from Python!")

def greet(name):
    return f"Hello, {name}!"

print(greet("Tailang"))
` + "```" + `

# You can also use other languages:
# - Go, Rust, JavaScript, TypeScript, Java, C++, etc.
# Just add another code block!
`
		mainMengPath := filepath.Join(projectName, "src", "main.meng")
		if err := os.WriteFile(mainMengPath, []byte(mainMeng), 0644); err != nil {
			return fmt.Errorf("failed to create src/main.meng: %w", err)
		}
		fmt.Printf("  ✓ Created src/main.meng\n")
		
		// Create tests/main_test.meng
		testMeng := `# Test file example

测试 打印功能:
  给定 输入 "Hello"
  当 打印 "Hello"
  期望 输出 "Hello"
`
		testMengPath := filepath.Join(projectName, "tests", "main_test.meng")
		if err := os.WriteFile(testMengPath, []byte(testMeng), 0644); err != nil {
			return fmt.Errorf("failed to create tests/main_test.meng: %w", err)
		}
		fmt.Printf("  ✓ Created tests/main_test.meng\n")
		
		// Create README.md
		readme := fmt.Sprintf(`# %s

**Created with Tailang (太语言)**

Generated on: %s

## 🚀 Quick Start

### Build
`+"```bash"+`
meng build src/main.meng
`+"```"+`

### Run
`+"```bash"+`
meng run src/main.meng
`+"```"+`

## 📁 Project Structure

`+"```"+`
%s/
├── src/
│   └── main.meng      # Main entry point
├── tests/
│   └── main_test.meng # Test file
├── docs/              # Documentation
├── assets/            # Assets (images, etc.)
├── .gitignore
└── README.md
`+"```"+`

## 📚 Learn More

- [Tailang Documentation](https://github.com/XueClaw/tailang)
- [Language Specification](https://github.com/XueClaw/tailang/docs)
- [Examples](https://github.com/XueClaw/tailang/examples)

## 🎯 Next Steps

1. Edit `+"`src/main.meng`"+` to write your code
2. Run `+"`meng build src/main.meng`"+` to compile
3. Run `+"`./main.exe`"+` (Windows) or `+"`./main`"+` (macOS/Linux) to execute

Happy coding! 🎉
`, projectName, time.Now().Format("2006-01-02"), projectName)
		
		readmePath := filepath.Join(projectName, "README.md")
		if err := os.WriteFile(readmePath, []byte(readme), 0644); err != nil {
			return fmt.Errorf("failed to create README.md: %w", err)
		}
		fmt.Printf("  ✓ Created README.md\n")
		
		// Create project config (optional)
		config := fmt.Sprintf(`name: %s
version: 0.1.0
tailang: 0.1.0
entry: src/main.meng
`, projectName)
		
		configPath := filepath.Join(projectName, "tailang.yaml")
		if err := os.WriteFile(configPath, []byte(config), 0644); err != nil {
			return fmt.Errorf("failed to create tailang.yaml: %w", err)
		}
		fmt.Printf("  ✓ Created tailang.yaml\n")
		
		// Print success message
		fmt.Printf("\n✅ Project '%s' initialized successfully!\n\n", projectName)
		fmt.Println("📚 Next steps:")
		fmt.Printf("  cd %s\n", projectName)
		fmt.Println("  meng build src/main.meng    # Build the project")
		fmt.Println("  meng run src/main.meng      # Build and run")
		fmt.Println("  edit src/main.meng          # Start coding!")
		fmt.Println("\n🎉 Happy coding!")
		
		return nil
	},
}

func isValidProjectName(name string) bool {
	if len(name) == 0 || len(name) > 100 {
		return false
	}
	
	// Must start with a letter
	if !strings.HasPrefix(name, "-") && !strings.HasPrefix(name, "_") {
		firstChar := name[0]
		if !((firstChar >= 'a' && firstChar <= 'z') || (firstChar >= 'A' && firstChar <= 'Z')) {
			return false
		}
	}
	
	// Can only contain letters, numbers, hyphens, and underscores
	for _, char := range name {
		if !((char >= 'a' && char <= 'z') || 
		     (char >= 'A' && char <= 'Z') || 
		     (char >= '0' && char <= '9') || 
		     char == '-' || 
		     char == '_') {
			return false
		}
	}
	
	return true
}

func init() {
	rootCmd.AddCommand(initCmd)
}
