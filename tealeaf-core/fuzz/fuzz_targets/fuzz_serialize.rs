#![no_main]
use libfuzzer_sys::fuzz_target;
use tealeaf::{Value, FormatOptions};

/// Deep equality for Values. Handles NaN (via to_bits) and compares
/// objects by key set (not insertion order).
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
        // JsonNumber coercion: may roundtrip as Int/UInt if it fits
        (Value::JsonNumber(a), Value::JsonNumber(b)) => a == b,
        (Value::JsonNumber(s), Value::Int(i)) => s.parse::<i64>().ok() == Some(*i),
        (Value::Int(i), Value::JsonNumber(s)) => s.parse::<i64>().ok() == Some(*i),
        (Value::JsonNumber(s), Value::UInt(u)) => s.parse::<u64>().ok() == Some(*u),
        (Value::UInt(u), Value::JsonNumber(s)) => s.parse::<u64>().ok() == Some(*u),
        _ => false,
    }
}

/// Like values_equal but also accepts Float ↔ Int/UInt coercion for whole-number floats.
/// Used to validate compact_floats roundtrips where re-parsing produces Int for values like 42.0.
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
        // Float ↔ Int/UInt coercion: compact_floats strips .0, re-parsing produces Int
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

fuzz_target!(|data: &str| {
    // Parse text format
    let tl = match tealeaf::TeaLeaf::parse(data) {
        Ok(tl) => tl,
        Err(_) => return,
    };

    // Serialize to JSON — must not panic
    let _ = tl.to_json();
    let _ = tl.to_json_compact();

    // Serialize to TL text and re-parse
    let tl_text = tl.to_tl_with_schemas();
    let original = match tealeaf::loads(data) {
        Ok(d) => d,
        Err(_) => return,
    };
    let reparsed = match tealeaf::loads(&tl_text) {
        Ok(d) => d,
        Err(_) => {
            panic!("Re-parse of to_tl_with_schemas output failed");
        }
    };

    // Invariant: text roundtrip must preserve all keys and values
    assert_eq!(
        original.len(),
        reparsed.len(),
        "text roundtrip key count mismatch: {} vs {}",
        original.len(),
        reparsed.len(),
    );
    for (key, orig_val) in &original {
        match reparsed.get(key) {
            Some(re_val) => {
                assert!(
                    values_equal(orig_val, re_val),
                    "text roundtrip value mismatch for key '{}'",
                    key,
                );
            }
            None => {
                panic!("text roundtrip lost key '{}'", key);
            }
        }
    }

    // ---- Compact text roundtrip ----
    let compact_text = tl.to_tl_with_options(&FormatOptions::compact());
    let compact_reparsed = match tealeaf::loads(&compact_text) {
        Ok(d) => d,
        Err(_) => {
            panic!("Re-parse of compact to_tl_with_options output failed");
        }
    };
    assert_eq!(
        original.len(),
        compact_reparsed.len(),
        "compact text roundtrip key count mismatch",
    );
    for (key, orig_val) in &original {
        match compact_reparsed.get(key) {
            Some(re_val) => {
                assert!(
                    values_equal(orig_val, re_val),
                    "compact text roundtrip value mismatch for key '{}'",
                    key,
                );
            }
            None => {
                panic!("compact text roundtrip lost key '{}'", key);
            }
        }
    }

    // ---- Compact floats text roundtrip ----
    // compact_floats strips .0 from whole-number floats, so re-parsing may
    // produce Int instead of Float. Use values_numeric_equal for comparison.
    let cf_text = tl.to_tl_with_options(&FormatOptions::compact().with_compact_floats());
    let cf_reparsed = match tealeaf::loads(&cf_text) {
        Ok(d) => d,
        Err(_) => {
            panic!("Re-parse of compact_floats to_tl_with_options output failed");
        }
    };
    assert_eq!(
        original.len(),
        cf_reparsed.len(),
        "compact_floats text roundtrip key count mismatch",
    );
    for (key, orig_val) in &original {
        match cf_reparsed.get(key) {
            Some(re_val) => {
                assert!(
                    values_numeric_equal(orig_val, re_val),
                    "compact_floats text roundtrip value mismatch for key '{}'",
                    key,
                );
            }
            None => {
                panic!("compact_floats text roundtrip lost key '{}'", key);
            }
        }
    }
});
