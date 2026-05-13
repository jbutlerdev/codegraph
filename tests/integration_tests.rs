//! Integration tests for CLI

use assert_cmd::Command;

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("--help")
       .assert()
       .success();
}

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("--version")
       .assert()
       .success();
}

#[test]
fn test_config_ls_empty() {
    // Just test that config ls doesn't crash
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("config")
       .arg("ls")
       .assert()
       .success();
}

#[test]
fn test_search_without_args() {
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("search")
       .assert()
       .failure();
}

#[test]
fn test_ls_command() {
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("ls")
       .assert()
       .success();
}

#[test]
fn test_stats_command() {
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("stats")
       .assert()
       .success();
}

#[test]
fn test_index_without_url() {
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("index")
       .assert()
       .failure();
}

#[test]
fn test_ingest_without_path() {
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("ingest")
       .assert()
       .failure();
}

#[test]
fn test_delete_without_id() {
    let mut cmd = Command::cargo_bin("codegraph").unwrap();
    cmd.arg("delete")
       .assert()
       .failure();
}
