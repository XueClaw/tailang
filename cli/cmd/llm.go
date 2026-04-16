package cmd

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"
)

const (
	defaultLLMProvider    = "dashscope"
	defaultTemperature    = 0.0
	defaultTimeoutSeconds = 60
)

type llmConfig struct {
	Provider    string
	BaseURL     string
	Model       string
	APIKey      string
	Temperature float64
	Timeout     time.Duration
	MaxTokens   int
}

type llmProvider interface {
	Name() string
	Precompile(content string, config llmConfig) (string, error)
}

type chatMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type chatCompletionsRequest struct {
	Model       string        `json:"model"`
	Messages    []chatMessage `json:"messages"`
	Temperature float64       `json:"temperature"`
	Stream      bool          `json:"stream"`
	MaxTokens   int           `json:"max_tokens,omitempty"`
}

type chatCompletionsResponse struct {
	Choices []struct {
		Message chatMessage `json:"message"`
	} `json:"choices"`
}

type taiSchema struct {
	Version         string             `json:"version"`
	Source          taiSource          `json:"source"`
	Modules         []taiModule        `json:"modules"`
	CodeBlocks      []taiCodeBlock     `json:"code_blocks"`
	UnresolvedItems []taiUnresolvedItem `json:"unresolved_items"`
}

type taiSource struct {
	Provider    string `json:"provider"`
	Model       string `json:"model"`
	Temperature string `json:"temperature"`
}

type taiModule struct {
	Name        string        `json:"name"`
	Description string        `json:"description"`
	Functions   []taiFunction `json:"functions"`
}

type taiFunction struct {
	Name        string   `json:"name"`
	Params      []string `json:"params"`
	Description string   `json:"description"`
	Validations []string `json:"validations"`
}

type taiCodeBlock struct {
	Language string `json:"language"`
	Code     string `json:"code"`
	LinkedTo *string `json:"linked_to,omitempty"`
}

type taiUnresolvedItem struct {
	Kind        string `json:"kind"`
	Description string `json:"description"`
}

type taiJSONSchemaDoc struct {
	Required []string `json:"required"`
	Defs     struct {
		Source struct {
			Required []string `json:"required"`
		} `json:"source"`
		Module struct {
			Required []string `json:"required"`
		} `json:"module"`
		Function struct {
			Required []string `json:"required"`
		} `json:"function"`
		CodeBlock struct {
			Required []string `json:"required"`
		} `json:"codeBlock"`
		UnresolvedItem struct {
			Required []string `json:"required"`
		} `json:"unresolvedItem"`
	} `json:"$defs"`
}

var (
	taiSchemaOnce sync.Once
	taiSchemaDoc  taiJSONSchemaDoc
	taiSchemaErr  error
)

type dashScopeProvider struct{}
type ollamaProvider struct{}
type customProvider struct{}

func loadLLMConfigFromEnv() (llmConfig, error) {
	provider := strings.ToLower(strings.TrimSpace(getEnvOrDefault("TAILANG_LLM_PROVIDER", defaultLLMProvider)))

	config := llmConfig{
		Provider:    provider,
		Temperature: getEnvFloat("TAILANG_LLM_TEMPERATURE", defaultTemperature),
		Timeout:     time.Duration(getEnvInt("TAILANG_LLM_TIMEOUT_SECS", defaultTimeoutSeconds)) * time.Second,
		MaxTokens:   getEnvInt("TAILANG_LLM_MAX_TOKENS", 0),
	}

	switch provider {
	case "dashscope", "bailian", "":
		config.Provider = "dashscope"
		config.BaseURL = strings.TrimRight(firstNonEmpty(
			os.Getenv("DASHSCOPE_BASE_URL"),
			os.Getenv("TAILANG_LLM_BASE_URL"),
			"https://dashscope.aliyuncs.com/compatible-mode/v1",
		), "/")
		config.Model = firstNonEmpty(
			os.Getenv("TAILANG_LLM_MODEL"),
			"qwen-plus",
		)
		config.APIKey = firstNonEmpty(
			os.Getenv("DASHSCOPE_API_KEY"),
			os.Getenv("TAILANG_LLM_API_KEY"),
		)
		if config.APIKey == "" {
			return llmConfig{}, fmt.Errorf("missing DashScope API key: set DASHSCOPE_API_KEY or TAILANG_LLM_API_KEY")
		}
	case "ollama":
		config.BaseURL = strings.TrimRight(firstNonEmpty(
			os.Getenv("OLLAMA_BASE_URL"),
			os.Getenv("TAILANG_LLM_BASE_URL"),
			"http://localhost:11434/v1",
		), "/")
		config.Model = firstNonEmpty(
			os.Getenv("TAILANG_LLM_MODEL"),
			"qwen2.5-coder:latest",
		)
		config.APIKey = firstNonEmpty(
			os.Getenv("OLLAMA_API_KEY"),
			os.Getenv("TAILANG_LLM_API_KEY"),
		)
	case "custom":
		config.BaseURL = strings.TrimRight(os.Getenv("TAILANG_LLM_BASE_URL"), "/")
		config.Model = firstNonEmpty(os.Getenv("TAILANG_LLM_MODEL"), "tailang-provider")
		config.APIKey = os.Getenv("TAILANG_LLM_API_KEY")
		if config.BaseURL == "" {
			return llmConfig{}, fmt.Errorf("missing custom provider base URL: set TAILANG_LLM_BASE_URL")
		}
	default:
		return llmConfig{}, fmt.Errorf("unsupported provider: %s", provider)
	}

	return config, nil
}

