//! Canonical sample validation tests
//!
//! These tests validate the TeaLeaf toolchain against canonical sample files
//! in the `canonical/` directory.

use tealeaf::{TeaLeaf, Reader};

const CANONICAL_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../canonical");

fn samples_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(CANONICAL_DIR).join("samples")
}

fn expected_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(CANONICAL_DIR).join("expected")
}

fn binary_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(CANONICAL_DIR).join("binary")
}

/// Load expected JSON and normalize whitespace for comparison
fn load_expected_json(name: &str) -> serde_json::Value {
    let path = expected_dir().join(format!("{}.json", name));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", path, e))
}

/// Parse a .tl file and convert to JSON value for comparison
fn parse_to_json(name: &str) -> serde_json::Value {
    let path = samples_dir().join(format!("{}.tl", name));
    let doc = TeaLeaf::load(&path)
        .unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", path, e));
    let json_str = doc.to_json()
        .unwrap_or_else(|e| panic!("Failed to convert to JSON: {}", e));
    serde_json::from_str(&json_str)
        .unwrap_or_else(|e| panic!("Failed to parse generated JSON: {}", e))
}

/// Read a .tlbx file and convert to JSON value for comparison
fn read_binary_to_json(name: &str) -> serde_json::Value {
    let path = binary_dir().join(format!("{}.tlbx", name));
    let reader = Reader::open(&path)
        .unwrap_or_else(|e| panic!("Failed to open {:?}: {}", path, e));

    // Convert reader contents to JSON
    let mut obj = serde_json::Map::new();
    for key in reader.keys() {
        let value = reader.get(key)
            .unwrap_or_else(|e| panic!("Failed to get key {}: {}", key, e));
        let json_value = value_to_json(&value);
        obj.insert(key.to_string(), json_value);
    }
    serde_json::Value::Object(obj)
}

fn value_to_json(value: &tealeaf::Value) -> serde_json::Value {
    match value {
        tealeaf::Value::Null => serde_json::Value::Null,
        tealeaf::Value::Bool(b) => serde_json::Value::Bool(*b),
        tealeaf::Value::Int(i) => serde_json::json!(*i),
        tealeaf::Value::UInt(u) => serde_json::json!(*u),
        tealeaf::Value::Float(f) => {
            // Always output floats as floats - the type distinction is intentional
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        tealeaf::Value::String(s) => serde_json::Value::String(s.clone()),
        tealeaf::Value::Bytes(b) => {
            let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
            serde_json::Value::String(format!("0x{}", hex))
        }
        tealeaf::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(value_to_json).collect())
        }
        tealeaf::Value::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        tealeaf::Value::Map(pairs) => {
            let arr: Vec<serde_json::Value> = pairs
                .iter()
                .map(|(k, v)| serde_json::json!([value_to_json(k), value_to_json(v)]))
                .collect();
            serde_json::Value::Array(arr)
        }
        tealeaf::Value::Ref(r) => {
            serde_json::json!({"$ref": r})
        }
        tealeaf::Value::Tagged(tag, inner) => {
            serde_json::json!({"$tag": tag, "$value": value_to_json(inner)})
        }
        tealeaf::Value::Timestamp(ts) => {
            // Convert to ISO 8601
            let secs = ts / 1000;
            let millis = ts % 1000;
            let days = secs / 86400;
            let time_secs = secs % 86400;
            let hours = time_secs / 3600;
            let mins = (time_secs % 3600) / 60;
            let secs_rem = time_secs % 60;

            // Simple date calculation (days since 1970-01-01)
            let z = days + 719468;
            let era = if z >= 0 { z } else { z - 146096 } / 146097;
            let doe = (z - era * 146097) as u32;
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let y = yoe as i64 + era * 400;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            let y = if m <= 2 { y + 1 } else { y };

            let iso = if millis > 0 {
                format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                    y, m, d, hours, mins, secs_rem, millis)
            } else {
                format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    y, m, d, hours, mins, secs_rem)
            };
            serde_json::Value::String(iso)
        }
    }
}

