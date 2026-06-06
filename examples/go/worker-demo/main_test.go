package main

import (
	"testing"

	tikee "github.com/yhyzgn/tikee/sdks/go/tikee"
)

func TestDemoBuilds(t *testing.T) {}

func TestDemoDoesNotAdvertiseLocalScriptsByDefault(t *testing.T) {
	if enabled("TIKEE_ENABLE_LOCAL_SCRIPT_SHELL") {
		t.Fatal("local script runner must be explicit; it is not a sandbox backend")
	}
}

func TestShellLocalRunnerAddsStructuredCapability(t *testing.T) {
	config := tikee.LocalConfig("http://127.0.0.1:9998", "go-worker-test")
	runner, err := tikee.NewLocalCommandScriptRunner("shell", "custom")
	if err != nil {
		t.Fatalf("NewLocalCommandScriptRunner(shell) error = %v", err)
	}
	registry := tikee.NewScriptRunnerRegistry().Register(runner)
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
