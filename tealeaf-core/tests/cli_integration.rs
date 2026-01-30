//! CLI integration tests for the TeaLeaf binary.
//!
//! Tests exercise the `tealeaf` CLI through `std::process::Command`,
//! covering command routing, argument validation, file I/O errors,
//! successful operations, round-trip chains, stdout/file output modes,
//! exit codes, stderr/stdout separation, and edge cases.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// =============================================================================
// Constants
// =============================================================================

const CANONICAL_SAMPLES: &[&str] = &[
    "primitives",
    "arrays",
    "objects",
    "schemas",
    "special_types",
    "timestamps",
    "numbers_extended",
    "unions",
    "multiline_strings",
    "unicode_escaping",
    "refs_tags_maps",
    "mixed_schemas",
    "large_data",
    "cyclic_refs",
];

// =============================================================================
// Helper Functions
// =============================================================================

fn tealeaf_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tealeaf"))
}

fn run(args: &[&str]) -> Output {
    Command::new(tealeaf_bin())
        .args(args)
        .output()
        .expect("Failed to execute tealeaf binary")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "Expected exit code 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1, got {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stdout_str(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_str(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("canonical")
        .join("samples")
}

fn expected_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("canonical")
        .join("expected")
}

fn binary_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("canonical")
        .join("binary")
}

fn errors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("canonical")
        .join("errors")
}

fn sample_tl(name: &str) -> PathBuf {
    samples_dir().join(format!("{}.tl", name))
}

fn sample_tlbx(name: &str) -> PathBuf {
    binary_dir().join(format!("{}.tlbx", name))
}

fn expected_json_path(name: &str) -> PathBuf {
    expected_dir().join(format!("{}.json", name))
}

fn load_json_file(path: &Path) -> serde_json::Value {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse JSON {:?}: {}", path, e))
}

fn parse_json_str(s: &str) -> serde_json::Value {
    serde_json::from_str(s)
        .unwrap_or_else(|e| panic!("Failed to parse JSON string: {}\nInput: {}", e, &s[..s.len().min(200)]))
}

fn path_str(p: &Path) -> &str {
    p.to_str().expect("Path is not valid UTF-8")
}

// =============================================================================
// 1. Command Routing
// =============================================================================

#[test]
fn no_args_prints_usage_and_exits_1() {
    let output = Command::new(tealeaf_bin())
        .output()
        .expect("Failed to execute");
    assert_failure(&output);
}

#[test]
fn help_prints_usage_and_exits_0() {
    let output = run(&["help"]);
    assert_success(&output);
    let out = stdout_str(&output);
    assert!(out.contains("Usage:"), "Expected usage text, got: {}", out);
}

#[test]
fn dash_h_prints_usage_and_exits_0() {
    let output = run(&["-h"]);
    assert_success(&output);
    assert!(stdout_str(&output).contains("Usage:"));
}

#[test]
fn dash_dash_help_prints_usage_and_exits_0() {
    let output = run(&["--help"]);
    assert_success(&output);
    assert!(stdout_str(&output).contains("Usage:"));
}

#[test]
fn unknown_command_prints_error_and_exits_1() {
    let output = run(&["frobnicate"]);
    assert_failure(&output);
    let err = stderr_str(&output);
    assert!(err.contains("Unknown command"), "Expected 'Unknown command' on stderr, got: {}", err);
}

#[test]
fn each_valid_command_recognized_with_no_args() {
    // Each command with no further args should exit 1 (missing args), proving routing works
    for cmd in &["compile", "decompile", "info", "validate", "to-json", "from-json", "tlbx-to-json", "json-to-tlbx"] {
        let output = run(&[cmd]);
        assert_failure(&output);
        // Should NOT say "Unknown command"
        let err = stderr_str(&output);
        assert!(!err.contains("Unknown command"),
            "Command '{}' was not recognized: {}", cmd, err);
    }
}

// =============================================================================
// 2. Argument Validation
// =============================================================================

#[test]
fn compile_missing_output_flag() {
    let input = sample_tl("primitives");
    let output = run(&["compile", path_str(&input)]);
    assert_failure(&output);
}

