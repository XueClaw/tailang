package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/spf13/cobra"
)

var docCmd = &cobra.Command{
	Use:   "doc [path]",
	Short: "Generate documentation",
	Long: `Generate documentation from .tai-first Tailang sources.

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
		if strings.HasSuffix(path, ".meng") {
			if _, err := os.Stat(key + ".tai"); err == nil {
				return nil
			}
			if _, exists := entries[key]; exists {
				return nil
			}
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
	decoded, err := decodeUTF8Source(content)
	if err != nil {
		return taiSchema{}, err
	}

	taiText := decoded
	trimmed := strings.TrimSpace(taiText)
	if !looksLikeLegacyTaiJSON(trimmed) && !isTextualTaiSource(trimmed) && !strings.HasSuffix(strings.ToLower(path), ".tai") {
		taiText, err = precompileMeng(taiText)
		if err != nil {
			return taiSchema{}, fmt.Errorf("precompile source: %w", err)
		}
	}

	if !looksLikeLegacyTaiJSON(strings.TrimSpace(taiText)) {
		return taiSchema{
			Version: "v0.3",
			Source: taiSource{
				Provider:    "textual-tai",
				Model:       "manual",
				Temperature: "0",
			},
			Modules:         extractTextualTaiModules(taiText),
			CodeBlocks:      extractTextualTaiCodeBlocks(taiText),
			UnresolvedItems: extractTextualTaiUnresolved(taiText),
		}, nil
	}

	var doc taiSchema
	if err := json.Unmarshal([]byte(taiText), &doc); err != nil {
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

func isTextualTaiSource(content string) bool {
	return strings.HasPrefix(content, ".版本 ") ||
		strings.HasPrefix(content, ".程序集 ") ||
		strings.Contains(content, "\n.程序集 ") ||
		strings.Contains(content, "\n.子程序 ")
}

func extractTextualTaiModules(source string) []taiModule {
	lines := strings.Split(source, "\n")
	modules := []taiModule{}
	var currentModule *taiModule
	var currentFunction *taiFunction

	for _, raw := range lines {
		line := strings.TrimSpace(raw)
		switch {
		case strings.HasPrefix(line, ".程序集 "):
			name := strings.TrimSpace(strings.TrimPrefix(line, ".程序集 "))
			modules = append(modules, taiModule{Name: name})
			currentModule = &modules[len(modules)-1]
			currentFunction = nil
		case strings.HasPrefix(line, ".说明 "):
			doc := strings.Trim(strings.TrimSpace(strings.TrimPrefix(line, ".说明 ")), `"`)
			if currentFunction != nil {
				currentFunction.Description = doc
			} else if currentModule != nil {
				currentModule.Description = doc
			}
		case strings.HasPrefix(line, ".子程序 "):
			if currentModule == nil {
				continue
			}
			header := strings.TrimSpace(strings.TrimPrefix(line, ".子程序 "))
			name, params := parseTextualTaiFunctionHeader(header)
			currentModule.Functions = append(currentModule.Functions, taiFunction{Name: name})
			currentFunction = &currentModule.Functions[len(currentModule.Functions)-1]
			currentFunction.Params = append(currentFunction.Params, params...)
		case strings.HasPrefix(line, ".参数 "):
			if currentFunction == nil {
				continue
			}
			header := strings.TrimSpace(strings.TrimPrefix(line, ".参数 "))
			name := strings.TrimSpace(strings.Split(strings.Split(header, "=")[0], ",")[0])
			if name != "" {
				currentFunction.Params = append(currentFunction.Params, name)
			}
		case strings.HasPrefix(line, ".校验 "):
			if currentFunction == nil {
				continue
			}
			currentFunction.Validations = append(currentFunction.Validations, strings.Trim(strings.TrimSpace(strings.TrimPrefix(line, ".校验 ")), `"`))
		}
	}

	return modules
}

func parseTextualTaiFunctionHeader(header string) (string, []string) {
	header = strings.TrimSpace(header)
	if before, _, ok := strings.Cut(header, "->"); ok {
		header = strings.TrimSpace(before)
	}
	if !strings.Contains(header, "(") {
		name := strings.TrimSpace(strings.Split(header, ",")[0])
		return name, nil
	}
	name, rest, _ := strings.Cut(header, "(")
	paramsText, _, _ := strings.Cut(rest, ")")
	params := []string{}
	for _, raw := range strings.Split(paramsText, ",") {
		item := strings.TrimSpace(raw)
		if item == "" {
			continue
		}
		if paramName, _, ok := strings.Cut(item, ":"); ok {
			params = append(params, strings.TrimSpace(paramName))
		} else {
			params = append(params, item)
		}
	}
	return strings.TrimSpace(name), params
}

func extractTextualTaiCodeBlocks(source string) []taiCodeBlock {
	lines := strings.Split(source, "\n")
	blocks := []taiCodeBlock{}
	var language string
	var body []string
	inBlock := false

	for _, raw := range lines {
		line := strings.TrimSpace(raw)
		if inBlock {
			if line == ".代码结束" {
				blocks = append(blocks, taiCodeBlock{
					Language: language,
					Code:     strings.TrimSpace(strings.Join(body, "\n")),
				})
				language = ""
				body = nil
				inBlock = false
				continue
			}
			body = append(body, raw)
			continue
		}

		if strings.HasPrefix(line, ".代码 ") {
			language = strings.TrimSpace(strings.TrimPrefix(line, ".代码 "))
			body = []string{}
			inBlock = true
		}
	}

	return blocks
}

func extractTextualTaiUnresolved(source string) []taiUnresolvedItem {
	lines := strings.Split(source, "\n")
	items := []taiUnresolvedItem{}
	for _, raw := range lines {
		line := strings.TrimSpace(raw)
		if !strings.HasPrefix(line, ".待定 ") {
			continue
		}
		rest := strings.TrimSpace(strings.TrimPrefix(line, ".待定 "))
		if strings.Contains(rest, ",") {
			parts := strings.SplitN(rest, ",", 2)
			items = append(items, taiUnresolvedItem{
				Kind:        strings.TrimSpace(parts[0]),
				Description: strings.Trim(strings.TrimSpace(parts[1]), `"`),
			})
			continue
		}
		parts := strings.SplitN(rest, " ", 2)
		if len(parts) == 2 {
			items = append(items, taiUnresolvedItem{
				Kind:        strings.TrimSpace(parts[0]),
				Description: strings.Trim(strings.TrimSpace(parts[1]), `"`),
			})
		}
	}
	return items
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
	rootCmd.AddCommand(docCmd)
	docCmd.Flags().String("format", "markdown", "Output format (markdown, html, pdf)")
	docCmd.Flags().StringP("output", "o", "docs", "Output directory")
}
