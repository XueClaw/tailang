package cmd

import (
	"os"
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

	if !strings.Contains(out, `"provider": "ollama"`) {
		t.Fatalf("expected provider default to be applied, got: %s", out)
	}
	if !strings.Contains(out, `"model": "qwen2.5-coder:latest"`) {
		t.Fatalf("expected model default to be applied, got: %s", out)
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

func TestCompilePlaceholderFromNormalizedTaiFlow(t *testing.T) {
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
	if err := compileToExecutable(ir, output, "windows"); err != nil {
		t.Fatalf("compileToExecutable returned error: %v", err)
	}

	if _, err := os.Stat(output); err != nil {
		t.Fatalf("expected output file to exist: %v", err)
	}
}
