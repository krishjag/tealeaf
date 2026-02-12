#![no_main]
use arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use tealeaf::{Value, IndexMap, FormatOptions};

// Valid ISO 8601 4-digit year range. format_timestamp_millis clamps to this
// range, so generated timestamps must stay within it for text roundtrips.
const MIN_TS: i64 = -62_167_219_200_000;   // 0000-01-01T00:00:00Z
const MAX_TS: i64 = 253_402_300_799_999;    // 9999-12-31T23:59:59.999Z

/// Generate an arbitrary Value from fuzzer bytes, with bounded depth
/// to prevent stack overflow on deeply nested structures.
fn arbitrary_value(u: &mut Unstructured<'_>, depth: usize) -> arbitrary::Result<Value> {
    if depth == 0 {
        // At max depth, only produce leaf values
        return arbitrary_leaf(u);
    }

    let variant: u8 = u.int_in_range(0..=13)?;
    match variant {
        0 => Ok(Value::Null),
        1 => Ok(Value::Bool(u.arbitrary()?)),
        2 => Ok(Value::Int(u.arbitrary()?)),
        3 => Ok(Value::UInt(u.arbitrary()?)),
        4 => {
            let f: f64 = u.arbitrary()?;
            // Avoid NaN/Inf which have special roundtrip behavior
            if f.is_finite() {
                Ok(Value::Float(f))
            } else {
                Ok(Value::Float(0.0))
            }
        }
        5 => Ok(Value::String(arbitrary_safe_string(u)?)),
        6 => {
            let len: usize = u.int_in_range(0..=8)?;
            let bytes: Vec<u8> = (0..len).map(|_| u.arbitrary()).collect::<arbitrary::Result<_>>()?;
            Ok(Value::Bytes(bytes))
        }
        7 => Ok(Value::Timestamp(u.int_in_range(MIN_TS..=MAX_TS)?, 0)),
        8 => {
            // JsonNumber — generate a valid numeric string
            Ok(Value::JsonNumber(arbitrary_json_number(u)?))
        }
        9 => {
            // Array
            let len: usize = u.int_in_range(0..=4)?;
            let arr: Vec<Value> = (0..len)
                .map(|_| arbitrary_value(u, depth - 1))
                .collect::<arbitrary::Result<_>>()?;
            Ok(Value::Array(arr))
        }
        10 => {
            // Object
            let len: usize = u.int_in_range(0..=4)?;
            let mut obj = IndexMap::new();
            for _ in 0..len {
                let key = arbitrary_key(u)?;
                let val = arbitrary_value(u, depth - 1)?;
                obj.insert(key, val);
            }
            Ok(Value::Object(obj))
        }
        11 => {
            // Ref
            Ok(Value::Ref(arbitrary_identifier(u)?))
        }
        12 => {
            // Tagged
            let tag = arbitrary_identifier(u)?;
            let inner = arbitrary_value(u, depth - 1)?;
            Ok(Value::Tagged(tag, Box::new(inner)))
        }
        13 => {
            // Map (ordered key-value pairs; keys restricted to string | name | integer per spec)
            let len: usize = u.int_in_range(0..=4)?;
            let mut pairs = Vec::with_capacity(len);
            for _ in 0..len {
                let k = arbitrary_map_key(u)?;
                let v = arbitrary_value(u, depth - 1)?;
                pairs.push((k, v));
            }
            Ok(Value::Map(pairs))
        }
        _ => Ok(Value::Null),
    }
}

/// Generate only leaf (non-recursive) values
fn arbitrary_leaf(u: &mut Unstructured<'_>) -> arbitrary::Result<Value> {
    let variant: u8 = u.int_in_range(0..=7)?;
    match variant {
        0 => Ok(Value::Null),
        1 => Ok(Value::Bool(u.arbitrary()?)),
        2 => Ok(Value::Int(u.arbitrary()?)),
        3 => Ok(Value::UInt(u.arbitrary()?)),
        4 => {
            let f: f64 = u.arbitrary()?;
            Ok(Value::Float(if f.is_finite() { f } else { 0.0 }))
        }
        5 => Ok(Value::String(arbitrary_safe_string(u)?)),
        6 => Ok(Value::Timestamp(u.int_in_range(MIN_TS..=MAX_TS)?, 0)),
        7 => Ok(Value::JsonNumber(arbitrary_json_number(u)?)),
        _ => Ok(Value::Null),
    }
}

