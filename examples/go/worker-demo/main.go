package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"strings"
	"time"

	tikee "github.com/yhyzgn/tikee/sdks/go/tikee"
)

func main() {
	config := tikee.LocalConfig(envOr("TIKEE_WORKER_ENDPOINT", "http://127.0.0.1:9998"), envOr("TIKEE_WORKER_CLIENT_INSTANCE_ID", "go-worker-demo-local"))
	config.Namespace = envOr("TIKEE_WORKER_NAMESPACE", "dev-alpha")
	config.App = envOr("TIKEE_WORKER_APP", "orders")
	config.Cluster = envOr("TIKEE_WORKER_CLUSTER", "local")
	config.Region = envOr("TIKEE_WORKER_REGION", "local")
	config.AddTag("go")
	config.AddTag("manual-demo")
	for _, processor := range csvOr("TIKEE_WORKER_SDK_PROCESSORS", "demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail") {
		config.AddSDKProcessor(processor)
	}
	config.Labels["worker_pool"] = envOr("TIKEE_WORKER_POOL", "go-blue")
	if enabledByDefault("TIKEE_ENABLE_PLUGIN_SQL") {
		config.AddPluginProcessor(envOr("TIKEE_PLUGIN_SQL_TYPE", "sql"), envOr("TIKEE_PLUGIN_SQL_PROCESSOR", "billing.sql-sync"))
		config.Labels["plugin_sql"] = "enabled"
	}
	scripts := tikee.NewScriptRunnerRegistry()
	for _, lang := range csvOr("TIKEE_WORKER_SCRIPT_LANGUAGES", "shell,python,javascript,typescript,powershell,php,groovy,rhai") {
		if disabled("TIKEE_ENABLE_SCRIPT_" + strings.ToUpper(lang)) {
			continue
		}
		if !enabled("TIKEE_ENABLE_LOCAL_SCRIPT_"+strings.ToUpper(lang)) && !enabled("TIKEE_ENABLE_UNAVAILABLE_SCRIPT_ADAPTERS") {
			continue
		}
		backend := scriptSandboxBackend(lang)
		if enabled("TIKEE_ENABLE_LOCAL_SCRIPT_" + strings.ToUpper(lang)) {
			runner, err := tikee.NewLocalCommandScriptRunner(lang, "custom")
			if err != nil {
				log.Printf("local script runner %s skipped: %v", lang, err)
				continue
			}
			scripts.Register(runner)
			continue
		}
		reason := backend + " backend is declared but not executable in this Go demo process"
		scripts.Register(tikee.NewUnavailableScriptRunner(lang, backend, reason))
	}
	scripts.AddCapabilities(&config)

	client, err := tikee.NewClient(config)
	if err != nil {
		log.Fatal(err)
	}
	processor := tikee.TaskProcessorFunc(func(_ context.Context, task tikee.TaskContext) (tikee.TaskOutcome, error) {
		fmt.Printf("[go-worker] processor=%s instance=%s payload_bytes=%d\n", task.ProcessorName, task.InstanceID, len(task.Payload))
		switch task.ProcessorName {
		case "", "demo.echo":
			fmt.Printf("[demo.echo] payload='%s'\n", string(task.Payload))
			return tikee.TaskOutcome{Success: true, Message: "go demo echo processed"}, nil
		case "demo.context":
			fmt.Printf("[demo.context] jobId=%s instanceId=%s\n", task.JobID, task.InstanceID)
			return tikee.TaskOutcome{Success: true, Message: fmt.Sprintf("go demo context processed instance=%s", task.InstanceID)}, nil
		case "demo.bytes":
			fmt.Printf("[demo.bytes] payload='%s' length=%d\n", string(task.Payload), len(task.Payload))
			return tikee.TaskOutcome{Success: true, Message: fmt.Sprintf("go demo bytes processed payload_bytes=%d", len(task.Payload))}, nil
		case "demo.heartbeat":
			fmt.Printf("[demo.heartbeat] tick jobId=%s instanceId=%s\n", task.JobID, task.InstanceID)
			return tikee.TaskOutcome{Success: true, Message: "go demo heartbeat processed"}, nil
		case "billing.sql-sync":
			fmt.Printf("[billing.sql-sync] plugin SQL processor received payload='%s'\n", string(task.Payload))
			return tikee.TaskOutcome{Success: true, Message: "go demo sql plugin processed"}, nil
		case "demo.fail":
			fmt.Fprintf(os.Stderr, "[demo.fail] intentional failure payload='%s'\n", string(task.Payload))
			return tikee.Failed("go demo intentional failure"), nil
		default:
			fmt.Fprintf(os.Stderr, "[go-worker] unsupported processor=%s\n", task.ProcessorName)
			return tikee.Failed("unsupported go demo processor: " + task.ProcessorName), nil
		}
	})

	registration := client.Registration()
	pretty, _ := json.MarshalIndent(registration, "", "  ")
	fmt.Printf("go worker demo configured: %s\n", pretty)

	if enabled("TIKEE_MANAGEMENT_CREATE_EXAMPLES") {
		mgmt := tikee.NewManagementClient(envOr("TIKEE_HTTP_URL", "http://127.0.0.1:8080"), os.Getenv("TIKEE_API_KEY"), config.Namespace, config.App)
		for _, job := range []tikee.CreateJobRequest{
			tikee.APIJob("go-echo-api", "demo.echo"),
			tikee.PluginAPIJob("go-sql-sync-api", "sql", "billing.sql-sync"),
		} {
			created, err := mgmt.CreateJob(context.Background(), job)
			if err != nil {
				log.Printf("create job %s failed: %v", job.Name, err)
				continue
			}
			log.Printf("created job %s/%s %s", created.Namespace, created.App, created.Name)
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

	oneshot := enabled("TIKEE_WORKER_ONESHOT")
	for {
		if runWorkerSession(client, processor, scripts, oneshot) {
			return
		}
		time.Sleep(2 * time.Second)
	}
}

func runWorkerSession(
	client *tikee.Client,
	processor tikee.TaskProcessor,
	scripts *tikee.ScriptRunnerRegistry,
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
	if enabled("TIKEE_WORKER_HEARTBEAT_ON_START") {
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

func envOr(key, fallback string) string {
	if value := strings.TrimSpace(os.Getenv(key)); value != "" {
		return value
	}
	return fallback
}

func dryRunEnabled() bool {
	return enabled("TIKEE_WORKER_DRY_RUN") || disabled("TIKEE_WORKER_CONNECT")
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
	if value := strings.TrimSpace(os.Getenv("TIKEE_WORKER_SCRIPT_SANDBOX")); value != "" {
		return strings.ToLower(value)
	}
	switch strings.ToLower(strings.TrimSpace(language)) {
	case "javascript", "js", "typescript", "ts":
		return "deno"
	default:
		return "srt"
	}
}
