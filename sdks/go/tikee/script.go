package tikee

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"os/exec"
	"strings"
	"time"

	"github.com/yhyzgn/tikee/sdks/go/tikee/internal/workerpb"
)

// ScriptRunner executes one dynamic script binding for a declared language.
type ScriptRunner interface {
	Language() string
	SandboxBackend() string
	Run(context.Context, ScriptRunnerTask) (TaskOutcome, error)
}

// ScriptRunnerTask is the immutable script snapshot delivered by tikee server.
type ScriptRunnerTask struct {
	ScriptID            string
	VersionID           string
	VersionNumber       uint64
	Language            string
	Content             []byte
	ContentSHA256       string
	Timeout             time.Duration
	MaxOutputBytes      uint64
	AllowNetwork        bool
	AllowedEnvVars      []string
	ReadOnlyPaths       []string
	WritablePaths       []string
	SecretRefs          []string
	AllowedNetworkHosts []string
	SandboxBackend      string
	InstanceID          string
	JobID               string
}

// ScriptRunnerRegistry stores explicitly enabled script runners.
type ScriptRunnerRegistry struct {
	runners map[string]ScriptRunner
}

// NewScriptRunnerRegistry creates an empty script runner registry.
func NewScriptRunnerRegistry() *ScriptRunnerRegistry {
	return &ScriptRunnerRegistry{runners: map[string]ScriptRunner{}}
}

// Register enables one runner and returns the registry for fluent setup.
func (r *ScriptRunnerRegistry) Register(runner ScriptRunner) *ScriptRunnerRegistry {
	if r == nil || runner == nil {
		return r
	}
	language := strings.TrimSpace(strings.ToLower(runner.Language()))
	if language == "" {
		return r
	}
	r.runners[language] = runner
	return r
}

func (r *ScriptRunnerRegistry) get(language string) ScriptRunner {
	if r == nil {
		return nil
	}
	return r.runners[strings.TrimSpace(strings.ToLower(language))]
}

// AddCapabilities advertises registered runners on the worker config.
func (r *ScriptRunnerRegistry) AddCapabilities(config *WorkerConfig) {
	if r == nil || config == nil {
		return
	}
	for _, runner := range r.runners {
		config.AddScriptRunner(runner.Language(), runner.SandboxBackend())
	}
}

// UnavailableScriptRunner advertises one language but fails closed until a backend is configured.
type UnavailableScriptRunner struct {
	language       string
	sandboxBackend string
	reason         string
}

func NewUnavailableScriptRunner(language, sandboxBackend, reason string) *UnavailableScriptRunner {
	language = normalizeScriptLanguage(language)
	backend, err := normalizeScriptSandboxBackend(sandboxBackend, language)
	if err != nil {
		backend = defaultSandboxBackend(language)
		reason = strings.TrimSpace(reason + "; " + err.Error())
	}
	return &UnavailableScriptRunner{language: language, sandboxBackend: backend, reason: reason}
}

func (r *UnavailableScriptRunner) Language() string { return r.language }

func (r *UnavailableScriptRunner) SandboxBackend() string { return r.sandboxBackend }

func (r *UnavailableScriptRunner) Run(_ context.Context, task ScriptRunnerTask) (TaskOutcome, error) {
	if err := validateScriptTask(r.language, task); err != nil {
		return Failed(err.Error()), nil
	}
	return Failed(fmt.Sprintf("%s script runner backend is unavailable: %s", r.language, r.reason)), nil
}

// LocalCommandScriptRunner executes scripts with a local command. It is a development-only implementation detail and always advertises the Java-compatible custom backend.
type LocalCommandScriptRunner struct {
	language       string
	sandboxBackend string
	command        string
	args           []string
}

// NewLocalCommandScriptRunner creates a local command runner for Java-parity demo languages.
func NewLocalCommandScriptRunner(language, sandboxBackend string) (*LocalCommandScriptRunner, error) {
	language = normalizeScriptLanguage(language)
	backend, err := normalizeScriptSandboxBackend(sandboxBackend, language)
	if err != nil {
		return nil, err
	}
	if backend != "custom" {
		return nil, fmt.Errorf("local command script runner must use custom sandbox backend, got %s", backend)
	}
	switch language {
	case "shell":
		return &LocalCommandScriptRunner{language: language, sandboxBackend: backend, command: "sh", args: []string{"-s"}}, nil
	case "python":
		return &LocalCommandScriptRunner{language: language, sandboxBackend: backend, command: "python3", args: []string{"-"}}, nil
	case "javascript", "typescript":
		return &LocalCommandScriptRunner{language: language, sandboxBackend: backend, command: "deno", args: []string{"run", "--no-prompt", "-"}}, nil
	case "powershell":
		return &LocalCommandScriptRunner{language: language, sandboxBackend: backend, command: "pwsh", args: []string{"-NoProfile", "-NonInteractive", "-Command", "-"}}, nil
	case "php":
		return &LocalCommandScriptRunner{language: language, sandboxBackend: backend, command: "php"}, nil
	case "groovy":
		return &LocalCommandScriptRunner{language: language, sandboxBackend: backend, command: "groovy"}, nil
	case "rhai":
		return &LocalCommandScriptRunner{language: language, sandboxBackend: backend, command: "rhai"}, nil
	default:
		return nil, fmt.Errorf("unsupported local script runner language: %s", language)
	}
}

