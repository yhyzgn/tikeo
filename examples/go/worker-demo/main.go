package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"log/slog"
	"os"
	"strings"
	"time"

	tikeo "github.com/yhyzgn/tikeo/sdks/go/tikeo"
)

func main() {
	config := tikeo.LocalConfig(envOr("TIKEO_WORKER_ENDPOINT", "http://127.0.0.1:9998"), envOr("TIKEO_WORKER_CLIENT_INSTANCE_ID", "go-worker-demo-local"))
	config.Namespace = envOr("TIKEO_WORKER_NAMESPACE", "dev-alpha")
	config.App = envOr("TIKEO_WORKER_APP", "orders")
	config.Cluster = envOr("TIKEO_WORKER_CLUSTER", "local")
	config.Region = envOr("TIKEO_WORKER_REGION", "local")
	config.AddTag("go")
	config.AddTag("manual-demo")
	for _, processor := range csvOr("TIKEO_WORKER_SDK_PROCESSORS", "demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception") {
		config.AddSDKProcessor(processor)
	}
	config.Labels["worker_pool"] = envOr("TIKEO_WORKER_POOL", "go-blue")
	if enabledByDefault("TIKEO_ENABLE_PLUGIN_SQL") {
		config.AddPluginProcessor(envOr("TIKEO_PLUGIN_SQL_TYPE", "sql"), envOr("TIKEO_PLUGIN_SQL_PROCESSOR", "billing.sql-sync"))
		config.Labels["plugin_sql"] = "enabled"
	}
	scripts := tikeo.NewScriptRunnerRegistry()
	resolver := tikeo.NewSandboxToolResolver()
	resolver.StateDir = envOr("TIKEO_WORKER_STATE_DIR", "")
	resolver.AutoInstall = !disabled("TIKEO_SANDBOX_AUTO_INSTALL")
	resolver.RequireManagedTools = enabled("TIKEO_SANDBOX_REQUIRE_MANAGED_TOOLS")
	for _, lang := range csvOr("TIKEO_WORKER_SCRIPT_LANGUAGES", "shell,python,javascript,typescript,powershell,php,groovy,rhai") {
		if disabled("TIKEO_ENABLE_SCRIPT_" + strings.ToUpper(lang)) {
			continue
		}
		backend := scriptSandboxBackend(lang)
		if backend == "srt" {
			srtCommand, srtOK := resolver.ResolveSrt()
			rgCommand, rgOK := resolver.ResolveRipgrep()
			if srtOK && rgOK {
				interpreter, ok := resolveSrtInterpreter(lang, resolver)
				if !ok {
					log.Printf("srt script runner %s skipped: interpreter unavailable", lang)
					continue
				}
				runner, err := tikeo.NewSrtScriptRunner(lang, srtCommand, interpreter, sandboxToolPathEntries(srtCommand, rgCommand, interpreter, resolver)...)
				if err == nil {
					scripts.Register(runner)
					continue
				}
				log.Printf("srt script runner %s skipped: %v", lang, err)
			}
		} else if backend == "deno" || backend == "v8" {
			if denoCommand, ok := resolver.ResolveDeno(); ok {
				runner, err := tikeo.NewDenoScriptRunner(lang, denoCommand)
				if err == nil {
					scripts.Register(runner)
					continue
				}
				log.Printf("deno script runner %s skipped: %v", lang, err)
			}
		} else if backend == "docker" || backend == "podman" {
			runner, err := tikeo.NewContainerScriptRunner(lang, backend, scriptImage(lang))
			if err != nil {
				log.Printf("container script runner %s skipped: %v", lang, err)
				continue
			}
			scripts.Register(runner)
			continue
		} else if enabled("TIKEO_ENABLE_LOCAL_SCRIPT_" + strings.ToUpper(lang)) {
			runner, err := tikeo.NewLocalCommandScriptRunner(lang, "custom")
			if err != nil {
				log.Printf("development local script runner %s skipped: %v", lang, err)
				continue
			}
			scripts.Register(runner)
			continue
		}
		if !enabled("TIKEO_ENABLE_UNAVAILABLE_SCRIPT_ADAPTERS") {
			continue
		}
		reason := backend + " sandbox backend is unavailable; auto requires SRT+rg for native scripts and Deno for JavaScript/TypeScript"
		scripts.Register(tikeo.NewUnavailableScriptRunner(lang, backend, reason))
	}
	scripts.AddCapabilities(&config)

	client, err := tikeo.NewClient(config)
	if err != nil {
		log.Fatal(err)
	}
	processor := tikeo.TaskProcessorFunc(demoProcessTask)

	registration := client.Registration()
	pretty, _ := json.MarshalIndent(registration, "", "  ")
	fmt.Printf("go worker demo configured: %s\n", pretty)

	if enabled("TIKEO_MANAGEMENT_CREATE_EXAMPLES") {
		mgmt := tikeo.NewManagementClient(envOr("TIKEO_HTTP_URL", "http://127.0.0.1:9090"), os.Getenv("TIKEO_API_KEY"), config.Namespace, config.App)
		for _, job := range []tikeo.CreateJobRequest{
			tikeo.APIJob("go-echo-api", "demo.echo"),
			tikeo.PluginAPIJob("go-sql-sync-api", "sql", "billing.sql-sync"),
		} {
			created, err := mgmt.CreateJob(context.Background(), job)
			if err != nil {
				log.Printf("create job %s failed: %v", job.Name, err)
				continue
			}
			instance, err := mgmt.TriggerJob(context.Background(), created.ID, tikeo.APITrigger())
			if err != nil {
				log.Printf("trigger job %s failed: %v", created.ID, err)
				continue
			}
			log.Printf("created and triggered job %s/%s %s instance=%s trigger_type=%s", created.Namespace, created.App, created.Name, instance.ID, instance.TriggerType)
		}
	}

	if dryRunEnabled() {
		if err := client.StartDryRun(context.Background(), processor); err != nil {
			log.Fatal(err)
		}
		heartbeat, err := client.NextHeartbeat("dry-run-worker", "dry-run-fence", 1)
		if err != nil {
			log.Fatal(err)
		}
		fmt.Printf("dry_run_heartbeat_sequence=%d\n", heartbeat.Sequence)
		return
	}

	oneshot := enabled("TIKEO_WORKER_ONESHOT")
	for {
		if runWorkerSession(client, processor, scripts, oneshot) {
			return
		}
		time.Sleep(2 * time.Second)
	}
}

