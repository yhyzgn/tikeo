package tikee

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
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

type capabilityAdvertiser interface {
	AdvertiseCapability() bool
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
	Log                 func(level, message string)
}

func (t ScriptRunnerTask) withLogSink(log func(level, message string)) ScriptRunnerTask {
	t.Log = log
	return t
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
	languages := make([]string, 0, len(r.runners))
	for language := range r.runners {
		languages = append(languages, language)
	}
	sort.Strings(languages)
	for _, language := range languages {
		runner := r.runners[language]
		if advertiser, ok := runner.(capabilityAdvertiser); ok && !advertiser.AdvertiseCapability() {
			continue
		}
		config.AddScriptRunner(runner.Language(), runner.SandboxBackend())
	}
}

// UnavailableScriptRunner keeps a fail-closed handler registered for one language but is not advertised as executable capability.
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

func (r *UnavailableScriptRunner) AdvertiseCapability() bool { return false }

func (r *UnavailableScriptRunner) Run(_ context.Context, task ScriptRunnerTask) (TaskOutcome, error) {
	if err := validateScriptTask(r.language, task); err != nil {
		return Failed(err.Error()), nil
	}
	return Failed(fmt.Sprintf("%s script runner backend is unavailable: %s", r.language, r.reason)), nil
}

// ContainerScriptRunner executes scripts inside a Docker/Podman-compatible sandbox.
type ContainerScriptRunner struct {
	language       string
	sandboxBackend string
	runtimeCommand string
	image          string
	runtimeArgs    []string
}

// NewContainerScriptRunner creates a default-deny container sandbox runner for one language.
func NewContainerScriptRunner(language, runtimeCommand, image string, runtimeArgs ...string) (*ContainerScriptRunner, error) {
	language = normalizeScriptLanguage(language)
	backend, err := normalizeScriptSandboxBackend(runtimeCommand, language)
	if err != nil {
		return nil, err
	}
	if backend != "docker" && backend != "podman" {
		return nil, fmt.Errorf("container script runner requires docker or podman backend, got %s", backend)
	}
	if strings.TrimSpace(image) == "" {
		return nil, fmt.Errorf("container script runner requires an image for %s", language)
	}
	return &ContainerScriptRunner{
		language:       language,
		sandboxBackend: backend,
		runtimeCommand: backend,
		image:          strings.TrimSpace(image),
		runtimeArgs:    append([]string(nil), runtimeArgs...),
	}, nil
}

func (r *ContainerScriptRunner) Language() string { return r.language }

func (r *ContainerScriptRunner) SandboxBackend() string { return r.sandboxBackend }

func (r *ContainerScriptRunner) Run(ctx context.Context, task ScriptRunnerTask) (TaskOutcome, error) {
	if err := validateScriptTask(r.language, task); err != nil {
		return Failed(err.Error()), nil
	}
	if task.AllowNetwork || len(task.AllowedNetworkHosts) > 0 {
		return Failed("container script runner rejects network grants without host-level filtering"), nil
	}
	if len(task.SecretRefs) > 0 {
		return Failed("container script runner rejects secret refs without a worker-local secret provider"), nil
	}
	args, err := r.containerArgs(task)
	if err != nil {
		return Failed(err.Error()), nil
	}
	timeout := task.Timeout
	if timeout <= 0 {
		timeout = 30 * time.Second
	}
	runCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	cmd := exec.CommandContext(runCtx, r.runtimeCommand, args...)
	cmd.Stdin = bytes.NewReader(task.Content)
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err = cmd.Run()
	emitScriptCommandOutput(task.Log, "info", stdout.Bytes())
	emitScriptCommandOutput(task.Log, "error", stderr.Bytes())
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

func (r *ContainerScriptRunner) containerArgs(task ScriptRunnerTask) ([]string, error) {
	args := []string{"run", "--rm", "-i", "--network=none", "--read-only", "--tmpfs", "/tmp:rw,noexec,nosuid,size=16m"}
	if task.MaxOutputBytes > 0 {
		// Output is still enforced after process completion; the memory limit is a sandbox hint.
		args = append(args, "--memory", fmt.Sprintf("%d", maxTaskMemoryBytes(task)))
	}
	args = append(args, r.runtimeArgs...)
	for _, path := range task.ReadOnlyPaths {
		mount, err := containerMount(path, true)
		if err != nil {
			return nil, err
		}
		args = append(args, "--mount", mount)
	}
	for _, path := range task.WritablePaths {
		mount, err := containerMount(path, false)
		if err != nil {
			return nil, err
		}
		args = append(args, "--mount", mount)
	}
	args = append(args,
		"--env", "TIKEE_SCRIPT_ID="+task.ScriptID,
		"--env", "TIKEE_SCRIPT_VERSION_ID="+task.VersionID,
		"--env", fmt.Sprintf("TIKEE_SCRIPT_VERSION_NUMBER=%d", task.VersionNumber),
	)
	for _, name := range task.AllowedEnvVars {
		if value, ok := os.LookupEnv(name); ok {
			args = append(args, "--env", name+"="+value)
		}
	}
	args = append(args, r.image)
	command, commandArgs := defaultScriptCommand(r.language)
	args = append(args, command)
	args = append(args, commandArgs...)
	return args, nil
}

func maxTaskMemoryBytes(task ScriptRunnerTask) uint64 {
	if task.MaxOutputBytes == 0 {
		return 64 * 1024 * 1024
	}
	return 64 * 1024 * 1024
}

func containerMount(path string, readOnly bool) (string, error) {
	trimmed := strings.TrimSpace(path)
	if trimmed == "" || trimmed != path || !filepath.IsAbs(trimmed) || strings.Contains(trimmed, "..") {
		return "", fmt.Errorf("script file grant path must be clean and absolute: %s", path)
	}
	mode := ""
	if readOnly {
		mode = ",readonly"
	}
	return fmt.Sprintf("type=bind,src=%s,dst=%s%s", trimmed, trimmed, mode), nil
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

func defaultScriptCommand(language string) (string, []string) {
	switch normalizeScriptLanguage(language) {
	case "shell":
		return "sh", []string{"-s"}
	case "python":
		return "python3", []string{"-"}
	case "javascript", "typescript":
		return "deno", []string{"run", "--no-prompt", "-"}
	case "powershell":
		return "pwsh", []string{"-NoProfile", "-NonInteractive", "-Command", "-"}
	case "php":
		return "php", nil
	case "groovy":
		return "groovy", nil
	case "rhai":
		return "rhai", nil
	default:
		return "sh", []string{"-s"}
	}
}

func (r *LocalCommandScriptRunner) Language() string { return r.language }

func (r *LocalCommandScriptRunner) SandboxBackend() string { return r.sandboxBackend }

func (r *LocalCommandScriptRunner) Run(ctx context.Context, task ScriptRunnerTask) (TaskOutcome, error) {
	if err := validateLocalScriptTask(r.language, task); err != nil {
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
	emitScriptCommandOutput(task.Log, "info", stdout.Bytes())
	emitScriptCommandOutput(task.Log, "error", stderr.Bytes())
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

func emitScriptCommandOutput(log func(level, message string), level string, output []byte) {
	if log == nil || len(output) == 0 {
		return
	}
	text := strings.ReplaceAll(string(output), "\r\n", "\n")
	for _, line := range strings.Split(text, "\n") {
		line = strings.TrimSpace(line)
		if line != "" {
			log(level, "[script] "+line)
		}
	}
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
	return nil
}

func validateLocalScriptTask(language string, task ScriptRunnerTask) error {
	if err := validateScriptTask(language, task); err != nil {
		return err
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