func precompileMeng(content string) (string, error) {
	config, err := loadLLMConfigFromEnv()
	if err != nil {
		return "", err
	}

	provider, err := createLLMProvider(config.Provider)
	if err != nil {
		return "", err
	}

	raw, err := provider.Precompile(content, config)
	if err != nil {
		return "", err
	}

	return normalizeTaiOutput(raw, config)
}

func createLLMProvider(name string) (llmProvider, error) {
	switch name {
	case "dashscope":
		return dashScopeProvider{}, nil
	case "ollama":
		return ollamaProvider{}, nil
	case "custom":
		return customProvider{}, nil
	default:
		return nil, fmt.Errorf("unsupported provider: %s", name)
	}
}

func (dashScopeProvider) Name() string { return "dashscope" }
func (ollamaProvider) Name() string    { return "ollama" }
func (customProvider) Name() string    { return "custom" }

func (p dashScopeProvider) Precompile(content string, config llmConfig) (string, error) {
	return callChatCompletions(content, config, true)
}

func (p ollamaProvider) Precompile(content string, config llmConfig) (string, error) {
	return callChatCompletions(content, config, false)
}

func (p customProvider) Precompile(content string, config llmConfig) (string, error) {
	return callChatCompletions(content, config, config.APIKey != "")
}