/// Generate a string that's safe for TL serialization (no control chars that break parsing)
fn arbitrary_safe_string(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    let len: usize = u.int_in_range(0..=32)?;
    let mut s = String::with_capacity(len);
    for _ in 0..len {
        // Printable ASCII range plus some common chars
        let c: u8 = u.int_in_range(0x20..=0x7E)?;
        s.push(c as char);
    }
    Ok(s)
}

/// Generate a valid identifier for tags/refs (starts with letter, alphanumeric + underscore)
fn arbitrary_identifier(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    let len: usize = u.int_in_range(1..=12)?;
    let mut s = String::with_capacity(len);
    // First char must be alphabetic
    let first: u8 = u.int_in_range(0..=25)?;
    s.push((b'a' + first) as char);
    for _ in 1..len {
        let variant: u8 = u.int_in_range(0..=2)?;
        match variant {
            0 => { let c: u8 = u.int_in_range(0..=25)?; s.push((b'a' + c) as char); }
            1 => { let c: u8 = u.int_in_range(0..=9)?; s.push((b'0' + c) as char); }
            2 => s.push('_'),
            _ => {}
        }
    }
    Ok(s)
}

/// Edge-case strings that require quoting in TL text format
const EDGE_CASE_KEYS: &[&str] = &[
    "",             // empty key
    "true",         // reserved word
    "false",        // reserved word
    "null",         // reserved word
    "~",            // null literal
    "NaN",          // special float
    "inf",          // special float
    "Infinity",     // special float
    "-inf",         // special float
    "0x1F",         // hex prefix
    "0b101",        // binary prefix
    "123abc",       // leading digit
    "hello world",  // space
    "key:value",    // colon
    "@struct",      // directive prefix
    "!ref",         // ref prefix
    "#tag",         // tag prefix
    "a\"b",         // embedded quote
];

/// Generate a @map key per spec grammar: `map_key = string | name | integer`
fn arbitrary_map_key(u: &mut Unstructured<'_>) -> arbitrary::Result<Value> {
    let variant: u8 = u.int_in_range(0..=3)?;
    match variant {
        0 => Ok(Value::String(arbitrary_safe_string(u)?)),
        1 => Ok(Value::String(arbitrary_identifier(u)?)),
        2 => Ok(Value::Int(u.arbitrary()?)),
        3 => Ok(Value::UInt(u.arbitrary()?)),
        _ => Ok(Value::String(arbitrary_identifier(u)?)),
    }
}

/// Generate a key for object fields — mostly valid identifiers, sometimes edge cases
fn arbitrary_key(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    let variant: u8 = u.int_in_range(0..=2)?;
    match variant {
        // ~60% normal identifiers
        0 | 1 => arbitrary_identifier(u),
        // ~33% edge-case keys that exercise quoting logic
        2 => {
            let idx: usize = u.int_in_range(0..=(EDGE_CASE_KEYS.len() - 1))?;
            Ok(EDGE_CASE_KEYS[idx].to_string())
        }
        _ => arbitrary_identifier(u),
    }
}

/// Generate a valid JSON number string
fn arbitrary_json_number(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    let variant: u8 = u.int_in_range(0..=2)?;
    match variant {
        0 => {
            // Large integer (overflows u64)
            let digits: usize = u.int_in_range(20..=40)?;
            let mut s = String::with_capacity(digits);
            // First digit non-zero
            let d: u8 = u.int_in_range(1..=9)?;
            s.push((b'0' + d) as char);
            for _ in 1..digits {
                let d: u8 = u.int_in_range(0..=9)?;
                s.push((b'0' + d) as char);
            }
            Ok(s)
        }
        1 => {
            // Huge exponent float (overflows f64)
            let base: u8 = u.int_in_range(1..=9)?;
            let exp: u16 = u.int_in_range(309..=999)?;
            Ok(format!("{}e{}", base, exp))
        }
        2 => {
            // Negative large integer
            let digits: usize = u.int_in_range(20..=40)?;
            let mut s = String::from("-");
            let d: u8 = u.int_in_range(1..=9)?;
            s.push((b'0' + d) as char);
            for _ in 1..digits {
                let d: u8 = u.int_in_range(0..=9)?;
                s.push((b'0' + d) as char);
            }
            Ok(s)
        }
        _ => Ok("99999999999999999999".to_string()),
    }
}

