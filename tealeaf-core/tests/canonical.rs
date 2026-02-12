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
        tealeaf::Value::JsonNumber(s) => {
            s.parse::<serde_json::Number>()
                .map(serde_json::Value::Number)
                .unwrap_or_else(|_| serde_json::Value::String(s.clone()))
        }
        tealeaf::Value::Timestamp(ts, tz) => {
            // Apply timezone offset to get local time for display
            let local_ts = ts + (*tz as i64) * 60_000;

            // Convert to ISO 8601
            let secs = local_ts.div_euclid(1000);
            let millis = local_ts.rem_euclid(1000);
            let days = secs.div_euclid(86400);
            let time_secs = secs.rem_euclid(86400);
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

            let tz_suffix = if *tz == 0 {
                "Z".to_string()
            } else {
                let sign = if *tz > 0 { '+' } else { '-' };
                let abs = tz.unsigned_abs();
                format!("{}{:02}:{:02}", sign, abs / 60, abs % 60)
            };

            let iso = if millis > 0 {
                format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}{}",
                    y, m, d, hours, mins, secs_rem, millis, tz_suffix)
            } else {
                format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
                    y, m, d, hours, mins, secs_rem, tz_suffix)
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

#[test]
fn canonical_unions_schema_binary_roundtrip() {
    // Verify that union definitions survive text -> binary -> reader roundtrip
    let path = samples_dir().join("unions.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    // Verify unions were parsed from text
    assert!(!doc.unions.is_empty(), "unions should be parsed from text");
    assert!(doc.unions.contains_key("Shape"), "Shape union should be present");
    assert!(doc.unions.contains_key("Result"), "Result union should be present");
    assert!(doc.unions.contains_key("Maybe"), "Maybe union should be present");
    assert!(doc.unions.contains_key("Event"), "Event union should be present");

    // Verify Shape union structure
    let shape = doc.unions.get("Shape").unwrap();
    assert_eq!(shape.variants.len(), 3);
    assert_eq!(shape.variants[0].name, "circle");
    assert_eq!(shape.variants[0].fields.len(), 1);
    assert_eq!(shape.variants[1].name, "rectangle");
    assert_eq!(shape.variants[1].fields.len(), 2);
    assert_eq!(shape.variants[2].name, "point");
    assert_eq!(shape.variants[2].fields.len(), 0);

    // Compile to binary
    let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    doc.compile(temp.path(), false).expect("Failed to compile");

    // Read back from binary
    let reader = Reader::open(temp.path()).expect("Failed to read binary");

    // Verify union definitions survived binary roundtrip
    assert_eq!(reader.unions.len(), 4, "should have 4 unions from binary");

    // Find Shape union in reader
    let shape_reader = reader.unions.iter().find(|u| u.name == "Shape")
        .expect("Shape union should be in binary reader");
    assert_eq!(shape_reader.variants.len(), 3);
    assert_eq!(shape_reader.variants[0].name, "circle");
    assert_eq!(shape_reader.variants[0].fields.len(), 1);
    assert_eq!(shape_reader.variants[0].fields[0].name, "radius");
    assert_eq!(shape_reader.variants[1].name, "rectangle");
    assert_eq!(shape_reader.variants[1].fields.len(), 2);
    assert_eq!(shape_reader.variants[2].name, "point");
    assert_eq!(shape_reader.variants[2].fields.len(), 0);

    // Find Event union in reader
    let event_reader = reader.unions.iter().find(|u| u.name == "Event")
        .expect("Event union should be in binary reader");
    assert_eq!(event_reader.variants.len(), 4);

    // Verify full text roundtrip: text -> binary -> reader -> TeaLeaf -> text
    let doc2 = TeaLeaf::from_reader(&reader).expect("Failed to create doc from reader");
    assert_eq!(doc2.unions.len(), 4, "doc from reader should have 4 unions");

    // The text output should contain @union definitions
    let text = doc2.to_tl_with_schemas();
    assert!(text.contains("@union"), "Output text should contain @union definitions");
    assert!(text.contains("Shape"), "Output text should contain Shape union");
    assert!(text.contains("Event"), "Output text should contain Event union");
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

// =============================================================================
// Compact Text Roundtrip Tests
// =============================================================================
// Verifies that compact formatting (FormatOptions::compact()) is lossless:
// parse .tl → compact text → re-parse → JSON matches expected.

fn compact_roundtrip(name: &str) {
    let path = samples_dir().join(format!("{}.tl", name));
    let doc = TeaLeaf::load(&path)
        .unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", path, e));

    let opts = tealeaf::FormatOptions::compact();
    let compact_text = doc.to_tl_with_options(&opts);

    // Compact text must be parseable
    let reparsed = TeaLeaf::parse(&compact_text)
        .unwrap_or_else(|e| panic!("{}: compact text failed to re-parse: {}", name, e));

    let json_str = reparsed.to_json()
        .unwrap_or_else(|e| panic!("{}: failed to convert reparsed to JSON: {}", name, e));
    let actual: serde_json::Value = serde_json::from_str(&json_str)
        .unwrap_or_else(|e| panic!("{}: failed to parse reparsed JSON: {}", name, e));

    let expected = load_expected_json(name);
    assert_eq!(actual, expected, "{}: compact roundtrip JSON mismatch", name);
}

fn compact_is_not_larger(name: &str) {
    let path = samples_dir().join(format!("{}.tl", name));
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let pretty = doc.to_tl_with_schemas();
    let compact = doc.to_tl_with_options(&tealeaf::FormatOptions::compact());

    assert!(compact.len() <= pretty.len(),
        "{}: compact ({}) should not be larger than pretty ({})", name, compact.len(), pretty.len());
}

#[test]
fn canonical_compact_roundtrip_primitives() { compact_roundtrip("primitives"); }
#[test]
fn canonical_compact_roundtrip_arrays() { compact_roundtrip("arrays"); }
#[test]
fn canonical_compact_roundtrip_objects() { compact_roundtrip("objects"); }
#[test]
fn canonical_compact_roundtrip_schemas() { compact_roundtrip("schemas"); }
#[test]
fn canonical_compact_roundtrip_special_types() { compact_roundtrip("special_types"); }
#[test]
fn canonical_compact_roundtrip_timestamps() { compact_roundtrip("timestamps"); }
#[test]
fn canonical_compact_roundtrip_numbers_extended() { compact_roundtrip("numbers_extended"); }
#[test]
fn canonical_compact_roundtrip_unions() { compact_roundtrip("unions"); }
#[test]
fn canonical_compact_roundtrip_multiline_strings() { compact_roundtrip("multiline_strings"); }
#[test]
fn canonical_compact_roundtrip_unicode_escaping() { compact_roundtrip("unicode_escaping"); }
#[test]
fn canonical_compact_roundtrip_refs_tags_maps() { compact_roundtrip("refs_tags_maps"); }
#[test]
fn canonical_compact_roundtrip_mixed_schemas() { compact_roundtrip("mixed_schemas"); }
#[test]
fn canonical_compact_roundtrip_large_data() { compact_roundtrip("large_data"); }
#[test]
fn canonical_compact_roundtrip_cyclic_refs() { compact_roundtrip("cyclic_refs"); }

#[test]
fn canonical_compact_is_not_larger_primitives() { compact_is_not_larger("primitives"); }
#[test]
fn canonical_compact_is_not_larger_schemas() { compact_is_not_larger("schemas"); }
#[test]
fn canonical_compact_is_not_larger_large_data() { compact_is_not_larger("large_data"); }
#[test]
fn canonical_compact_is_not_larger_mixed_schemas() { compact_is_not_larger("mixed_schemas"); }

// =============================================================================
// Compact Floats Tests
// =============================================================================
// Verifies compact_floats behavior: whole-number floats lose `.0` suffix,
// re-parsing produces Int instead of Float (documented lossy behavior),
// but numeric values are preserved.

/// Compare two JSON values treating whole-number floats and ints as equivalent.
fn json_numeric_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(na), serde_json::Value::Number(nb)) => {
            // Compare as f64 to handle int/float equivalence
            let fa = na.as_f64().unwrap_or(f64::NAN);
            let fb = nb.as_f64().unwrap_or(f64::NAN);
            (fa - fb).abs() < 1e-10 || (fa.is_nan() && fb.is_nan())
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| json_numeric_equal(x, y))
        }
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, v)| {
                b.get(k).map_or(false, |bv| json_numeric_equal(v, bv))
            })
        }
        _ => a == b,
    }
}

