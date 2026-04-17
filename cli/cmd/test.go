package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"

	"github.com/spf13/cobra"
)

var testCmd = &cobra.Command{
	Use:   "test [path]",
	Short: "Run tests",
	Long: `Run Tailang tests.

Examples:
  meng test              # Run all tests
  meng test tests/       # Run tests in directory
  meng test tests/foo.meng  # Run specific test file`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		testPath := "."
		if len(args) > 0 {
			testPath = args[0]
		}

		fmt.Println("🧪 Running tests...")
		fmt.Println()

		// Find all .meng test files
		var testFiles []string
		filepath.Walk(testPath, func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if strings.HasSuffix(path, "_test.meng") || strings.HasSuffix(path, ".test.meng") {
				testFiles = append(testFiles, path)
			}
			return nil
		})

		if len(testFiles) == 0 {
			fmt.Println("No test files found")
			return nil
		}

		sort.Strings(testFiles)
		fmt.Printf("Found %d test file(s):\n\n", len(testFiles))

		passed := 0
		failed := 0

		for _, testFile := range testFiles {
			fmt.Printf("  Running %s... ", testFile)

			if err := runMengTestFile(testFile); err != nil {
				fmt.Printf("✗ FAILED (%s)\n", err)
				failed++
			} else {
				fmt.Println("✓ PASSED")
				passed++
			}
		}

		fmt.Printf("\n✅ Tests complete: %d passed, %d failed\n", passed, failed)

		if failed > 0 {
			os.Exit(1)
		}

		return nil
	},
}

var expectedOutputPattern = regexp.MustCompile(`期望\s+输出\s+"([^"]+)"`)
var quotedDoublePattern = regexp.MustCompile(`"([^"]+)"`)
var quotedSinglePattern = regexp.MustCompile(`'([^']+)'`)

func runMengTestFile(testFile string) error {
	content, err := os.ReadFile(testFile)
	if err != nil {
		return fmt.Errorf("read test file: %w", err)
	}

	expectedOutputs := parseExpectedOutputs(string(content))
	if len(expectedOutputs) == 0 {
		return fmt.Errorf("no supported assertions found")
	}

	targetFile, err := resolveTargetMengFile(testFile)
	if err != nil {
		return err
	}

	targetContent, err := os.ReadFile(targetFile)
	if err != nil {
		return fmt.Errorf("read target file: %w", err)
	}

	taiSource := string(targetContent)
	trimmedTai := strings.TrimSpace(taiSource)
	if !looksLikeLegacyTaiJSON(trimmedTai) && !isTextualTaiSource(trimmedTai) && !strings.HasSuffix(strings.ToLower(targetFile), ".tai") {
		taiSource, err = precompileMeng(taiSource)
		if err != nil {
			return fmt.Errorf("precompile target: %w", err)
		}
	}

	actualOutputs, err := collectOutputsFromTai(taiSource)
	if err != nil {
		return err
	}

	for _, expected := range expectedOutputs {
		if _, ok := actualOutputs[expected]; !ok {
			return fmt.Errorf("expected output %q not found in target flow", expected)
		}
	}

	return nil
}

func parseExpectedOutputs(content string) []string {
	matches := expectedOutputPattern.FindAllStringSubmatch(content, -1)
	outputs := make([]string, 0, len(matches))
	for _, match := range matches {
		if len(match) > 1 {
			outputs = append(outputs, match[1])
		}
	}
	return outputs
}

func resolveTargetMengFile(testFile string) (string, error) {
	base := filepath.Base(testFile)
	dir := filepath.Dir(testFile)

	candidates := []string{
		strings.TrimSuffix(base, "_test.meng") + ".meng",
		strings.TrimSuffix(base, ".test.meng") + ".meng",
	}

	searchRoots := []string{
		dir,
		filepath.Join(filepath.Dir(dir), "src"),
		filepath.Dir(dir),
	}

	for _, root := range searchRoots {
		for _, candidate := range candidates {
			path := filepath.Join(root, candidate)
			if _, err := os.Stat(path); err == nil {
				return path, nil
			}
		}
	}

	return "", fmt.Errorf("unable to resolve target .meng file")
}

func collectOutputsFromTai(taiSource string) (map[string]struct{}, error) {
	outputs := map[string]struct{}{}
	if looksLikeLegacyTaiJSON(strings.TrimSpace(taiSource)) {
		var doc taiSchema
		if err := json.Unmarshal([]byte(taiSource), &doc); err != nil {
			return nil, fmt.Errorf("invalid .tai JSON: %w", err)
		}

		for _, module := range doc.Modules {
			collectQuotedStrings(module.Description, outputs)
			for _, fn := range module.Functions {
				collectQuotedStrings(fn.Description, outputs)
				for _, validation := range fn.Validations {
					collectQuotedStrings(validation, outputs)
				}
			}
		}

		for _, block := range doc.CodeBlocks {
			collectQuotedStrings(block.Code, outputs)
		}
		return outputs, nil
	}

	collectQuotedStrings(taiSource, outputs)
	return outputs, nil
}

func collectQuotedStrings(input string, outputs map[string]struct{}) {
	for _, pattern := range []*regexp.Regexp{quotedDoublePattern, quotedSinglePattern} {
		matches := pattern.FindAllStringSubmatch(input, -1)
		for _, match := range matches {
			if len(match) > 1 && strings.TrimSpace(match[1]) != "" {
				outputs[match[1]] = struct{}{}
			}
		}
	}
}

func init() {
	rootCmd.AddCommand(testCmd)
}
