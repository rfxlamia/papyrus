use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn invalid_arguments_exit_with_code_2() {
    let mut cmd = Command::cargo_bin("papyrus").unwrap();
    cmd.arg("convert").assert().code(2);
}

#[test]
fn single_file_to_output_file() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("out.md");

    let mut cmd = Command::cargo_bin("papyrus").unwrap();
    cmd.args([
        "convert",
        fixture_path("simple.pdf").to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ])
    .assert()
    .success();

    let markdown = fs::read_to_string(out).unwrap();
    assert!(markdown.contains("Chapter 1"));
}

#[test]
fn stdout_mode_without_output_flag() {
    let mut cmd = Command::cargo_bin("papyrus").unwrap();
    cmd.args(["convert", fixture_path("simple.pdf").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Chapter 1"));
}

#[test]
fn batch_mode_writes_multiple_files() {
    let input = tempdir().unwrap();
    let output = tempdir().unwrap();

    fs::copy(fixture_path("simple.pdf"), input.path().join("simple.pdf")).unwrap();
    fs::copy(
        fixture_path("multi-page.pdf"),
        input.path().join("multi-page.pdf"),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("papyrus").unwrap();
    cmd.args([
        "convert",
        input.path().to_str().unwrap(),
        "-o",
        output.path().to_str().unwrap(),
    ])
    .assert()
    .success();

    assert!(output.path().join("simple.md").exists());
    assert!(output.path().join("multi-page.md").exists());
}

#[test]
fn invalid_input_returns_exit_code_1() {
    let mut cmd = Command::cargo_bin("papyrus").unwrap();
    cmd.args(["convert", "tests/fixtures/does-not-exist.pdf"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("error:"));
}

#[test]
fn custom_flags_change_output() {
    let mut cmd1 = Command::cargo_bin("papyrus").unwrap();
    let default = cmd1
        .args(["convert", fixture_path("bold-italic.pdf").to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let mut cmd2 = Command::cargo_bin("papyrus").unwrap();
    let no_bold = cmd2
        .args([
            "convert",
            fixture_path("bold-italic.pdf").to_str().unwrap(),
            "--no-bold",
            "--heading-ratio",
            "2.0",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_ne!(default, no_bold);
}

#[test]
fn pipe_mode_reads_stdin_and_writes_stdout() {
    let bytes = fs::read(fixture_path("simple.pdf")).unwrap();
    let mut cmd = Command::cargo_bin("papyrus").unwrap();
    cmd.args(["convert", "-"])
        .write_stdin(bytes)
        .assert()
        .success()
        .stdout(predicate::str::contains("Chapter 1"));
}

#[test]
fn warning_output_visible_and_quiet_suppresses() {
    let mut cmd1 = Command::cargo_bin("papyrus").unwrap();
    cmd1.args(["convert", fixture_path("corrupted.pdf").to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning:"));

    let mut cmd2 = Command::cargo_bin("papyrus").unwrap();
    cmd2.args([
        "convert",
        fixture_path("corrupted.pdf").to_str().unwrap(),
        "--quiet",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("Warning:").not());
}
