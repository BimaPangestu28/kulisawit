#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

fn bin_path() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_kulisawit").into()
}

#[test]
fn help_lists_version_and_run_subcommands() {
    let out = Command::new(bin_path())
        .args(["--help"])
        .output()
        .expect("run");
    assert!(out.status.success(), "help exit: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        combined.contains("version"),
        "help missing 'version': {combined}"
    );
    assert!(combined.contains("run"), "help missing 'run': {combined}");
    assert!(
        combined.contains("serve"),
        "help missing 'serve': {combined}"
    );
}

#[test]
fn version_subcommand_prints_version() {
    let out = Command::new(bin_path())
        .args(["version"])
        .output()
        .expect("run");
    assert!(out.status.success(), "version exit: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("kulisawit "),
        "version stdout unexpected: {stdout}"
    );
}
