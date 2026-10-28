//! End-to-end integration tests for `sawhorse` CLI commands.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "Cargo.toml",
            "-p",
            "sawhorse-cli",
            "--",
            "--help",
        ])
        .output()
        .expect("Failed to execute sawhorse --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Provider-agnostic 3-mode LLM runtime"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("ping"));
    assert!(stdout.contains("complete"));
    assert!(stdout.contains("bench"));
}

#[test]
fn test_cli_init_in_tempdir() {
    let temp_dir = std::env::temp_dir().join(format!("sawhorse_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "Cargo.toml",
            "-p",
            "sawhorse-cli",
            "--",
            "init",
            temp_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run sawhorse init");

    assert!(output.status.success());
    let config_file = temp_dir.join("sawhorse.toml");
    assert!(config_file.is_file());

    let content = std::fs::read_to_string(&config_file).unwrap();
    assert!(content.contains("[providers.local_server]"));
    assert!(content.contains("[providers.cloud_openai]"));
    assert!(content.contains("[roles.planner]"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
