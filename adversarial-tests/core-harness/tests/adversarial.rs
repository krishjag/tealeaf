use tealeaf::{Reader, TeaLeaf, Value};

fn assert_parse_err(input: &str) {
    assert!(TeaLeaf::parse(input).is_err(), "Expected parse error for: {}", input);
}

#[test]
fn parse_invalid_syntax_unclosed_string() {
    assert_parse_err("name: \"Alice");
}

#[test]
fn parse_invalid_escape_sequence() {
    assert_parse_err("name: \"Alice\\q\"");
}

#[test]
fn parse_missing_colon() {
    assert_parse_err("name \"Alice\"");
}

#[test]
fn parse_schema_unclosed() {
    assert_parse_err("@struct user (id: int, name: string\nusers: @table user [(1, \"Alice\")]\n");
}

#[test]
fn parse_table_wrong_arity() {
    assert_parse_err("@struct user (id: int, name: string)\nusers: @table user [(1, \"Alice\", \"extra\")]\n");
}

#[test]
fn parse_number_overflow_falls_to_json_number() {
    // Numbers exceeding i64/u64 range are stored as JsonNumber (exact decimal string)
    let doc = TeaLeaf::parse("big: 18446744073709551616").expect("parse overflow to json number");
    assert!(matches!(doc.get("big"), Some(Value::JsonNumber(_))));
}

#[test]
fn parse_unicode_escape_short() {
    assert_parse_err("text: \"\\u12\"");
}

#[test]
fn parse_unicode_escape_invalid_hex() {
    assert_parse_err("text: \"\\uZZZZ\"");
}

#[test]
fn parse_unicode_escape_surrogate() {
    assert_parse_err("text: \"\\uD800\"");
}

#[test]
fn parse_unterminated_multiline_string() {
    assert_parse_err("text: \"\"\"unterminated");
}

#[test]
fn parse_deep_nesting_ok() {
    let doc = TeaLeaf::parse("root: [[[[[[[1]]]]]]]").expect("parse deep nesting");
    assert!(doc.get("root").is_some());
}

#[test]
fn from_json_invalid() {
    assert!(TeaLeaf::from_json("{\"a\":1,}").is_err());
}

#[test]
fn from_json_large_number_falls_to_json_number() {
    let doc = TeaLeaf::from_json("{\"big\": 18446744073709551616}").expect("from_json");
    let value = doc.get("big").expect("big");
    assert!(matches!(value, tealeaf::Value::JsonNumber(_)));
}

#[test]
fn from_json_root_array_is_preserved() {
    let doc = TeaLeaf::from_json("[1,2,3]").expect("from_json");
    let value = doc.get("root").expect("root");
    assert!(matches!(value, tealeaf::Value::Array(_)));
}

#[test]
fn reader_rejects_bad_magic() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("bad_magic.tlbx");
    std::fs::write(&path, [0x58u8, 0x58u8, 0x58u8, 0x58u8]).expect("write");
    assert!(Reader::open(&path).is_err());
}

#[test]
fn reader_rejects_bad_version() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("bad_version.tlbx");
    let mut bytes = vec![0u8; 64];
    bytes[0..4].copy_from_slice(b"TLBX");
    bytes[4..6].copy_from_slice(&3u16.to_le_bytes());
    bytes[6..8].copy_from_slice(&0u16.to_le_bytes());
    std::fs::write(&path, bytes).expect("write");
    assert!(Reader::open(&path).is_err());
}

#[test]
fn load_invalid_file_errors() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("bad.tl");
    std::fs::write(&path, "name: \"Alice").expect("write");
    assert!(TeaLeaf::load(&path).is_err());
}

#[test]
fn load_invalid_utf8_errors() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("invalid_utf8.tl");
    std::fs::write(&path, [0xFFu8, 0xFEu8, 0xFAu8]).expect("write");
    assert!(TeaLeaf::load(&path).is_err());
}

// =========================================================================
// Error variant coverage: UnknownStruct
// =========================================================================

#[test]
fn parse_unknown_struct_in_table() {
    // References a struct name that was never defined
    let result = TeaLeaf::parse(
        "@struct user (id: int, name: string)\ndata: @table nonexistent [(1, \"Alice\")]\n",
    );
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(err.contains("nonexistent"), "Error should mention the unknown struct name: {}", err);
}

