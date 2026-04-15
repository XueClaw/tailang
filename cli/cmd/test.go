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

	taiJSON := string(targetContent)
	if !looksLikeTaiJSON(taiJSON) {
		taiJSON, err = precompileMeng(taiJSON)
		if err != nil {
			return fmt.Errorf("precompile target: %w", err)
		}
	}

	actualOutputs, err := collectOutputsFromTai(taiJSON)
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

func looksLikeTaiJSON(content string) bool {
	trimmed := strings.TrimSpace(content)
	return strings.HasPrefix(trimmed, "{") &&
		strings.Contains(trimmed, `"modules"`) &&
		strings.Contains(trimmed, `"code_blocks"`)
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

func collectOutputsFromTai(taiJSON string) (map[string]struct{}, error) {
	var doc taiSchema
	if err := json.Unmarshal([]byte(taiJSON), &doc); err != nil {
		return nil, fmt.Errorf("invalid .tai JSON: %w", err)
	}

	outputs := map[string]struct{}{}
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

var docCmd = &cobra.Command{
	Use:   "doc [path]",
	Short: "Generate documentation",
	Long: `Generate documentation from .meng files.

Examples:
  meng doc              # Generate docs for current project
  meng doc src/         # Generate docs for directory
  meng doc --format html  # Output as HTML`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		docPath := "."
		if len(args) > 0 {
			docPath = args[0]
		}

		format, _ := cmd.Flags().GetString("format")
		output, _ := cmd.Flags().GetString("output")

		if output == "" {
			output = "docs"
		}

		fmt.Printf("📚 Generating documentation...\n")
		fmt.Printf("   Source: %s\n", docPath)
		fmt.Printf("   Format: %s\n", format)
		fmt.Printf("   Output: %s\n\n", output)

		indexPath, count, err := generateTaiDocs(docPath, output, format)
		if err != nil {
			return err
		}

		fmt.Printf("✅ Documentation generated: %s (%d file(s))\n", indexPath, count)
		return nil
	},
}

type taiDocFile struct {
	Name string
	Path string
	Doc  taiSchema
}

func generateTaiDocs(inputPath string, outputDir string, format string) (string, int, error) {
	if format != "markdown" {
		return "", 0, fmt.Errorf("unsupported doc format: %s", format)
	}

	files, err := collectTaiDocFiles(inputPath)
	if err != nil {
		return "", 0, err
	}
	if len(files) == 0 {
		return "", 0, fmt.Errorf("no .meng or .tai files found")
	}

	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return "", 0, fmt.Errorf("create output directory: %w", err)
	}

	pagesDir := filepath.Join(outputDir, "modules")
	if err := os.MkdirAll(pagesDir, 0755); err != nil {
		return "", 0, fmt.Errorf("create modules directory: %w", err)
	}

	for _, file := range files {
		pagePath := filepath.Join(pagesDir, file.Name+".md")
		if err := os.WriteFile(pagePath, []byte(renderTaiDocPage(file)), 0644); err != nil {
			return "", 0, fmt.Errorf("write doc page: %w", err)
		}
	}

	indexPath := filepath.Join(outputDir, "README.md")
	if err := os.WriteFile(indexPath, []byte(renderTaiDocIndex(files)), 0644); err != nil {
		return "", 0, fmt.Errorf("write doc index: %w", err)
	}

	return indexPath, len(files), nil
}

func collectTaiDocFiles(inputPath string) ([]taiDocFile, error) {
	info, err := os.Stat(inputPath)
	if err != nil {
		return nil, fmt.Errorf("stat input path: %w", err)
	}

	if !info.IsDir() {
		doc, err := loadTaiFromPath(inputPath)
		if err != nil {
			return nil, err
		}
		return []taiDocFile{{
			Name: docPageName(inputPath),
			Path: inputPath,
			Doc:  doc,
		}}, nil
	}

	entries := map[string]taiDocFile{}
	err = filepath.Walk(inputPath, func(path string, info os.FileInfo, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if info.IsDir() {
			return nil
		}
		if strings.HasSuffix(path, "_test.meng") || strings.HasSuffix(path, ".test.meng") {
			return nil
		}
		if !strings.HasSuffix(path, ".meng") && !strings.HasSuffix(path, ".tai") {
			return nil
		}

		key := strings.TrimSuffix(path, filepath.Ext(path))
		if _, exists := entries[key]; exists && strings.HasSuffix(path, ".meng") {
			return nil
		}

		doc, err := loadTaiFromPath(path)
		if err != nil {
			return err
		}

		entries[key] = taiDocFile{
			Name: docPageName(path),
			Path: path,
			Doc:  doc,
		}
		return nil
	})
	if err != nil {
		return nil, err
	}

	files := make([]taiDocFile, 0, len(entries))
	for _, file := range entries {
		files = append(files, file)
	}
	sort.Slice(files, func(i, j int) bool {
		return files[i].Name < files[j].Name
	})
	return files, nil
}

