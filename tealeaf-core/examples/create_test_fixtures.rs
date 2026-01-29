//! Creates test fixture files for cross-language testing
//!
//! Run with: cargo run --example create_test_fixtures
//!
//! These fixtures ensure FFI, .NET, and CLI all produce identical output
//! for the same binary input. Each fixture has a corresponding .json file
//! with the expected JSON output.

use tealeaf::{TeaLeaf, Value};
use std::collections::HashMap;
use std::fs;

fn main() {
    let fixtures_dir = std::path::Path::new("bindings/dotnet/TeaLeaf.Tests/fixtures");
    fs::create_dir_all(fixtures_dir).expect("Failed to create fixtures dir");

    // Create fixture with bytes
    create_bytes_fixture(fixtures_dir);

    // Create fixture with timestamp
    create_timestamp_fixture(fixtures_dir);

    // Create comprehensive fixture with ALL special types
    create_comprehensive_fixture(fixtures_dir);

    println!("Test fixtures created in {:?}", fixtures_dir);
    println!("\nTo verify cross-language parity:");
    println!("  1. Run .NET tests: dotnet test");
    println!("  2. Run CLI test: cargo run -- tojson fixtures/comprehensive.tlbx");
    println!("  3. Compare outputs match comprehensive.expected.json");
}

fn create_bytes_fixture(dir: &std::path::Path) {
    let mut data = HashMap::new();
    data.insert("binary_data".to_string(), Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]));
    data.insert("empty_bytes".to_string(), Value::Bytes(vec![]));
    data.insert("name".to_string(), Value::String("test".to_string()));

    let doc = TeaLeaf::new(HashMap::new(), data);

    let path = dir.join("bytes_test.tlbx");
    doc.compile(&path, false).expect("Failed to compile bytes fixture");
    println!("Created: {:?}", path);
}

fn create_timestamp_fixture(dir: &std::path::Path) {
    let mut data = HashMap::new();
    data.insert("created".to_string(), Value::Timestamp(1705315800000)); // 2024-01-15T10:30:00Z
    data.insert("epoch".to_string(), Value::Timestamp(0)); // 1970-01-01T00:00:00Z

    let doc = TeaLeaf::new(HashMap::new(), data);

    let path = dir.join("timestamp_test.tlbx");
    doc.compile(&path, false).expect("Failed to compile timestamp fixture");
    println!("Created: {:?}", path);
}

fn create_comprehensive_fixture(dir: &std::path::Path) {
    let mut data = HashMap::new();

    // Primitives
    data.insert("null_val".to_string(), Value::Null);
    data.insert("bool_true".to_string(), Value::Bool(true));
    data.insert("bool_false".to_string(), Value::Bool(false));
    data.insert("int_val".to_string(), Value::Int(42));
    data.insert("int_neg".to_string(), Value::Int(-123));
    data.insert("uint_val".to_string(), Value::UInt(999));
    data.insert("float_val".to_string(), Value::Float(3.14159));
    data.insert("string_val".to_string(), Value::String("hello world".to_string()));

    // Bytes (special - can't be created from text)
    data.insert("bytes_val".to_string(), Value::Bytes(vec![0xca, 0xfe, 0xba, 0xbe]));
    data.insert("bytes_empty".to_string(), Value::Bytes(vec![]));

    // Timestamp (special - milliseconds since epoch)
    data.insert("timestamp_val".to_string(), Value::Timestamp(1705315800000));

    // Array
    data.insert("array_val".to_string(), Value::Array(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ]));

    // Object
    data.insert("object_val".to_string(), Value::Object(
        vec![
            ("name".to_string(), Value::String("alice".to_string())),
            ("age".to_string(), Value::Int(30)),
        ].into_iter().collect()
    ));

    // Ref (special - reference to another key)
    data.insert("ref_val".to_string(), Value::Ref("object_val".to_string()));

    // Tagged (special - tagged union style value)
    data.insert("tagged_val".to_string(), Value::Tagged(
        "ok".to_string(),
        Box::new(Value::Int(200))
    ));

    // Map (special - non-string keys)
    data.insert("map_val".to_string(), Value::Map(vec![
        (Value::Int(1), Value::String("one".to_string())),
        (Value::Int(2), Value::String("two".to_string())),
    ]));

    let doc = TeaLeaf::new(HashMap::new(), data);

    let path = dir.join("comprehensive.tlbx");
    doc.compile(&path, false).expect("Failed to compile comprehensive fixture");
    println!("Created: {:?}", path);

    // Also create expected JSON output for verification
    let expected_json = r#"{
  "null_val": null,
  "bool_true": true,
  "bool_false": false,
  "int_val": 42,
  "int_neg": -123,
  "uint_val": 999,
  "float_val": 3.14159,
  "string_val": "hello world",
  "bytes_val": "0xcafebabe",
  "bytes_empty": "0x",
  "timestamp_val": "2024-01-15T10:30:00.000Z",
  "array_val": [1, 2, 3],
  "object_val": {"name": "alice", "age": 30},
  "ref_val": {"$ref": "object_val"},
  "tagged_val": {"$tag": "ok", "$value": 200},
  "map_val": [[1, "one"], [2, "two"]]
}"#;

    let json_path = dir.join("comprehensive.expected.json");
    fs::write(&json_path, expected_json).expect("Failed to write expected JSON");
    println!("Created: {:?}", json_path);
}
