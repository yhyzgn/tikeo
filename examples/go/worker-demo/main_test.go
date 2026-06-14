package main

import (
	"context"
	"strings"
	"testing"

	tikeo "github.com/yhyzgn/tikeo/sdks/go/tikeo"
)

func TestDemoBuilds(t *testing.T) {}

func TestDemoDoesNotAdvertiseLocalScriptsByDefault(t *testing.T) {
	if enabled("TIKEO_ENABLE_LOCAL_SCRIPT_SHELL") {
		t.Fatal("local script runner must be explicit; it is not a sandbox backend")
	}
}

func TestShellLocalRunnerAddsStructuredCapability(t *testing.T) {
	config := tikeo.LocalConfig("http://127.0.0.1:9998", "go-worker-test")
	runner, err := tikeo.NewLocalCommandScriptRunner("shell", "custom")
	if err != nil {
		t.Fatalf("NewLocalCommandScriptRunner(shell) error = %v", err)
	}
	registry := tikeo.NewScriptRunnerRegistry().Register(runner)
	registry.AddCapabilities(&config)
	if got := config.Structured.ScriptRunners; len(got) != 1 || got[0].Language != "shell" || got[0].SandboxBackend != "custom" {
		t.Fatalf("script runners = %+v, want shell/custom", got)
	}
}

func TestAutoSandboxBackendMatchesJavaLightweightDefaults(t *testing.T) {
	if got := scriptSandboxBackend("python"); got != "srt" {
		t.Fatalf("python auto backend = %s, want srt", got)
	}
	if got := scriptSandboxBackend("javascript"); got != "deno" {
		t.Fatalf("javascript auto backend = %s, want deno", got)
	}
	if got := scriptSandboxBackend("typescript"); got != "deno" {
		t.Fatalf("typescript auto backend = %s, want deno", got)
	}
}

func TestDemoFailReturnsBusinessFailureAndExceptionPanics(t *testing.T) {
	failLogs := []string{}
	failSink := func(level, message string) { failLogs = append(failLogs, level+":"+message) }
	failCtx := tikeo.ContextWithTaskLogScope(context.Background(), tikeo.TaskLogScope{
		InstanceID:    "inst-fail",
		JobID:         "job-1",
		ProcessorName: "demo.fail",
		Log:           failSink,
	})
	failure, err := demoProcessTask(failCtx, tikeo.TaskContext{
		InstanceID:    "inst-fail",
		JobID:         "job-1",
		ProcessorName: "demo.fail",
		Payload:       []byte("bad-input"),
		Log:           failSink,
	})
	if err != nil {
		t.Fatalf("demo.fail returned error: %v", err)
	}
	if failure.Success || failure.Message != "go demo intentional failure" {
		t.Fatalf("demo.fail outcome = %+v", failure)
	}
	if !containsDemoLog(failLogs, "error:demo intentional failure", "bad-input") {
		t.Fatalf("missing demo.fail task log: %+v", failLogs)
	}

	exceptionLogs := []string{}
	defer func() {
		recovered := recover()
		if recovered == nil {
			t.Fatal("demo.exception should panic")
		}
		if recovered != "go demo runtime exception" {
			t.Fatalf("panic = %v", recovered)
		}
		if !containsDemoLog(exceptionLogs, "error:demo runtime exception", "bad-input") {
			t.Fatalf("missing demo.exception task log: %+v", exceptionLogs)
		}
	}()
	exceptionSink := func(level, message string) { exceptionLogs = append(exceptionLogs, level+":"+message) }
	exceptionCtx := tikeo.ContextWithTaskLogScope(context.Background(), tikeo.TaskLogScope{
		InstanceID:    "inst-exception",
		JobID:         "job-1",
		ProcessorName: "demo.exception",
		Log:           exceptionSink,
	})
	_, _ = demoProcessTask(exceptionCtx, tikeo.TaskContext{
		InstanceID:    "inst-exception",
		JobID:         "job-1",
		ProcessorName: "demo.exception",
		Payload:       []byte("bad-input"),
		Log:           exceptionSink,
	})
}

func containsDemoLog(logs []string, prefix string, substring string) bool {
	for _, line := range logs {
		if strings.Contains(line, prefix) && strings.Contains(line, substring) {
			return true
		}
	}
	return false
}