func normalizeScriptLanguage(language string) string {
	switch strings.TrimSpace(strings.ToLower(language)) {
	case "shell", "sh", "bash":
		return "shell"
	case "python", "py":
		return "python"
	case "node", "nodejs", "javascript", "js":
		return "javascript"
	case "typescript", "ts":
		return "typescript"
	case "powershell", "pwsh":
		return "powershell"
	case "php":
		return "php"
	case "groovy":
		return "groovy"
	case "rhai":
		return "rhai"
	default:
		return strings.TrimSpace(strings.ToLower(language))
	}
}

func normalizeScriptSandboxBackend(backend string, language string) (string, error) {
	normalized := strings.TrimSpace(strings.ToLower(backend))
	if normalized == "" || normalized == "auto" {
		return defaultSandboxBackend(language), nil
	}
	switch normalized {
	case "wasmtime":
		return "wasmtime", nil
	case "wasmedge", "wasm_edge", "wasm-edge":
		return "wasmedge", nil
	case "srt", "anthropic_srt", "anthropic-srt", "sandbox_runtime", "sandbox-runtime":
		return "srt", nil
	case "deno":
		return "deno", nil
	case "v8", "v8_isolate", "v8-isolate":
		return "v8", nil
	case "docker":
		return "docker", nil
	case "podman":
		return "podman", nil
	case "custom":
		return "custom", nil
	default:
		return "", fmt.Errorf("unsupported script sandbox backend: %s", backend)
	}
}

func defaultSandboxBackend(language string) string {
	switch normalizeScriptLanguage(language) {
	case "javascript", "typescript":
		return "deno"
	default:
		return "srt"
	}
}

func (r *LocalCommandScriptRunner) Language() string { return r.language }

func (r *LocalCommandScriptRunner) SandboxBackend() string { return r.sandboxBackend }

func (r *LocalCommandScriptRunner) Run(ctx context.Context, task ScriptRunnerTask) (TaskOutcome, error) {
	if err := validateScriptTask(r.language, task); err != nil {
		return Failed(err.Error()), nil
	}
	timeout := task.Timeout
	if timeout <= 0 {
		timeout = 30 * time.Second
	}
	runCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	cmd := exec.CommandContext(runCtx, r.command, r.args...)
	cmd.Stdin = bytes.NewReader(task.Content)
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	if runCtx.Err() != nil {
		return Failed("script runner timed out"), nil
	}
	message := strings.TrimSpace(stdout.String())
	if err != nil {
		if message == "" {
			message = strings.TrimSpace(stderr.String())
		}
		if message == "" {
			message = err.Error()
		}
		return Failed(limitOutput(message, task.MaxOutputBytes)), nil
	}
	return TaskOutcome{Success: true, Message: limitOutput(message, task.MaxOutputBytes)}, nil
}

func validateScriptTask(language string, task ScriptRunnerTask) error {
	if normalizeScriptLanguage(task.Language) != language {
		return fmt.Errorf("script runner language mismatch: task=%s runner=%s", task.Language, language)
	}
	if task.ScriptID == "" || task.VersionNumber == 0 || len(task.Content) == 0 {
		return errors.New("script runner requires a released immutable script version snapshot")
	}
	if task.ContentSHA256 == "" {
		return errors.New("script runner requires a content sha256 digest")
	}
	digest := sha256.Sum256(task.Content)
	if hex.EncodeToString(digest[:]) != strings.ToLower(task.ContentSHA256) {
		return errors.New("script content digest mismatch")
	}
	if task.AllowNetwork || len(task.AllowedNetworkHosts) > 0 {
		return errors.New("local script runner rejects network access")
	}
	if len(task.SecretRefs) > 0 {
		return errors.New("local script runner rejects secret refs")
	}
	if len(task.ReadOnlyPaths) > 0 || len(task.WritablePaths) > 0 {
		return errors.New("local script runner rejects filesystem grants")
	}
	return nil
}

func scriptRunnerTask(task *workerpb.DispatchTask, binding *workerpb.ScriptProcessorBinding) ScriptRunnerTask {
	return ScriptRunnerTask{
		ScriptID:            binding.GetScriptId(),
		VersionID:           binding.GetVersionId(),
		VersionNumber:       binding.GetVersionNumber(),
		Language:            binding.GetLanguage(),
		Content:             binding.GetContent(),
		ContentSHA256:       binding.GetContentSha256(),
		Timeout:             time.Duration(binding.GetTimeoutMs()) * time.Millisecond,
		MaxOutputBytes:      binding.GetMaxOutputBytes(),
		AllowNetwork:        binding.GetAllowNetwork(),
		AllowedEnvVars:      append([]string(nil), binding.GetAllowedEnvVars()...),
		ReadOnlyPaths:       append([]string(nil), binding.GetReadOnlyPaths()...),
		WritablePaths:       append([]string(nil), binding.GetWritablePaths()...),
		SecretRefs:          append([]string(nil), binding.GetSecretRefs()...),
		AllowedNetworkHosts: append([]string(nil), binding.GetAllowedNetworkHosts()...),
		SandboxBackend:      binding.GetSandboxBackend(),
		InstanceID:          task.GetInstanceId(),
		JobID:               task.GetJobId(),
	}
}

func limitOutput(message string, max uint64) string {
	if max == 0 || uint64(len(message)) <= max {
		return message
	}
	return message[:max]
}