func runWorkerSession(
	client *tikeo.Client,
	processor tikeo.TaskProcessor,
	scripts *tikeo.ScriptRunnerRegistry,
	oneshot bool,
) bool {
	session, err := client.Connect(context.Background())
	if err != nil {
		log.Printf("connect failed, retrying: %v", err)
		return false
	}
	stopHeartbeat := session.StartHeartbeat(context.Background())
	log.Printf("go worker connected: worker_id=%s generation=%d lease_seconds=%d", session.WorkerID(), session.Generation(), session.LeaseSeconds())
	defer func() {
		stopHeartbeat()
		if err := session.Close(); err != nil {
			log.Printf("worker session close skipped/failed: %v", err)
		}
	}()
	if enabled("TIKEO_WORKER_HEARTBEAT_ON_START") {
		ping, err := session.Heartbeat()
		if err != nil {
			log.Printf("heartbeat-on-start failed, reconnecting: %v", err)
			return false
		}
		log.Printf("heartbeat ack sequence=%d", ping.GetSequence())
	}
	for {
		outcome, err := session.ProcessNextWithScriptRunners(context.Background(), processor, scripts)
		if err != nil {
			log.Printf("worker tunnel ended, reconnecting: %v", err)
			return false
		}
		log.Printf("processed task success=%v message=%s", outcome.Success, outcome.Message)
		if oneshot {
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
}

func demoProcessTask(ctx context.Context, task tikeo.TaskContext) (tikeo.TaskOutcome, error) {
	logger := slog.New(tikeo.TaskSlogHandler{})
	logger.InfoContext(ctx, "go worker task started", "processor", task.ProcessorName, "instance", task.InstanceID, "payload_bytes", len(task.Payload))
	switch task.ProcessorName {
	case "", "demo.echo":
		logger.InfoContext(ctx, "demo echo payload", "payload", string(task.Payload))
		return tikeo.TaskOutcome{Success: true, Message: "go demo echo processed"}, nil
	case "demo.context":
		logger.InfoContext(ctx, "demo context", "job_id", task.JobID, "instance_id", task.InstanceID)
		return tikeo.TaskOutcome{Success: true, Message: fmt.Sprintf("go demo context processed instance=%s", task.InstanceID)}, nil
	case "demo.bytes":
		logger.InfoContext(ctx, "demo bytes", "payload", string(task.Payload), "payload_bytes", len(task.Payload))
		return tikeo.TaskOutcome{Success: true, Message: fmt.Sprintf("go demo bytes processed payload_bytes=%d", len(task.Payload))}, nil
	case "demo.heartbeat":
		logger.InfoContext(ctx, "demo heartbeat", "job_id", task.JobID, "instance_id", task.InstanceID)
		return tikeo.TaskOutcome{Success: true, Message: "go demo heartbeat processed"}, nil
	case "billing.sql-sync":
		logger.InfoContext(ctx, "billing sql sync payload", "payload", string(task.Payload))
		return tikeo.TaskOutcome{Success: true, Message: "go demo sql plugin processed"}, nil
	case "demo.fail":
		logger.ErrorContext(ctx, "demo intentional failure", "payload", string(task.Payload))
		return tikeo.Failed("go demo intentional failure"), nil
	case "demo.exception":
		logger.ErrorContext(ctx, "demo runtime exception", "payload", string(task.Payload))
		panic("go demo runtime exception")
	default:
		logger.ErrorContext(ctx, "unsupported processor", "processor", task.ProcessorName)
		return tikeo.Failed("unsupported go demo processor: " + task.ProcessorName), nil
	}
}

func envOr(key, fallback string) string {
	if value := strings.TrimSpace(os.Getenv(key)); value != "" {
		return value
	}
	return fallback
}

func dryRunEnabled() bool {
	return enabled("TIKEO_WORKER_DRY_RUN") || disabled("TIKEO_WORKER_CONNECT")
}

func enabledByDefault(key string) bool {
	return !disabled(key)
}

func enabled(key string) bool {
	switch strings.ToLower(strings.TrimSpace(os.Getenv(key))) {
	case "1", "true", "yes", "on":
		return true
	default:
		return false
	}
}

func disabled(key string) bool {
	switch strings.ToLower(strings.TrimSpace(os.Getenv(key))) {
	case "0", "false", "no", "off":
		return true
	default:
		return false
	}
}

func csvOr(key, fallback string) []string {
	value := envOr(key, fallback)
	if strings.TrimSpace(value) == "" {
		return nil
	}
	parts := strings.Split(value, ",")
	out := make([]string, 0, len(parts))
	for _, part := range parts {
		if item := strings.TrimSpace(part); item != "" {
			out = append(out, item)
		}
	}
	return out
}

func scriptSandboxBackend(language string) string {
	if value := strings.TrimSpace(os.Getenv("TIKEO_WORKER_SCRIPT_SANDBOX")); value != "" && !strings.EqualFold(value, "auto") {
		return strings.ToLower(value)
	}
	switch strings.ToLower(strings.TrimSpace(language)) {
	case "javascript", "js", "typescript", "ts":
		return "deno"
	default:
		return "srt"
	}
}

func resolveSrtInterpreter(language string, resolver tikeo.SandboxToolResolver) (string, bool) {
	switch strings.ToLower(strings.TrimSpace(language)) {
	case "shell", "sh", "bash":
		return resolver.ResolveInterpreter("sh")
	case "python", "py":
		return resolver.ResolveInterpreter("python3")
	case "powershell", "pwsh":
		return resolver.ResolvePowerShell()
	case "php":
		return resolver.ResolveInterpreter("php")
	case "groovy":
		return resolver.ResolveInterpreter("groovy")
	case "rhai":
		return resolver.ResolveRhai()
	default:
		return resolver.ResolveInterpreter("sh")
	}
}

func sandboxToolPathEntries(srtCommand, rgCommand, interpreter string, resolver tikeo.SandboxToolResolver) []string {
	entries := []string{}
	for _, command := range []string{srtCommand, rgCommand, interpreter} {
		if parent := toolParent(command); parent != "" {
			entries = append(entries, parent)
		}
	}
	if node, ok := resolver.ResolveNode(); ok {
		if parent := toolParent(node); parent != "" {
			entries = append(entries, parent)
		}
	}
	if npm, ok := resolver.ResolveNpm(); ok {
		if parent := toolParent(npm); parent != "" {
			entries = append(entries, parent)
		}
	}
	return dedupeStrings(entries)
}

func dedupeStrings(values []string) []string {
	seen := map[string]struct{}{}
	out := []string{}
	for _, value := range values {
		value = strings.TrimSpace(value)
		if value == "" {
			continue
		}
		if _, exists := seen[value]; exists {
			continue
		}
		seen[value] = struct{}{}
		out = append(out, value)
	}
	return out
}

func toolParent(command string) string {
	if idx := strings.LastIndex(command, string(os.PathSeparator)); idx > 0 {
		return command[:idx]
	}
	return ""
}

func scriptImage(language string) string {
	switch strings.ToLower(strings.TrimSpace(language)) {
	case "shell", "sh", "bash":
		return envOr("TIKEO_SHELL_IMAGE", "alpine:latest")
	case "python", "py":
		return envOr("TIKEO_PYTHON_IMAGE", "python:alpine")
	case "javascript", "js":
		return envOr("TIKEO_JAVASCRIPT_IMAGE", "denoland/deno:alpine")
	case "typescript", "ts":
		return envOr("TIKEO_TYPESCRIPT_IMAGE", "denoland/deno:alpine")
	case "powershell", "pwsh":
		return envOr("TIKEO_POWERSHELL_IMAGE", "mcr.microsoft.com/powershell:latest")
	case "php":
		return envOr("TIKEO_PHP_IMAGE", "php:cli-alpine")
	case "groovy":
		return envOr("TIKEO_GROOVY_IMAGE", "groovy:latest")
	case "rhai":
		return envOr("TIKEO_RHAI_IMAGE", "rhaiscript/rhai:latest")
	default:
		return ""
	}
}