#[test]
fn compile_missing_input() {
    let output = run(&["compile"]);
    assert_failure(&output);
}

#[test]
fn decompile_missing_output_flag() {
    let input = sample_tlbx("primitives");
    let output = run(&["decompile", path_str(&input)]);
    assert_failure(&output);
}

#[test]
fn decompile_missing_input() {
    let output = run(&["decompile"]);
    assert_failure(&output);
}

#[test]
fn from_json_missing_output_flag() {
    let input = expected_json_path("primitives");
    let output = run(&["from-json", path_str(&input)]);
    assert_failure(&output);
}

#[test]
fn from_json_missing_input() {
    let output = run(&["from-json"]);
    assert_failure(&output);
}

#[test]
fn json_to_tlbx_missing_output_flag() {
    let input = expected_json_path("primitives");
    let output = run(&["json-to-tlbx", path_str(&input)]);
    assert_failure(&output);
}

#[test]
fn json_to_tlbx_missing_input() {
    let output = run(&["json-to-tlbx"]);
    assert_failure(&output);
}

#[test]
fn info_missing_input() {
    let output = run(&["info"]);
    assert_failure(&output);
}

#[test]
fn validate_missing_input() {
    let output = run(&["validate"]);
    assert_failure(&output);
}

#[test]
fn to_json_missing_input() {
    let output = run(&["to-json"]);
    assert_failure(&output);
}

#[test]
fn tlbx_to_json_missing_input() {
    let output = run(&["tlbx-to-json"]);
    assert_failure(&output);
}

// =============================================================================
// 3. File I/O Errors
// =============================================================================

#[test]
fn compile_nonexistent_input() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tlbx");
    let output = run(&["compile", "nonexistent_file.tl", "-o", path_str(&out)]);
    assert_failure(&output);
}

#[test]
fn decompile_nonexistent_input() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tl");
    let output = run(&["decompile", "nonexistent_file.tlbx", "-o", path_str(&out)]);
    assert_failure(&output);
}

#[test]
fn info_nonexistent_input() {
    let output = run(&["info", "nonexistent_file.tl"]);
    assert_failure(&output);
}

#[test]
fn validate_nonexistent_input() {
    let output = run(&["validate", "nonexistent_file.tl"]);
    assert_failure(&output);
}

#[test]
fn to_json_nonexistent_input() {
    let output = run(&["to-json", "nonexistent_file.tl"]);
    assert_failure(&output);
}

#[test]
fn from_json_nonexistent_input() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tl");
    let output = run(&["from-json", "nonexistent_file.json", "-o", path_str(&out)]);
    assert_failure(&output);
}

#[test]
fn tlbx_to_json_nonexistent_input() {
    let output = run(&["tlbx-to-json", "nonexistent_file.tlbx"]);
    assert_failure(&output);
}

#[test]
fn json_to_tlbx_nonexistent_input() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tlbx");
    let output = run(&["json-to-tlbx", "nonexistent_file.json", "-o", path_str(&out)]);
    assert_failure(&output);
}

#[test]
fn compile_unwritable_output() {
    let input = sample_tl("primitives");
    // Use a path with a nonexistent parent directory
    let output = run(&["compile", path_str(&input), "-o", "nonexistent_dir/sub/out.tlbx"]);
    assert_failure(&output);
}

#[test]
fn decompile_unwritable_output() {
    let input = sample_tlbx("primitives");
    let output = run(&["decompile", path_str(&input), "-o", "nonexistent_dir/sub/out.tl"]);
    assert_failure(&output);
}

// =============================================================================
// 4. Successful Operations
// =============================================================================