// =============================================================================
// Text → JSON Validation Tests
// =============================================================================

#[test]
fn canonical_primitives_text_to_json() {
    let actual = parse_to_json("primitives");
    let expected = load_expected_json("primitives");
    assert_eq!(actual, expected, "primitives.tl → JSON mismatch");
}

#[test]
fn canonical_arrays_text_to_json() {
    let actual = parse_to_json("arrays");
    let expected = load_expected_json("arrays");
    assert_eq!(actual, expected, "arrays.tl → JSON mismatch");
}

#[test]
fn canonical_objects_text_to_json() {
    let actual = parse_to_json("objects");
    let expected = load_expected_json("objects");
    assert_eq!(actual, expected, "objects.tl → JSON mismatch");
}

#[test]
fn canonical_schemas_text_to_json() {
    let actual = parse_to_json("schemas");
    let expected = load_expected_json("schemas");
    assert_eq!(actual, expected, "schemas.tl → JSON mismatch");
}

#[test]
fn canonical_special_types_text_to_json() {
    let actual = parse_to_json("special_types");
    let expected = load_expected_json("special_types");
    assert_eq!(actual, expected, "special_types.tl → JSON mismatch");
}

// =============================================================================
// Binary → JSON Roundtrip Tests
// =============================================================================

#[test]
fn canonical_primitives_binary_roundtrip() {
    let actual = read_binary_to_json("primitives");
    let expected = load_expected_json("primitives");
    assert_eq!(actual, expected, "primitives.tlbx → JSON mismatch");
}

#[test]
fn canonical_arrays_binary_roundtrip() {
    let actual = read_binary_to_json("arrays");
    let expected = load_expected_json("arrays");
    assert_eq!(actual, expected, "arrays.tlbx → JSON mismatch");
}

#[test]
fn canonical_objects_binary_roundtrip() {
    let actual = read_binary_to_json("objects");
    let expected = load_expected_json("objects");
    assert_eq!(actual, expected, "objects.tlbx → JSON mismatch");
}

#[test]
fn canonical_schemas_binary_roundtrip() {
    let actual = read_binary_to_json("schemas");
    let expected = load_expected_json("schemas");
    assert_eq!(actual, expected, "schemas.tlbx → JSON mismatch");
}

#[test]
fn canonical_special_types_binary_roundtrip() {
    let actual = read_binary_to_json("special_types");
    let expected = load_expected_json("special_types");
    assert_eq!(actual, expected, "special_types.tlbx → JSON mismatch");
}

// =============================================================================
// Text → Binary → JSON Full Roundtrip Tests
// =============================================================================