// =========================================================================
// Error variant coverage: UnexpectedToken at EOF
// =========================================================================

#[test]
fn parse_unexpected_eof_unclosed_brace() {
    // Unclosed brace leads to unexpected EOF token
    let result = TeaLeaf::parse("obj: {x: 1,");
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(err.contains("Eof") || err.contains("end of input") || err.contains("Expected"),
        "Error should indicate unexpected EOF: {}", err);
}

#[test]
fn parse_unexpected_eof_unclosed_bracket() {
    let result = TeaLeaf::parse("arr: [1, 2,");
    assert!(result.is_err());
}

// =========================================================================
// Error variant coverage: MissingField (reader)
// =========================================================================

#[test]
fn reader_missing_field() {
    // Create a valid binary file, then query a nonexistent key
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("missing_field.tlbx");

    let doc = TeaLeaf::parse("exists: 42").expect("parse");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::open(&path).expect("open");
    let result = reader.get("nonexistent");
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(err.contains("nonexistent"), "Error should mention the missing key: {}", err);
}

// =========================================================================
// Type coercion coverage: best-effort numeric coercion (spec §2.5)
// =========================================================================

#[test]
fn writer_int_overflow_coerces_to_zero() {
    // Schema with int8 field, but value exceeds int8 range — coerces to 0
    let doc = TeaLeaf::parse(
        "@struct tiny (val: int8)\nitems: @table tiny [(999)]\n",
    ).expect("parse should succeed");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("int_overflow.tlbx");
    doc.compile(&path, false).expect("compile should succeed with coercion");
    let reader = Reader::open(&path).expect("reader");
    let items = reader.get("items").expect("items");
    if let Value::Array(arr) = items {
        if let Value::Object(obj) = &arr[0] {
            let val = obj.get("val").expect("val field");
            assert_eq!(val, &Value::Int(0), "out-of-range int8 should coerce to 0");
        } else { panic!("expected object"); }
    } else { panic!("expected array"); }
}

#[test]
fn writer_uint_negative_coerces_to_zero() {
    // Schema with uint8 field, but value is negative — coerces to 0
    let doc = TeaLeaf::parse(
        "@struct tiny (val: uint8)\nitems: @table tiny [(-1)]\n",
    ).expect("parse should succeed");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("uint_neg.tlbx");
    doc.compile(&path, false).expect("compile should succeed with coercion");
    let reader = Reader::open(&path).expect("reader");
    let items = reader.get("items").expect("items");
    if let Value::Array(arr) = items {
        if let Value::Object(obj) = &arr[0] {
            let val = obj.get("val").expect("val field");
            assert_eq!(val, &Value::UInt(0), "negative uint8 should coerce to 0");
        } else { panic!("expected object"); }
    } else { panic!("expected array"); }
}

// =========================================================================
// Binary corruption tests (Item 3)
// =========================================================================

/// Helper: produce valid binary bytes from a TeaLeaf text input
fn make_valid_binary(tl_text: &str, compress: bool) -> Vec<u8> {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("valid.tlbx");
    let doc = TeaLeaf::parse(tl_text).expect("parse");
    doc.compile(&path, compress).expect("compile");
    std::fs::read(&path).expect("read")
}

#[test]
fn reader_corrupted_magic_byte() {
    let mut data = make_valid_binary("val: 42", false);
    data[0] ^= 0xFF; // flip first magic byte
    assert!(Reader::from_bytes(data).is_err());
}

#[test]
fn reader_corrupted_string_table_offset() {
    let mut data = make_valid_binary("val: 42", false);
    // String table offset is at bytes 24-31 in the header
    // Point it past the end of file
    let bad_offset = (data.len() as u64 + 1000).to_le_bytes();
    data[24..32].copy_from_slice(&bad_offset);
    let result = Reader::from_bytes(data);
    // Should error or at least not panic
    if let Ok(r) = result {
        let _ = r.get("val"); // should not panic
    }
}

#[test]
fn reader_truncated_string_table() {
    let data = make_valid_binary("greeting: \"hello world\"", false);
    // Truncate right after the header (64 bytes), cutting into string table
    let truncated = data[..65.min(data.len())].to_vec();
    let result = Reader::from_bytes(truncated);
    if let Ok(r) = result {
        let _ = r.get("greeting"); // should not panic
    }
}

