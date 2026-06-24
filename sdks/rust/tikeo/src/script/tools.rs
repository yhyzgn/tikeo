use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

static BACKGROUND_INSTALLS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Resolves lightweight script sandbox tools and prewarms missing tools in the background.
#[derive(Debug, Clone)]
pub struct SandboxToolResolver {
    /// Optional state directory used for managed tool installs.
    pub state_dir: Option<PathBuf>,
    /// Automatically install missing tools when installer prerequisites exist.
    pub auto_install: bool,
    /// Installer timeout.
    pub install_timeout: Duration,
}

impl Default for SandboxToolResolver {
    fn default() -> Self {
        Self {
            state_dir: None,
            auto_install: true,
            install_timeout: Duration::from_mins(2),
        }
    }
}

impl SandboxToolResolver {
    /// Resolve Anthropic Sandbox Runtime.
    #[must_use]
    pub fn resolve_srt(&self) -> Option<PathBuf> {
        let timeout = self.install_timeout;
        self.resolve_tool("srt", move |dir, bin_dir| {
            run_installer(
                timeout,
                "npm",
                &[
                    "install",
                    "-g",
                    "--prefix",
                    &dir.to_string_lossy(),
                    "@anthropic-ai/sandbox-runtime",
                ],
                &[bin_dir.to_path_buf()],
            )
        })
    }

    /// Resolve Node.js required by npm-installed SRT launchers.
    #[must_use]
    pub fn resolve_node(&self) -> Option<PathBuf> {
        find_command("node").filter(|command| command_available(command))
    }

    /// Resolve npm used to install SRT. Its parent directory is also useful for PATH repair.
    #[must_use]
    pub fn resolve_npm(&self) -> Option<PathBuf> {
        find_command("npm").filter(|command| command_available(command))
    }

    /// Resolve ripgrep required by SRT.
    #[must_use]
    pub fn resolve_ripgrep(&self) -> Option<PathBuf> {
        let timeout = self.install_timeout;
        self.resolve_tool("rg", move |dir, bin_dir| {
            run_installer(
                timeout,
                "cargo",
                &["install", "--root", &dir.to_string_lossy(), "ripgrep"],
                &[bin_dir.to_path_buf()],
            )
        })
    }

    /// Resolve Deno for JavaScript/TypeScript sandboxing.
    #[must_use]
    pub fn resolve_deno(&self) -> Option<PathBuf> {
        let timeout = self.install_timeout;
        self.resolve_tool("deno", move |dir, bin_dir| {
            let script = format!(
                "curl -fsSL https://deno.land/install.sh | DENO_INSTALL={} sh",
                shell_quote(&dir.to_string_lossy())
            );
            run_installer(timeout, "sh", &["-c", &script], &[bin_dir.to_path_buf()])
        })
    }

    /// Resolve Rhai CLI runner.
    #[must_use]
    pub fn resolve_rhai(&self) -> Option<PathBuf> {
        let timeout = self.install_timeout;
        self.resolve_tool("rhai-run", move |dir, bin_dir| {
            run_installer(
                timeout,
                "cargo",
                &[
                    "install",
                    "--root",
                    &dir.to_string_lossy(),
                    "rhai",
                    "--bins",
                    "--features",
                    "bin-features",
                ],
                &[bin_dir.to_path_buf()],
            )
        })
    }

    /// Resolve PowerShell Core for SRT-backed PowerShell script execution.
    #[must_use]
    pub fn resolve_powershell(&self) -> Option<PathBuf> {
        let timeout = self.install_timeout;
        self.resolve_tool_with_local_binary("pwsh", "pwsh", move |dir, bin_dir| {
            install_powershell(timeout, dir, bin_dir)
        })
    }

    /// Resolve an already-installed native interpreter command used by SRT.
    #[must_use]
    pub fn resolve_interpreter(&self, binary: &str) -> Option<PathBuf> {
        find_command(binary).filter(|command| command_available(command))
    }

