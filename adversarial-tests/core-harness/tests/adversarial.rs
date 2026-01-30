use tealeaf::{Reader, TeaLeaf};

fn assert_parse_err(input: &str) {
    assert!(TeaLeaf::parse(input).is_err());
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
fn parse_number_overflow() {
    assert_parse_err("big: 18446744073709551616");
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
fn from_json_large_number_falls_to_float() {
    let doc = TeaLeaf::from_json("{\"big\": 18446744073709551616}").expect("from_json");
    let value = doc.get("big").expect("big");
    assert!(matches!(value, tealeaf::Value::Float(_)));
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
