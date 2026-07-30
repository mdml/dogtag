//! Integration tests for the `dogtag` binary, run against the built
//! executable via `CARGO_BIN_EXE_dogtag` (no external test-harness crates).

use std::process::Command;

/// `dogtag version` prints `dogtag <workspace version>` and exits 0.
///
/// `CARGO_PKG_VERSION` here is this crate's version, which is inherited from
/// `[workspace.package]` — so this asserts the binary reports the workspace
/// version, sourced from the SDK's public API.
#[test]
fn version_subcommand_prints_workspace_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_dogtag"))
        .arg("version")
        .output()
        .expect("failed to run the dogtag binary");
    assert!(output.status.success(), "expected exit 0: {output:?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    assert_eq!(
        stdout.trim_end(),
        format!("dogtag {}", env!("CARGO_PKG_VERSION"))
    );
}

/// `dogtag --version` reports the same string as `dogtag version`.
#[test]
fn version_flag_matches_version_subcommand() {
    let flag = Command::new(env!("CARGO_BIN_EXE_dogtag"))
        .arg("--version")
        .output()
        .expect("failed to run the dogtag binary");
    assert!(flag.status.success(), "expected exit 0: {flag:?}");
    let stdout = String::from_utf8(flag.stdout).expect("stdout was not UTF-8");
    assert_eq!(
        stdout.trim_end(),
        format!("dogtag {}", env!("CARGO_PKG_VERSION"))
    );
}

/// Bare `dogtag` prints help and exits 2 (clap's default for a missing
/// required subcommand with `arg_required_else_help`).
#[test]
fn no_args_prints_help_and_exits_2() {
    let output = Command::new(env!("CARGO_BIN_EXE_dogtag"))
        .output()
        .expect("failed to run the dogtag binary");
    assert_eq!(output.status.code(), Some(2), "expected exit 2: {output:?}");
    let stderr = String::from_utf8(output.stderr).expect("stderr was not UTF-8");
    assert!(
        stderr.contains("Usage: dogtag"),
        "no usage in help: {stderr}"
    );
}
