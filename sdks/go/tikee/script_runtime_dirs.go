package tikee

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync/atomic"
	"time"
)

// scriptTaskRuntimeDirs is the single source of truth for task-scoped script sandbox
// HOME/TMPDIR/XDG/runtime paths. Sandbox-backed Go runners must use this instead
// of scattering env names or ad-hoc temp directories per runner.
type scriptTaskRuntimeDirs struct {
	root       string
	home       string
	config     string
	cache      string
	data       string
	modules    string
	dotnetHome string
	tmp        string
	denoDir    string
}

func newScriptTaskRuntimeDirs(prefix string) (*scriptTaskRuntimeDirs, error) {
	root, err := os.MkdirTemp("", strings.TrimSpace(prefix)+"-*")
	if err != nil {
		return nil, err
	}
	data := filepath.Join(root, "data")
	cache := filepath.Join(root, "cache")
	dirs := &scriptTaskRuntimeDirs{
		root:       root,
		home:       filepath.Join(root, "home"),
		config:     filepath.Join(root, "config"),
		cache:      cache,
		data:       data,
		modules:    filepath.Join(data, "powershell", "Modules"),
		dotnetHome: filepath.Join(root, "dotnet"),
		tmp:        filepath.Join(root, "tmp"),
		denoDir:    filepath.Join(cache, "deno"),
	}
	for _, dir := range dirs.requiredDirectories() {
		if err := os.MkdirAll(dir, 0o700); err != nil {
			dirs.cleanup()
			return nil, err
		}
	}
	return dirs, nil
}

func (d *scriptTaskRuntimeDirs) requiredDirectories() []string {
	return []string{d.root, d.home, d.config, d.cache, d.data, d.modules, d.dotnetHome, d.tmp, d.denoDir}
}

func (d *scriptTaskRuntimeDirs) writablePaths() []string {
	return []string{d.root, d.home, d.config, d.cache, d.data, d.dotnetHome, d.tmp, d.denoDir}
}

func (d *scriptTaskRuntimeDirs) workingDir() string { return d.home }

func (d *scriptTaskRuntimeDirs) scriptFile(extension string) string {
	return filepath.Join(d.home, fmt.Sprintf("script-%d-%d.%s", time.Now().UnixMilli(), nextScriptTempSequence(), extension))
}

func (d *scriptTaskRuntimeDirs) applySrtEnvironment(cmd *exec.Cmd, extraPath []string) {
	cmd.Env = d.baseEnvironment(extraPath)
	cmd.Env = append(cmd.Env,
		"CLAUDE_CODE_TMPDIR="+d.tmp,
		"CLAUDE_TMPDIR="+d.tmp,
	)
}

func (d *scriptTaskRuntimeDirs) applyPowerShellEnvironment(cmd *exec.Cmd) {
	cmd.Env = append(cmd.Env,
		"PSModulePath="+d.modules,
		"DOTNET_CLI_HOME="+d.dotnetHome,
		"POWERSHELL_TELEMETRY_OPTOUT=1",
		"POWERSHELL_UPDATECHECK=Off",
	)
}

func (d *scriptTaskRuntimeDirs) applyDenoEnvironment(cmd *exec.Cmd) {
	cmd.Env = d.baseEnvironment(nil)
	cmd.Env = append(cmd.Env, "DENO_DIR="+d.denoDir)
}

func (d *scriptTaskRuntimeDirs) baseEnvironment(extraPath []string) []string {
	env := []string{
		"HOME=" + d.home,
		"XDG_CONFIG_HOME=" + d.config,
		"XDG_CACHE_HOME=" + d.cache,
		"XDG_DATA_HOME=" + d.data,
		"TMPDIR=" + d.tmp,
		"TERM=dumb",
		"NO_COLOR=1",
	}
	pathEntries := append([]string(nil), extraPath...)
	if path := os.Getenv("PATH"); path != "" {
		pathEntries = append(pathEntries, filepath.SplitList(path)...)
	}
	if len(pathEntries) > 0 {
		env = append(env, pathEnvKey()+"="+strings.Join(dedupePathList(pathEntries), string(os.PathListSeparator)))
	}
	return env
}

func appendAllowedUnmanagedEnv(env []string, allowed []string) []string {
	for _, name := range allowed {
		name = strings.TrimSpace(name)
		if name == "" || isManagedScriptEnvironmentName(name) {
			continue
		}
		if value, ok := os.LookupEnv(name); ok {
			env = append(env, name+"="+value)
		}
	}
	return env
}

func isManagedScriptEnvironmentName(name string) bool {
	switch name {
	case "HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME", "TMPDIR", "TERM", "NO_COLOR",
		"CLAUDE_CODE_TMPDIR", "CLAUDE_TMPDIR", "PSModulePath", "DOTNET_CLI_HOME", "POWERSHELL_TELEMETRY_OPTOUT",
		"POWERSHELL_UPDATECHECK", "DENO_DIR":
		return true
	default:
		return false
	}
}

func (d *scriptTaskRuntimeDirs) cleanup() { _ = os.RemoveAll(d.root) }

func pathEnvKey() string {
	if runtime.GOOS == "windows" {
		return "Path"
	}
	return "PATH"
}

var scriptTempSequence atomic.Uint64

func nextScriptTempSequence() uint64 { return scriptTempSequence.Add(1) - 1 }