    fn resolve_tool<F>(&self, binary: &str, installer: F) -> Option<PathBuf>
    where
        F: FnOnce(&Path, &Path) -> bool + Send + 'static,
    {
        self.resolve_tool_with_local_binary(binary, binary, installer)
    }

    fn resolve_tool_with_local_binary<F>(
        &self,
        tool_key: &str,
        binary: &str,
        installer: F,
    ) -> Option<PathBuf>
    where
        F: FnOnce(&Path, &Path) -> bool + Send + 'static,
    {
        if let Some(command) = find_command(binary).filter(|command| command_available(command)) {
            return Some(command);
        }
        if let Some(legacy_dir) = self.legacy_install_dir(tool_key) {
            let legacy_local = legacy_dir.join("bin").join(binary);
            if command_available(&legacy_local) {
                return Some(legacy_local);
            }
        }
        let install_dir = Self::install_dir(tool_key);
        let bin_dir = install_dir.join("bin");
        let local = bin_dir.join(binary);
        if command_available(&local) {
            return Some(local);
        }
        if !self.auto_install {
            return None;
        }
        self.schedule_background_install(tool_key, binary, install_dir, bin_dir, installer);
        None
    }

    fn schedule_background_install<F>(
        &self,
        tool_key: &str,
        binary: &str,
        install_dir: PathBuf,
        bin_dir: PathBuf,
        installer: F,
    ) where
        F: FnOnce(&Path, &Path) -> bool + Send + 'static,
    {
        let key = format!("{}@{}", tool_key, install_dir.display());
        if let Ok(mut installs) = BACKGROUND_INSTALLS.lock() {
            if !installs.insert(key) {
                return;
            }
        }
        let binary = binary.to_owned();
        let _ = std::thread::Builder::new()
            .name(format!("tikeo-sandbox-install-{binary}"))
            .spawn(move || {
                if !installer(&install_dir, &bin_dir) {
                    eprintln!("[tikeo.sandbox] background auto-install failed tool={binary}");
                    return;
                }
                let local = bin_dir.join(&binary);
                if !command_available(&local) {
                    eprintln!(
                        "[tikeo.sandbox] background auto-install completed but tool is still unavailable tool={binary} path={}",
                        local.display()
                    );
                }
            });
    }

    fn install_dir(binary: &str) -> PathBuf {
        host_sandbox_tools_root().join(binary)
    }

    fn legacy_install_dir(&self, binary: &str) -> Option<PathBuf> {
        self.state_dir
            .as_ref()
            .map(|base| base.join("sandbox-tools").join(binary))
    }
}

fn host_sandbox_tools_root() -> PathBuf {
    std::env::var_os("TIKEO_SANDBOX_TOOLS_DIR")
        .filter(|value| !value.is_empty())
        .map_or_else(
            || {
                std::env::var_os("HOME")
                    .map_or_else(|| PathBuf::from("."), PathBuf::from)
                    .join(".tikeo")
                    .join("sandbox-tools")
            },
            PathBuf::from,
        )
}

