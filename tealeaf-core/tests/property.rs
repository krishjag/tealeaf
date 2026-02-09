//! Property-based tests for TeaLeaf using proptest

use proptest::prelude::*;
use std::collections::HashMap;
use tealeaf::{TeaLeaf, Value, Reader, IndexMap};

// =========================================================================
// Value generation strategies
// =========================================================================

/// Generate a leaf Value (no containers)
#[allow(dead_code)]
fn arb_leaf_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::Int),
        any::<u64>().prop_map(Value::UInt),
        // Only finite floats (NaN/Inf don't round-trip through text)
        any::<f64>()
            .prop_filter("finite only", |f| f.is_finite())
            .prop_map(Value::Float),
        "[a-zA-Z0-9_ ]{0,50}".prop_map(|s| Value::String(s)),
        prop::collection::vec(any::<u8>(), 0..20).prop_map(Value::Bytes),
        // Timestamps: reasonable range
        (-1_000_000_000_000i64..1_000_000_000_000i64).prop_map(|ts| Value::Timestamp(ts, 0)),
    ]
}

/// Generate a Value with optional nesting (max depth 2)
#[allow(dead_code)]
fn arb_value(depth: u32) -> BoxedStrategy<Value> {
    if depth == 0 {
        arb_leaf_value().boxed()
    } else {
        prop_oneof![
            4 => arb_leaf_value(),
            1 => prop::collection::vec(arb_value(depth - 1), 0..5)
                .prop_map(Value::Array),
            1 => prop::collection::hash_map(
                    "[a-z]{1,10}",
                    arb_value(depth - 1),
                    0..5
                ).prop_map(|hm: HashMap<String, Value>| Value::Object(hm.into_iter().collect())),
        ]
        .boxed()
    }
}

// =========================================================================
// Property: Text round-trip (Value -> dumps -> parse -> Value)
// =========================================================================

/// Only test values that survive text round-trip cleanly.
/// Excludes: UInt (text parser produces Int), Bytes (need b"..." syntax),
/// Timestamp (need ISO format), Float precision issues
fn arb_text_roundtrip_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        // Restrict to i32 range to avoid representation ambiguity
        (-2_000_000_000i64..2_000_000_000i64).prop_map(Value::Int),
        "[a-zA-Z0-9_ ]{0,30}".prop_map(|s| Value::String(s)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn text_roundtrip_simple(value in arb_text_roundtrip_value()) {
        let key = "test_key";
        // Build TeaLeaf doc with the value
        let mut data = IndexMap::new();
        data.insert(key.to_string(), value.clone());
        let doc = TeaLeaf::new(IndexMap::new(), data);
        let text = doc.to_tl_with_schemas();

        // Parse back
        let doc2 = TeaLeaf::parse(&text).expect("re-parse");
        let got = doc2.get(key).expect("get key back");
        match (&value, got) {
            (Value::Null, Value::Null) => {}
            (Value::Bool(a), Value::Bool(b)) => prop_assert_eq!(a, b),
            (Value::Int(a), Value::Int(b)) => prop_assert_eq!(a, b),
            (Value::String(a), Value::String(b)) => prop_assert_eq!(a, b),
            _ => prop_assert!(false, "Type mismatch: expected {:?}, got {:?}", value, got),
        }
    }
}

// =========================================================================
// Property: Binary round-trip (Value -> compile -> read -> Value)
// =========================================================================

/// Values that survive binary round-trip (most types work)
fn arb_binary_roundtrip_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::Int),
        any::<u64>().prop_map(Value::UInt),
        any::<f64>()
            .prop_filter("finite only", |f| f.is_finite())
            .prop_map(Value::Float),
        "[a-zA-Z0-9_ ]{0,30}".prop_map(|s| Value::String(s)),
        prop::collection::vec(any::<u8>(), 0..20).prop_map(Value::Bytes),
        (-1_000_000_000_000i64..1_000_000_000_000i64).prop_map(|ts| Value::Timestamp(ts, 0)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn binary_roundtrip(value in arb_binary_roundtrip_value()) {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("prop_test.tlbx");

        let mut data = IndexMap::new();
        data.insert("val".to_string(), value.clone());
        let doc = TeaLeaf::new(IndexMap::new(), data);
        doc.compile(&path, false).expect("compile");

        let reader = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let got = reader.get("val").unwrap();

        match (&value, &got) {
            (Value::Null, Value::Null) => {}
            (Value::Bool(a), Value::Bool(b)) => prop_assert_eq!(a, b),
            (Value::Int(a), Value::Int(b)) => prop_assert_eq!(a, b),
            (Value::UInt(a), Value::UInt(b)) => prop_assert_eq!(a, b),
            (Value::Float(a), Value::Float(b)) => {
                prop_assert!((a - b).abs() < f64::EPSILON || (a.is_nan() && b.is_nan()),
                    "Float mismatch: {} vs {}", a, b);
            }
            (Value::String(a), Value::String(b)) => prop_assert_eq!(a, b),
            (Value::Bytes(a), Value::Bytes(b)) => prop_assert_eq!(a, b),
            (Value::Timestamp(a, a_tz), Value::Timestamp(b, b_tz)) => { prop_assert_eq!(a, b); prop_assert_eq!(a_tz, b_tz); },
            _ => prop_assert!(false,
                "Type mismatch: expected {:?}, got {:?}", value, got),
        }
    }
}

// =========================================================================
// Property: Integer value accessor consistency
// =========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn int_as_int_roundtrip(i in any::<i64>()) {
        let v = Value::Int(i);
        prop_assert_eq!(v.as_int(), Some(i));
    }

    #[test]
    fn uint_as_uint_roundtrip(u in any::<u64>()) {
        let v = Value::UInt(u);
        prop_assert_eq!(v.as_uint(), Some(u));
    }

    #[test]
    fn uint_as_int_overflow(u in (i64::MAX as u64 + 1)..=u64::MAX) {
        let v = Value::UInt(u);
        // UInt values larger than i64::MAX should not return from as_int
        prop_assert_eq!(v.as_int(), None);
    }
}

// =========================================================================
// Property: Binary round-trip with arrays of same-type values
// =========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn binary_roundtrip_int_array(ints in prop::collection::vec(any::<i64>(), 0..100)) {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("prop_arr.tlbx");

        let arr = Value::Array(ints.iter().map(|&i| Value::Int(i)).collect());
        let mut data = IndexMap::new();
        data.insert("arr".to_string(), arr);
        let doc = TeaLeaf::new(IndexMap::new(), data);
        doc.compile(&path, false).expect("compile");

        let reader = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let got = reader.get("arr").unwrap();
        if let Value::Array(items) = got {
            prop_assert_eq!(items.len(), ints.len());
            for (i, item) in items.iter().enumerate() {
                prop_assert_eq!(item.as_int(), Some(ints[i]),
                    "Mismatch at index {}", i);
            }
        } else {
            prop_assert!(false, "Expected array, got {:?}", got);
        }
    }
}