#[test]
fn reader_oversized_string_count() {
    let mut data = make_valid_binary("val: 42", false);
    // String count is at header offset 48-51
    data[48..52].copy_from_slice(&u32::MAX.to_le_bytes());
    let result = Reader::from_bytes(data);
    // Should error (can't allocate u32::MAX strings) or handle gracefully
    if let Ok(r) = result {
        let _ = r.get("val"); // should not panic
    }
}

#[test]
fn reader_oversized_section_count() {
    let mut data = make_valid_binary("val: 42", false);
    // Section count is at header offset 56-59
    data[56..60].copy_from_slice(&u32::MAX.to_le_bytes());
    let result = Reader::from_bytes(data);
    if let Ok(r) = result {
        let _ = r.get("val"); // should not panic
    }
}

#[test]
fn reader_corrupted_schema_count() {
    let data_with_schema = make_valid_binary(
        "@struct point (x: int, y: int)\npts: @table point [(1,2)]\n",
        false,
    );
    let mut data = data_with_schema;
    // Schema count is at header offset 52-55
    data[52..56].copy_from_slice(&u32::MAX.to_le_bytes());
    let result = Reader::from_bytes(data);
    if let Ok(r) = result {
        let _ = r.get("pts"); // should not panic
    }
}

#[test]
fn reader_flipped_bytes_in_section_data() {
    let mut data = make_valid_binary("val: 42\nname: \"hello\"", false);
    // Flip some bytes near the end (likely in section data area)
    let len = data.len();
    if len > 70 {
        for i in (len - 10)..len {
            data[i] ^= 0xFF;
        }
    }
    let result = Reader::from_bytes(data);
    // Should not panic; may error or return wrong data
    if let Ok(r) = result {
        let _ = r.get("val");
        let _ = r.get("name");
    }
}

#[test]
fn reader_truncated_compressed_data() {
    // Create large enough data to trigger compression
    let mut items = String::new();
    for i in 0..200 {
        if !items.is_empty() { items.push_str(", "); }
        items.push_str(&i.to_string());
    }
    let tl = format!("nums: [{}]", items);
    let data = make_valid_binary(&tl, true);
    // Truncate 20 bytes off the end
    if data.len() > 84 {
        let truncated = data[..data.len() - 20].to_vec();
        let result = Reader::from_bytes(truncated);
        if let Ok(r) = result {
            let _ = r.get("nums"); // should not panic
        }
    }
}

#[test]
fn reader_invalid_zlib_stream() {
    let mut items = String::new();
    for i in 0..200 {
        if !items.is_empty() { items.push_str(", "); }
        items.push_str(&i.to_string());
    }
    let tl = format!("nums: [{}]", items);
    let mut data = make_valid_binary(&tl, true);
    // Replace data section (after header + string table + schema table + index) with garbage
    let len = data.len();
    if len > 100 {
        for i in (len - 30)..len {
            data[i] = 0xBA;
        }
    }
    let result = Reader::from_bytes(data);
    if let Ok(r) = result {
        let _ = r.get("nums"); // should not panic even with corrupted zlib
    }
}

#[test]
fn reader_zero_length_file() {
    assert!(Reader::from_bytes(vec![]).is_err());
}

#[test]
fn reader_just_magic_no_header() {
    assert!(Reader::from_bytes(b"TLBX".to_vec()).is_err());
}

#[test]
fn reader_corrupted_type_code() {
    let mut data = make_valid_binary("val: 42", false);
    // Find the data section and corrupt the type code byte in the index
    // Index entries start after string table + schema table; type byte is at offset +18 in each entry
    // For simplicity, just flip all bytes >= 0xFE to invalid type codes in the last quarter
    let len = data.len();
    let quarter = len * 3 / 4;
    for i in quarter..len {
        if data[i] < 0x40 && data[i] > 0 {
            data[i] = 0xFE; // invalid type code
            break;
        }
    }
    let result = Reader::from_bytes(data);
    if let Ok(r) = result {
        let _ = r.get("val"); // should not panic
    }
}

// =========================================================================
// Compression stress tests (Item 6)
// =========================================================================

