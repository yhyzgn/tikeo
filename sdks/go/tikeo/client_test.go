package tikeo

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/yhyzgn/tikeo/sdks/go/tikeo/internal/workerpb"
)

func TestProcessDispatchTaskRecordsOnlyTaskLoggerLinesForProcessors(t *testing.T) {
	processor := TaskProcessorFunc(func(_ context.Context, task TaskContext) (TaskOutcome, error) {
		fmt.Println("go processor stdout line should stay console-only")
		fmt.Fprintln(os.Stderr, "go processor stderr line should stay console-only")
		log.Print("go processor log package line should stay console-only")
		task.LogInfo("go processor task logger info")
		task.LogError("go processor task logger error")
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
	if !containsCapturedLog(logs, "info", "go processor task logger info") {
		t.Fatalf("missing task logger info log: %+v", logs)
	}
	if !containsCapturedLog(logs, "error", "go processor task logger error") {
		t.Fatalf("missing task logger error log: %+v", logs)
	}
	if containsCapturedLogWithSubstring(logs, "info", "stdout line should stay console-only") {
		t.Fatalf("stdout was incorrectly captured as task log: %+v", logs)
	}
	if containsCapturedLogWithSubstring(logs, "error", "stderr line should stay console-only") || containsCapturedLogWithSubstring(logs, "error", "log package line should stay console-only") {
		t.Fatalf("stderr/log package output was incorrectly captured as task log: %+v", logs)
	}
}

func TestProcessDispatchTaskBridgesSlogAndTaskLoggerFromContext(t *testing.T) {
	collector := newCapturedTaskLogCollector()
	processor := TaskProcessorFunc(func(ctx context.Context, _ TaskContext) (TaskOutcome, error) {
		slog.New(TaskSlogHandler{}).InfoContext(ctx, "go slog bridge info")
		slog.New(TaskSlogHandler{}).ErrorContext(ctx, "go slog bridge error")
		NewTaskLogger(ctx, "", 0).Print("go stdlib task logger line")
		return Succeeded(), nil
	})

	// A bridged logger without task context must not attach unrelated process logs to an instance.
	slog.New(TaskSlogHandler{}).InfoContext(context.Background(), "go outside task scope should stay console-only")

	outcome, err := processDispatchTaskWithLogs(context.Background(), processor, nil, &workerpb.DispatchTask{
		InstanceId:      "inst-go-logger",
		JobId:           "job-go-logger",
		ProcessorName:   "demo.logger",
		AssignmentToken: "assign-token-logger",
	}, collector.add)
	logs := collector.logs()
	if err != nil {
		t.Fatalf("processDispatchTaskWithLogs() error = %v", err)
	}
	if !outcome.Success {
		t.Fatalf("unexpected outcome: %+v", outcome)
	}
	if !containsCapturedLog(logs, "info", "go slog bridge info") {
		t.Fatalf("missing slog info bridge log: %+v", logs)
	}
	if !containsCapturedLog(logs, "error", "go slog bridge error") {
		t.Fatalf("missing slog error bridge log: %+v", logs)
	}
	if !containsCapturedLog(logs, "info", "go stdlib task logger line") {
		t.Fatalf("missing stdlib task logger bridge log: %+v", logs)
	}
	if containsCapturedLogWithSubstring(logs, "info", "outside task scope") {
		t.Fatalf("outside-scope slog was incorrectly captured: %+v", logs)
	}
}

func TestProcessDispatchTaskCapturesProcessorPanicStack(t *testing.T) {
	collector := newCapturedTaskLogCollector()
	outcome, err := processDispatchTaskWithLogs(context.Background(), TaskProcessorFunc(func(context.Context, TaskContext) (TaskOutcome, error) {
		panic("go runtime boom")
	}), nil, &workerpb.DispatchTask{
		InstanceId:      "inst-go-exception",
		JobId:           "job-go-exception",
		ProcessorName:   "demo.exception",
		AssignmentToken: "assign-token-exception",
	}, collector.add)
	logs := collector.logs()
	if err != nil {
		t.Fatalf("processDispatchTaskWithLogs() error = %v", err)
	}
	if outcome.Success || !strings.Contains(outcome.Message, "go runtime boom") {
		t.Fatalf("outcome = %+v, want failed go runtime boom", outcome)
	}
	if !containsCapturedLogWithSubstring(logs, "error", "goroutine") || !containsCapturedLogWithSubstring(logs, "error", "go runtime boom") {
		t.Fatalf("missing panic stack in task logs: %+v", logs)
	}
}

func TestProcessDispatchTaskRecordsScriptRunnerPipeOutput(t *testing.T) {
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
	if !containsCapturedLog(logs, "info", "[script] go script stdout") {
		t.Fatalf("missing script stdout task log: %+v", logs)
	}
	if !containsCapturedLog(logs, "error", "[script] go script stderr") {
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
	var paths []string
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get(apiKeyHeader) != "key-1" {
			t.Fatalf("missing api key header")
		}
		paths = append(paths, r.URL.Path)
		var body map[string]any
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			t.Fatalf("decode body: %v", err)
		}
		bodies = append(bodies, body)
		if strings.HasSuffix(r.URL.Path, ":trigger") {
			_ = json.NewEncoder(w).Encode(map[string]any{
				"code":    0,
				"message": "ok",
				"data": map[string]any{
					"id":            "inst-1",
					"jobId":         "job-1",
					"status":        "pending",
					"triggerType":   body["triggerType"],
					"executionMode": "single",
					"createdAt":     "now",
					"updatedAt":     "now",
				},
			})
			return
		}
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
	instance, err := client.TriggerJob(context.Background(), "job-1", APITrigger())
	if err != nil {
		t.Fatalf("TriggerJob() error = %v", err)
	}

	if got := bodies[0]["processorType"]; got != "sql" {
		t.Fatalf("processorType = %v, want sql", got)
	}
	if got := bodies[1]["scriptId"]; got != "script_manual_shell_echo" {
		t.Fatalf("scriptId = %v", got)
	}
	if got := paths[2]; got != "/api/v1/jobs/job-1:trigger" {
		t.Fatalf("trigger path = %q", got)
	}
	if got := bodies[2]["executionMode"]; got != "single" {
		t.Fatalf("trigger executionMode = %v, want single", got)
	}
	if instance.TriggerType != "api" || instance.JobID != "job-1" {
		t.Fatalf("unexpected trigger instance: %+v", instance)
	}
	broadcast := BroadcastAPITrigger(&BroadcastSelectorRequest{Region: "us-east-1"})
	if broadcast.ExecutionMode != "broadcast" || broadcast.BroadcastSelector.Region != "us-east-1" {
		t.Fatalf("unexpected broadcast trigger helper: %+v", broadcast)
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

func TestSandboxToolResolverDoesNotAdvertiseMissingToolsWhenAutoInstallDisabled(t *testing.T) {
	t.Setenv("PATH", "")
	t.Setenv("TIKEO_SANDBOX_TOOLS_DIR", t.TempDir())
	resolver := SandboxToolResolver{StateDir: t.TempDir(), AutoInstall: false}
	if _, ok := resolver.ResolveSrt(); ok {
		t.Fatal("missing SRT tool must not resolve when auto install is disabled")
	}
}

func TestSandboxToolResolverUsesHostCacheWhenWorkerStateIsEmpty(t *testing.T) {
	resolver := SandboxToolResolver{StateDir: t.TempDir(), AutoInstall: false}
	home, err := os.UserHomeDir()
	if err != nil {
		t.Fatal(err)
	}
	want := filepath.Join(home, ".tikeo", "sandbox-tools", "srt")
	if got := resolver.installDir("srt"); got != want {
		t.Fatalf("installDir=%q, want host cache %q", got, want)
	}
}

func TestSandboxInstallerPathPrependsManagedBinOnce(t *testing.T) {
	managed := filepath.Join(t.TempDir(), "sandbox-tools", "rhai-run", "bin")
	env := []string{"PATH=/usr/local/bin" + string(os.PathListSeparator) + managed + string(os.PathListSeparator) + "/usr/bin"}

	updated := pathWithManagedBin(env, managed)
	if len(updated) != 1 {
		t.Fatalf("updated env = %v, want one PATH entry", updated)
	}
	pathValue := strings.TrimPrefix(updated[0], "PATH=")
	parts := filepath.SplitList(pathValue)
	if len(parts) < 3 || parts[0] != managed {
		t.Fatalf("PATH parts = %v, want managed bin first", parts)
	}
	count := 0
	for _, part := range parts {
		if part == managed {
			count++
		}
	}
	if count != 1 {
		t.Fatalf("PATH parts = %v, want managed bin once", parts)
	}
}

func TestSrtSettingsSerializeEmptyPolicyListsAsArrays(t *testing.T) {
	runtimeDirs, err := newScriptTaskRuntimeDirs("tikeo-srt-empty-arrays-test")
	if err != nil {
		t.Fatalf("newScriptTaskRuntimeDirs() error = %v", err)
	}
	defer runtimeDirs.cleanup()
	settings, cleanup, err := writeSrtSettings(ScriptRunnerTask{
		ScriptID:      "script-shell",
		VersionID:     "sv_1",
		VersionNumber: 1,
		Language:      "shell",
		Content:       []byte("echo ok"),
		ContentSHA256: sha256Hex([]byte("echo ok")),
		Timeout:       time.Second,
	}, runtimeDirs, "")
	if err != nil {
		t.Fatalf("writeSrtSettings() error = %v", err)
	}
	defer cleanup()
	parsed := readSrtSettingsJSON(t, settings)
	network := parsed["network"].(map[string]any)
	filesystem := parsed["filesystem"].(map[string]any)
	if _, ok := network["allowedDomains"].([]any); !ok {
		t.Fatalf("network.allowedDomains = %#v, want JSON array", network["allowedDomains"])
	}
	if _, ok := filesystem["allowRead"].([]any); !ok {
		t.Fatalf("filesystem.allowRead = %#v, want JSON array", filesystem["allowRead"])
	}
}

func TestSrtSettingsAllowPowerShellCacheWrites(t *testing.T) {
	runtimeDirs, err := newScriptTaskRuntimeDirs("tikeo-srt-powershell-cache-test")
	if err != nil {
		t.Fatalf("newScriptTaskRuntimeDirs() error = %v", err)
	}
	defer runtimeDirs.cleanup()
	settings, cleanup, err := writeSrtSettings(ScriptRunnerTask{
		ScriptID:      "script-powershell",
		VersionID:     "sv_1",
		VersionNumber: 1,
		Language:      "powershell",
		Content:       []byte("Write-Output ok"),
		ContentSHA256: sha256Hex([]byte("Write-Output ok")),
		Timeout:       time.Second,
	}, runtimeDirs, "")
	if err != nil {
		t.Fatalf("writeSrtSettings() error = %v", err)
	}
	defer cleanup()
	parsed := readSrtSettingsJSON(t, settings)
	filesystem := parsed["filesystem"].(map[string]any)
	rawAllowWrite := filesystem["allowWrite"].([]any)
	want := filepath.Join(runtimeDirs.cache, "powershell")
	for _, raw := range rawAllowWrite {
		if raw == want {
			return
		}
	}
	t.Fatalf("allowWrite=%v, want explicit PowerShell cache path %s", rawAllowWrite, want)
}

func readSrtSettingsJSON(t *testing.T, settings string) map[string]any {
	t.Helper()
	data, err := os.ReadFile(settings)
	if err != nil {
		t.Fatalf("ReadFile(%s) error = %v", settings, err)
	}
	var parsed map[string]any
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("settings json = %s, error = %v", data, err)
	}
	return parsed
}

func TestSrtSettingsDoNotMaskManagedRuntimeUnderHome(t *testing.T) {
	runtimeDirs, err := newScriptTaskRuntimeDirs("tikeo-srt-settings-test")
	if err != nil {
		t.Fatalf("newScriptTaskRuntimeDirs() error = %v", err)
	}
	defer runtimeDirs.cleanup()
	settings, cleanup, err := writeSrtSettings(ScriptRunnerTask{
		ScriptID:      "script-shell",
		VersionID:     "sv_1",
		VersionNumber: 1,
		Language:      "shell",
		Content:       []byte("echo ok"),
		ContentSHA256: sha256Hex([]byte("echo ok")),
		Timeout:       time.Second,
	}, runtimeDirs, "")
	if err != nil {
		t.Fatalf("writeSrtSettings() error = %v", err)
	}
	defer cleanup()
	data, err := os.ReadFile(settings)
	if err != nil {
		t.Fatalf("ReadFile(%s) error = %v", settings, err)
	}
	var parsed map[string]any
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("settings json = %s, error = %v", data, err)
	}
	filesystem, ok := parsed["filesystem"].(map[string]any)
	if !ok {
		t.Fatalf("filesystem settings missing or invalid: %v", parsed)
	}
	rawDenyRead, ok := filesystem["denyRead"].([]any)
	if !ok {
		t.Fatalf("denyRead missing or invalid: %v", filesystem)
	}
	denyRead := make([]string, 0, len(rawDenyRead))
	for _, raw := range rawDenyRead {
		if value, ok := raw.(string); ok {
			denyRead = append(denyRead, value)
		}
	}
	home := userHome()
	for _, path := range denyRead {
		if path == home {
			t.Fatalf("denyRead must not mask the whole HOME and hide managed SRT runtime: %v", denyRead)
		}
	}
	if !containsPathWithSuffix(denyRead, string(filepath.Separator)+".ssh") {
		t.Fatalf("denyRead should still protect sensitive home paths: %v", denyRead)
	}
}

func TestRhaiDiagnosticOutputIsFailureMessage(t *testing.T) {
	message := rhaiDiagnosticMessage(
		[]byte("                                                   ^ Syntax error: 'case' is a reserved keyword\n"),
		[]byte("1: let result = #{ language: \"rhai\", status: \"ok\", case: \"manual-acceptance\" };\n"),
	)
	if !strings.Contains(message, "Syntax error") || !strings.Contains(message, "manual-acceptance") {
		t.Fatalf("diagnostic message = %q", message)
	}
}

func TestSrtAndDenoRunnersAdvertiseStructuredSandboxBackends(t *testing.T) {
	srt, err := NewSrtScriptRunner("python", "srt", "python3")
	if err != nil {
		t.Fatalf("NewSrtScriptRunner() error = %v", err)
	}
	deno, err := NewDenoScriptRunner("javascript", "deno")
	if err != nil {
		t.Fatalf("NewDenoScriptRunner() error = %v", err)
	}
	registry := NewScriptRunnerRegistry().Register(srt).Register(deno)
	config := LocalConfig("http://127.0.0.1:9998", "go-sandbox-test")
	registry.AddCapabilities(&config)
	if len(config.Structured.ScriptRunners) != 2 {
		t.Fatalf("script runners = %+v, want 2", config.Structured.ScriptRunners)
	}
	seen := map[string]string{}
	for _, runner := range config.Structured.ScriptRunners {
		seen[runner.Language] = runner.SandboxBackend
	}
	if seen["python"] != "srt" || seen["javascript"] != "deno" {
		t.Fatalf("script runner backends = %+v", seen)
	}
}

func containsPathWithSuffix(paths []string, suffix string) bool {
	for _, path := range paths {
		if strings.HasSuffix(path, suffix) {
			return true
		}
	}
	return false
}

func TestSrtRunnerStartsSupportedKindsInsideTaskSandboxHome(t *testing.T) {
	cases := []struct {
		language    string
		interpreter string
		content     []byte
	}{
		{language: "shell", interpreter: "sh", content: []byte("pwd\n")},
		{language: "python", interpreter: "python3", content: []byte("import os; print(os.getcwd())\n")},
		{language: "powershell", interpreter: "pwsh", content: []byte("Get-Location\n")},
		{language: "rhai", interpreter: "rhai-run", content: []byte("print(\"ok\");\n")},
		{language: "php", interpreter: "php", content: []byte("<?php echo getcwd(); ?>\n")},
		{language: "groovy", interpreter: "groovy", content: []byte("println System.getProperty('user.dir')\n")},
	}
	for _, tc := range cases {
		t.Run(tc.language, func(t *testing.T) {
			tempRoot := t.TempDir()
			report := filepath.Join(tempRoot, "report.txt")
			runtime := filepath.Join(tempRoot, "srt")
			writeTestExecutable(t, runtime, fmt.Sprintf(`#!/bin/sh
printf 'cwd=%%s\n' "$(pwd)" > %s
printf 'home=%%s\n' "$HOME" >> %s
printf 'tmp=%%s\n' "$TMPDIR" >> %s
printf 'claude_tmp=%%s\n' "$CLAUDE_CODE_TMPDIR" >> %s
printf 'args=%%s\n' "$*" >> %s
exit 0
`, shellQuote(report), shellQuote(report), shellQuote(report), shellQuote(report), shellQuote(report)))
			runner, err := NewSrtScriptRunner(tc.language, runtime, tc.interpreter)
			if err != nil {
				t.Fatalf("NewSrtScriptRunner() error = %v", err)
			}
			outcome, err := runner.Run(context.Background(), scriptRunnerTestTask(tc.language, tc.content, ScriptRunnerTask{
				AllowedEnvVars: []string{"HOME", "TMPDIR", "CLAUDE_CODE_TMPDIR"},
			}))
			if err != nil {
				t.Fatalf("Run() error = %v", err)
			}
			if !outcome.Success {
				t.Fatalf("Run() outcome = %+v", outcome)
			}
			values := readReportValues(t, report)
			if values["cwd"] != values["home"] {
				t.Fatalf("cwd=%q home=%q, want SRT started in sandbox HOME", values["cwd"], values["home"])
			}
			if !strings.Contains(values["home"], "tikeo-srt-"+normalizeScriptLanguage(tc.language)+"-runtime") {
				t.Fatalf("home=%q, want task runtime dir", values["home"])
			}
			if values["tmp"] != filepath.Join(filepath.Dir(values["home"]), "tmp") {
				t.Fatalf("tmp=%q, want runtime tmp beside home=%q", values["tmp"], values["home"])
			}
			if values["claude_tmp"] != values["tmp"] {
				t.Fatalf("claude tmp=%q tmp=%q, want same task tmp", values["claude_tmp"], values["tmp"])
			}
			if tc.language == "rhai" && !strings.Contains(values["args"], string(filepath.Separator)+"home"+string(filepath.Separator)+"script-") {
				t.Fatalf("rhai args=%q, want script file under sandbox HOME", values["args"])
			}
		})
	}
}

func TestDenoRunnerStartsJsAndTsInsideTaskSandboxHome(t *testing.T) {
	for _, language := range []string{"javascript", "typescript"} {
		t.Run(language, func(t *testing.T) {
			tempRoot := t.TempDir()
			report := filepath.Join(tempRoot, "report.txt")
			runtime := filepath.Join(tempRoot, "deno")
			writeTestExecutable(t, runtime, fmt.Sprintf(`#!/bin/sh
cat >/dev/null
printf 'cwd=%%s\n' "$(pwd)" > %s
printf 'home=%%s\n' "$HOME" >> %s
printf 'tmp=%%s\n' "$TMPDIR" >> %s
printf 'deno_dir=%%s\n' "$DENO_DIR" >> %s
printf 'args=%%s\n' "$*" >> %s
exit 0
`, shellQuote(report), shellQuote(report), shellQuote(report), shellQuote(report), shellQuote(report)))
			runner, err := NewDenoScriptRunner(language, runtime)
			if err != nil {
				t.Fatalf("NewDenoScriptRunner() error = %v", err)
			}
			outcome, err := runner.Run(context.Background(), scriptRunnerTestTask(language, []byte("console.log('ok')\n"), ScriptRunnerTask{
				AllowedEnvVars: []string{"HOME", "TMPDIR", "DENO_DIR"},
			}))
			if err != nil {
				t.Fatalf("Run() error = %v", err)
			}
			if !outcome.Success {
				t.Fatalf("Run() outcome = %+v", outcome)
			}
			values := readReportValues(t, report)
			if values["cwd"] != values["home"] {
				t.Fatalf("cwd=%q home=%q, want Deno started in sandbox HOME", values["cwd"], values["home"])
			}
			if !strings.Contains(values["home"], "tikeo-deno-"+normalizeScriptLanguage(language)+"-runtime") {
				t.Fatalf("home=%q, want task runtime dir", values["home"])
			}
			root := filepath.Dir(values["home"])
			if values["tmp"] != filepath.Join(root, "tmp") {
				t.Fatalf("tmp=%q, want runtime tmp", values["tmp"])
			}
			if values["deno_dir"] != filepath.Join(root, "cache", "deno") {
				t.Fatalf("deno_dir=%q, want runtime deno cache", values["deno_dir"])
			}
			if !strings.Contains(values["args"], "run --no-prompt") {
				t.Fatalf("args=%q, want deno run args", values["args"])
			}
		})
	}
}

func scriptRunnerTestTask(language string, content []byte, overrides ScriptRunnerTask) ScriptRunnerTask {
	task := ScriptRunnerTask{
		ScriptID:       "script-" + normalizeScriptLanguage(language),
		VersionID:      "sv-test",
		VersionNumber:  1,
		Language:       language,
		Content:        content,
		ContentSHA256:  sha256Hex(content),
		Timeout:        time.Second,
		MaxOutputBytes: 4096,
	}
	task.AllowedEnvVars = append(task.AllowedEnvVars, overrides.AllowedEnvVars...)
	task.ReadOnlyPaths = append(task.ReadOnlyPaths, overrides.ReadOnlyPaths...)
	task.WritablePaths = append(task.WritablePaths, overrides.WritablePaths...)
	task.AllowedNetworkHosts = append(task.AllowedNetworkHosts, overrides.AllowedNetworkHosts...)
	task.AllowNetwork = overrides.AllowNetwork
	task.SecretRefs = append(task.SecretRefs, overrides.SecretRefs...)
	return task
}

func writeTestExecutable(t *testing.T, path string, content string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(content), 0o755); err != nil {
		t.Fatalf("write executable %s: %v", path, err)
	}
}

func readReportValues(t *testing.T, path string) map[string]string {
	t.Helper()
	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read report %s: %v", path, err)
	}
	values := map[string]string{}
	for _, line := range strings.Split(strings.TrimSpace(string(content)), "\n") {
		key, value, ok := strings.Cut(line, "=")
		if ok {
			values[key] = value
		}
	}
	return values
}
