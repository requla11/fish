package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestContractReadableFromPyWorker verifies that the TaskEvent contract owned
// by py-worker is present, parseable, and declares the fields this service
// relies on. It reaches across the project boundary on purpose — this file is
// also the evidence fish uses to infer the go-service → py-worker dependency.
func TestContractReadableFromPyWorker(t *testing.T) {
	path := filepath.Join("..", "py-worker", "contracts", "events.schema.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("shared contract missing: %v", err)
	}

	if !strings.Contains(string(data), `"TaskEvent"`) {
		t.Error("contract no longer describes TaskEvent; update consumers")
	}
	for _, field := range []string{"id", "topic", "created_at"} {
		if !strings.Contains(string(data), `"`+field+`"`) {
			t.Errorf("contract lost required field %q", field)
		}
	}
}

func TestLoadSchema(t *testing.T) {
	schema, err := loadSchema()
	if err != nil {
		t.Fatalf("loadSchema: %v", err)
	}
	if title, _ := schema["title"].(string); title != "TaskEvent" {
		t.Errorf("unexpected schema title: %q", title)
	}
}
