//! Emits compile-time runtime metadata for /api/v1/system/info.

use std::env;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn command_output(args: &[&str]) -> Option<String> {
    let output = Command::new(args[0]).args(&args[1..]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn env_or_git(env_name: &str, git_args: &[&str]) -> String {
    env::var(env_name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .or_else(|| command_output(git_args))
        .unwrap_or_default()
}

fn main() {
    println!("cargo:rerun-if-env-changed=TIKEO_GIT_TAG");
    println!("cargo:rerun-if-env-changed=TIKEO_GIT_SHA");
    println!("cargo:rerun-if-env-changed=TIKEO_BUILD_TIME");
    println!("cargo:rerun-if-env-changed=TIKEO_GIT_DIRTY");
    println!("cargo:rerun-if-changed=../../.git/HEAD");

    let version = env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let git_tag = env_or_git(
        "TIKEO_GIT_TAG",
        &["git", "describe", "--tags", "--exact-match", "HEAD"],
    );
    if !git_tag.is_empty() {
        let expected = format!("v{version}");
        assert!(
            git_tag == expected,
            "release git tag/version mismatch: gitTag={git_tag} must equal v{{CARGO_PKG_VERSION}}={expected}"
        );
    }

    let git_sha = env_or_git("TIKEO_GIT_SHA", &["git", "rev-parse", "--short=12", "HEAD"]);
    let build_time = env::var("TIKEO_BUILD_TIME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
                |_| "0".to_owned(),
                |duration| duration.as_secs().to_string(),
            )
        });
    let dirty = env::var("TIKEO_GIT_DIRTY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            Command::new("git")
                .args(["status", "--porcelain"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map_or_else(
                    || "unknown".to_owned(),
                    |status| (!status.trim().is_empty()).to_string(),
                )
        });

    println!("cargo:rustc-env=TIKEO_GIT_TAG={git_tag}");
    println!("cargo:rustc-env=TIKEO_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=TIKEO_BUILD_TIME={build_time}");
    println!("cargo:rustc-env=TIKEO_GIT_DIRTY={dirty}");
}
