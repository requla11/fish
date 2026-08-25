// go-service — HTTP API of the polyglot demo stack.
//
// Contract-first: the event schema is owned by py-worker and read from
// ../py-worker/contracts/events.schema.json. The relative path is the real
// cross-project dependency that `fish build` infers automatically.
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
)

// schemaPath points at the contract owned by py-worker. Overridable for
// deployments that relocate it, but the default keeps the monorepo layout.
var schemaPath = func() string {
	if p := os.Getenv("EVENTS_SCHEMA_PATH"); p != "" {
		return p
	}
	return "../py-worker/contracts/events.schema.json"
}()

func loadSchema() (map[string]any, error) {
	data, err := os.ReadFile(schemaPath)
	if err != nil {
		return nil, fmt.Errorf("read event schema from py-worker: %w", err)
	}
	var schema map[string]any
	if err := json.Unmarshal(data, &schema); err != nil {
		return nil, fmt.Errorf("parse event schema: %w", err)
	}
	return schema, nil
}

func handleHealth(w http.ResponseWriter, _ *http.Request) {
	schema, err := loadSchema()
	status, detail := "ok", "contract loaded"
	if err != nil {
		status, detail = "degraded", err.Error()
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"service":     "go-service",
		"status":      status,
		"contract":    detail,
		"schemaTitle": schema["title"],
	})
}

func main() {
	http.HandleFunc("/health", handleHealth)
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintf(w, "Go Service: Hello from Go! Endpoint: %s", r.URL.Path)
	})

	fmt.Println("🚀 Go Service starting on port 8081...")
	log.Fatal(http.ListenAndServe(":8081", nil))
}