func loadTaiFromPath(path string) (taiSchema, error) {
	content, err := os.ReadFile(path)
	if err != nil {
		return taiSchema{}, fmt.Errorf("read input file: %w", err)
	}

	taiJSON := string(content)
	if !looksLikeTaiJSON(taiJSON) {
		taiJSON, err = precompileMeng(taiJSON)
		if err != nil {
			return taiSchema{}, fmt.Errorf("precompile source: %w", err)
		}
	}

	var doc taiSchema
	if err := json.Unmarshal([]byte(taiJSON), &doc); err != nil {
		return taiSchema{}, fmt.Errorf("invalid .tai JSON: %w", err)
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
		return taiSchema{}, err
	}

	return doc, nil
}

func docPageName(path string) string {
	base := strings.TrimSuffix(filepath.Base(path), filepath.Ext(path))
	replacer := strings.NewReplacer(" ", "-", "_", "-", ".", "-")
	name := strings.ToLower(replacer.Replace(base))
	if name == "" {
		return "document"
	}
	return name
}

func renderTaiDocIndex(files []taiDocFile) string {
	var b strings.Builder
	totalModules := 0
	totalFunctions := 0
	totalCodeBlocks := 0
	totalUnresolved := 0

	for _, file := range files {
		totalModules += len(file.Doc.Modules)
		totalFunctions += countFunctions(file.Doc)
		totalCodeBlocks += len(file.Doc.CodeBlocks)
		totalUnresolved += len(file.Doc.UnresolvedItems)
	}

	b.WriteString("# Tailang Documentation\n\n")
	b.WriteString("Generated from normalized `.tai` documents.\n\n")
	b.WriteString("## Summary\n\n")
	b.WriteString("- Files: " + strconv.Itoa(len(files)) + "\n")
	b.WriteString("- Modules: " + strconv.Itoa(totalModules) + "\n")
	b.WriteString("- Functions: " + strconv.Itoa(totalFunctions) + "\n")
	b.WriteString("- Code Blocks: " + strconv.Itoa(totalCodeBlocks) + "\n")
	b.WriteString("- Unresolved Items: " + strconv.Itoa(totalUnresolved) + "\n\n")
	b.WriteString("## Files\n\n")
	for _, file := range files {
		b.WriteString("- [" + file.Name + "](modules/" + file.Name + ".md)\n")
	}
	return b.String()
}

func renderTaiDocPage(file taiDocFile) string {
	var b strings.Builder
	doc := file.Doc

	b.WriteString("# " + file.Name + "\n\n")
	b.WriteString("- Source: `" + filepath.ToSlash(file.Path) + "`\n")
	b.WriteString("- Version: `" + doc.Version + "`\n")
	b.WriteString("- Provider: `" + doc.Source.Provider + "`\n")
	b.WriteString("- Model: `" + doc.Source.Model + "`\n")
	b.WriteString("- Temperature: `" + doc.Source.Temperature + "`\n\n")

	if len(doc.Modules) > 0 {
		b.WriteString("## Modules\n\n")
		for _, module := range doc.Modules {
			b.WriteString("### " + module.Name + "\n\n")
			if strings.TrimSpace(module.Description) != "" {
				b.WriteString(module.Description + "\n\n")
			}
			if len(module.Functions) == 0 {
				b.WriteString("No functions.\n\n")
				continue
			}
			for _, fn := range module.Functions {
				b.WriteString("#### " + fn.Name + "\n\n")
				if len(fn.Params) > 0 {
					b.WriteString("- Params: `" + strings.Join(fn.Params, "`, `") + "`\n")
				} else {
					b.WriteString("- Params: none\n")
				}
				if strings.TrimSpace(fn.Description) != "" {
					b.WriteString("- Description: " + fn.Description + "\n")
				}
				if len(fn.Validations) > 0 {
					b.WriteString("- Validations: " + strings.Join(fn.Validations, "; ") + "\n")
				}
				b.WriteString("\n")
			}
		}
	}

	if len(doc.CodeBlocks) > 0 {
		b.WriteString("## Code Blocks\n\n")
		for i, block := range doc.CodeBlocks {
			title := "Block " + strconv.Itoa(i+1)
			if block.LinkedTo != nil && strings.TrimSpace(*block.LinkedTo) != "" {
				title += " (" + *block.LinkedTo + ")"
			}
			b.WriteString("### " + title + "\n\n")
			b.WriteString("```" + block.Language + "\n" + strings.TrimSpace(block.Code) + "\n```\n\n")
		}
	}

	if len(doc.UnresolvedItems) > 0 {
		b.WriteString("## Unresolved Items\n\n")
		for _, item := range doc.UnresolvedItems {
			b.WriteString("- `" + item.Kind + "`: " + item.Description + "\n")
		}
		b.WriteString("\n")
	}

	return b.String()
}

func countFunctions(doc taiSchema) int {
	total := 0
	for _, module := range doc.Modules {
		total += len(module.Functions)
	}
	return total
}

func init() {
	rootCmd.AddCommand(testCmd)
	rootCmd.AddCommand(docCmd)
	docCmd.Flags().String("format", "markdown", "Output format (markdown, html, pdf)")
	docCmd.Flags().StringP("output", "o", "docs", "Output directory")
}
