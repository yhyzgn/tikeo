package tikee

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"
)

// SandboxToolResolver resolves and optionally installs lightweight script sandbox tools.
type SandboxToolResolver struct {
	StateDir       string
	AutoInstall    bool
	InstallTimeout time.Duration
}

func NewSandboxToolResolver() SandboxToolResolver {
	return SandboxToolResolver{AutoInstall: true, InstallTimeout: 2 * time.Minute}
}

func (r SandboxToolResolver) ResolveSrt() (string, bool) {
	return r.resolveTool("srt", func(dir string) error {
		pkg := envOrDefault("TIKEE_SRT_NPM_PACKAGE", "@anthropic-ai/sandbox-runtime")
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

func (r SandboxToolResolver) ResolveInterpreter(binary string) (string, bool) {
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
	if path, err := exec.LookPath(binary); err == nil && toolWorks(binary, path) {
		return path, true
	}
	local := filepath.Join(r.installDir(toolKey), "bin", executableName(binary))
	if toolWorks(binary, local) {
		return local, true
	}
	if !r.AutoInstall {
		return local, false
	}
	if err := installer(r.installDir(toolKey)); err != nil {
		return local, false
	}
	return local, toolWorks(binary, local)
}

func (r SandboxToolResolver) installDir(binary string) string {
	base := strings.TrimSpace(r.StateDir)
	if base == "" {
		if home, err := os.UserHomeDir(); err == nil {
			base = filepath.Join(home, ".tikee")
		} else {
			base = ".tikee"
		}
	}
	return filepath.Join(base, "sandbox-tools", binary)
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
	file, err := os.CreateTemp("", "tikee-rhai-smoke-*.rhai")
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
	version := envOrDefault("TIKEE_POWERSHELL_VERSION", "7.5.4")
	archiveName := fmt.Sprintf("powershell-%s-%s.tar.gz", version, platform)
	url := envOrDefault("TIKEE_POWERSHELL_DOWNLOAD_URL", fmt.Sprintf("https://github.com/PowerShell/PowerShell/releases/download/v%s/%s", version, archiveName))
	archive := filepath.Join(installDir, archiveName)
	extractDir := filepath.Join(installDir, "powershell-"+version)
	if err := os.MkdirAll(binDir, 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(extractDir, 0o755); err != nil {
		return err
	}
	if err := runInstaller(timeout, binDir, "curl", "-fsSL", url, "-o", archive); err != nil {
		return err
	}
	defer os.Remove(archive)
	if err := runInstaller(timeout, binDir, "tar", "-xzf", archive, "-C", extractDir); err != nil {
		return err
	}
	pwsh := filepath.Join(extractDir, "pwsh")
	_ = os.Chmod(pwsh, 0o755)
	link := filepath.Join(binDir, executableName("pwsh"))
	_ = os.Remove(link)
	if err := os.Symlink(pwsh, link); err != nil {
		if copyErr := copyFile(pwsh, link); copyErr != nil {
			return err
		}
	}
	return nil
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
