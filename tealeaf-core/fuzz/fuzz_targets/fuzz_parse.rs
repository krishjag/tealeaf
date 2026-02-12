#![no_main]
use libfuzzer_sys::fuzz_target;
use tealeaf::{Value, FormatOptions};

/// Deep equality for Values. Handles NaN (via to_bits) and compares
/// objects by key set (not insertion order, which schemas may reorder).
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
        // Int/UInt coercion: writer may encode a positive Int as UInt or vice versa
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

fuzz_target!(|data: &str| {
    // Parse fuzzer input
    let tl = match tealeaf::TeaLeaf::parse(data) {
        Ok(tl) => tl,
        Err(_) => return,
    };

    // Serialize back to text and re-parse
    let serialized = tealeaf::dumps(&tl.data);
    let reparsed = match tealeaf::TeaLeaf::parse(&serialized) {
        Ok(r) => r,
        Err(_) => {
            panic!("Re-parse of dumps() output failed");
        }
    };

    // Invariant: key count must match
    assert_eq!(
        tl.data.len(), reparsed.data.len(),
        "parse roundtrip key count mismatch: {} vs {}",
        tl.data.len(), reparsed.data.len(),
    );

    // Invariant: every original value must survive the roundtrip
    for (key, orig_val) in &tl.data {
        match reparsed.data.get(key) {
            Some(re_val) => {
                assert!(
                    values_equal(orig_val, re_val),
                    "parse roundtrip value mismatch for key '{}'", key,
                );
            }
            None => {
                panic!("parse roundtrip lost key '{}'", key);
            }
        }
    }

    // ---- Compact text roundtrip ----
    let compact = tealeaf::dumps_with_options(&tl.data, &FormatOptions::compact());
    let compact_reparsed = match tealeaf::TeaLeaf::parse(&compact) {
        Ok(r) => r,
        Err(_) => {
            panic!("Re-parse of compact dumps output failed");
        }
    };
    assert_eq!(
        tl.data.len(), compact_reparsed.data.len(),
        "compact parse roundtrip key count mismatch",
    );
    for (key, orig_val) in &tl.data {
        match compact_reparsed.data.get(key) {
            Some(re_val) => {
                assert!(
                    values_equal(orig_val, re_val),
                    "compact parse roundtrip value mismatch for key '{}'", key,
                );
            }
            None => {
                panic!("compact parse roundtrip lost key '{}'", key);
            }
        }
    }

    // ---- Compact floats text roundtrip ----
    let cf = tealeaf::dumps_with_options(&tl.data, &FormatOptions::compact().with_compact_floats());
    let cf_reparsed = match tealeaf::TeaLeaf::parse(&cf) {
        Ok(r) => r,
        Err(_) => {
            panic!("Re-parse of compact_floats dumps output failed");
        }
    };
    assert_eq!(
        tl.data.len(), cf_reparsed.data.len(),
        "compact_floats parse roundtrip key count mismatch",
    );
    for (key, orig_val) in &tl.data {
        match cf_reparsed.data.get(key) {
            Some(re_val) => {
                assert!(
                    values_numeric_equal(orig_val, re_val),
                    "compact_floats parse roundtrip value mismatch for key '{}'", key,
                );
            }
            None => {
                panic!("compact_floats parse roundtrip lost key '{}'", key);
            }
        }
    }
});