/// Deep equality for Values (same as other fuzz targets)
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::UInt(a), Value::UInt(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bytes(a), Value::Bytes(b)) => a == b,
        (Value::Timestamp(a, a_tz), Value::Timestamp(b, b_tz)) => a == b && a_tz == b_tz,
        (Value::Ref(a), Value::Ref(b)) => a == b,
        (Value::Tagged(ta, va), Value::Tagged(tb, vb)) => ta == tb && values_equal(va, vb),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| values_equal(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter().all(|(k, v)| b.get(k).map_or(false, |bv| values_equal(v, bv)))
        }
        (Value::Map(a), Value::Map(b)) => {
            a.len() == b.len()
                && a.iter().zip(b).all(|((ak, av), (bk, bv))| {
                    values_equal(ak, bk) && values_equal(av, bv)
                })
        }
        (Value::Int(a), Value::UInt(b)) => *a >= 0 && *a as u64 == *b,
        (Value::UInt(a), Value::Int(b)) => *b >= 0 && *a == *b as u64,
        (Value::JsonNumber(a), Value::JsonNumber(b)) => a == b,
        (Value::JsonNumber(s), Value::Int(i)) => s.parse::<i64>().ok() == Some(*i),
        (Value::Int(i), Value::JsonNumber(s)) => s.parse::<i64>().ok() == Some(*i),
        (Value::JsonNumber(s), Value::UInt(u)) => s.parse::<u64>().ok() == Some(*u),
        (Value::UInt(u), Value::JsonNumber(s)) => s.parse::<u64>().ok() == Some(*u),
        _ => false,
    }
}