#[test]
fn canonical_primitives_full_roundtrip() {
    let path = samples_dir().join("primitives.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("primitives");
    assert_eq!(actual, expected, "primitives: text → binary → JSON mismatch");
}

#[test]
fn canonical_arrays_full_roundtrip() {
    let path = samples_dir().join("arrays.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("arrays");
    assert_eq!(actual, expected, "arrays: text → binary → JSON mismatch");
}

#[test]
fn canonical_objects_full_roundtrip() {
    let path = samples_dir().join("objects.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("objects");
    assert_eq!(actual, expected, "objects: text → binary → JSON mismatch");
}

#[test]
fn canonical_schemas_full_roundtrip() {
    let path = samples_dir().join("schemas.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("schemas");
    assert_eq!(actual, expected, "schemas: text → binary → JSON mismatch");
}

#[test]
fn canonical_special_types_full_roundtrip() {
    let path = samples_dir().join("special_types.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("special_types");
    assert_eq!(actual, expected, "special_types: text → binary → JSON mismatch");
}

// =============================================================================
// Timestamps Tests
// =============================================================================

#[test]
fn canonical_timestamps_text_to_json() {
    let actual = parse_to_json("timestamps");
    let expected = load_expected_json("timestamps");
    assert_eq!(actual, expected, "timestamps.tl → JSON mismatch");
}

#[test]
fn canonical_timestamps_binary_roundtrip() {
    let actual = read_binary_to_json("timestamps");
    let expected = load_expected_json("timestamps");
    assert_eq!(actual, expected, "timestamps.tlbx → JSON mismatch");
}

#[test]
fn canonical_timestamps_full_roundtrip() {
    let path = samples_dir().join("timestamps.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("timestamps");
    assert_eq!(actual, expected, "timestamps: text → binary → JSON mismatch");
}

// =============================================================================
// Numbers Extended Tests
// =============================================================================

#[test]
fn canonical_numbers_extended_text_to_json() {
    let actual = parse_to_json("numbers_extended");
    let expected = load_expected_json("numbers_extended");
    assert_eq!(actual, expected, "numbers_extended.tl → JSON mismatch");
}

#[test]
fn canonical_numbers_extended_binary_roundtrip() {
    let actual = read_binary_to_json("numbers_extended");
    let expected = load_expected_json("numbers_extended");
    assert_eq!(actual, expected, "numbers_extended.tlbx → JSON mismatch");
}

#[test]
fn canonical_numbers_extended_full_roundtrip() {
    let path = samples_dir().join("numbers_extended.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("numbers_extended");
    assert_eq!(actual, expected, "numbers_extended: text → binary → JSON mismatch");
}

// =============================================================================
// Unions Tests
// =============================================================================

#[test]
fn canonical_unions_text_to_json() {
    let actual = parse_to_json("unions");
    let expected = load_expected_json("unions");
    assert_eq!(actual, expected, "unions.tl → JSON mismatch");
}

#[test]
fn canonical_unions_binary_roundtrip() {
    let actual = read_binary_to_json("unions");
    let expected = load_expected_json("unions");
    assert_eq!(actual, expected, "unions.tlbx → JSON mismatch");
}

#[test]
fn canonical_unions_full_roundtrip() {
    let path = samples_dir().join("unions.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("unions");
    assert_eq!(actual, expected, "unions: text → binary → JSON mismatch");
}

// =============================================================================
// Multiline Strings Tests
// =============================================================================

#[test]
fn canonical_multiline_strings_text_to_json() {
    let actual = parse_to_json("multiline_strings");
    let expected = load_expected_json("multiline_strings");
    assert_eq!(actual, expected, "multiline_strings.tl → JSON mismatch");
}

#[test]
fn canonical_multiline_strings_binary_roundtrip() {
    let actual = read_binary_to_json("multiline_strings");
    let expected = load_expected_json("multiline_strings");
    assert_eq!(actual, expected, "multiline_strings.tlbx → JSON mismatch");
}

#[test]
fn canonical_multiline_strings_full_roundtrip() {
    let path = samples_dir().join("multiline_strings.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("multiline_strings");
    assert_eq!(actual, expected, "multiline_strings: text → binary → JSON mismatch");
}

// =============================================================================
// Unicode and Escaping Tests
// =============================================================================

#[test]
fn canonical_unicode_escaping_text_to_json() {
    let actual = parse_to_json("unicode_escaping");
    let expected = load_expected_json("unicode_escaping");
    assert_eq!(actual, expected, "unicode_escaping.tl → JSON mismatch");
}

#[test]
fn canonical_unicode_escaping_binary_roundtrip() {
    let actual = read_binary_to_json("unicode_escaping");
    let expected = load_expected_json("unicode_escaping");
    assert_eq!(actual, expected, "unicode_escaping.tlbx → JSON mismatch");
}

#[test]
fn canonical_unicode_escaping_full_roundtrip() {
    let path = samples_dir().join("unicode_escaping.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("unicode_escaping");
    assert_eq!(actual, expected, "unicode_escaping: text → binary → JSON mismatch");
}

// =============================================================================
// Refs, Tags, Maps Tests
// =============================================================================

#[test]
fn canonical_refs_tags_maps_text_to_json() {
    let actual = parse_to_json("refs_tags_maps");
    let expected = load_expected_json("refs_tags_maps");
    assert_eq!(actual, expected, "refs_tags_maps.tl → JSON mismatch");
}

#[test]
fn canonical_refs_tags_maps_binary_roundtrip() {
    let actual = read_binary_to_json("refs_tags_maps");
    let expected = load_expected_json("refs_tags_maps");
    assert_eq!(actual, expected, "refs_tags_maps.tlbx → JSON mismatch");
}

#[test]
fn canonical_refs_tags_maps_full_roundtrip() {
    let path = samples_dir().join("refs_tags_maps.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("refs_tags_maps");
    assert_eq!(actual, expected, "refs_tags_maps: text → binary → JSON mismatch");
}

// =============================================================================
// Mixed Schemas Tests
// =============================================================================

#[test]
fn canonical_mixed_schemas_text_to_json() {
    let actual = parse_to_json("mixed_schemas");
    let expected = load_expected_json("mixed_schemas");
    assert_eq!(actual, expected, "mixed_schemas.tl → JSON mismatch");
}

#[test]
fn canonical_mixed_schemas_binary_roundtrip() {
    let actual = read_binary_to_json("mixed_schemas");
    let expected = load_expected_json("mixed_schemas");
    assert_eq!(actual, expected, "mixed_schemas.tlbx → JSON mismatch");
}

#[test]
fn canonical_mixed_schemas_full_roundtrip() {
    let path = samples_dir().join("mixed_schemas.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("mixed_schemas");
    assert_eq!(actual, expected, "mixed_schemas: text → binary → JSON mismatch");
}

// =============================================================================
// Large Data Stress Tests
// =============================================================================

#[test]
fn canonical_large_data_text_to_json() {
    let actual = parse_to_json("large_data");
    let expected = load_expected_json("large_data");
    assert_eq!(actual, expected, "large_data.tl → JSON mismatch");
}

#[test]
fn canonical_large_data_binary_roundtrip() {
    let actual = read_binary_to_json("large_data");
    let expected = load_expected_json("large_data");
    assert_eq!(actual, expected, "large_data.tlbx → JSON mismatch");
}

#[test]
fn canonical_large_data_full_roundtrip() {
    let path = samples_dir().join("large_data.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("large_data");
    assert_eq!(actual, expected, "large_data: text → binary → JSON mismatch");
}

// =============================================================================
// Cyclic References Tests
// =============================================================================

#[test]
fn canonical_cyclic_refs_text_to_json() {
    let actual = parse_to_json("cyclic_refs");
    let expected = load_expected_json("cyclic_refs");
    assert_eq!(actual, expected, "cyclic_refs.tl → JSON mismatch");
}

#[test]
fn canonical_cyclic_refs_binary_roundtrip() {
    let actual = read_binary_to_json("cyclic_refs");
    let expected = load_expected_json("cyclic_refs");
    assert_eq!(actual, expected, "cyclic_refs.tlbx → JSON mismatch");
}

#[test]
fn canonical_cyclic_refs_full_roundtrip() {
    let path = samples_dir().join("cyclic_refs.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    let reader = Reader::open(temp.path()).expect("Failed to read");
    let actual = read_binary_to_json_from_reader(&reader);
    let expected = load_expected_json("cyclic_refs");
    assert_eq!(actual, expected, "cyclic_refs: text → binary → JSON mismatch");
}

fn read_binary_to_json_from_reader(reader: &Reader) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for key in reader.keys() {
        let value = reader.get(key).expect("Failed to get key");
        obj.insert(key.to_string(), value_to_json(&value));
    }
    serde_json::Value::Object(obj)
}

// =============================================================================
// Error Message Golden Tests
// =============================================================================

fn errors_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("canonical")
        .join("errors")
}

/// Helper to extract error message from Result
fn get_error_message<T>(result: Result<T, tealeaf::Error>) -> String {
    match result {
        Ok(_) => panic!("Expected error but got Ok"),
        Err(e) => format!("{}", e),
    }
}

#[test]
fn error_unterminated_string() {
    let path = errors_dir().join("unterminated_string.tl");
    let result = TeaLeaf::load(&path);
    assert!(result.is_err(), "Should fail to parse");
    let msg = get_error_message(result);
    assert!(msg.contains("Unterminated string"), "Error message should contain 'Unterminated string': {}", msg);
}

#[test]
fn error_unterminated_multiline() {
    let path = errors_dir().join("unterminated_multiline.tl");
    let result = TeaLeaf::load(&path);
    assert!(result.is_err(), "Should fail to parse");
    let msg = get_error_message(result);
    assert!(msg.contains("Unterminated multiline string"), "Error message should contain 'Unterminated multiline string': {}", msg);
}

#[test]
fn error_invalid_hex() {
    let path = errors_dir().join("invalid_hex.tl");
    let result = TeaLeaf::load(&path);
    assert!(result.is_err(), "Should fail to parse");
    let msg = get_error_message(result);
    assert!(msg.contains("Invalid hex"), "Error message should contain 'Invalid hex': {}", msg);
}

#[test]
fn error_invalid_binary() {
    let path = errors_dir().join("invalid_binary.tl");
    let result = TeaLeaf::load(&path);
    assert!(result.is_err(), "Should fail to parse");
    let msg = get_error_message(result);
    assert!(msg.contains("Invalid binary"), "Error message should contain 'Invalid binary': {}", msg);
}

#[test]
fn error_unexpected_token() {
    let path = errors_dir().join("unexpected_token.tl");
    let result = TeaLeaf::load(&path);
    assert!(result.is_err(), "Should fail to parse");
    let msg = get_error_message(result);
    assert!(msg.contains("Expected") && msg.contains("Colon"), "Error message should indicate expected token: {}", msg);
}

#[test]
fn error_unclosed_brace() {
    let path = errors_dir().join("unclosed_brace.tl");
    let result = TeaLeaf::load(&path);
    assert!(result.is_err(), "Should fail to parse");
    let msg = get_error_message(result);
    assert!(msg.contains("Eof") || msg.contains("EOF") || msg.contains("end"), "Error message should indicate unexpected end: {}", msg);
}

#[test]
fn error_unclosed_bracket() {
    let path = errors_dir().join("unclosed_bracket.tl");
    let result = TeaLeaf::load(&path);
    assert!(result.is_err(), "Should fail to parse");
    let msg = get_error_message(result);
    assert!(msg.contains("Eof") || msg.contains("EOF") || msg.contains("end"), "Error message should indicate unexpected end: {}", msg);
}

#[test]
fn error_include_not_found() {
    let path = errors_dir().join("include_not_found.tl");
    let result = TeaLeaf::load(&path);
    assert!(result.is_err(), "Should fail to parse");
    let msg = get_error_message(result);
    assert!(msg.contains("Failed to include") || msg.contains("not found") || msg.contains("error"),
            "Error message should indicate include failure: {}", msg);
}

#[test]
fn error_invalid_magic() {
    let path = errors_dir().join("invalid_magic.tlbx");
    let result = Reader::open(&path);
    assert!(result.is_err(), "Should fail to read invalid binary");
    let msg = get_error_message(result);
    assert!(msg.contains("magic") || msg.contains("Magic"), "Error message should mention magic bytes: {}", msg);
}

#[test]
fn error_invalid_json() {
    let result = TeaLeaf::from_json("{invalid json}");
    assert!(result.is_err(), "Should fail to parse invalid JSON");
    let msg = get_error_message(result);
    assert!(msg.contains("JSON") || msg.contains("parse"), "Error message should indicate JSON parse error: {}", msg);
}
