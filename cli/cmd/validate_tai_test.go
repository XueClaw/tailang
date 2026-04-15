package cmd

import (
	"os"
	"path/filepath"
	"testing"
)

func TestValidateTaiCommandSuccess(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "valid.tai")
	content := `{
	  "version": "0.1.0",
	  "source": {
	    "provider": "dashscope",
	    "model": "qwen-plus",
	    "temperature": "0"
	  },
	  "modules": [],
	  "code_blocks": [],
	  "unresolved_items": []
	}`

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write .tai file: %v", err)
	}

	if err := validateTaiCmd.RunE(validateTaiCmd, []string{input}); err != nil {
		t.Fatalf("validateTaiCmd returned error: %v", err)
	}
}

func TestValidateTaiCommandFailure(t *testing.T) {
	tempDir := t.TempDir()
	input := filepath.Join(tempDir, "invalid.tai")
	content := `{
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

	if err := os.WriteFile(input, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write .tai file: %v", err)
	}

	if err := validateTaiCmd.RunE(validateTaiCmd, []string{input}); err == nil {
		t.Fatal("expected validateTaiCmd to fail for invalid .tai")
	}
}