fn install_powershell(timeout: Duration, install_dir: &Path, bin_dir: &Path) -> bool {
    if cfg!(windows) {
        return run_installer(
            timeout,
            "winget",
            &["install", "-e", "--id", "Microsoft.PowerShell"],
            &[bin_dir.to_path_buf()],
        );
    }

    let version = std::env::var("TIKEO_POWERSHELL_VERSION").unwrap_or_else(|_| "7.5.4".to_owned());
    let Some(platform) = powershell_archive_platform() else {
        return false;
    };
    let archive_name = format!("powershell-{version}-{platform}.tar.gz");
    let url = std::env::var("TIKEO_POWERSHELL_DOWNLOAD_URL").unwrap_or_else(|_| {
        format!(
            "https://github.com/PowerShell/PowerShell/releases/download/v{version}/{archive_name}"
        )
    });
    if std::fs::create_dir_all(install_dir).is_err() || std::fs::create_dir_all(bin_dir).is_err() {
        return false;
    }
    let link = bin_dir.join("pwsh");
    if command_available(&link) {
        return true;
    }
    let archive = install_dir.join(&archive_name);
    let partial_archive = install_dir.join(format!("{archive_name}.part"));
    let Ok(tmp_root) = std::fs::create_dir_all(install_dir).and_then(|()| {
        std::process::Command::new("mktemp")
            .arg("-d")
            .arg(install_dir.join(".pwsh-install-XXXXXX"))
            .output()
            .map_err(std::io::Error::other)
            .and_then(|output| {
                if output.status.success() {
                    Ok(PathBuf::from(
                        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
                    ))
                } else {
                    Err(std::io::Error::other("mktemp failed"))
                }
            })
    }) else {
        return false;
    };
    let tmp_archive = tmp_root.join(&archive_name);
    let tmp_extract_dir = tmp_root.join("extract");
    let final_extract_dir = install_dir.join(format!("powershell-{version}"));
    let result = (|| {
        std::fs::create_dir_all(&tmp_extract_dir).ok()?;
        if archive.is_file() {
            std::fs::copy(&archive, &tmp_archive).ok()?;
        } else {
            download_file(power_shell_install_timeout(timeout), &url, &partial_archive)
                .then_some(())?;
            std::fs::copy(&partial_archive, &tmp_archive).ok()?;
        }
        run_installer(
            power_shell_install_timeout(timeout),
            "tar",
            &[
                "-xzf",
                &tmp_archive.to_string_lossy(),
                "-C",
                &tmp_extract_dir.to_string_lossy(),
            ],
            &[bin_dir.to_path_buf()],
        )
        .then_some(())?;
        let pwsh = tmp_extract_dir.join("pwsh");
        if !pwsh.is_file() {
            return None;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&pwsh) {
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o755);
                let _ = std::fs::set_permissions(&pwsh, permissions);
            }
        }
        let _ = std::fs::remove_dir_all(&final_extract_dir);
        std::fs::rename(&tmp_extract_dir, &final_extract_dir).ok()?;
        let installed_pwsh = final_extract_dir.join("pwsh");
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_file(&partial_archive);
        #[cfg(unix)]
        {
            if std::os::unix::fs::symlink(&installed_pwsh, &link).is_err()
                && std::fs::copy(&installed_pwsh, &link).is_err()
            {
                return None;
            }
        }
        #[cfg(not(unix))]
        {
            if std::fs::copy(&installed_pwsh, &link).is_err() {
                return None;
            }
        }
        Some(())
    })()
    .is_some();
    let _ = std::fs::remove_dir_all(tmp_root);
    result
}

fn power_shell_install_timeout(timeout: Duration) -> Duration {
    if let Ok(configured) = std::env::var("TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS")
        && let Ok(millis) = configured.trim().parse::<u64>()
    {
        return Duration::from_millis(millis.max(1_000));
    }
    timeout.max(Duration::from_mins(30))
}

fn powershell_archive_platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x64"),
        ("linux", "aarch64") => Some("linux-arm64"),
        ("macos", "x86_64") => Some("osx-x64"),
        ("macos", "aarch64") => Some("osx-arm64"),
        _ => None,
    }
}

fn download_file(timeout: Duration, url: &str, output: &Path) -> bool {
    let Some(parent) = output.parent() else {
        return false;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let output_arg = output.to_string_lossy();
    run_installer(
        timeout,
        "curl",
        &["-fL", "-C", "-", url, "-o", &output_arg],
        &[],
    ) || run_installer(timeout, "wget", &["-q", url, "-O", &output_arg], &[])
}

fn find_command(binary: &str) -> Option<PathBuf> {
    let candidate = PathBuf::from(binary);
    if candidate.components().count() > 1 {
        return candidate.exists().then_some(candidate);
    }
    let path = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path) {
        let candidate = entry.join(binary);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn command_available(command: impl Into<PathBuf>) -> bool {
    let command = command.into();
    if command
        .file_name()
        .is_some_and(|name| name.to_string_lossy() == "rhai-run")
    {
        return rhai_available(&command);
    }
    let Ok(mut child) = Command::new(&command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                let _ = child.kill();
                return false;
            }
            Err(_) => return false,
        }
    }
}

