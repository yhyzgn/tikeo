package tikee

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/yhyzgn/tikee/sdks/go/tikee/internal/workerpb"
)

func TestProcessDispatchTaskCapturesProcessorConsoleLogs(t *testing.T) {
	processor := TaskProcessorFunc(func(context.Context, TaskContext) (TaskOutcome, error) {
		fmt.Println("go processor stdout line")
		fmt.Fprintln(os.Stderr, "go processor stderr line")
		log.Print("go processor log package line")
		return Succeeded(), nil
	})
	collector := newCapturedTaskLogCollector()
	outcome, err := processDispatchTaskWithLogs(context.Background(), processor, nil, &workerpb.DispatchTask{
		InstanceId:      "inst-go-stdout",
		JobId:           "job-go-stdout",
		ProcessorName:   "demo.echo",
		AssignmentToken: "assign-token-go",
	}, collector.add)
	logs := collector.logs()
	if err != nil {
		t.Fatalf("processDispatchTaskWithLogs() error = %v", err)
	}
	if !outcome.Success {
		t.Fatalf("unexpected outcome: %+v", outcome)
	}
	if !containsCapturedLog(logs, "info", "go processor stdout line") {
		t.Fatalf("missing stdout task log: %+v", logs)
	}
	if !containsCapturedLog(logs, "error", "go processor stderr line") {
		t.Fatalf("missing stderr task log: %+v", logs)
	}
	if !containsCapturedLogWithSubstring(logs, "error", "go processor log package line") {
		t.Fatalf("missing log package stderr task log: %+v", logs)
	}
}

func TestProcessDispatchTaskCapturesScriptRunnerConsoleLogs(t *testing.T) {
	content := []byte("printf 'go script stdout\\n'; printf 'go script stderr\\n' >&2\n")
	runner, err := NewLocalCommandScriptRunner("shell", "custom")
	if err != nil {
		t.Fatalf("NewLocalCommandScriptRunner() error = %v", err)
	}
	registry := NewScriptRunnerRegistry().Register(runner)
	collector := newCapturedTaskLogCollector()
	outcome, err := processDispatchTaskWithLogs(context.Background(), TaskProcessorFunc(func(context.Context, TaskContext) (TaskOutcome, error) {
		t.Fatal("script binding must not invoke normal processor")
		return Succeeded(), nil
	}), registry, &workerpb.DispatchTask{
		InstanceId:      "inst-go-script",
		JobId:           "job-go-script",
		AssignmentToken: "assign-token-script",
		ProcessorBinding: &workerpb.TaskProcessorBinding{Kind: &workerpb.TaskProcessorBinding_Script{Script: &workerpb.ScriptProcessorBinding{
			ScriptId:       "script-shell-log",
			VersionId:      "sv-shell-log",
			VersionNumber:  1,
			Language:       "shell",
			Content:        content,
			ContentSha256:  sha256Hex(content),
			TimeoutMs:      1000,
			MaxOutputBytes: 4096,
		}}},
	}, collector.add)
	logs := collector.logs()
	if err != nil {
		t.Fatalf("processDispatchTaskWithLogs() error = %v", err)
	}
	if !outcome.Success {
		t.Fatalf("unexpected outcome: %+v", outcome)
	}
	if !containsCapturedLog(logs, "info", "go script stdout") {
		t.Fatalf("missing script stdout task log: %+v", logs)
	}
	if !containsCapturedLog(logs, "error", "go script stderr") {
		t.Fatalf("missing script stderr task log: %+v", logs)
	}
}

type capturedTaskLog struct {
	Level   string
	Message string
}

type capturedTaskLogCollector struct {
	mu      sync.Mutex
	entries []capturedTaskLog
}

func newCapturedTaskLogCollector() *capturedTaskLogCollector {
	return &capturedTaskLogCollector{}
}

func (c *capturedTaskLogCollector) add(level, message string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.entries = append(c.entries, capturedTaskLog{Level: level, Message: message})
}

func (c *capturedTaskLogCollector) logs() []capturedTaskLog {
	c.mu.Lock()
	defer c.mu.Unlock()
	return append([]capturedTaskLog(nil), c.entries...)
}

func containsCapturedLog(logs []capturedTaskLog, level string, message string) bool {
	for _, log := range logs {
		if log.Level == level && log.Message == message {
			return true
		}
	}
	return false
}

func containsCapturedLogWithSubstring(logs []capturedTaskLog, level string, fragment string) bool {
	for _, log := range logs {
		if log.Level == level && strings.Contains(log.Message, fragment) {
			return true
		}
	}
	return false
}