#[test]
fn canonical_compact_floats_schemas() {
    // schemas.tl has whole-number floats: 0.0, 1.0, 2.0, 95000.0, 75000.0, 88000.0
    // and non-whole floats: 3.5, -4.2
    let path = samples_dir().join("schemas.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let opts = tealeaf::FormatOptions::compact().with_compact_floats();
    let compact_text = doc.to_tl_with_options(&opts);

    // Whole-number floats should NOT have .0 suffix
    assert!(!compact_text.contains("95000.0"), "95000.0 should be stripped to 95000");
    assert!(!compact_text.contains("75000.0"), "75000.0 should be stripped to 75000");
    assert!(!compact_text.contains("88000.0"), "88000.0 should be stripped to 88000");

    // Non-whole floats should be preserved
    assert!(compact_text.contains("3.5"), "3.5 should be preserved");
    assert!(compact_text.contains("-4.2"), "-4.2 should be preserved");

    // Re-parse and compare numerically
    let reparsed = TeaLeaf::parse(&compact_text).expect("Failed to re-parse compact_floats text");
    let json_str = reparsed.to_json().expect("Failed to convert to JSON");
    let actual: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");
    let expected = load_expected_json("schemas");

    assert!(json_numeric_equal(&actual, &expected),
        "schemas: compact_floats roundtrip should be numerically equivalent\nactual: {}\nexpected: {}",
        serde_json::to_string_pretty(&actual).unwrap(),
        serde_json::to_string_pretty(&expected).unwrap());
}

#[test]
fn canonical_compact_floats_large_data() {
    // large_data.tl has whole-number floats: 0.0, 10.0, 20.0, 3.0
    // and non-whole floats: 1.1, 2.2, 3.3, ..., 9.9, 11.1, ..., 29.9
    let path = samples_dir().join("large_data.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let opts = tealeaf::FormatOptions::compact().with_compact_floats();
    let compact_text = doc.to_tl_with_options(&opts);

    // Non-whole floats should be preserved
    assert!(compact_text.contains("1.1"), "1.1 should be preserved");
    assert!(compact_text.contains("9.9"), "9.9 should be preserved");

    // Re-parse and compare numerically
    let reparsed = TeaLeaf::parse(&compact_text).expect("Failed to re-parse compact_floats text");
    let json_str = reparsed.to_json().expect("Failed to convert to JSON");
    let actual: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");
    let expected = load_expected_json("large_data");

    assert!(json_numeric_equal(&actual, &expected),
        "large_data: compact_floats roundtrip should be numerically equivalent");
}

#[test]
fn canonical_compact_floats_primitives() {
    // primitives.tl has non-whole floats only: 3.14159, -273.15
    // compact_floats should not alter these
    let path = samples_dir().join("primitives.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let opts = tealeaf::FormatOptions::compact().with_compact_floats();
    let compact_text = doc.to_tl_with_options(&opts);

    assert!(compact_text.contains("3.14159"), "3.14159 should be preserved");
    assert!(compact_text.contains("-273.15"), "-273.15 should be preserved");

    // Since no whole-number floats, output should be exactly equivalent
    let reparsed = TeaLeaf::parse(&compact_text).expect("Failed to re-parse");
    let json_str = reparsed.to_json().expect("Failed to convert to JSON");
    let actual: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");
    let expected = load_expected_json("primitives");
    assert_eq!(actual, expected, "primitives: compact_floats should be identical (no whole-number floats)");
}

#[test]
fn canonical_compact_floats_numbers_extended() {
    // numbers_extended.tl has scientific notation floats
    let path = samples_dir().join("numbers_extended.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let opts = tealeaf::FormatOptions::compact().with_compact_floats();
    let compact_text = doc.to_tl_with_options(&opts);

    // Must re-parse without errors
    let reparsed = TeaLeaf::parse(&compact_text).expect("Failed to re-parse compact_floats text");
    let json_str = reparsed.to_json().expect("Failed to convert to JSON");
    let actual: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");
    let expected = load_expected_json("numbers_extended");

    assert!(json_numeric_equal(&actual, &expected),
        "numbers_extended: compact_floats roundtrip should be numerically equivalent");
}

#[test]
fn canonical_compact_floats_is_smaller_than_compact() {
    // For schemas.tl which has many whole-number floats, compact_floats should save chars
    let path = samples_dir().join("schemas.tl");
    let doc = TeaLeaf::load(&path).expect("Failed to parse");

    let compact = doc.to_tl_with_options(&tealeaf::FormatOptions::compact());
    let compact_floats = doc.to_tl_with_options(&tealeaf::FormatOptions::compact().with_compact_floats());

    assert!(compact_floats.len() < compact.len(),
        "schemas: compact_floats ({}) should be smaller than compact ({}) due to whole-number floats",
        compact_floats.len(), compact.len());
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