fn rhai_available(command: &PathBuf) -> bool {
    let script = std::env::temp_dir().join(format!(
        "tikeo-rhai-smoke-{}-{}.rhai",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));
    if std::fs::write(&script, "print(\"ok\");").is_err() {
        return false;
    }
    let result = run_bounded_command(command, &[&script.to_string_lossy()]);
    let _ = std::fs::remove_file(script);
    result
}

fn run_bounded_command(command: &PathBuf, args: &[&str]) -> bool {
    let Ok(mut child) = Command::new(command)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                let _ = child.kill();
                return false;
            }
            Err(_) => return false,
        }
    }
}

fn run_installer(
    timeout: Duration,
    command: &str,
    args: &[&str],
    path_entries: &[PathBuf],
) -> bool {
    let mut installer = Command::new(command);
    installer.args(args);
    if let Some(path) = path_with_entries(path_entries, std::env::var_os("PATH")) {
        installer.env("PATH", path);
    }
    let Ok(mut child) = installer.spawn() else {
        return false;
    };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                return false;
            }
            Err(_) => return false,
        }
    }
}

fn path_with_entries(entries: &[PathBuf], inherited_path: Option<OsString>) -> Option<OsString> {
    let mut path_entries = entries
        .iter()
        .filter(|entry| !entry.as_os_str().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if let Some(path) = inherited_path {
        path_entries.extend(std::env::split_paths(&path));
    }
    let mut unique = Vec::new();
    for path in path_entries {
        if !unique.iter().any(|entry: &PathBuf| entry == &path) {
            unique.push(path);
        }
    }
    (!unique.is_empty())
        .then(|| std::env::join_paths(unique).ok())
        .flatten()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_install_resolves_missing_tool_without_blocking_on_installer() {
        let resolver = SandboxToolResolver {
            state_dir: None,
            auto_install: true,
            install_timeout: Duration::from_millis(1),
        };
        let started = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let started_in_thread = std::sync::Arc::clone(&started);
        let tool_key = format!("tikeo-test-missing-{}", std::process::id());

        let started_at = Instant::now();
        let resolved = resolver.resolve_tool_with_local_binary(
            &tool_key,
            "definitely-missing-tikeo-sandbox-tool",
            move |_dir, _bin_dir| {
                started_in_thread.store(true, std::sync::atomic::Ordering::SeqCst);
                std::thread::sleep(Duration::from_secs(2));
                false
            },
        );

        assert!(resolved.is_none());
        assert!(started_at.elapsed() < Duration::from_secs(1));
        for _ in 0..20 {
            if started.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("background installer should have been scheduled");
    }

    #[test]
    fn install_dir_prefers_host_cache_root() {
        assert_eq!(
            SandboxToolResolver::install_dir("pwsh"),
            host_sandbox_tools_root().join("pwsh")
        );
    }

    #[test]
    fn installer_path_prepends_managed_bin_directory_once() {
        let managed = PathBuf::from("/home/neo/.tikeo/sandbox-tools/rhai-run/bin");
        let inherited = match std::env::join_paths([
            PathBuf::from("/usr/local/bin"),
            managed.clone(),
            PathBuf::from("/usr/bin"),
        ]) {
            Ok(path) => path,
            Err(error) => panic!("test PATH should be joinable: {error}"),
        };

        let Some(path) = path_with_entries(std::slice::from_ref(&managed), Some(inherited)) else {
            panic!("PATH should be generated");
        };
        let entries = std::env::split_paths(&path).collect::<Vec<_>>();

        assert_eq!(entries.first(), Some(&managed));
        assert_eq!(entries.iter().filter(|entry| *entry == &managed).count(), 1);
    }
}
