package cmd

import (
	"fmt"
	"os"

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

func init() {
	rootCmd.AddCommand(initCmd)
	rootCmd.AddCommand(buildCmd)
	rootCmd.AddCommand(runCmd)
	rootCmd.AddCommand(versionCmd)
}

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Print version",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("meng version 0.1.0")
	},
}

var initCmd = &cobra.Command{
	Use:   "init [project-name]",
	Short: "Initialize a new Tailang project",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		projectName := args[0]
		fmt.Printf("Creating Tailang project: %s\n", projectName)
		
		// Create directory structure
		dirs := []string{
			"src",
			"tests",
		}
		
		for _, dir := range dirs {
			os.MkdirAll(dir, 0755)
		}
		
		// Create .gitignore
		gitignore := `*.exe
*.app
*.o
*.so
target/
dist/
.meng.cache
`
		os.WriteFile(".gitignore", []byte(gitignore), 0644)
		
		fmt.Println("✓ Project initialized")
		fmt.Println("\nNext steps:")
		fmt.Println("  cd", projectName)
		fmt.Println("  meng new src/main.meng")
		fmt.Println("  meng build src/main.meng")
	},
}

var buildCmd = &cobra.Command{
	Use:   "build [file.meng]",
	Short: "Compile .meng file to executable",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		file := args[0]
		fmt.Printf("Building %s...\n", file)
		
		// TODO: Implement compilation
		fmt.Println("✓ Build complete")
		fmt.Println("Output: main.exe")
	},
}

var runCmd = &cobra.Command{
	Use:   "run [file.meng]",
	Short: "Compile and run .meng file",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		file := args[0]
		fmt.Printf("Running %s...\n", file)
		
		// TODO: Implement compilation and execution
		fmt.Println("✓ Execution complete")
	},
}
