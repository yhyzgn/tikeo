use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Resolves and optionally installs lightweight script sandbox tools.
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
        self.resolve_tool("srt", |dir, bin_dir| {
            run_installer(
                self.install_timeout,
                "npm",
                &[
                    "install",
                    "-g",
                    "--prefix",
                    &dir.to_string_lossy(),
                    "@anthropic-ai/sandbox-runtime",
                ],
                std::slice::from_ref(&bin_dir),
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
        self.resolve_tool("rg", |dir, bin_dir| {
            run_installer(
                self.install_timeout,
                "cargo",
                &["install", "--root", &dir.to_string_lossy(), "ripgrep"],
                std::slice::from_ref(&bin_dir),
            )
        })
    }

    /// Resolve Deno for JavaScript/TypeScript sandboxing.
    #[must_use]
    pub fn resolve_deno(&self) -> Option<PathBuf> {
        self.resolve_tool("deno", |dir, bin_dir| {
            let script = format!(
                "curl -fsSL https://deno.land/install.sh | DENO_INSTALL={} sh",
                shell_quote(&dir.to_string_lossy())
            );
            run_installer(
                self.install_timeout,
                "sh",
                &["-c", &script],
                std::slice::from_ref(&bin_dir),
            )
        })
    }

    /// Resolve Rhai CLI runner.
    #[must_use]
    pub fn resolve_rhai(&self) -> Option<PathBuf> {
        self.resolve_tool("rhai-run", |dir, bin_dir| {
            run_installer(
                self.install_timeout,
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
                std::slice::from_ref(&bin_dir),
            )
        })
    }

    /// Resolve PowerShell Core for SRT-backed PowerShell script execution.
    #[must_use]
    pub fn resolve_powershell(&self) -> Option<PathBuf> {
        self.resolve_tool_with_local_binary("pwsh", "pwsh", |dir, bin_dir| {
            install_powershell(self.install_timeout, dir, bin_dir)
        })
    }

    /// Resolve an already-installed native interpreter command used by SRT.
    #[must_use]
    pub fn resolve_interpreter(&self, binary: &str) -> Option<PathBuf> {
        find_command(binary).filter(|command| command_available(command))
    }

    fn resolve_tool<F>(&self, binary: &str, installer: F) -> Option<PathBuf>
    where
        F: FnOnce(PathBuf, PathBuf) -> bool,
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
        F: FnOnce(PathBuf, PathBuf) -> bool,
    {
        if let Some(command) = find_command(binary).filter(|command| command_available(command)) {
            return Some(command);
        }
        let install_dir = self.install_dir(tool_key);
        let bin_dir = install_dir.join("bin");
        let local = bin_dir.join(binary);
        if command_available(&local) {
            return Some(local);
        }
        if !self.auto_install || !installer(install_dir, bin_dir) {
            return None;
        }
        command_available(&local).then_some(local)
    }

    fn install_dir(&self, binary: &str) -> PathBuf {
        let base = self.state_dir.clone().unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map_or_else(|| PathBuf::from("."), PathBuf::from)
                .join(".tikee")
        });
        base.join("sandbox-tools").join(binary)
    }
}

fn install_powershell(timeout: Duration, install_dir: PathBuf, bin_dir: PathBuf) -> bool {
    if cfg!(windows) {
        return run_installer(
            timeout,
            "winget",
            &["install", "-e", "--id", "Microsoft.PowerShell"],
            std::slice::from_ref(&bin_dir),
        );
    }

    let version = std::env::var("TIKEE_POWERSHELL_VERSION").unwrap_or_else(|_| "7.5.4".to_owned());
    let platform = match powershell_archive_platform() {
        Some(value) => value,
        None => return false,
    };
    let archive_name = format!("powershell-{version}-{platform}.tar.gz");
    let url = std::env::var("TIKEE_POWERSHELL_DOWNLOAD_URL").unwrap_or_else(|_| {
        format!(
            "https://github.com/PowerShell/PowerShell/releases/download/v{version}/{archive_name}"
        )
    });
    let archive = install_dir.join(&archive_name);
    let extract_dir = install_dir.join(format!("powershell-{version}"));
    if std::fs::create_dir_all(&bin_dir).is_err() || std::fs::create_dir_all(&extract_dir).is_err()
    {
        return false;
    }
    if !download_file(timeout, &url, &archive) {
        return false;
    }
    if !run_installer(
        timeout,
        "tar",
        &[
            "-xzf",
            &archive.to_string_lossy(),
            "-C",
            &extract_dir.to_string_lossy(),
        ],
        std::slice::from_ref(&bin_dir),
    ) {
        let _ = std::fs::remove_file(&archive);
        return false;
    }
    let pwsh = extract_dir.join("pwsh");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&pwsh) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = std::fs::set_permissions(&pwsh, permissions);
        }
    }
    let link = bin_dir.join("pwsh");
    let _ = std::fs::remove_file(&link);
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(&pwsh, &link).is_err() && std::fs::copy(&pwsh, &link).is_err()
        {
            let _ = std::fs::remove_file(&archive);
            return false;
        }
    }
    #[cfg(not(unix))]
    {
        if std::fs::copy(&pwsh, &link).is_err() {
            let _ = std::fs::remove_file(&archive);
            return false;
        }
    }
    let _ = std::fs::remove_file(archive);
    true
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
    run_installer(timeout, "curl", &["-fsSL", url, "-o", &output_arg], &[])
        || run_installer(timeout, "wget", &["-q", url, "-O", &output_arg], &[])
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
        "tikee-rhai-smoke-{}-{}.rhai",
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
    fn installer_path_prepends_managed_bin_directory_once() {
        let managed = PathBuf::from("/home/neo/.tikee/sandbox-tools/rhai-run/bin");
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