/// Like values_equal but also accepts Float ↔ Int/UInt coercion for whole-number floats.
fn values_numeric_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::UInt(a), Value::UInt(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bytes(a), Value::Bytes(b)) => a == b,
        (Value::Timestamp(a, a_tz), Value::Timestamp(b, b_tz)) => a == b && a_tz == b_tz,
        (Value::Ref(a), Value::Ref(b)) => a == b,
        (Value::Tagged(ta, va), Value::Tagged(tb, vb)) => ta == tb && values_numeric_equal(va, vb),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| values_numeric_equal(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter().all(|(k, v)| b.get(k).map_or(false, |bv| values_numeric_equal(v, bv)))
        }
        (Value::Map(a), Value::Map(b)) => {
            a.len() == b.len()
                && a.iter().zip(b).all(|((ak, av), (bk, bv))| {
                    values_numeric_equal(ak, bk) && values_numeric_equal(av, bv)
                })
        }
        (Value::Int(a), Value::UInt(b)) => *a >= 0 && *a as u64 == *b,
        (Value::UInt(a), Value::Int(b)) => *b >= 0 && *a == *b as u64,
        (Value::Float(f), Value::Int(i)) => {
            f.is_finite() && *f == f.trunc() && *f == *i as f64
        }
        (Value::Int(i), Value::Float(f)) => {
            f.is_finite() && *f == f.trunc() && *f == *i as f64
        }
        (Value::Float(f), Value::UInt(u)) => {
            f.is_finite() && *f >= 0.0 && *f == f.trunc() && *f == *u as f64
        }
        (Value::UInt(u), Value::Float(f)) => {
            f.is_finite() && *f >= 0.0 && *f == f.trunc() && *f == *u as f64
        }
        (Value::JsonNumber(a), Value::JsonNumber(b)) => a == b,
        (Value::JsonNumber(s), Value::Int(i)) => s.parse::<i64>().ok() == Some(*i),
        (Value::Int(i), Value::JsonNumber(s)) => s.parse::<i64>().ok() == Some(*i),
        (Value::JsonNumber(s), Value::UInt(u)) => s.parse::<u64>().ok() == Some(*u),
        (Value::UInt(u), Value::JsonNumber(s)) => s.parse::<u64>().ok() == Some(*u),
        _ => false,
    }
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    // Generate 1-4 key-value pairs
    let num_keys: usize = match u.int_in_range(1..=4) {
        Ok(n) => n,
        Err(_) => return,
    };
    let mut kvs = IndexMap::new();
    for _ in 0..num_keys {
        let key = match arbitrary_key(&mut u) {
            Ok(k) => k,
            Err(_) => return,
        };
        let val = match arbitrary_value(&mut u, 3) {
            Ok(v) => v,
            Err(_) => return,
        };
        kvs.insert(key, val);
    }

    // Build a TeaLeaf document from generated data
    let tl = tealeaf::TeaLeaf::new(IndexMap::new(), kvs.clone());

    // ---- Test 1: Text serialize → re-parse roundtrip ----
    let text = tealeaf::dumps(&tl.data);
    let reparsed = match tealeaf::TeaLeaf::parse(&text) {
        Ok(r) => r,
        Err(e) => {
            panic!("Re-parse of dumps() output failed for structured input.\nError: {}\nText:\n{}\n", e, text);
        }
    };

    assert_eq!(
        tl.data.len(), reparsed.data.len(),
        "structured text roundtrip key count mismatch"
    );
    for (key, orig_val) in &tl.data {
        match reparsed.data.get(key) {
            Some(re_val) => {
                assert!(
                    values_equal(orig_val, re_val),
                    "structured text roundtrip value mismatch for key '{}'", key,
                );
            }
            None => {
                panic!("structured text roundtrip lost key '{}'", key);
            }
        }
    }

    // ---- Test 1b: Compact text serialize → re-parse roundtrip ----
    let compact_text = tealeaf::dumps_with_options(&tl.data, &FormatOptions::compact());
    let compact_reparsed = match tealeaf::TeaLeaf::parse(&compact_text) {
        Ok(r) => r,
        Err(e) => {
            panic!("Re-parse of compact dumps() output failed for structured input.\nError: {}\nText:\n{}\n", e, compact_text);
        }
    };
    assert_eq!(
        tl.data.len(), compact_reparsed.data.len(),
        "structured compact text roundtrip key count mismatch"
    );
    for (key, orig_val) in &tl.data {
        match compact_reparsed.data.get(key) {
            Some(re_val) => {
                assert!(
                    values_equal(orig_val, re_val),
                    "structured compact text roundtrip value mismatch for key '{}'", key,
                );
            }
            None => {
                panic!("structured compact text roundtrip lost key '{}'", key);
            }
        }
    }

    // ---- Test 1c: Compact floats text serialize → re-parse roundtrip ----
    let cf_text = tealeaf::dumps_with_options(&tl.data, &FormatOptions::compact().with_compact_floats());
    let cf_reparsed = match tealeaf::TeaLeaf::parse(&cf_text) {
        Ok(r) => r,
        Err(e) => {
            panic!("Re-parse of compact_floats dumps() output failed for structured input.\nError: {}\nText:\n{}\n", e, cf_text);
        }
    };
    assert_eq!(
        tl.data.len(), cf_reparsed.data.len(),
        "structured compact_floats text roundtrip key count mismatch"
    );
    for (key, orig_val) in &tl.data {
        match cf_reparsed.data.get(key) {
            Some(re_val) => {
                assert!(
                    values_numeric_equal(orig_val, re_val),
                    "structured compact_floats text roundtrip value mismatch for key '{}'", key,
                );
            }
            None => {
                panic!("structured compact_floats text roundtrip lost key '{}'", key);
            }
        }
    }

    // ---- Test 2: Binary compile → read roundtrip ----
    let tmp = match tempfile::NamedTempFile::new() {
        Ok(t) => t,
        Err(_) => return,
    };
    let path = tmp.path().to_path_buf();

    // Structure-aware inputs are intentionally bounded and spec-conformant.
    // If compile() rejects any of these values, that's a writer bug — surface it.
    tl.compile(&path, false).unwrap_or_else(|e| {
        panic!("compile() failed on structured input: {}\nText:\n{}", e, text);
    });

    let reader = match tealeaf::Reader::open(&path) {
        Ok(r) => r,
        Err(_) => {
            panic!("Reader::open failed on compiled output from structured input");
        }
    };
    for (key, orig_val) in &tl.data {
        match reader.get(key) {
            Ok(re_val) => {
                assert!(
                    values_equal(orig_val, &re_val),
                    "structured binary roundtrip value mismatch for key '{}'", key,
                );
            }
            Err(_) => {
                panic!("structured binary roundtrip failed to read key '{}'", key);
            }
        }
    }

    // ---- Test 3: JSON serialization (no-panic check) ----
    // JSON can't represent all TL types (Bytes, Ref, Tagged, Timestamp, Map)
    // faithfully, so we only verify serialization doesn't panic, and that
    // re-import of successfully serialized JSON also doesn't panic.
    if let Ok(json) = tl.to_json() {
        let _ = tealeaf::TeaLeaf::from_json(&json);
    }
    let _ = tl.to_json_compact();
});
