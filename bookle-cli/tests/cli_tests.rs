//! Integration tests for the Bookle CLI

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Create a simple Markdown file for testing
fn create_test_markdown(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("Failed to write test file");
    path
}

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("convert"))
        .stdout(predicate::str::contains("info"))
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("batch"));
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("bookle"));
}

#[test]
fn test_convert_help() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["convert", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Convert an ebook"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn test_info_help() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["info", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Display information"))
        .stdout(predicate::str::contains("--json"));
}

#[test]
fn test_validate_help() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["validate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Validate an ebook"))
        .stdout(predicate::str::contains("--strict"));
}

#[test]
fn test_batch_help() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["batch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Batch convert"))
        .stdout(predicate::str::contains("--output-dir"))
        .stdout(predicate::str::contains("--jobs"));
}

#[test]
fn test_convert_missing_input() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["convert", "--output", "out.epub"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_convert_missing_output() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_test_markdown(&temp_dir, "test.md", "# Test\n\nContent");

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["convert", input.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--output"));
}

#[test]
fn test_convert_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let output = temp_dir.path().join("output.epub");

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args([
        "convert",
        "/nonexistent/file.epub",
        "--output",
        output.to_str().unwrap(),
    ])
    .assert()
    .failure();
}

#[test]
fn test_info_nonexistent_file() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["info", "/nonexistent/file.epub"])
        .assert()
        .failure();
}

#[test]
fn test_validate_nonexistent_file() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["validate", "/nonexistent/file.epub"])
        .assert()
        .failure();
}

#[test]
fn test_batch_missing_output_dir() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["batch", "/some/input/dir"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--output-dir"));
}

#[test]
fn test_batch_invalid_jobs() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args([
        "batch",
        "/some/input/dir",
        "--output-dir",
        "/some/output/dir",
        "--jobs",
        "0",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("at least 1"));
}

#[test]
fn test_batch_negative_jobs() {
    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args([
        "batch",
        "/some/input/dir",
        "--output-dir",
        "/some/output/dir",
        "--jobs",
        "-1",
    ])
    .assert()
    .failure();
}

#[test]
fn test_convert_unsupported_format() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_test_markdown(&temp_dir, "test.md", "# Test\n\nContent");
    let output = temp_dir.path().join("output.xyz");

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--format",
        "xyz",
    ])
    .assert()
    .failure();
}

#[test]
fn test_convert_markdown_to_epub() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_test_markdown(
        &temp_dir,
        "test.md",
        "# My Book\n\nThis is the introduction.\n\n## Chapter 1\n\nContent here.",
    );
    let output = temp_dir.path().join("output.epub");

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--format",
        "epub",
    ])
    .assert()
    .success();

    // Verify output file was created
    assert!(output.exists(), "Output file should exist");

    // Verify it's a valid ZIP file (EPUB is ZIP)
    let file = fs::File::open(&output).unwrap();
    let archive = zip::ZipArchive::new(file);
    assert!(archive.is_ok(), "Output should be a valid ZIP/EPUB file");
}

#[test]
fn test_convert_markdown_to_typst() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_test_markdown(
        &temp_dir,
        "test.md",
        "# My Book\n\nThis is the introduction.\n\n## Chapter 1\n\nContent here.",
    );
    let output = temp_dir.path().join("output.typ");

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--format",
        "pdf", // PDF format produces Typst source
    ])
    .assert()
    .success();

    // Verify output file was created
    assert!(output.exists(), "Output file should exist");

    // Verify it contains Typst content
    let content = fs::read_to_string(&output).unwrap();
    assert!(
        content.contains("#set") || content.contains("="),
        "Output should contain Typst markup"
    );
}

#[test]
fn test_info_markdown() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_test_markdown(
        &temp_dir,
        "test.md",
        "# My Amazing Book\n\nIntroduction.\n\n## Chapter 1\n\nContent.",
    );

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["info", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("My Amazing Book"));
}

#[test]
fn test_info_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_test_markdown(
        &temp_dir,
        "test.md",
        "# My Amazing Book\n\nIntroduction.\n\n## Chapter 1\n\nContent.",
    );

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    let output = cmd
        .args(["info", "--json", input.to_str().unwrap()])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Verify it's valid JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(json["title"].is_string(), "Should have title field");
}

#[test]
fn test_validate_markdown() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_test_markdown(&temp_dir, "test.md", "# Valid Book\n\nContent here.");

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["validate", input.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_batch_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");

    fs::create_dir_all(&input_dir).unwrap();

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args([
        "batch",
        input_dir.to_str().unwrap(),
        "--output-dir",
        output_dir.to_str().unwrap(),
    ])
    .assert()
    .success();
}

#[test]
fn test_batch_with_files() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");

    fs::create_dir_all(&input_dir).unwrap();

    // Create test files
    fs::write(
        input_dir.join("book1.md"),
        "# Book 1\n\nContent of book 1.",
    )
    .unwrap();
    fs::write(
        input_dir.join("book2.md"),
        "# Book 2\n\nContent of book 2.",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args([
        "batch",
        input_dir.to_str().unwrap(),
        "--output-dir",
        output_dir.to_str().unwrap(),
        "--format",
        "epub",
        "--jobs",
        "2",
    ])
    .assert()
    .success();

    // Verify output files were created
    assert!(output_dir.exists(), "Output directory should exist");
}

#[test]
fn test_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_test_markdown(&temp_dir, "test.md", "# Test\n\nContent");

    let mut cmd = Command::cargo_bin("bookle-cli").unwrap();
    cmd.args(["--verbose", "info", input.to_str().unwrap()])
        .assert()
        .success();
}