#[test]
fn compression_at_threshold_boundary() {
    // Data just over 64 bytes — should trigger compression attempt
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("compress_boundary.tlbx");
    // Create a string that produces >64 bytes of section data
    let doc = TeaLeaf::parse("data: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"")
        .expect("parse");
    doc.compile(&path, true).expect("compile");
    // Verify round-trip
    let reader = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
    let val = reader.get("data").unwrap();
    assert!(val.as_str().unwrap().starts_with("aaaa"));
}

#[test]
fn compression_skipped_when_not_beneficial() {
    // High-entropy data — compression won't help
    let dir = tempfile::tempdir().expect("tmpdir");
    let path_compressed = dir.path().join("high_entropy_c.tlbx");
    let path_raw = dir.path().join("high_entropy_r.tlbx");

    // Build a string of pseudo-random chars (high entropy)
    let mut s = String::new();
    for i in 0..200 {
        s.push((33 + (i * 7 + 13) % 94) as u8 as char);
    }
    let tl = format!("data: \"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    let doc = TeaLeaf::parse(&tl).expect("parse");
    doc.compile(&path_compressed, true).expect("compile compressed");
    doc.compile(&path_raw, false).expect("compile raw");

    // With high entropy, compressed file should be similar size to raw
    let c_size = std::fs::metadata(&path_compressed).unwrap().len();
    let r_size = std::fs::metadata(&path_raw).unwrap().len();
    // Compressed should not be more than 10% larger than raw (compression was skipped if not beneficial)
    assert!(c_size <= r_size + r_size / 10,
        "Compressed {} should not be much larger than raw {}", c_size, r_size);

    // Both should round-trip correctly
    let rc = Reader::from_bytes(std::fs::read(&path_compressed).unwrap()).unwrap();
    let rr = Reader::from_bytes(std::fs::read(&path_raw).unwrap()).unwrap();
    assert_eq!(rc.get("data").unwrap(), rr.get("data").unwrap());
}

#[test]
fn compression_all_identical_bytes() {
    // Highly compressible: 10K identical integers (all zeros)
    // Uses array data which lives in section data (not string table), so compression applies
    let dir = tempfile::tempdir().expect("tmpdir");
    let path_c = dir.path().join("all_same_c.tlbx");
    let path_r = dir.path().join("all_same_r.tlbx");
    let mut items = String::new();
    for i in 0..10_000 {
        if i > 0 { items.push_str(", "); }
        items.push('0');
    }
    let tl = format!("data: [{}]", items);
    let doc = TeaLeaf::parse(&tl).expect("parse");
    doc.compile(&path_c, true).expect("compile compressed");
    doc.compile(&path_r, false).expect("compile raw");

    let c_size = std::fs::metadata(&path_c).unwrap().len();
    let r_size = std::fs::metadata(&path_r).unwrap().len();
    // Compressed file should be significantly smaller than raw (all zeros compress very well)
    assert!(c_size < r_size / 2,
        "Compressed size {} should be less than half of raw size {}", c_size, r_size);

    // Verify round-trip
    let reader = Reader::from_bytes(std::fs::read(&path_c).unwrap()).unwrap();
    let val = reader.get("data").unwrap();
    if let Value::Array(a) = val {
        assert_eq!(a.len(), 10_000);
        assert_eq!(a[0].as_int(), Some(0));
    } else { panic!("Expected array"); }
}

#[test]
fn compression_below_threshold_stored_raw() {
    // Small data with compress=true — stored raw because data < 64 bytes
    let dir = tempfile::tempdir().expect("tmpdir");
    let path_c = dir.path().join("small_c.tlbx");
    let path_r = dir.path().join("small_r.tlbx");

    let doc = TeaLeaf::parse("x: 42").expect("parse");
    doc.compile(&path_c, true).expect("compile compressed");
    doc.compile(&path_r, false).expect("compile raw");

    // Files should be identical (compression skipped for small data)
    let c_bytes = std::fs::read(&path_c).unwrap();
    let r_bytes = std::fs::read(&path_r).unwrap();
    // Data sections are the same, only flags byte in header differs
    assert_eq!(c_bytes.len(), r_bytes.len(),
        "Small data with compress=true should be same size as raw");
}

// =========================================================================
// Large-corpus / soak tests (Item 7)
// =========================================================================

#[test]
fn soak_deeply_nested_arrays() {
    // Build 200-level nested array: [[[...[1]...]]]
    // (1000 levels overflows the stack in debug mode)
    let mut tl = "root: ".to_string();
    for _ in 0..200 {
        tl.push('[');
    }
    tl.push('1');
    for _ in 0..200 {
        tl.push(']');
    }
    let doc = TeaLeaf::parse(&tl).expect("parse deep nesting");
    assert!(doc.get("root").is_some());
}

#[test]
fn soak_wide_object() {
    // Object with 10,000 fields
    let mut tl = String::from("data: {\n");
    for i in 0..10_000 {
        if i > 0 { tl.push_str(",\n"); }
        tl.push_str(&format!("    field_{}: {}", i, i));
    }
    tl.push_str("\n}\n");
    let doc = TeaLeaf::parse(&tl).expect("parse wide object");
    let obj = doc.get("data").expect("data");
    assert!(matches!(obj, Value::Object(_)));
    if let Value::Object(m) = obj {
        assert_eq!(m.len(), 10_000);
        assert_eq!(m.get("field_0").unwrap().as_int(), Some(0));
        assert_eq!(m.get("field_9999").unwrap().as_int(), Some(9999));
    }
}

#[test]
fn soak_large_array() {
    // Array with 100,000 integer elements
    let mut items = String::new();
    for i in 0..100_000 {
        if i > 0 { items.push_str(", "); }
        items.push_str(&i.to_string());
    }
    let tl = format!("nums: [{}]", items);
    let doc = TeaLeaf::parse(&tl).expect("parse large array");
    let arr = doc.get("nums").expect("nums");
    if let Value::Array(a) = arr {
        assert_eq!(a.len(), 100_000);
        assert_eq!(a[0].as_int(), Some(0));
        assert_eq!(a[99_999].as_int(), Some(99_999));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn soak_large_array_binary_roundtrip() {
    // 100K integers through binary compile + read
    let mut items = String::new();
    for i in 0..100_000u64 {
        if i > 0 { items.push_str(", "); }
        items.push_str(&i.to_string());
    }
    let tl = format!("nums: [{}]", items);
    let doc = TeaLeaf::parse(&tl).expect("parse");

    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("large_array.tlbx");
    doc.compile(&path, true).expect("compile");

    let reader = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
    let val = reader.get("nums").unwrap();
    if let Value::Array(a) = val {
        assert_eq!(a.len(), 100_000);
        assert_eq!(a[0].as_int(), Some(0));
        assert_eq!(a[99_999].as_int(), Some(99_999));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn soak_many_sections() {
    // 5,000 top-level sections
    let mut tl = String::new();
    for i in 0..5_000 {
        tl.push_str(&format!("key_{}: {}\n", i, i));
    }
    let doc = TeaLeaf::parse(&tl).expect("parse many sections");

    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("many_sections.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
    let keys = reader.keys();
    assert_eq!(keys.len(), 5_000);
    assert_eq!(reader.get("key_0").unwrap().as_int(), Some(0));
    assert_eq!(reader.get("key_4999").unwrap().as_int(), Some(4999));
}

#[test]
fn soak_many_schemas() {
    // 500 schemas, each with 3 fields
    let mut tl = String::new();
    for i in 0..500 {
        tl.push_str(&format!(
            "@struct Schema{} (id: int, name: string, value: float)\n",
            i
        ));
    }
    // Add one table for the first schema
    tl.push_str("items: @table Schema0 [(1, \"test\", 3.14)]\n");

    let doc = TeaLeaf::parse(&tl).expect("parse many schemas");

    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("many_schemas.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
    // Should have parsed 500 schemas
    assert_eq!(reader.schemas.len(), 500);
    let items = reader.get("items").unwrap();
    if let Value::Array(a) = items {
        assert_eq!(a.len(), 1);
    }
}

#[test]
fn soak_string_deduplication() {
    // 15,000 string values: 10K unique + 5K duplicates
    let mut tl = String::from("data: {\n");
    for i in 0..15_000 {
        if i > 0 { tl.push_str(",\n"); }
        let str_val = format!("str_{}", i % 10_000); // 5K will be duplicates
        tl.push_str(&format!("    key_{}: \"{}\"", i, str_val));
    }
    tl.push_str("\n}\n");

    let doc = TeaLeaf::parse(&tl).expect("parse string dedup");

    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("string_dedup.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
    let val = reader.get("data").unwrap();
    if let Value::Object(m) = val {
        assert_eq!(m.len(), 15_000);
        // Verify a deduplicated string reads correctly
        assert_eq!(m.get("key_10000").unwrap().as_str(), Some("str_0"));
    } else {
        panic!("Expected object");
    }
}

#[test]
fn soak_long_string() {
    // 1MB string
    let big_str = "A".repeat(1_000_000);
    let tl = format!("data: \"{}\"", big_str);
    let doc = TeaLeaf::parse(&tl).expect("parse long string");

    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("long_string.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
    let val = reader.get("data").unwrap();
    assert_eq!(val.as_str().unwrap().len(), 1_000_000);
}

// =========================================================================
// Memory-mapped reader tests
// =========================================================================

#[test]
fn mmap_roundtrip_all_primitive_types() {
    let tl = r#"
int_val: 42
neg_int: -999
large_int: 9223372036854775807
float_val: 3.14159
bool_true: true
bool_false: false
str_val: "hello mmap"
ts_val: 2024-01-15T10:30:00Z
"#;
    let doc = TeaLeaf::parse(tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_primitives.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::open_mmap(&path).expect("open_mmap");
    assert_eq!(reader.get("int_val").unwrap().as_int(), Some(42));
    assert_eq!(reader.get("neg_int").unwrap().as_int(), Some(-999));
    assert_eq!(reader.get("large_int").unwrap().as_int(), Some(i64::MAX));
    assert!((reader.get("float_val").unwrap().as_float().unwrap() - 3.14159).abs() < 1e-10);
    assert_eq!(reader.get("bool_true").unwrap().as_bool(), Some(true));
    assert_eq!(reader.get("bool_false").unwrap().as_bool(), Some(false));
    assert_eq!(reader.get("str_val").unwrap().as_str(), Some("hello mmap"));
    assert!(reader.get("ts_val").unwrap().as_timestamp().is_some());
}

#[test]
fn mmap_roundtrip_containers() {
    let tl = r#"
arr: [1, 2, 3, "four", true]
obj: {name: "Alice", age: 30, active: true}
nested: [[1, 2], [3, 4]]
"#;
    let doc = TeaLeaf::parse(tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_containers.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::open_mmap(&path).expect("open_mmap");

    if let Value::Array(a) = reader.get("arr").unwrap() {
        assert_eq!(a.len(), 5);
        assert_eq!(a[0].as_int(), Some(1));
        assert_eq!(a[3].as_str(), Some("four"));
        assert_eq!(a[4].as_bool(), Some(true));
    } else { panic!("Expected array"); }

    if let Value::Object(m) = reader.get("obj").unwrap() {
        assert_eq!(m.get("name").unwrap().as_str(), Some("Alice"));
        assert_eq!(m.get("age").unwrap().as_int(), Some(30));
    } else { panic!("Expected object"); }

    if let Value::Array(outer) = reader.get("nested").unwrap() {
        assert_eq!(outer.len(), 2);
        if let Value::Array(inner) = &outer[0] {
            assert_eq!(inner.len(), 2);
            assert_eq!(inner[0].as_int(), Some(1));
        }
    } else { panic!("Expected nested array"); }
}

#[test]
fn mmap_roundtrip_schemas() {
    let tl = r#"
@struct user (id: int, name: string, active: bool)
users: @table user [
    (1, "Alice", true),
    (2, "Bob", false),
    (3, "Charlie", true)
]
"#;
    let doc = TeaLeaf::parse(tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_schemas.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::open_mmap(&path).expect("open_mmap");
    assert!(!reader.schemas.is_empty());
    if let Value::Array(rows) = reader.get("users").unwrap() {
        assert_eq!(rows.len(), 3);
        if let Value::Object(row) = &rows[0] {
            assert_eq!(row.get("id").unwrap().as_int(), Some(1));
            assert_eq!(row.get("name").unwrap().as_str(), Some("Alice"));
            assert_eq!(row.get("active").unwrap().as_bool(), Some(true));
        } else { panic!("Expected object row"); }
    } else { panic!("Expected array"); }
}

#[test]
fn mmap_roundtrip_compressed() {
    let mut items = String::new();
    for i in 0..500 {
        if i > 0 { items.push_str(", "); }
        items.push_str(&i.to_string());
    }
    let tl = format!("nums: [{}]", items);
    let doc = TeaLeaf::parse(&tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_compressed.tlbx");
    doc.compile(&path, true).expect("compile");

    let reader = Reader::open_mmap(&path).expect("open_mmap");
    if let Value::Array(a) = reader.get("nums").unwrap() {
        assert_eq!(a.len(), 500);
        assert_eq!(a[0].as_int(), Some(0));
        assert_eq!(a[499].as_int(), Some(499));
    } else { panic!("Expected array"); }
}

#[test]
fn mmap_vs_open_equivalence() {
    let tl = r#"
@struct point (x: float, y: float)
name: "equivalence test"
count: 12345
items: [1, "two", 3.0, null, true]
pts: @table point [(1.0, 2.0), (3.0, 4.0)]
obj: {a: 1, b: "hello"}
"#;
    let doc = TeaLeaf::parse(tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_equiv.tlbx");
    doc.compile(&path, false).expect("compile");

    let r_mmap = Reader::open_mmap(&path).expect("open_mmap");
    let r_open = Reader::open(&path).expect("open");

    let mut keys_mmap = r_mmap.keys().to_vec();
    let mut keys_open = r_open.keys().to_vec();
    keys_mmap.sort();
    keys_open.sort();
    assert_eq!(keys_mmap, keys_open);

    for key in r_mmap.keys() {
        assert_eq!(
            r_mmap.get(key).unwrap(),
            r_open.get(key).unwrap(),
            "Mismatch for key '{}'", key
        );
    }

    assert_eq!(r_mmap.schemas.len(), r_open.schemas.len());
}

#[test]
fn mmap_vs_from_bytes_equivalence() {
    let tl = "val: 42\ntext: \"hello world\"\narr: [1, 2, 3]";
    let doc = TeaLeaf::parse(tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_vs_bytes.tlbx");
    doc.compile(&path, false).expect("compile");

    let bytes = std::fs::read(&path).unwrap();
    let r_mmap = Reader::open_mmap(&path).expect("open_mmap");
    let r_bytes = Reader::from_bytes(bytes).expect("from_bytes");

    for key in r_mmap.keys() {
        assert_eq!(
            r_mmap.get(key).unwrap(),
            r_bytes.get(key).unwrap(),
            "Mismatch for key '{}'", key
        );
    }
}

#[test]
fn mmap_large_file() {
    let mut items = String::new();
    for i in 0..50_000u64 {
        if i > 0 { items.push_str(", "); }
        items.push_str(&i.to_string());
    }
    let tl = format!("big: [{}]", items);
    let doc = TeaLeaf::parse(&tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_large.tlbx");
    doc.compile(&path, true).expect("compile");

    let reader = Reader::open_mmap(&path).expect("open_mmap");
    if let Value::Array(a) = reader.get("big").unwrap() {
        assert_eq!(a.len(), 50_000);
        assert_eq!(a[0].as_int(), Some(0));
        assert_eq!(a[49_999].as_int(), Some(49_999));
    } else { panic!("Expected array"); }
}

#[test]
fn mmap_nonexistent_file() {
    let result = Reader::open_mmap("definitely_does_not_exist_xyz.tlbx");
    assert!(result.is_err());
}

#[test]
fn mmap_multiple_sections() {
    let mut tl = String::new();
    for i in 0..100 {
        tl.push_str(&format!("sec_{}: {}\n", i, i * 10));
    }
    let doc = TeaLeaf::parse(&tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_sections.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::open_mmap(&path).expect("open_mmap");
    assert_eq!(reader.keys().len(), 100);
    assert_eq!(reader.get("sec_0").unwrap().as_int(), Some(0));
    assert_eq!(reader.get("sec_50").unwrap().as_int(), Some(500));
    assert_eq!(reader.get("sec_99").unwrap().as_int(), Some(990));
    assert!(reader.get("sec_100").is_err());
}

#[test]
fn mmap_string_dedup() {
    let mut tl = String::from("data: {\n");
    for i in 0..100 {
        if i > 0 { tl.push_str(",\n"); }
        tl.push_str(&format!("    key_{}: \"shared_value\"", i));
    }
    tl.push_str("\n}\n");
    let doc = TeaLeaf::parse(&tl).expect("parse");
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("mmap_dedup.tlbx");
    doc.compile(&path, false).expect("compile");

    let reader = Reader::open_mmap(&path).expect("open_mmap");
    if let Value::Object(m) = reader.get("data").unwrap() {
        assert_eq!(m.len(), 100);
        assert_eq!(m.get("key_0").unwrap().as_str(), Some("shared_value"));
        assert_eq!(m.get("key_99").unwrap().as_str(), Some("shared_value"));
    } else { panic!("Expected object"); }
}
