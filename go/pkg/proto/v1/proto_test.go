package fishv1

import (
	"math"
	"testing"
)

func TestBuildTaskRoundTrip(t *testing.T) {
	task := BuildTask{
		ID:           "task-go-01",
		PackageName:  "fish-coordinator",
		Toolchain:    "go",
		Command:      "go build",
		Args:         []string{"-v", "./..."},
		Inputs:       []string{"pkg/**/*.go", "go.mod"},
		Outputs:      []string{"bin/fish-coordinator"},
		Dependencies: []string{"task-dep-01"},
		Env:          map[string]string{"CGO_ENABLED": "0", "GOOS": "linux"},
		TimeoutMs:    45000,
	}

	data := task.Encode()
	if len(data) == 0 {
		t.Fatal("encoded data is empty")
	}

	var decoded BuildTask
	if err := decoded.Decode(data); err != nil {
		t.Fatalf("failed to decode task: %v", err)
	}

	if decoded.ID != task.ID || decoded.PackageName != task.PackageName || decoded.TimeoutMs != task.TimeoutMs {
		t.Fatalf("decoded task mismatch: %+v vs %+v", decoded, task)
	}
	if len(decoded.Args) != len(task.Args) || decoded.Args[0] != task.Args[0] {
		t.Fatalf("args mismatch: %v vs %v", decoded.Args, task.Args)
	}
	if decoded.Env["CGO_ENABLED"] != "0" {
		t.Fatalf("env mismatch: %v", decoded.Env)
	}
}

func TestTaskResultRoundTrip(t *testing.T) {
	res := TaskResult{
		TaskID:        "t-01",
		ExitCode:      0,
		Stdout:        "success",
		Stderr:        "",
		DurationMs:    850,
		Cached:        true,
		Fingerprint:   "blake3:998877",
		OutputDigests: map[string]string{"bin": "hash123"},
	}

	data := res.Encode()
	var decoded TaskResult
	if err := decoded.Decode(data); err != nil {
		t.Fatalf("decode failed: %v", err)
	}

	if decoded.TaskID != res.TaskID || decoded.ExitCode != res.ExitCode || !decoded.Cached {
		t.Fatalf("result mismatch: %+v", decoded)
	}
}

func TestBuildGraphRoundTrip(t *testing.T) {
	graph := BuildGraph{
		RootPackage: "root-pkg",
		Tasks: []BuildTask{
			{
				ID:          "sub-1",
				PackageName: "sub",
				Toolchain:   "rust",
				Command:     "cargo check",
			},
		},
		ExecutionOrder: []string{"sub-1"},
	}

	data := graph.Encode()
	var decoded BuildGraph
	if err := decoded.Decode(data); err != nil {
		t.Fatalf("decode graph failed: %v", err)
	}

	if decoded.RootPackage != "root-pkg" || len(decoded.Tasks) != 1 || decoded.Tasks[0].ID != "sub-1" {
		t.Fatalf("graph mismatch: %+v", decoded)
	}
}

func TestCoordinatorMessagesRoundTrip(t *testing.T) {
	reg := WorkerRegistration{
		WorkerID:            "w-99",
		Address:             "10.0.0.5:8080",
		CPUCores:            8,
		MemoryBytes:         16 * 1024 * 1024 * 1024,
		SupportedToolchains: []string{"rust", "go"},
		Tags:                map[string]string{"env": "prod"},
	}

	data := reg.Encode()
	var decReg WorkerRegistration
	if err := decReg.Decode(data); err != nil {
		t.Fatalf("decode registration failed: %v", err)
	}
	if decReg.WorkerID != "w-99" || decReg.CPUCores != 8 || decReg.Tags["env"] != "prod" {
		t.Fatalf("reg mismatch: %+v", decReg)
	}

	hb := WorkerHeartbeat{
		WorkerID:             "w-99",
		CPULoad:              0.65,
		AvailableMemoryBytes: 8 * 1024 * 1024 * 1024,
		ActiveJobs:           2,
		Timestamp:            1725360000,
	}

	hbData := hb.Encode()
	var decHb WorkerHeartbeat
	if err := decHb.Decode(hbData); err != nil {
		t.Fatalf("decode heartbeat failed: %v", err)
	}
	if decHb.WorkerID != "w-99" || math.Abs(decHb.CPULoad-0.65) > 1e-6 || decHb.ActiveJobs != 2 {
		t.Fatalf("heartbeat mismatch: %+v", decHb)
	}

	ack := HeartbeatAck{
		Accepted:                true,
		NextHeartbeatIntervalMs: 3000,
	}
	ackData := ack.Encode()
	var decAck HeartbeatAck
	if err := decAck.Decode(ackData); err != nil {
		t.Fatalf("decode ack failed: %v", err)
	}
	if !decAck.Accepted || decAck.NextHeartbeatIntervalMs != 3000 {
		t.Fatalf("ack mismatch: %+v", decAck)
	}
}

func TestFailureAnalysisRoundTrip(t *testing.T) {
	req := FailureAnalysisRequest{
		TaskID:    "task-fail",
		Toolchain: "c++",
		Command:   "clang++ -c main.cpp",
		Stderr:    "fatal error: 'stdio.h' file not found",
		Stdout:    "",
		ExitCode:  1,
	}
	data := req.Encode()
	var decReq FailureAnalysisRequest
	if err := decReq.Decode(data); err != nil {
		t.Fatalf("decode req failed: %v", err)
	}
	if decReq.TaskID != "task-fail" || decReq.ExitCode != 1 {
		t.Fatalf("req mismatch: %+v", decReq)
	}

	resp := FailureAnalysisResponse{
		ErrorCategory:  "header_missing",
		RootCause:      "C runtime headers missing",
		Confidence:     0.98,
		SuggestedFixes: []string{"apt-get install build-essential"},
		AffectedFiles:  []string{"main.cpp"},
	}
	respData := resp.Encode()
	var decResp FailureAnalysisResponse
	if err := decResp.Decode(respData); err != nil {
		t.Fatalf("decode resp failed: %v", err)
	}
	if decResp.ErrorCategory != "header_missing" || math.Abs(decResp.Confidence-0.98) > 1e-6 {
		t.Fatalf("resp mismatch: %+v", decResp)
	}
}