func TestClientRegistrationAndHeartbeatDryRun(t *testing.T) {
	config := LocalConfig("http://127.0.0.1:9998", "go-worker-1")
	config.Namespace = "tenant-a"
	config.App = "billing"
	config.Capabilities = []string{"legacy-tag", "legacy-tag", ""}
	config.AddTag("go")
	config.AddSDKProcessor("demo.echo")
	config.AddScriptRunner("python", "container")
	config.AddPluginProcessor("sql", "billing.sql-sync")
	client, err := NewClient(config)
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}

	registration := client.Registration()
	if registration.ClientInstanceID != "go-worker-1" || registration.Namespace != "tenant-a" || registration.App != "billing" {
		t.Fatalf("unexpected registration: %+v", registration)
	}
	if got, want := strings.Join(registration.Capabilities, ","), "legacy-tag"; got != want {
		t.Fatalf("capabilities = %q, want %q", got, want)
	}
	if got := registration.Structured.SDKProcessors; len(got) != 1 || got[0] != "demo.echo" {
		t.Fatalf("structured sdk processors = %+v", got)
	}
	if got := registration.Structured.ScriptRunners; len(got) != 1 || got[0].Language != "python" || got[0].SandboxBackend != "container" {
		t.Fatalf("structured script runners = %+v", got)
	}
	if got := registration.Structured.PluginProcessors; len(got) != 1 || got[0].Type != "sql" || strings.Join(got[0].ProcessorNames, ",") != "billing.sql-sync" {
		t.Fatalf("structured plugin processors = %+v", got)
	}
	register := client.registerMessage().GetRegister()
	if register == nil || register.GetStructuredCapabilities() == nil {
		t.Fatalf("register message missing structured capabilities: %+v", register)
	}
	if len(register.GetStructuredCapabilities().GetScriptRunners()) != 1 {
		t.Fatalf("proto structured script runners = %+v", register.GetStructuredCapabilities())
	}

	processor := TaskProcessorFunc(func(context.Context, TaskContext) (TaskOutcome, error) {
		return Succeeded(), nil
	})
	if err := client.StartDryRun(context.Background(), processor); err != nil {
		t.Fatalf("StartDryRun() error = %v", err)
	}
	heartbeat, err := client.NextHeartbeat("worker-1", "fence-1", 3)
	if err != nil {
		t.Fatalf("NextHeartbeat() error = %v", err)
	}
	if heartbeat.Sequence != 1 || heartbeat.Generation != 3 || heartbeat.FencingToken != "fence-1" {
		t.Fatalf("unexpected heartbeat: %+v", heartbeat)
	}
}

func TestConfigValidationFailsClosed(t *testing.T) {
	_, err := NewClient(WorkerConfig{})
	if err == nil || !strings.Contains(err.Error(), "endpoint") {
		t.Fatalf("expected endpoint validation error, got %v", err)
	}

	config := LocalConfig("http://127.0.0.1:9998", "go-worker-2")
	config.HeartbeatEvery = 0
	_, err = NewClient(config)
	if err == nil || !strings.Contains(err.Error(), "heartbeat") {
		t.Fatalf("expected heartbeat validation error, got %v", err)
	}
}

func TestGRPCTargetNormalizesHTTPURLs(t *testing.T) {
	cases := map[string]string{
		"127.0.0.1:9998":             "127.0.0.1:9998",
		" http://127.0.0.1:9998 ":    "127.0.0.1:9998",
		"https://worker.example:443": "worker.example:443",
		"dns:///worker.example:443":  "dns:///worker.example:443",
	}
	for endpoint, want := range cases {
		got, err := grpcTarget(endpoint)
		if err != nil {
			t.Fatalf("grpcTarget(%q) error = %v", endpoint, err)
		}
		if got != want {
			t.Fatalf("grpcTarget(%q) = %q, want %q", endpoint, got, want)
		}
	}
}

func TestConnectGRPCUsesOfficialClientBoundary(t *testing.T) {
	client, err := NewClient(LocalConfig("http://127.0.0.1:9998", "go-worker-grpc"))
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}

	if _, err := client.ConnectGRPC(nil); err == nil || !strings.Contains(err.Error(), "context") {
		t.Fatalf("expected nil context error, got %v", err)
	}

	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if _, err := client.ConnectGRPC(ctx); err == nil || !strings.Contains(err.Error(), "context") {
		t.Fatalf("expected canceled context error, got %v", err)
	}

	conn, err := client.ConnectGRPC(context.Background())
	if err != nil {
		t.Fatalf("ConnectGRPC() error = %v", err)
	}
	if err := conn.Close(); err != nil {
		t.Fatalf("ClientConn.Close() error = %v", err)
	}
}

func TestGeneratedWorkerTunnelClientCanBeConstructed(t *testing.T) {
	client, err := NewClient(LocalConfig("127.0.0.1:9998", "go-worker-generated"))
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}
	conn, err := client.ConnectGRPC(context.Background())
	if err != nil {
		t.Fatalf("ConnectGRPC() error = %v", err)
	}
	defer conn.Close()
	if generated := NewWorkerTunnelClient(conn); generated == nil {
		t.Fatal("NewWorkerTunnelClient() returned nil")
	}
}

