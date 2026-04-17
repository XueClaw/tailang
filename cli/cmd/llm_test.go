package cmd

import (
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestNormalizeTaiOutputAppliesDefaults(t *testing.T) {
	raw := `{
	  "version": "",
	  "source": {
	    "provider": "",
	    "model": "",
	    "temperature": ""
	  },
	  "modules": [],
	  "code_blocks": [],
	  "unresolved_items": []
	}`

	out, err := normalizeTaiOutput(raw, llmConfig{
		Provider:    "ollama",
		Model:       "qwen2.5-coder:latest",
		Temperature: 0,
		Timeout:     30 * time.Second,
	})
	if err != nil {
		t.Fatalf("normalizeTaiOutput returned error: %v", err)
	}

	if !strings.Contains(out, `.元信息 提供者 = "ollama"`) {
		t.Fatalf("expected provider default to be applied, got: %s", out)
	}
	if !strings.Contains(out, `.元信息 模型 = "qwen2.5-coder:latest"`) {
		t.Fatalf("expected model default to be applied, got: %s", out)
	}
	if !strings.Contains(out, ".版本 0.1.0") {
		t.Fatalf("expected textual .tai output, got: %s", out)
	}
}

func TestNormalizeTaiOutputRejectsEmptyModuleName(t *testing.T) {
	raw := `{
	  "version": "0.1.0",
	  "source": {
	    "provider": "dashscope",
	    "model": "qwen-plus",
	    "temperature": "0"
	  },
	  "modules": [
	    {
	      "name": "",
	      "description": "bad",
	      "functions": []
	    }
	  ],
	  "code_blocks": [],
	  "unresolved_items": []
	}`

	_, err := normalizeTaiOutput(raw, llmConfig{
		Provider:    "dashscope",
		Model:       "qwen-plus",
		Temperature: 0,
	})
	if err == nil {
		t.Fatal("expected schema validation error")
	}
}

func TestExtractCodeBlocksFromTai(t *testing.T) {
	input := `{
	  "version": "0.1.0",
	  "source": {
	    "provider": "dashscope",
	    "model": "qwen-plus",
	    "temperature": "0"
	  },
	  "modules": [],
	  "code_blocks": [
	    {
	      "language": "python",
	      "code": "print(\"hello\")",
	      "linked_to": "main"
	    }
	  ],
	  "unresolved_items": []
	}`

	blocks, err := extractCodeBlocksFromTai(input)
	if err != nil {
		t.Fatalf("extractCodeBlocksFromTai returned error: %v", err)
	}
	if len(blocks) != 1 {
		t.Fatalf("expected 1 code block, got %d", len(blocks))
	}
	if blocks[0].Language != "python" {
		t.Fatalf("unexpected language: %s", blocks[0].Language)
	}
}

func TestExtractCodeBlocksFromTaiWithoutLinkedTo(t *testing.T) {
	input := `{
	  "version": "0.1.0",
	  "source": {
	    "provider": "dashscope",
	    "model": "qwen-plus",
	    "temperature": "0"
	  },
	  "modules": [],
	  "code_blocks": [
	    {
	      "language": "python",
	      "code": "print(\"hello\")"
	    }
	  ],
	  "unresolved_items": []
	}`

	blocks, err := extractCodeBlocksFromTai(input)
	if err != nil {
		t.Fatalf("extractCodeBlocksFromTai returned error: %v", err)
	}
	if len(blocks) != 1 {
		t.Fatalf("expected 1 code block, got %d", len(blocks))
	}
	if blocks[0].Code != "print(\"hello\")" {
		t.Fatalf("unexpected code: %s", blocks[0].Code)
	}
}

func TestGenerateIRFromNormalizedTaiFlow(t *testing.T) {
	input := `{
	  "version": "0.1.0",
	  "source": {
	    "provider": "dashscope",
	    "model": "qwen-plus",
	    "temperature": "0"
	  },
	  "modules": [
	    {
	      "name": "auth",
	      "description": "登录模块",
	      "functions": [
	        {
	          "name": "login",
	          "params": ["email", "password"],
	          "description": "邮箱密码登录",
	          "validations": []
	        }
	      ]
	    }
	  ],
	  "code_blocks": [
	    {
	      "language": "python",
	      "code": "print(\"hello\")",
	      "linked_to": "login"
	    }
	  ],
	  "unresolved_items": []
	}`

	blocks, err := extractCodeBlocksFromTai(input)
	if err != nil {
		t.Fatalf("extractCodeBlocksFromTai returned error: %v", err)
	}

	ir, err := generateIR(input, blocks)
	if err != nil {
		t.Fatalf("generateIR returned error: %v", err)
	}

	if ir.Source == "" {
		t.Fatal("expected IR source to be preserved")
	}
	if len(ir.CodeBlocks) != 1 {
		t.Fatalf("expected 1 IR code block, got %d", len(ir.CodeBlocks))
	}
}

func TestCompileToExecutableProducesNativeExecutable(t *testing.T) {
	input := `{
	  "version": "0.1.0",
	  "source": {
	    "provider": "dashscope",
	    "model": "qwen-plus",
	    "temperature": "0"
	  },
	  "modules": [
	    {
	      "name": "auth",
	      "description": "登录模块",
	      "functions": [
	        {
	          "name": "login",
	          "params": ["email", "password"],
	          "description": "邮箱密码登录",
	          "validations": []
	        }
	      ]
	    }
	  ],
	  "code_blocks": [
	    {
	      "language": "python",
	      "code": "print(\"hello\")",
	      "linked_to": "login"
	    }
	  ],
	  "unresolved_items": []
	}`

	blocks, err := extractCodeBlocksFromTai(input)
	if err != nil {
		t.Fatalf("extractCodeBlocksFromTai returned error: %v", err)
	}

	ir, err := generateIR(input, blocks)
	if err != nil {
		t.Fatalf("generateIR returned error: %v", err)
	}

	tempDir := t.TempDir()
	output := filepath.Join(tempDir, "tailang-test.exe")
	err = compileToExecutable(ir, output, "windows")
	if err != nil {
		t.Fatalf("expected native compilation to succeed, got %v", err)
	}
	if _, err := os.Stat(output); err != nil {
		t.Fatalf("expected native executable output, got stat error: %v", err)
	}
}

func TestRenderTaiTextFromSchema(t *testing.T) {
	doc := taiSchema{
		Version: "3",
		Source: taiSource{
			Provider:    "custom",
			Model:       "test-model",
			Temperature: "0",
		},
		Modules: []taiModule{
			{
				Name:        "认证",
				Description: "认证流程",
				Functions: []taiFunction{
					{
						Name:        "登录",
						Params:      []string{"邮箱", "密码"},
						Description: "邮箱密码登录",
						Validations: []string{"邮箱不能为空"},
					},
				},
			},
		},
		CodeBlocks: []taiCodeBlock{
			{Language: "Rust", Code: "println!(\"hello\");"},
		},
		UnresolvedItems: []taiUnresolvedItem{
			{Kind: "规则", Description: "缺少密码复杂度规则"},
		},
	}

	out := renderTaiTextFromSchema(doc)
	if !strings.Contains(out, ".程序集 认证") {
		t.Fatalf("expected module declaration, got: %s", out)
	}
	if !strings.Contains(out, ".子程序 登录") {
		t.Fatalf("expected function declaration, got: %s", out)
	}
	if !strings.Contains(out, ".代码 Rust") {
		t.Fatalf("expected code block, got: %s", out)
	}
	if !strings.Contains(out, `.待定 规则, "缺少密码复杂度规则"`) {
		t.Fatalf("expected unresolved declaration, got: %s", out)
	}
}