#[test]
fn compile_primitives_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tlbx");
    let input = sample_tl("primitives");
    let output = run(&["compile", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    assert!(out.exists(), "Output file should exist");
    assert!(std::fs::metadata(&out).unwrap().len() > 0, "Output should be nonempty");
}

#[test]
fn compile_schemas_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tlbx");
    let input = sample_tl("schemas");
    let output = run(&["compile", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    assert!(out.exists());
}

#[test]
fn decompile_primitives_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tl");
    let input = sample_tlbx("primitives");
    let output = run(&["decompile", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    assert!(out.exists());
    let content = std::fs::read_to_string(&out).unwrap();
    assert!(!content.is_empty(), "Decompiled output should be nonempty");
}

#[test]
fn decompile_schemas_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tl");
    let input = sample_tlbx("schemas");
    let output = run(&["decompile", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    let content = std::fs::read_to_string(&out).unwrap();
    assert!(content.contains("@struct"), "Decompiled schemas should contain @struct directives");
}

#[test]
fn info_text_file_succeeds() {
    let input = sample_tl("primitives");
    let output = run(&["info", path_str(&input)]);
    assert_success(&output);
    let out = stdout_str(&output);
    assert!(out.contains("Format: Text"), "Expected 'Format: Text', got: {}", out);
}

#[test]
fn info_binary_file_succeeds() {
    let input = sample_tlbx("primitives");
    let output = run(&["info", path_str(&input)]);
    assert_success(&output);
    let out = stdout_str(&output);
    assert!(out.contains("Format: Binary"), "Expected 'Format: Binary', got: {}", out);
}

#[test]
fn validate_valid_file_succeeds() {
    let input = sample_tl("primitives");
    let output = run(&["validate", path_str(&input)]);
    assert_success(&output);
    let out = stdout_str(&output);
    assert!(out.contains("Valid"), "Expected 'Valid' in output, got: {}", out);
}

#[test]
fn validate_schemas_file_succeeds() {
    let input = sample_tl("schemas");
    let output = run(&["validate", path_str(&input)]);
    assert_success(&output);
}

#[test]
fn validate_invalid_file_fails() {
    let input = errors_dir().join("unterminated_string.tl");
    let output = run(&["validate", path_str(&input)]);
    assert_failure(&output);
}

#[test]
fn to_json_with_output_file_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.json");
    let input = sample_tl("primitives");
    let output = run(&["to-json", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    let actual = load_json_file(&out);
    let expected = load_json_file(&expected_json_path("primitives"));
    assert_eq!(actual, expected, "to-json output should match expected JSON");
}

#[test]
fn from_json_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tl");
    let input = expected_json_path("primitives");
    let output = run(&["from-json", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    assert!(out.exists());
    let content = std::fs::read_to_string(&out).unwrap();
    assert!(!content.is_empty(), "from-json output should be nonempty");
}

#[test]
fn tlbx_to_json_with_output_file_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.json");
    let input = sample_tlbx("primitives");
    let output = run(&["tlbx-to-json", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    let actual = load_json_file(&out);
    let expected = load_json_file(&expected_json_path("primitives"));
    assert_eq!(actual, expected, "tlbx-to-json output should match expected JSON");
}

#[test]
fn json_to_tlbx_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tlbx");
    let input = expected_json_path("primitives");
    let output = run(&["json-to-tlbx", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    assert!(out.exists());
    // Verify it starts with TLBX magic
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.len() >= 4, "Output too small");
    assert_eq!(&bytes[..4], b"TLBX", "Output should start with TLBX magic bytes");
}

#[test]
fn compile_all_canonical_samples() {
    let dir = tempfile::tempdir().unwrap();
    for name in CANONICAL_SAMPLES {
        let input = sample_tl(name);
        let out = dir.path().join(format!("{}.tlbx", name));
        let output = run(&["compile", path_str(&input), "-o", path_str(&out)]);
        assert_success(&output);
        assert!(out.exists(), "Failed to compile {}", name);
    }
}

#[test]
fn to_json_all_canonical_samples_match_expected() {
    let dir = tempfile::tempdir().unwrap();
    for name in CANONICAL_SAMPLES {
        let input = sample_tl(name);
        let out = dir.path().join(format!("{}.json", name));
        let output = run(&["to-json", path_str(&input), "-o", path_str(&out)]);
        assert_success(&output);
        let actual = load_json_file(&out);
        let expected = load_json_file(&expected_json_path(name));
        assert_eq!(actual, expected, "{}.tl -> JSON mismatch via CLI", name);
    }
}

#[test]
fn tlbx_to_json_all_canonical_samples_match_expected() {
    let dir = tempfile::tempdir().unwrap();
    for name in CANONICAL_SAMPLES {
        let input = sample_tlbx(name);
        let out = dir.path().join(format!("{}.json", name));
        let output = run(&["tlbx-to-json", path_str(&input), "-o", path_str(&out)]);
        assert_success(&output);
        let actual = load_json_file(&out);
        let expected = load_json_file(&expected_json_path(name));
        assert_eq!(actual, expected, "{}.tlbx -> JSON mismatch via CLI", name);
    }
}

// =============================================================================
// 5. Round-Trip Chains
// =============================================================================

#[test]
fn tl_to_json_to_tlbx_to_json_content_equal() {
    let dir = tempfile::tempdir().unwrap();
    let input = sample_tl("primitives");

    // Step 1: tl -> json (A)
    let json_a = dir.path().join("a.json");
    let output = run(&["to-json", path_str(&input), "-o", path_str(&json_a)]);
    assert_success(&output);

    // Step 2: tl -> tlbx -> json (B) -- binary path preserves content reliably
    let tlbx = dir.path().join("mid.tlbx");
    let output = run(&["compile", path_str(&input), "-o", path_str(&tlbx)]);
    assert_success(&output);
    let json_b = dir.path().join("b.json");
    let output = run(&["tlbx-to-json", path_str(&tlbx), "-o", path_str(&json_b)]);
    assert_success(&output);

    let val_a = load_json_file(&json_a);
    let val_b = load_json_file(&json_b);
    assert_eq!(val_a, val_b, "tl -> json should match tl -> tlbx -> json");
}

#[test]
fn json_to_tlbx_to_json_content_equal() {
    let dir = tempfile::tempdir().unwrap();
    let input_json = expected_json_path("primitives");
    let val_a = load_json_file(&input_json);

    // json -> tlbx
    let tlbx = dir.path().join("out.tlbx");
    let output = run(&["json-to-tlbx", path_str(&input_json), "-o", path_str(&tlbx)]);
    assert_success(&output);

    // tlbx -> json
    let json_b = dir.path().join("out.json");
    let output = run(&["tlbx-to-json", path_str(&tlbx), "-o", path_str(&json_b)]);
    assert_success(&output);

    let val_b = load_json_file(&json_b);
    assert_eq!(val_a, val_b, "JSON round-trip: json -> tlbx -> json should preserve content");
}

#[test]
fn tl_to_tlbx_to_json_matches_direct_to_json() {
    let dir = tempfile::tempdir().unwrap();
    let input = sample_tl("schemas");

    // Path A: tl -> json directly
    let json_a = dir.path().join("direct.json");
    let output = run(&["to-json", path_str(&input), "-o", path_str(&json_a)]);
    assert_success(&output);

    // Path B: tl -> tlbx -> json
    let tlbx = dir.path().join("mid.tlbx");
    let output = run(&["compile", path_str(&input), "-o", path_str(&tlbx)]);
    assert_success(&output);
    let json_b = dir.path().join("indirect.json");
    let output = run(&["tlbx-to-json", path_str(&tlbx), "-o", path_str(&json_b)]);
    assert_success(&output);

    let val_a = load_json_file(&json_a);
    let val_b = load_json_file(&json_b);
    assert_eq!(val_a, val_b, "tl -> json should match tl -> tlbx -> json");
}

#[test]
fn round_trip_all_canonical_compile_to_json_matches_expected() {
    // For every canonical sample: compile to tlbx, then tlbx-to-json, compare with expected
    let dir = tempfile::tempdir().unwrap();
    for name in CANONICAL_SAMPLES {
        let input = sample_tl(name);

        // tl -> tlbx
        let tlbx = dir.path().join(format!("{}.tlbx", name));
        let output = run(&["compile", path_str(&input), "-o", path_str(&tlbx)]);
        assert_success(&output);

        // tlbx -> json
        let json_out = dir.path().join(format!("{}.json", name));
        let output = run(&["tlbx-to-json", path_str(&tlbx), "-o", path_str(&json_out)]);
        assert_success(&output);

        // Compare with expected
        let actual = load_json_file(&json_out);
        let expected = load_json_file(&expected_json_path(name));
        assert_eq!(actual, expected, "{}: tl -> tlbx -> json should match expected", name);
    }
}

// =============================================================================
// 6. stdout vs File Output
// =============================================================================

#[test]
fn to_json_stdout_when_no_output_flag() {
    let input = sample_tl("primitives");
    let output = run(&["to-json", path_str(&input)]);
    assert_success(&output);
    let out = stdout_str(&output);
    // stdout should contain valid JSON
    let actual = parse_json_str(&out);
    let expected = load_json_file(&expected_json_path("primitives"));
    assert_eq!(actual, expected, "to-json stdout should match expected JSON");
}

#[test]
fn to_json_file_when_output_flag() {
    let dir = tempfile::tempdir().unwrap();
    let out_file = dir.path().join("out.json");
    let input = sample_tl("primitives");
    let output = run(&["to-json", path_str(&input), "-o", path_str(&out_file)]);
    assert_success(&output);
    // File should contain the JSON
    let actual = load_json_file(&out_file);
    let expected = load_json_file(&expected_json_path("primitives"));
    assert_eq!(actual, expected);
}

#[test]
fn to_json_stdout_matches_file_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = sample_tl("primitives");

    // Get stdout version
    let output_stdout = run(&["to-json", path_str(&input)]);
    assert_success(&output_stdout);
    let json_stdout = parse_json_str(&stdout_str(&output_stdout));

    // Get file version
    let out_file = dir.path().join("out.json");
    let output_file = run(&["to-json", path_str(&input), "-o", path_str(&out_file)]);
    assert_success(&output_file);
    let json_file = load_json_file(&out_file);

    assert_eq!(json_stdout, json_file, "stdout and file output should produce same JSON");
}

#[test]
fn tlbx_to_json_stdout_when_no_output_flag() {
    let input = sample_tlbx("primitives");
    let output = run(&["tlbx-to-json", path_str(&input)]);
    assert_success(&output);
    let out = stdout_str(&output);
    let actual = parse_json_str(&out);
    let expected = load_json_file(&expected_json_path("primitives"));
    assert_eq!(actual, expected, "tlbx-to-json stdout should match expected JSON");
}

#[test]
fn tlbx_to_json_file_when_output_flag() {
    let dir = tempfile::tempdir().unwrap();
    let out_file = dir.path().join("out.json");
    let input = sample_tlbx("primitives");
    let output = run(&["tlbx-to-json", path_str(&input), "-o", path_str(&out_file)]);
    assert_success(&output);
    let actual = load_json_file(&out_file);
    let expected = load_json_file(&expected_json_path("primitives"));
    assert_eq!(actual, expected);
}

#[test]
fn tlbx_to_json_stdout_matches_file_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = sample_tlbx("primitives");

    let output_stdout = run(&["tlbx-to-json", path_str(&input)]);
    assert_success(&output_stdout);
    let json_stdout = parse_json_str(&stdout_str(&output_stdout));

    let out_file = dir.path().join("out.json");
    let output_file = run(&["tlbx-to-json", path_str(&input), "-o", path_str(&out_file)]);
    assert_success(&output_file);
    let json_file = load_json_file(&out_file);

    assert_eq!(json_stdout, json_file, "stdout and file output should produce same JSON");
}

// =============================================================================
// 7. Exit Codes
// =============================================================================

#[test]
fn success_commands_exit_0() {
    let dir = tempfile::tempdir().unwrap();

    // help
    assert!(run(&["help"]).status.success());

    // validate
    let input = sample_tl("primitives");
    assert!(run(&["validate", path_str(&input)]).status.success());

    // info (text)
    assert!(run(&["info", path_str(&input)]).status.success());

    // info (binary)
    let binput = sample_tlbx("primitives");
    assert!(run(&["info", path_str(&binput)]).status.success());

    // compile
    let out = dir.path().join("c.tlbx");
    assert!(run(&["compile", path_str(&input), "-o", path_str(&out)]).status.success());

    // decompile
    let out_tl = dir.path().join("d.tl");
    assert!(run(&["decompile", path_str(&binput), "-o", path_str(&out_tl)]).status.success());

    // to-json
    assert!(run(&["to-json", path_str(&input)]).status.success());

    // from-json
    let json_in = expected_json_path("primitives");
    let out_fj = dir.path().join("fj.tl");
    assert!(run(&["from-json", path_str(&json_in), "-o", path_str(&out_fj)]).status.success());

    // tlbx-to-json
    assert!(run(&["tlbx-to-json", path_str(&binput)]).status.success());

    // json-to-tlbx
    let out_jt = dir.path().join("jt.tlbx");
    assert!(run(&["json-to-tlbx", path_str(&json_in), "-o", path_str(&out_jt)]).status.success());
}

#[test]
fn all_error_scenarios_exit_1() {
    let dir = tempfile::tempdir().unwrap();

    // No args
    assert_eq!(Command::new(tealeaf_bin()).output().unwrap().status.code(), Some(1));
    // Unknown command
    assert_eq!(run(&["bogus"]).status.code(), Some(1));
    // Missing args
    assert_eq!(run(&["compile"]).status.code(), Some(1));
    assert_eq!(run(&["decompile"]).status.code(), Some(1));
    assert_eq!(run(&["info"]).status.code(), Some(1));
    assert_eq!(run(&["validate"]).status.code(), Some(1));
    assert_eq!(run(&["to-json"]).status.code(), Some(1));
    assert_eq!(run(&["from-json"]).status.code(), Some(1));
    assert_eq!(run(&["tlbx-to-json"]).status.code(), Some(1));
    assert_eq!(run(&["json-to-tlbx"]).status.code(), Some(1));
    // Nonexistent file
    let out = dir.path().join("x.tlbx");
    assert_eq!(run(&["compile", "nope.tl", "-o", path_str(&out)]).status.code(), Some(1));
    // Invalid input
    let inv = errors_dir().join("unterminated_string.tl");
    assert_eq!(run(&["validate", path_str(&inv)]).status.code(), Some(1));
}

// =============================================================================
// 8. stderr/stdout Separation
// =============================================================================

#[test]
fn error_messages_go_to_stderr() {
    // Unknown command error -> stderr
    let output = run(&["frobnicate"]);
    let err = stderr_str(&output);
    assert!(err.contains("Unknown command"), "Unknown command error should be on stderr");

    // Nonexistent file with compile -> Error goes to stderr via main()
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tlbx");
    let output = run(&["compile", "nonexistent.tl", "-o", path_str(&out)]);
    let err = stderr_str(&output);
    assert!(err.contains("Error"), "File error should appear on stderr, got: {}", err);

    // Missing args -> Usage goes to stderr
    let output = run(&["compile", "somefile.tl"]);
    let err = stderr_str(&output);
    assert!(err.contains("Usage"), "Missing args should print usage on stderr");
}

#[test]
fn successful_json_data_goes_to_stdout() {
    let input = sample_tl("primitives");
    let output = run(&["to-json", path_str(&input)]);
    assert_success(&output);
    let out = stdout_str(&output);
    // stdout should have JSON data
    assert!(out.contains("{"), "JSON data should be on stdout");
    // stderr should be empty or only informational
    let err = stderr_str(&output);
    assert!(err.is_empty(), "stderr should be empty for successful to-json, got: {}", err);
}

#[test]
fn compile_progress_goes_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let input = sample_tl("primitives");
    let out = dir.path().join("out.tlbx");
    let output = run(&["compile", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);
    let out_str = stdout_str(&output);
    assert!(out_str.contains("Compiling"), "Expected 'Compiling' on stdout, got: {}", out_str);
    assert!(out_str.contains("Done"), "Expected 'Done' on stdout, got: {}", out_str);
}

#[test]
fn info_output_goes_to_stdout() {
    let input = sample_tl("primitives");
    let output = run(&["info", path_str(&input)]);
    assert_success(&output);
    let out = stdout_str(&output);
    assert!(out.contains("File:"), "Expected 'File:' on stdout");
    assert!(out.contains("Format:"), "Expected 'Format:' on stdout");
    assert!(out.contains("Size:"), "Expected 'Size:' on stdout");
}

#[test]
fn usage_on_missing_args_goes_to_stderr() {
    // Commands that require -o flag print usage to stderr
    let output = run(&["compile", "somefile.tl"]);
    assert_failure(&output);
    let err = stderr_str(&output);
    assert!(err.contains("Usage:"), "Usage should be on stderr for missing -o, got: {}", err);
}

// =============================================================================
// 9. Edge Cases
// =============================================================================

#[test]
fn empty_tl_file_validate() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.tl");
    std::fs::write(&empty, "").unwrap();
    let output = run(&["validate", path_str(&empty)]);
    // Empty file should be valid (0 keys, 0 schemas)
    assert_success(&output);
}

#[test]
fn empty_tl_file_compile() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.tl");
    std::fs::write(&empty, "").unwrap();
    let out = dir.path().join("empty.tlbx");
    let output = run(&["compile", path_str(&empty), "-o", path_str(&out)]);
    assert_success(&output);
}

#[test]
fn empty_tl_file_to_json() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.tl");
    std::fs::write(&empty, "").unwrap();
    let output = run(&["to-json", path_str(&empty)]);
    assert_success(&output);
    // Should produce valid JSON (empty object)
    let _json = parse_json_str(&stdout_str(&output));
}

#[test]
fn binary_file_as_text_input() {
    // Feeding a .tlbx to validate (expects text) should fail
    let input = sample_tlbx("primitives");
    let output = run(&["validate", path_str(&input)]);
    assert_failure(&output);
}

#[test]
fn text_file_as_binary_input() {
    // Feeding a .tl to decompile (expects binary) should fail
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.tl");
    let input = sample_tl("primitives");
    let output = run(&["decompile", path_str(&input), "-o", path_str(&out)]);
    assert_failure(&output);
}

#[test]
fn text_file_as_binary_input_tlbx_to_json() {
    let input = sample_tl("primitives");
    let output = run(&["tlbx-to-json", path_str(&input)]);
    assert_failure(&output);
}

#[test]
fn random_bytes_as_tlbx_input() {
    let dir = tempfile::tempdir().unwrap();
    let garbage = dir.path().join("garbage.tlbx");
    std::fs::write(&garbage, &[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33]).unwrap();
    let out = dir.path().join("out.tl");
    let output = run(&["decompile", path_str(&garbage), "-o", path_str(&out)]);
    assert_failure(&output);
}

#[test]
fn output_file_already_exists_is_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.json");
    // Write junk content first
    std::fs::write(&out, "this is not json").unwrap();

    let input = sample_tl("primitives");
    let output = run(&["to-json", path_str(&input), "-o", path_str(&out)]);
    assert_success(&output);

    // Should now contain valid JSON, not the junk
    let actual = load_json_file(&out);
    let expected = load_json_file(&expected_json_path("primitives"));
    assert_eq!(actual, expected);
}

#[test]
fn large_data_compile_and_read_back_via_cli() {
    let dir = tempfile::tempdir().unwrap();
    let input = sample_tl("large_data");

    // compile
    let tlbx = dir.path().join("large.tlbx");
    let output = run(&["compile", path_str(&input), "-o", path_str(&tlbx)]);
    assert_success(&output);

    // tlbx-to-json (binary path is reliable)
    let json_out = dir.path().join("large.json");
    let output = run(&["tlbx-to-json", path_str(&tlbx), "-o", path_str(&json_out)]);
    assert_success(&output);

    let actual = load_json_file(&json_out);
    let expected = load_json_file(&expected_json_path("large_data"));
    assert_eq!(actual, expected, "large_data compile -> tlbx-to-json should preserve content");
}