func TestManagementClientCreatesStructuredPluginAndScriptJobs(t *testing.T) {
	var bodies []map[string]any
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get(apiKeyHeader) != "key-1" {
			t.Fatalf("missing api key header")
		}
		var body map[string]any
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			t.Fatalf("decode body: %v", err)
		}
		bodies = append(bodies, body)
		_ = json.NewEncoder(w).Encode(map[string]any{
			"code":    0,
			"message": "ok",
			"data": map[string]any{
				"id":            "job-1",
				"namespace":     body["namespace"],
				"app":           body["app"],
				"name":          body["name"],
				"scheduleType":  body["scheduleType"],
				"processorName": body["processorName"],
				"processorType": body["processorType"],
				"scriptId":      body["scriptId"],
				"enabled":       true,
			},
		})
	}))
	defer server.Close()

	client := NewManagementClient(server.URL, "key-1", "dev-alpha", "orders")
	if _, err := client.CreateJob(context.Background(), PluginAPIJob("go-sql", "sql", "billing.sql-sync")); err != nil {
		t.Fatalf("CreateJob(plugin) error = %v", err)
	}
	if _, err := client.CreateJob(context.Background(), ScriptAPIJob("go-script", "script_manual_shell_echo")); err != nil {
		t.Fatalf("CreateJob(script) error = %v", err)
	}

	if got := bodies[0]["processorType"]; got != "sql" {
		t.Fatalf("processorType = %v, want sql", got)
	}
	if got := bodies[1]["scriptId"]; got != "script_manual_shell_echo" {
		t.Fatalf("scriptId = %v", got)
	}
}

func TestLocalCommandScriptRunnerExecutesReleasedShellSnapshot(t *testing.T) {
	runner, err := NewLocalCommandScriptRunner("shell", "custom")
	if err != nil {
		t.Fatalf("NewLocalCommandScriptRunner() error = %v", err)
	}
	content := []byte("printf 'go-script-ok'\n")
	outcome, err := runner.Run(context.Background(), ScriptRunnerTask{
		ScriptID:      "script-shell-1",
		VersionNumber: 1,
		Language:      "shell",
		Content:       content,
		ContentSHA256: sha256Hex(content),
		Timeout:       time.Second,
	})
	if err != nil {
		t.Fatalf("Run() error = %v", err)
	}
	if !outcome.Success || outcome.Message != "go-script-ok" {
		t.Fatalf("unexpected outcome: %+v", outcome)
	}
}

func TestLocalCommandScriptRunnerRejectsUnsafePolicy(t *testing.T) {
	runner, err := NewLocalCommandScriptRunner("shell", "custom")
	if err != nil {
		t.Fatalf("NewLocalCommandScriptRunner() error = %v", err)
	}
	content := []byte("echo unsafe\n")
	outcome, err := runner.Run(context.Background(), ScriptRunnerTask{
		ScriptID:      "script-shell-unsafe",
		VersionNumber: 1,
		Language:      "shell",
		Content:       content,
		ContentSHA256: sha256Hex(content),
		AllowNetwork:  true,
	})
	if err != nil {
		t.Fatalf("Run() error = %v", err)
	}
	if outcome.Success || !strings.Contains(outcome.Message, "network") {
		t.Fatalf("expected network rejection, got %+v", outcome)
	}
}

func sha256Hex(content []byte) string {
	digest := sha256.Sum256(content)
	return hex.EncodeToString(digest[:])
}

func TestUnavailableScriptRunnerIsFailClosedButNotAdvertised(t *testing.T) {
	config := LocalConfig("http://127.0.0.1:9998", "go-worker-unavailable")
	registry := NewScriptRunnerRegistry()
	registry.Register(NewUnavailableScriptRunner("python", "srt", "srt is not installed"))
	registry.AddCapabilities(&config)

	if len(config.Structured.ScriptRunners) != 0 {
		t.Fatalf("unavailable script runner must not be advertised: %+v", config.Structured.ScriptRunners)
	}

	outcome, err := registry.get("python").Run(context.Background(), ScriptRunnerTask{
		ScriptID:      "script-python-1",
		VersionID:     "sv_1",
		VersionNumber: 1,
		Language:      "python",
		Content:       []byte("print(1)"),
		ContentSHA256: sha256Hex([]byte("print(1)")),
		Timeout:       time.Second,
	})
	if err != nil {
		t.Fatalf("Run() error = %v", err)
	}
	if outcome.Success || !strings.Contains(outcome.Message, "unavailable") {
		t.Fatalf("expected fail-closed unavailable outcome, got %+v", outcome)
	}
}
