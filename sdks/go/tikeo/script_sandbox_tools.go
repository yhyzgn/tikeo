package tikeo

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"time"
)

var sandboxToolInstalls sync.Map

// SandboxToolResolver resolves lightweight script sandbox tools and prewarms missing tools in the background.
type SandboxToolResolver struct {
	StateDir               string
	AutoInstall            bool
	StrictSandboxIsolation bool
	// Deprecated: use StrictSandboxIsolation.
	RequireManagedTools bool
	InstallTimeout      time.Duration
}

func NewSandboxToolResolver() SandboxToolResolver {
	return SandboxToolResolver{AutoInstall: true, InstallTimeout: 2 * time.Minute}
}

func (r SandboxToolResolver) ResolveSrt() (string, bool) {
	return r.resolveTool("srt", func(dir string) error {
		pkg := envOrDefault("TIKEO_SRT_NPM_PACKAGE", "@anthropic-ai/sandbox-runtime")
		return runInstaller(r.InstallTimeout, managedBinDir(dir), "npm", "install", "-g", "--prefix", dir, pkg)
	})
}

func (r SandboxToolResolver) ResolveNode() (string, bool) {
	return r.ResolveInterpreter("node")
}

func (r SandboxToolResolver) ResolveNpm() (string, bool) {
	return r.ResolveInterpreter("npm")
}

func (r SandboxToolResolver) ResolveRipgrep() (string, bool) {
	return r.resolveTool("rg", func(dir string) error {
		return runInstaller(r.InstallTimeout, managedBinDir(dir), "cargo", "install", "--root", dir, "ripgrep")
	})
}

func (r SandboxToolResolver) ResolveDeno() (string, bool) {
	return r.resolveTool("deno", func(dir string) error {
		if runtime.GOOS == "windows" {
			return runInstaller(r.InstallTimeout, managedBinDir(dir), "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "irm https://deno.land/install.ps1 | iex")
		}
		cmd := fmt.Sprintf("curl -fsSL https://deno.land/install.sh | DENO_INSTALL=%s sh", shellQuote(dir))
		return runInstaller(r.InstallTimeout, managedBinDir(dir), "sh", "-c", cmd)
	})
}

func (r SandboxToolResolver) ResolveRhai() (string, bool) {
	return r.resolveTool("rhai-run", func(dir string) error {
		return runInstaller(r.InstallTimeout, managedBinDir(dir), "cargo", "install", "--root", dir, "rhai", "--bins", "--features", "bin-features")
	})
}

func (r SandboxToolResolver) ResolvePowerShell() (string, bool) {
	return r.resolveToolWithLocalBinary("pwsh", "pwsh", func(dir string) error {
		return installPowerShell(r.InstallTimeout, dir)
	})
}

func (r SandboxToolResolver) strictSandboxIsolation() bool {
	return r.StrictSandboxIsolation || r.RequireManagedTools
}

func (r SandboxToolResolver) ResolveInterpreter(binary string) (string, bool) {
	if r.strictSandboxIsolation() {
		path := filepath.Join(r.installDir(binary), "bin", executableName(binary))
		if commandWorks(path, "--version") || (binary == "sh" && commandWorks(path, "-c", "exit 0")) {
			return path, true
		}
		return "", false
	}
	path, err := exec.LookPath(binary)
	if err != nil || !commandWorks(path, "--version") {
		return "", false
	}
	return path, true
}

func (r SandboxToolResolver) resolveTool(binary string, installer func(string) error) (string, bool) {
	return r.resolveToolWithLocalBinary(binary, binary, installer)
}

func (r SandboxToolResolver) resolveToolWithLocalBinary(toolKey, binary string, installer func(string) error) (string, bool) {
	if !r.strictSandboxIsolation() {
		if path, err := exec.LookPath(binary); err == nil && toolWorks(binary, path) {
			return path, true
		}
	}
	if legacy := r.legacyInstallDir(toolKey); legacy != "" {
		legacyLocal := filepath.Join(legacy, "bin", executableName(binary))
		if toolWorks(binary, legacyLocal) {
			return legacyLocal, true
		}
	}
	installDir := r.installDir(toolKey)
	local := filepath.Join(installDir, "bin", executableName(binary))
	if toolWorks(binary, local) {
		return local, true
	}
	if !r.AutoInstall {
		return local, false
	}
	r.scheduleBackgroundInstall(toolKey, installDir, binary, installer)
	return local, false
}

func (r SandboxToolResolver) scheduleBackgroundInstall(toolKey, installDir, binary string, installer func(string) error) {
	key := toolKey + "@" + installDir
	if _, loaded := sandboxToolInstalls.LoadOrStore(key, struct{}{}); loaded {
		return
	}
	go func() {
		if err := installer(installDir); err != nil {
			_, _ = fmt.Fprintf(os.Stderr, "[tikeo.sandbox] background auto-install failed tool=%s error=%v\n", binary, err)
			return
		}
		local := filepath.Join(installDir, "bin", executableName(binary))
		if !toolWorks(binary, local) {
			_, _ = fmt.Fprintf(os.Stderr, "[tikeo.sandbox] background auto-install completed but tool is still unavailable tool=%s path=%s\n", binary, local)
		}
	}()
}

func (r SandboxToolResolver) installDir(binary string) string {
	return filepath.Join(hostSandboxToolsRoot(), binary)
}

func (r SandboxToolResolver) legacyInstallDir(binary string) string {
	base := strings.TrimSpace(r.StateDir)
	if base == "" {
		return ""
	}
	return filepath.Join(base, "sandbox-tools", binary)
}

func hostSandboxToolsRoot() string {
	if configured := strings.TrimSpace(os.Getenv("TIKEO_SANDBOX_TOOLS_DIR")); configured != "" {
		return configured
	}
	if home, err := os.UserHomeDir(); err == nil {
		return filepath.Join(home, ".tikeo", "sandbox-tools")
	}
	return filepath.Join(".tikeo", "sandbox-tools")
}

func commandWorks(command string, args ...string) bool {
	ctxTimeout := 2 * time.Second
	if _, err := os.Stat(command); err != nil && strings.Contains(command, string(os.PathSeparator)) {
		return false
	}
	cmd := exec.Command(command, args...)
	if ctxTimeout > 0 {
		// Keep the probe bounded without needing context imports in this small helper.
		timer := time.AfterFunc(ctxTimeout, func() {
			if cmd.Process != nil {
				_ = cmd.Process.Kill()
			}
		})
		defer timer.Stop()
	}
	return cmd.Run() == nil
}

func toolWorks(binary string, command string) bool {
	switch binary {
	case "srt":
		return commandWorks(command, "--version") || commandWorks(command, "--help")
	case "rhai-run":
		return rhaiWorks(command)
	default:
		return commandWorks(command, "--version")
	}
}

func rhaiWorks(command string) bool {
	file, err := os.CreateTemp("", "tikeo-rhai-smoke-*.rhai")
	if err != nil {
		return false
	}
	name := file.Name()
	_, writeErr := file.WriteString("print(\"ok\");")
	closeErr := file.Close()
	defer os.Remove(name)
	if writeErr != nil || closeErr != nil {
		return false
	}
	return commandWorks(command, name)
}

func runInstaller(timeout time.Duration, managedBin string, command string, args ...string) error {
	cmd := exec.Command(command, args...)
	cmd.Env = pathWithManagedBin(os.Environ(), managedBin)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		return err
	}
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	select {
	case err := <-done:
		return err
	case <-time.After(timeout):
		_ = cmd.Process.Kill()
		return fmt.Errorf("installer timed out: %s", command)
	}
}