func callChatCompletions(content string, config llmConfig, requireAuth bool) (string, error) {
	requestBody := chatCompletionsRequest{
		Model:       config.Model,
		Messages:    buildPromptMessages(content),
		Temperature: config.Temperature,
		Stream:      false,
		MaxTokens:   config.MaxTokens,
	}

	payload, err := json.Marshal(requestBody)
	if err != nil {
		return "", fmt.Errorf("serialize request failed: %w", err)
	}

	req, err := http.NewRequest("POST", config.BaseURL+"/chat/completions", bytes.NewBuffer(payload))
	if err != nil {
		return "", fmt.Errorf("create request failed: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")
	if requireAuth {
		if config.APIKey == "" {
			return "", fmt.Errorf("missing API key for provider %s", config.Provider)
		}
		req.Header.Set("Authorization", "Bearer "+config.APIKey)
	} else if config.APIKey != "" {
		req.Header.Set("Authorization", "Bearer "+config.APIKey)
	}

	client := &http.Client{Timeout: config.Timeout}
	resp, err := client.Do(req)
	if err != nil {
		return "", fmt.Errorf("call provider %s failed: %w", config.Provider, err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return "", fmt.Errorf("provider %s error: %s", config.Provider, string(body))
	}

	var apiResp chatCompletionsResponse
	if err := json.NewDecoder(resp.Body).Decode(&apiResp); err != nil {
		return "", fmt.Errorf("parse response failed: %w", err)
	}

	if len(apiResp.Choices) == 0 {
		return "", fmt.Errorf("empty response from provider %s", config.Provider)
	}

	return apiResp.Choices[0].Message.Content, nil
}

func buildPromptMessages(content string) []chatMessage {
	return []chatMessage{
		{
			Role: "system",
			Content: `你是 Tailang 的预编译器，不是示例拼接器，也不是条件判断打印机。

任务目标：
- 将 .meng 中的自然语言意图整理为稳定、可审查、可编译的结构化表示
- 忠实保留用户语义，不降级为机械的“如果/验证/返回”样板
- 保留原有代码块、多语言补充和表达风格中的有效信息

硬性约束：
- 不得为了凑模板臆造模块、函数、参数、验证规则、分支逻辑
- 不得把没有出现的业务规则补成示例代码
- 不得把编程语言本体收缩成“输入 -> 条件判断 -> 打印输出”的玩具模型
- 输出必须是 JSON，对象结构必须符合指定 schema
- 信息不足时保留原描述并放入 unresolved_items
- 不要输出解释、前后缀、Markdown 代码块，只输出 JSON`,
		},
		{
			Role: "user",
			Content: fmt.Sprintf(`请将以下 Tailang .meng 内容预编译为结构化 .tai JSON。

输出 schema：
{
  "version": "string",
  "source": {
    "provider": "string",
    "model": "string",
    "temperature": "string"
  },
  "modules": [
    {
      "name": "string",
      "description": "string",
      "functions": [
        {
          "name": "string",
          "params": ["string"],
          "description": "string",
          "validations": ["string"]
        }
      ]
    }
  ],
  "code_blocks": [
    {
      "language": "string",
      "code": "string",
      "linked_to": "string"
    }
  ],
  "unresolved_items": [
    {
      "kind": "string",
      "description": "string"
    }
  ]
}

规则：
1. 只提取真实存在或可稳定推断的结构
2. 不要凭空补充条件分支、验证规则、打印语句或演示逻辑
3. 保留所有代码块
4. 若无法确定模块或函数，不要虚构，用 unresolved_items 表达

用户输入：
%s`, content),
		},
	}
}

func normalizeTaiOutput(raw string, config llmConfig) (string, error) {
	trimmed := strings.TrimSpace(raw)
	trimmed = strings.TrimPrefix(trimmed, "```json")
	trimmed = strings.TrimPrefix(trimmed, "```")
	trimmed = strings.TrimSuffix(trimmed, "```")
	trimmed = strings.TrimSpace(trimmed)

	var doc taiSchema
	if err := json.Unmarshal([]byte(trimmed), &doc); err != nil {
		return "", fmt.Errorf("provider returned invalid .tai JSON: %w", err)
	}

	if doc.Version == "" {
		doc.Version = "0.1.0"
	}
	if doc.Source.Provider == "" {
		doc.Source.Provider = config.Provider
	}
	if doc.Source.Model == "" {
		doc.Source.Model = config.Model
	}
	if doc.Source.Temperature == "" {
		doc.Source.Temperature = strconv.FormatFloat(config.Temperature, 'f', -1, 64)
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

	return renderTaiTextFromSchema(doc), nil
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		if strings.TrimSpace(value) != "" {
			return strings.TrimSpace(value)
		}
	}
	return ""
}

func getEnvOrDefault(key string, fallback string) string {
	if value := strings.TrimSpace(os.Getenv(key)); value != "" {
		return value
	}
	return fallback
}

func getEnvInt(key string, fallback int) int {
	value := strings.TrimSpace(os.Getenv(key))
	if value == "" {
		return fallback
	}

	parsed, err := strconv.Atoi(value)
	if err != nil {
		return fallback
	}
	return parsed
}

func getEnvFloat(key string, fallback float64) float64 {
	value := strings.TrimSpace(os.Getenv(key))
	if value == "" {
		return fallback
	}

	parsed, err := strconv.ParseFloat(value, 64)
	if err != nil {
		return fallback
	}
	return parsed
}

func validateTaiAgainstSchema(doc *taiSchema) error {
	schema, err := loadTaiSchema()
	if err != nil {
		return err
	}

	for _, field := range schema.Required {
		switch field {
		case "version":
			if strings.TrimSpace(doc.Version) == "" {
				return fmt.Errorf("invalid .tai schema: version is required")
			}
		case "source":
			if err := requireStringFields("source", map[string]string{
				"provider":    doc.Source.Provider,
				"model":       doc.Source.Model,
				"temperature": doc.Source.Temperature,
			}, schema.Defs.Source.Required); err != nil {
				return err
			}
		case "modules":
			for i := range doc.Modules {
				if doc.Modules[i].Functions == nil {
					doc.Modules[i].Functions = []taiFunction{}
				}
				if err := requireStringFields(fmt.Sprintf("modules[%d]", i), map[string]string{
					"name":        doc.Modules[i].Name,
					"description": doc.Modules[i].Description,
				}, []string{"name", "description"}); err != nil {
					return err
				}
				if err := requireArrayFields(fmt.Sprintf("modules[%d]", i), map[string]bool{
					"functions": doc.Modules[i].Functions != nil,
				}, []string{"functions"}); err != nil {
					return err
				}
				for j := range doc.Modules[i].Functions {
					if doc.Modules[i].Functions[j].Params == nil {
						doc.Modules[i].Functions[j].Params = []string{}
					}
					if doc.Modules[i].Functions[j].Validations == nil {
						doc.Modules[i].Functions[j].Validations = []string{}
					}
					if err := requireStringFields(fmt.Sprintf("modules[%d].functions[%d]", i, j), map[string]string{
						"name":        doc.Modules[i].Functions[j].Name,
						"description": doc.Modules[i].Functions[j].Description,
					}, []string{"name", "description"}); err != nil {
						return err
					}
					if err := requireArrayFields(fmt.Sprintf("modules[%d].functions[%d]", i, j), map[string]bool{
						"params":      doc.Modules[i].Functions[j].Params != nil,
						"validations": doc.Modules[i].Functions[j].Validations != nil,
					}, []string{"params", "validations"}); err != nil {
						return err
					}
				}
			}
		case "code_blocks":
			for i := range doc.CodeBlocks {
				if err := requireStringFields(fmt.Sprintf("code_blocks[%d]", i), map[string]string{
					"language": doc.CodeBlocks[i].Language,
					"code":     doc.CodeBlocks[i].Code,
				}, schema.Defs.CodeBlock.Required); err != nil {
					return err
				}
			}
		case "unresolved_items":
			for i := range doc.UnresolvedItems {
				if err := requireStringFields(fmt.Sprintf("unresolved_items[%d]", i), map[string]string{
					"kind":        doc.UnresolvedItems[i].Kind,
					"description": doc.UnresolvedItems[i].Description,
				}, schema.Defs.UnresolvedItem.Required); err != nil {
					return err
				}
			}
		}
	}

	return nil
}

func requireStringFields(prefix string, values map[string]string, required []string) error {
	for _, field := range required {
		if strings.TrimSpace(values[field]) == "" {
			return fmt.Errorf("invalid .tai schema: %s.%s is required", prefix, field)
		}
	}
	return nil
}

func requireArrayFields(prefix string, values map[string]bool, required []string) error {
	for _, field := range required {
		if !values[field] {
			return fmt.Errorf("invalid .tai schema: %s.%s is required", prefix, field)
		}
	}
	return nil
}

func loadTaiSchema() (taiJSONSchemaDoc, error) {
	taiSchemaOnce.Do(func() {
		path, err := findTaiSchemaPath()
		if err != nil {
			taiSchemaErr = err
			return
		}

		content, err := os.ReadFile(path)
		if err != nil {
			taiSchemaErr = fmt.Errorf("read .tai schema failed: %w", err)
			return
		}

		if err := json.Unmarshal(content, &taiSchemaDoc); err != nil {
			taiSchemaErr = fmt.Errorf("parse .tai schema failed: %w", err)
		}
	})

	return taiSchemaDoc, taiSchemaErr
}

func findTaiSchemaPath() (string, error) {
	wd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("get working directory failed: %w", err)
	}

	dir := wd
	for {
		candidate := filepath.Join(dir, "docs", "spec", "tai.schema.json")
		if _, err := os.Stat(candidate); err == nil {
			return candidate, nil
		}

		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}

	return "", fmt.Errorf("unable to locate docs/spec/tai.schema.json from %s", wd)
}

func renderTaiTextFromSchema(doc taiSchema) string {
	var b strings.Builder
	version := strings.TrimSpace(doc.Version)
	if version == "" {
		version = "3"
	}
	b.WriteString(".版本 ")
	b.WriteString(version)
	b.WriteString("\n")

	if strings.TrimSpace(doc.Source.Provider) != "" {
		b.WriteString(fmt.Sprintf(".元信息 提供者 = %q\n", doc.Source.Provider))
	}
	if strings.TrimSpace(doc.Source.Model) != "" {
		b.WriteString(fmt.Sprintf(".元信息 模型 = %q\n", doc.Source.Model))
	}
	if strings.TrimSpace(doc.Source.Temperature) != "" {
		b.WriteString(fmt.Sprintf(".元信息 温度 = %q\n", doc.Source.Temperature))
	}

	if len(doc.Modules) > 0 {
		b.WriteString("\n")
	}

	for moduleIndex, module := range doc.Modules {
		b.WriteString(".程序集 ")
		b.WriteString(strings.TrimSpace(module.Name))
		b.WriteString("\n")
		if strings.TrimSpace(module.Description) != "" {
			b.WriteString(fmt.Sprintf(".说明 %q\n", module.Description))
		}
		if len(module.Functions) > 0 {
			b.WriteString("\n")
		}

		for fnIndex, fn := range module.Functions {
			b.WriteString(".子程序 ")
			b.WriteString(strings.TrimSpace(fn.Name))
			b.WriteString("\n")
			for _, param := range fn.Params {
				if strings.TrimSpace(param) == "" {
					continue
				}
				b.WriteString(".参数 ")
				b.WriteString(strings.TrimSpace(param))
				b.WriteString(", 任意型\n")
			}
			if strings.TrimSpace(fn.Description) != "" {
				b.WriteString(fmt.Sprintf(".说明 %q\n", fn.Description))
			}
			for _, validation := range fn.Validations {
				if strings.TrimSpace(validation) == "" {
					continue
				}
				b.WriteString(fmt.Sprintf(".校验 %q\n", validation))
			}
			b.WriteString(".待定 实现, ")
			b.WriteString(fmt.Sprintf("%q\n", fmt.Sprintf("子程序 %s 尚未生成原生执行语法", strings.TrimSpace(fn.Name))))

			if linkedBlocks := collectLinkedCodeBlocks(doc.CodeBlocks, fn.Name); len(linkedBlocks) > 0 {
				b.WriteString("\n")
				for _, block := range linkedBlocks {
					b.WriteString(".代码 ")
					b.WriteString(strings.TrimSpace(block.Language))
					b.WriteString("\n")
					b.WriteString(strings.TrimSpace(block.Code))
					b.WriteString("\n.代码结束\n")
				}
			}

			if fnIndex != len(module.Functions)-1 {
				b.WriteString("\n")
			}
		}

		if len(module.Functions) == 0 {
			b.WriteString(".待定 模块, ")
			b.WriteString(fmt.Sprintf("%q\n", fmt.Sprintf("程序集 %s 尚未生成子程序", strings.TrimSpace(module.Name))))
		}

		if moduleIndex != len(doc.Modules)-1 {
			b.WriteString("\n")
		}
	}

	for _, block := range doc.CodeBlocks {
		if block.LinkedTo != nil && strings.TrimSpace(*block.LinkedTo) != "" {
			continue
		}
		if b.Len() > 0 && !strings.HasSuffix(b.String(), "\n\n") {
			b.WriteString("\n")
		}
		b.WriteString(".代码 ")
		b.WriteString(strings.TrimSpace(block.Language))
		b.WriteString("\n")
		b.WriteString(strings.TrimSpace(block.Code))
		b.WriteString("\n.代码结束\n")
	}

	if len(doc.UnresolvedItems) > 0 {
		if !strings.HasSuffix(b.String(), "\n") {
			b.WriteString("\n")
		}
		for _, item := range doc.UnresolvedItems {
			b.WriteString(".待定 ")
			b.WriteString(strings.TrimSpace(item.Kind))
			b.WriteString(", ")
			b.WriteString(fmt.Sprintf("%q\n", item.Description))
		}
	}

	return strings.TrimSpace(b.String()) + "\n"
}

func collectLinkedCodeBlocks(blocks []taiCodeBlock, functionName string) []taiCodeBlock {
	result := make([]taiCodeBlock, 0)
	for _, block := range blocks {
		if block.LinkedTo == nil {
			continue
		}
		if strings.TrimSpace(*block.LinkedTo) == strings.TrimSpace(functionName) {
			result = append(result, block)
		}
	}
	return result
}
