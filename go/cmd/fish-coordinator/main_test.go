package main

import (
	"bytes"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/requla11/fish/go/pkg/coordinator"
	fishv1 "github.com/requla11/fish/go/pkg/proto/v1"
)

func setupTestServer() (*http.ServeMux, *coordinator.NodeRegistry, *coordinator.TaskQueue) {
	registry := coordinator.NewNodeRegistry(30 * time.Second)
	taskQueue := coordinator.NewTaskQueue()
	mux := http.NewServeMux()

	mux.HandleFunc("/api/v1/workers", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodPost && r.Header.Get("Content-Type") == "application/x-protobuf" {
			body, err := io.ReadAll(r.Body)
			if err != nil {
				http.Error(w, err.Error(), http.StatusBadRequest)
				return
			}
			var reg fishv1.WorkerRegistration
			if err := reg.Decode(body); err != nil {
				http.Error(w, err.Error(), http.StatusBadRequest)
				return
			}
			node := coordinator.WorkerNode{
				ID:                  reg.WorkerID,
				Address:             reg.Address,
				CPUCores:            int(reg.CPUCores),
				MemoryBytes:         reg.MemoryBytes,
				SupportedToolchains: reg.SupportedToolchains,
				Tags:                reg.Tags,
			}
			if err := registry.Register(&node); err != nil {
				http.Error(w, err.Error(), http.StatusBadRequest)
				return
			}
			ack := fishv1.HeartbeatAck{
				Accepted:                true,
				NextHeartbeatIntervalMs: 5000,
			}
			w.Header().Set("Content-Type", "application/x-protobuf")
			w.WriteHeader(http.StatusCreated)
			_, _ = w.Write(ack.Encode())
			return
		}
	})

	mux.HandleFunc("/api/v1/tasks", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodPost && r.Header.Get("Content-Type") == "application/x-protobuf" {
			body, err := io.ReadAll(r.Body)
			if err != nil {
				http.Error(w, err.Error(), http.StatusBadRequest)
				return
			}
			var buildTask fishv1.BuildTask
			if err := buildTask.Decode(body); err != nil {
				http.Error(w, err.Error(), http.StatusBadRequest)
				return
			}
			task := coordinator.QueuedTask{
				TaskID:    buildTask.ID,
				Toolchain: buildTask.Toolchain,
				Priority:  10,
				Weight:    1.0,
			}
			taskQueue.Push(task)
			w.WriteHeader(http.StatusAccepted)
			return
		}

		if r.Method == http.MethodGet && r.Header.Get("Accept") == "application/x-protobuf" {
			toolchain := r.URL.Query().Get("toolchain")
			task, err := taskQueue.PopForToolchain(toolchain)
			if err != nil {
				http.Error(w, err.Error(), http.StatusNotFound)
				return
			}
			protoTask := fishv1.BuildTask{
				ID:        task.TaskID,
				Toolchain: task.Toolchain,
			}
			w.Header().Set("Content-Type", "application/x-protobuf")
			_, _ = w.Write(protoTask.Encode())
			return
		}
	})

	return mux, registry, taskQueue
}

func TestProtobufWorkerRegistrationEndpoint(t *testing.T) {
	mux, registry, _ := setupTestServer()
	ts := httptest.NewServer(mux)
	defer ts.Close()

	reg := fishv1.WorkerRegistration{
		WorkerID:            "worker-http-proto",
		Address:             "127.0.0.1:9090",
		CPUCores:            16,
		MemoryBytes:         34359738368,
		SupportedToolchains: []string{"rust", "go"},
		Tags:                map[string]string{"zone": "us-west"},
	}

	payload := reg.Encode()
	req, err := http.NewRequest(http.MethodPost, ts.URL+"/api/v1/workers", bytes.NewReader(payload))
	if err != nil {
		t.Fatalf("failed to create request: %v", err)
	}
	req.Header.Set("Content-Type", "application/x-protobuf")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusCreated {
		t.Fatalf("expected status 201, got %d", resp.StatusCode)
	}

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("failed to read response: %v", err)
	}

	var ack fishv1.HeartbeatAck
	if err := ack.Decode(respBody); err != nil {
		t.Fatalf("failed to decode HeartbeatAck: %v", err)
	}

	if !ack.Accepted || ack.NextHeartbeatIntervalMs != 5000 {
		t.Fatalf("unexpected ack response: %+v", ack)
	}

	workers := registry.ListHealthyWorkers()
	found := false
	for _, w := range workers {
		if w.ID == "worker-http-proto" {
			found = true
			if w.Address != "127.0.0.1:9090" || w.CPUCores != 16 {
				t.Fatalf("worker node mismatch: %+v", w)
			}
			break
		}
	}
	if !found {
		t.Fatal("worker not found in registry")
	}
}

func TestProtobufTaskQueueEndpoints(t *testing.T) {
	mux, _, _ := setupTestServer()
	ts := httptest.NewServer(mux)
	defer ts.Close()

	task := fishv1.BuildTask{
		ID:        "task-http-01",
		Toolchain: "rust",
		Command:   "cargo check",
	}

	pushReq, err := http.NewRequest(http.MethodPost, ts.URL+"/api/v1/tasks", bytes.NewReader(task.Encode()))
	if err != nil {
		t.Fatalf("failed to create request: %v", err)
	}
	pushReq.Header.Set("Content-Type", "application/x-protobuf")

	pushResp, err := http.DefaultClient.Do(pushReq)
	if err != nil {
		t.Fatalf("push task failed: %v", err)
	}
	pushResp.Body.Close()

	if pushResp.StatusCode != http.StatusAccepted {
		t.Fatalf("expected 202, got %d", pushResp.StatusCode)
	}

	popReq, err := http.NewRequest(http.MethodGet, ts.URL+"/api/v1/tasks?toolchain=rust", nil)
	if err != nil {
		t.Fatalf("failed to create pop request: %v", err)
	}
	popReq.Header.Set("Accept", "application/x-protobuf")

	popResp, err := http.DefaultClient.Do(popReq)
	if err != nil {
		t.Fatalf("pop task failed: %v", err)
	}
	defer popResp.Body.Close()

	if popResp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", popResp.StatusCode)
	}

	popBody, err := io.ReadAll(popResp.Body)
	if err != nil {
		t.Fatalf("failed to read pop body: %v", err)
	}

	var poppedTask fishv1.BuildTask
	if err := poppedTask.Decode(popBody); err != nil {
		t.Fatalf("failed to decode popped task: %v", err)
	}

	if poppedTask.ID != "task-http-01" || poppedTask.Toolchain != "rust" {
		t.Fatalf("unexpected popped task: %+v", poppedTask)
	}
}