func installPowerShell(timeout time.Duration, installDir string) error {
	binDir := managedBinDir(installDir)
	if runtime.GOOS == "windows" {
		return runInstaller(timeout, binDir, "winget", "install", "-e", "--id", "Microsoft.PowerShell")
	}
	platform := powershellArchivePlatform()
	if platform == "" {
		return fmt.Errorf("PowerShell auto-install does not support %s/%s", runtime.GOOS, runtime.GOARCH)
	}
	version := envOrDefault("TIKEO_POWERSHELL_VERSION", "7.5.4")
	archiveName := fmt.Sprintf("powershell-%s-%s.tar.gz", version, platform)
	url := envOrDefault("TIKEO_POWERSHELL_DOWNLOAD_URL", fmt.Sprintf("https://github.com/PowerShell/PowerShell/releases/download/v%s/%s", version, archiveName))
	archive := filepath.Join(installDir, archiveName)
	partialArchive := filepath.Join(installDir, archiveName+".part")
	if err := os.MkdirAll(installDir, 0o755); err != nil {
		return err
	}
	releaseLock, err := acquireInstallLock(installDir)
	if err != nil {
		return err
	}
	defer releaseLock()
	link := filepath.Join(binDir, executableName("pwsh"))
	if toolWorks("pwsh", link) {
		return nil
	}
	tmpDir, err := os.MkdirTemp(installDir, ".pwsh-install-")
	if err != nil {
		return err
	}
	defer os.RemoveAll(tmpDir)
	tmpArchive := filepath.Join(tmpDir, archiveName)
	tmpExtractDir := filepath.Join(tmpDir, "extract")
	finalExtractDir := filepath.Join(installDir, "powershell-"+version)
	if err := os.MkdirAll(binDir, 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(tmpExtractDir, 0o755); err != nil {
		return err
	}
	if _, err := os.Stat(archive); err == nil {
		if err := copyFile(archive, tmpArchive); err != nil {
			return err
		}
	} else {
		if err := runInstaller(powerShellInstallTimeout(timeout), binDir, "curl", "-fL", "-C", "-", url, "-o", partialArchive); err != nil {
			return err
		}
		if err := copyFile(partialArchive, tmpArchive); err != nil {
			return err
		}
	}
	if err := runInstaller(powerShellInstallTimeout(timeout), binDir, "tar", "-xzf", tmpArchive, "-C", tmpExtractDir); err != nil {
		return err
	}
	pwsh := filepath.Join(tmpExtractDir, "pwsh")
	if _, err := os.Stat(pwsh); err != nil {
		return err
	}
	_ = os.Chmod(pwsh, 0o755)
	_ = os.RemoveAll(finalExtractDir)
	if err := os.Rename(tmpExtractDir, finalExtractDir); err != nil {
		return err
	}
	installedPwsh := filepath.Join(finalExtractDir, "pwsh")
	_ = os.Remove(link)
	_ = os.Remove(partialArchive)
	if err := os.Symlink(installedPwsh, link); err != nil {
		if copyErr := copyFile(installedPwsh, link); copyErr != nil {
			return err
		}
	}
	return nil
}

func acquireInstallLock(installDir string) (func(), error) {
	lockDir := filepath.Join(installDir, ".install.lock")
	deadline := time.Now().Add(2 * time.Minute)
	for {
		err := os.Mkdir(lockDir, 0o755)
		if err == nil {
			return func() { _ = os.Remove(lockDir) }, nil
		}
		if !os.IsExist(err) {
			return nil, err
		}
		if info, statErr := os.Stat(lockDir); statErr == nil && !info.IsDir() {
			_ = os.Remove(lockDir)
			continue
		}
		if time.Now().After(deadline) {
			return nil, fmt.Errorf("timed out waiting for sandbox tool install lock: %s", lockDir)
		}
		time.Sleep(100 * time.Millisecond)
	}
}

func powerShellInstallTimeout(timeout time.Duration) time.Duration {
	if configured := strings.TrimSpace(os.Getenv("TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS")); configured != "" {
		if millis, err := strconv.ParseInt(configured, 10, 64); err == nil && millis > 0 {
			return time.Duration(millis) * time.Millisecond
		}
	}
	floor := 30 * time.Minute
	if timeout < floor {
		return floor
	}
	return timeout
}

func powershellArchivePlatform() string {
	switch runtime.GOOS + "/" + runtime.GOARCH {
	case "linux/amd64":
		return "linux-x64"
	case "linux/arm64":
		return "linux-arm64"
	case "darwin/amd64":
		return "osx-x64"
	case "darwin/arm64":
		return "osx-arm64"
	default:
		return ""
	}
}

func copyFile(from, to string) error {
	data, err := os.ReadFile(from)
	if err != nil {
		return err
	}
	return os.WriteFile(to, data, 0o755)
}

func managedBinDir(root string) string {
	return filepath.Join(root, "bin")
}

func pathWithManagedBin(env []string, managedBin string) []string {
	if strings.TrimSpace(managedBin) == "" {
		return env
	}
	pathKey := "PATH="
	if runtime.GOOS == "windows" {
		pathKey = "Path="
	}
	pathValue := managedBin
	updated := make([]string, 0, len(env)+1)
	found := false
	for _, entry := range env {
		if strings.HasPrefix(entry, pathKey) || (runtime.GOOS == "windows" && strings.HasPrefix(strings.ToUpper(entry), "PATH=")) {
			found = true
			current := strings.TrimPrefix(entry, entry[:strings.Index(entry, "=")+1])
			parts := append([]string{managedBin}, filepath.SplitList(current)...)
			pathValue = strings.Join(dedupePathList(parts), string(os.PathListSeparator))
			updated = append(updated, entry[:strings.Index(entry, "=")+1]+pathValue)
			continue
		}
		updated = append(updated, entry)
	}
	if !found {
		updated = append(updated, pathKey+pathValue)
	}
	return updated
}

func dedupePathList(parts []string) []string {
	seen := map[string]struct{}{}
	result := make([]string, 0, len(parts))
	for _, part := range parts {
		if strings.TrimSpace(part) == "" {
			continue
		}
		if _, exists := seen[part]; exists {
			continue
		}
		seen[part] = struct{}{}
		result = append(result, part)
	}
	return result
}

func executableName(binary string) string {
	if runtime.GOOS == "windows" {
		return binary + ".exe"
	}
	return binary
}

func envOrDefault(key, fallback string) string {
	if value := strings.TrimSpace(os.Getenv(key)); value != "" {
		return value
	}
	return fallback
}
