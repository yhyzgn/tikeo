use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use tokio::process::Command;

use crate::error::WorkerSdkError;

/// Per-task script sandbox runtime filesystem and environment contract.
///
/// This is the single source of truth for script runner HOME/TMPDIR/XDG runtime
/// paths. All sandbox-backed runners must use this instead of scattering env
/// names or ad-hoc temp directories in each runner implementation.
#[derive(Debug, Clone)]
pub(super) struct TaskRuntimeDirs {
    root: PathBuf,
    home: PathBuf,
    config: PathBuf,
    cache: PathBuf,
    data: PathBuf,
    modules: PathBuf,
    dotnet_home: PathBuf,
    tmp: PathBuf,
    deno_dir: PathBuf,
}

impl TaskRuntimeDirs {
    /// Create.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub(super) fn create(prefix: &str) -> Result<Self, WorkerSdkError> {
        let root = system_temp_dir(prefix);
        let data = root.join("data");
        let cache = root.join("cache");
        let runtime_dirs = Self {
            home: root.join("home"),
            config: root.join("config"),
            modules: data.join("powershell").join("Modules"),
            dotnet_home: root.join("dotnet"),
            deno_dir: cache.join("deno"),
            tmp: root.join("tmp"),
            cache,
            data,
            root,
        };
        runtime_dirs.create_directories()?;
        Ok(runtime_dirs)
    }

    fn create_directories(&self) -> Result<(), WorkerSdkError> {
        for dir in self.required_directories() {
            std::fs::create_dir_all(dir)
                .map_err(|error| WorkerSdkError::ScriptExecutionFailed(error.to_string()))?;
        }
        Ok(())
    }

    fn required_directories(&self) -> [&Path; 9] {
        [
            &self.home,
            &self.config,
            &self.cache,
            &self.data,
            &self.modules,
            &self.dotnet_home,
            &self.tmp,
            &self.deno_dir,
            &self.root,
        ]
    }

    /// Allow write paths.
    pub(super) fn allow_write_paths(&self) -> Vec<String> {
        self.writable_directories()
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect()
    }

    fn writable_directories(&self) -> [&Path; 8] {
        [
            &self.root,
            &self.home,
            &self.config,
            &self.cache,
            &self.data,
            &self.dotnet_home,
            &self.tmp,
            &self.deno_dir,
        ]
    }

    /// Apply srt environment.
    pub(super) fn apply_srt_environment(&self, command: &mut Command) {
        self.apply_base_environment(command);
        for (name, value) in self.srt_environment_entries() {
            command.env(name, value);
        }
    }

    /// Apply powershell environment.
    pub(super) fn apply_powershell_environment(&self, command: &mut Command) {
        for (name, value) in self.powershell_environment_entries() {
            command.env(name, value);
        }
    }

    /// Apply deno environment.
    pub(super) fn apply_deno_environment(&self, command: &mut Command) {
        self.apply_base_environment(command);
        for (name, value) in self.deno_environment_entries() {
            command.env(name, value);
        }
    }

    fn apply_base_environment(&self, command: &mut Command) {
        for (name, value) in self.base_environment_entries() {
            command.env(name, value);
        }
    }

    fn base_environment_entries(&self) -> Vec<(&'static str, OsString)> {
        vec![
            ("HOME", self.home.clone().into_os_string()),
            ("XDG_CONFIG_HOME", self.config.clone().into_os_string()),
            ("XDG_CACHE_HOME", self.cache.clone().into_os_string()),
            ("XDG_DATA_HOME", self.data.clone().into_os_string()),
            ("TMPDIR", self.tmp.clone().into_os_string()),
            ("TERM", OsString::from("dumb")),
            ("NO_COLOR", OsString::from("1")),
        ]
    }

    fn srt_environment_entries(&self) -> Vec<(&'static str, OsString)> {
        vec![
            ("CLAUDE_CODE_TMPDIR", self.tmp.clone().into_os_string()),
            ("CLAUDE_TMPDIR", self.tmp.clone().into_os_string()),
        ]
    }

    fn powershell_environment_entries(&self) -> Vec<(&'static str, OsString)> {
        vec![
            ("PSModulePath", self.modules_path()),
            ("DOTNET_CLI_HOME", self.dotnet_home.clone().into_os_string()),
            ("POWERSHELL_TELEMETRY_OPTOUT", OsString::from("1")),
            ("POWERSHELL_UPDATECHECK", OsString::from("Off")),
        ]
    }

    fn deno_environment_entries(&self) -> Vec<(&'static str, OsString)> {
        vec![("DENO_DIR", self.deno_dir.clone().into_os_string())]
    }

    /// Working dir.
    pub(super) fn working_dir(&self) -> &Path {
        &self.home
    }

    /// Script file.
    pub(super) fn script_file(&self, extension: &str) -> PathBuf {
        self.home.join(format!(
            "script-{}-{}.{}",
            monotonic_millis(),
            unique_temp_sequence(),
            extension
        ))
    }

    fn modules_path(&self) -> OsString {
        std::env::join_paths([self.modules.clone()])
            .unwrap_or_else(|_| self.modules.clone().into_os_string())
    }

    /// Is managed environment name.
    pub(super) fn is_managed_environment_name(name: &str) -> bool {
        matches!(
            name,
            "HOME"
                | "XDG_CONFIG_HOME"
                | "XDG_CACHE_HOME"
                | "XDG_DATA_HOME"
                | "TMPDIR"
                | "TERM"
                | "NO_COLOR"
                | "CLAUDE_CODE_TMPDIR"
                | "CLAUDE_TMPDIR"
                | "PSModulePath"
                | "DOTNET_CLI_HOME"
                | "POWERSHELL_TELEMETRY_OPTOUT"
                | "POWERSHELL_UPDATECHECK"
                | "DENO_DIR"
        )
    }

    /// Cleanup.
    pub(super) fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }

    #[cfg(test)]
    /// Root.
    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    #[cfg(test)]
    /// Home.
    pub(super) fn home(&self) -> &Path {
        &self.home
    }

    #[cfg(test)]
    /// Tmp.
    pub(super) fn tmp(&self) -> &Path {
        &self.tmp
    }
}

/// System temp file.
pub(super) fn system_temp_file(prefix: &str, extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}.{}",
        monotonic_millis(),
        unique_temp_sequence(),
        extension
    ))
}

fn system_temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        monotonic_millis(),
        unique_temp_sequence()
    ))
}

fn unique_temp_sequence() -> u64 {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

fn monotonic_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}
