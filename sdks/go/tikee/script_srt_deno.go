package tikee

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

// SrtScriptRunner executes native scripts through Anthropic Sandbox Runtime.
type SrtScriptRunner struct {
	language       string
	runtimeCommand string
	interpreter    string
	extraPath      []string
}

func NewSrtScriptRunner(language, runtimeCommand, interpreter string, extraPath ...string) (*SrtScriptRunner, error) {
	language = normalizeScriptLanguage(language)
	if strings.TrimSpace(runtimeCommand) == "" || strings.TrimSpace(interpreter) == "" {
		return nil, fmt.Errorf("SRT runner requires runtime and interpreter commands")
	}
	return &SrtScriptRunner{language: language, runtimeCommand: runtimeCommand, interpreter: interpreter, extraPath: append([]string(nil), extraPath...)}, nil
}

func (r *SrtScriptRunner) Language() string { return r.language }

func (r *SrtScriptRunner) SandboxBackend() string { return "srt" }

func (r *SrtScriptRunner) Run(ctx context.Context, task ScriptRunnerTask) (TaskOutcome, error) {
	if err := validateScriptTask(r.language, task); err != nil {
		return Failed(err.Error()), nil
	}
	if len(task.SecretRefs) > 0 {
		return Failed("SRT script runner rejects secret refs without a worker-local secret provider"), nil
	}
	runtimeDirs, err := newScriptTaskRuntimeDirs("tikee-srt-" + r.language + "-runtime")
	if err != nil {
		return Failed(err.Error()), nil
	}
	defer runtimeDirs.cleanup()
	scriptFile := ""
	if r.language == "rhai" {
		scriptFile = runtimeDirs.scriptFile("rhai")
		if err := os.WriteFile(scriptFile, task.Content, 0o600); err != nil {
			return Failed(err.Error()), nil
		}
	}
	settings, cleanup, err := writeSrtSettings(task, runtimeDirs, scriptFile)
	if err != nil {
		return Failed(err.Error()), nil
	}
	defer cleanup()
	timeout := task.Timeout
	if timeout <= 0 {
		timeout = 30 * time.Second
	}
	runCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	cmd := exec.CommandContext(runCtx, r.runtimeCommand, "--settings", settings, "-c", r.shellCommand(task.Content, scriptFile))
	cmd.Dir = runtimeDirs.workingDir()
	runtimeDirs.applySrtEnvironment(cmd, r.extraPath)
	if r.language == "powershell" {
		runtimeDirs.applyPowerShellEnvironment(cmd)
	}
	cmd.Env = append(cmd.Env,
		"TIKEE_SCRIPT_ID="+task.ScriptID,
		"TIKEE_SCRIPT_VERSION_ID="+task.VersionID,
		fmt.Sprintf("TIKEE_SCRIPT_VERSION_NUMBER=%d", task.VersionNumber),
	)
	cmd.Env = appendAllowedUnmanagedEnv(cmd.Env, task.AllowedEnvVars)
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
	if r.language == "rhai" {
		if diagnostic := rhaiDiagnosticMessage(stdout.Bytes(), stderr.Bytes()); diagnostic != "" {
			return Failed(limitOutput(diagnostic, task.MaxOutputBytes)), nil
		}
	}
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

func rhaiDiagnosticMessage(stdout []byte, stderr []byte) string {
	combined := string(stdout) + "\n" + string(stderr)
	if !strings.Contains(combined, "Syntax error:") &&
		!strings.Contains(combined, "Runtime error:") &&
		!strings.Contains(combined, "Parse error:") {
		return ""
	}
	lines := strings.Split(strings.ReplaceAll(combined, "\r\n", "\n"), "\n")
	trimmed := make([]string, 0, len(lines))
	for _, line := range lines {
		if item := strings.TrimSpace(line); item != "" {
			trimmed = append(trimmed, item)
		}
	}
	if len(trimmed) == 0 {
		return "Rhai script reported an execution diagnostic"
	}
	return strings.Join(trimmed, "\n")
}

func (r *SrtScriptRunner) shellCommand(content []byte, scriptFile string) string {
	source := string(content)
	switch r.language {
	case "shell":
		return source
	case "python":
		return heredoc(r.interpreter+" -", "PY", source)
	case "powershell":
		return heredoc(r.interpreter+" -NoLogo -NoProfile -NonInteractive -InputFormat Text -OutputFormat Text -Command -", "PWSH", source)
	case "php", "groovy":
		return heredoc(r.interpreter, strings.ToUpper(r.language), source)
	case "rhai":
		if scriptFile != "" {
			return r.interpreter + " " + shellQuote(scriptFile)
		}
		return heredoc(r.interpreter, "RHAI", source)
	default:
		return heredoc(r.interpreter, "SCRIPT", source)
	}
}

// DenoScriptRunner executes JavaScript/TypeScript through Deno's permission sandbox.
type DenoScriptRunner struct {
	language string
	command  string
}

func NewDenoScriptRunner(language, command string) (*DenoScriptRunner, error) {
	language = normalizeScriptLanguage(language)
	if language != "javascript" && language != "typescript" {
		return nil, fmt.Errorf("Deno runner supports JavaScript and TypeScript only")
	}
	if strings.TrimSpace(command) == "" {
		return nil, fmt.Errorf("Deno runner requires a command")
	}
	return &DenoScriptRunner{language: language, command: command}, nil
}

func (r *DenoScriptRunner) Language() string { return r.language }

func (r *DenoScriptRunner) SandboxBackend() string { return "deno" }

func (r *DenoScriptRunner) Run(ctx context.Context, task ScriptRunnerTask) (TaskOutcome, error) {
	if err := validateScriptTask(r.language, task); err != nil {
		return Failed(err.Error()), nil
	}
	if len(task.SecretRefs) > 0 {
		return Failed("Deno script runner rejects secret refs without a worker-local secret provider"), nil
	}
	runtimeDirs, err := newScriptTaskRuntimeDirs("tikee-deno-" + r.language + "-runtime")
	if err != nil {
		return Failed(err.Error()), nil
	}
	defer runtimeDirs.cleanup()
	args := []string{"run", "--no-prompt"}
	if task.AllowNetwork {
		args = append(args, "--allow-net")
	} else if len(task.AllowedNetworkHosts) > 0 {
		args = append(args, "--allow-net="+strings.Join(task.AllowedNetworkHosts, ","))
	}
	if len(task.AllowedEnvVars) > 0 {
		args = append(args, "--allow-env="+strings.Join(task.AllowedEnvVars, ","))
	}
	if len(task.ReadOnlyPaths) > 0 {
		args = append(args, "--allow-read="+strings.Join(task.ReadOnlyPaths, ","))
	}
	writablePaths := append([]string(nil), task.WritablePaths...)
	writablePaths = append(writablePaths, runtimeDirs.writablePaths()...)
	if len(writablePaths) > 0 {
		args = append(args, "--allow-write="+strings.Join(writablePaths, ","))
	}
	args = append(args, "-")
	return runDenoScript(ctx, r.command, args, task, runtimeDirs)
}

func runDenoScript(ctx context.Context, command string, args []string, task ScriptRunnerTask, runtimeDirs *scriptTaskRuntimeDirs) (TaskOutcome, error) {
	timeout := task.Timeout
	if timeout <= 0 {
		timeout = 30 * time.Second
	}
	runCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	cmd := exec.CommandContext(runCtx, command, args...)
	cmd.Dir = runtimeDirs.workingDir()
	cmd.Stdin = bytes.NewReader(task.Content)
	runtimeDirs.applyDenoEnvironment(cmd)
	cmd.Env = append(cmd.Env,
		"TIKEE_SCRIPT_ID="+task.ScriptID,
		"TIKEE_SCRIPT_VERSION_ID="+task.VersionID,
		fmt.Sprintf("TIKEE_SCRIPT_VERSION_NUMBER=%d", task.VersionNumber),
	)
	cmd.Env = appendAllowedUnmanagedEnv(cmd.Env, task.AllowedEnvVars)
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

func writeSrtSettings(task ScriptRunnerTask, runtimeDirs *scriptTaskRuntimeDirs, scriptFile string) (string, func(), error) {
	allowRead := append([]string(nil), task.ReadOnlyPaths...)
	if scriptFile != "" {
		allowRead = append(allowRead, scriptFile)
	}
	allowWrite := append([]string(nil), task.WritablePaths...)
	if runtimeDirs != nil {
		allowWrite = append(allowWrite, runtimeDirs.writablePaths()...)
	}
	settings := map[string]any{
		"network": map[string]any{
			"allowUnixSocket": false,
			"allowedDomains":  stringSliceOrEmpty(task.AllowedNetworkHosts),
			"deniedDomains":   []string{},
		},
		"filesystem": map[string]any{
			"allowRead":  stringSliceOrEmpty(allowRead),
			"allowWrite": stringSliceOrEmpty(allowWrite),
			"denyRead":   stringSliceOrEmpty(sensitiveReadDenies()),
			"denyWrite":  []string{},
		},
	}
	data, err := json.Marshal(settings)
	if err != nil {
		return "", func() {}, err
	}
	file, err := os.CreateTemp("", "tikee-srt-settings-*.json")
	if err != nil {
		return "", func() {}, err
	}
	if _, err := file.Write(data); err != nil {
		_ = file.Close()
		_ = os.Remove(file.Name())
		return "", func() {}, err
	}
	if err := file.Close(); err != nil {
		_ = os.Remove(file.Name())
		return "", func() {}, err
	}
	return file.Name(), func() { _ = os.Remove(file.Name()) }, nil
}

func stringSliceOrEmpty(values []string) []string {
	if values == nil {
		return []string{}
	}
	return values
}

func heredoc(command, marker, content string) string {
	delimiter := marker
	for strings.Contains(content, delimiter) {
		delimiter += "_TIKEE"
	}
	return command + " <<'" + delimiter + "'\n" + content + "\n" + delimiter
}

func shellQuote(value string) string {
	return "'" + strings.ReplaceAll(value, "'", "'\\''") + "'"
}

func userHome() string {
	if home, err := os.UserHomeDir(); err == nil {
		return filepath.Clean(home)
	}
	return ""
}

func sensitiveReadDenies() []string {
	home := userHome()
	if home == "" {
		return []string{}
	}
	paths := []string{".ssh", ".gnupg", ".aws", ".kube", ".docker", filepath.Join(".config", "tikee")}
	denies := make([]string, 0, len(paths))
	for _, path := range paths {
		denies = append(denies, filepath.Join(home, path))
	}
	return denies
}
