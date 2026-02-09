#![no_main]
use libfuzzer_sys::fuzz_target;
use tealeaf::Value;

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

fuzz_target!(|data: &str| {
    // Fuzz JSON import with schema inference
    let tl = match tealeaf::TeaLeaf::from_json_with_schemas(data) {
        Ok(tl) => tl,
        Err(_) => return,
    };

    // Exercise JSON serialization — must not panic
    let _ = tl.to_json();

    // Serialize to TL text with inferred schemas
    let tl_text = tl.to_tl_with_schemas();

    // Re-parse the emitted TL text — failure here means the serializer
    // produced output that the parser rejects, which is a bug.
    let reparsed = match tealeaf::TeaLeaf::parse(&tl_text) {
        Ok(r) => r,
        Err(_) => {
            panic!("Re-parse of to_tl_with_schemas output failed (from_json_with_schemas path)");
        }
    };

    // Invariant: roundtrip must preserve all keys and values.
    // Schemas may reorder fields within objects, so compare by key set.
    assert_eq!(
        tl.data.len(),
        reparsed.data.len(),
        "json-schema roundtrip key count mismatch: {} vs {}",
        tl.data.len(),
        reparsed.data.len(),
    );
    for (key, orig_val) in &tl.data {
        match reparsed.data.get(key) {
            Some(re_val) => {
                assert!(
                    values_equal(orig_val, re_val),
                    "json-schema roundtrip value mismatch for key '{}'",
                    key,
                );
            }
            None => {
                panic!("json-schema roundtrip lost key '{}'", key);
            }
        }
    }

    // Invariant: schema count must survive TL text roundtrip.
    // If the serializer emits @struct definitions, the parser must re-parse them.
    assert_eq!(
        tl.schemas.len(),
        reparsed.schemas.len(),
        "json-schema roundtrip schema count mismatch: {} vs {}",
        tl.schemas.len(),
        reparsed.schemas.len(),
    );

    // Invariant: union count must survive TL text roundtrip.
    assert_eq!(
        tl.unions.len(),
        reparsed.unions.len(),
        "json-schema roundtrip union count mismatch: {} vs {}",
        tl.unions.len(),
        reparsed.unions.len(),
    );

    // Invariant: @root-array directive must survive TL text roundtrip.
    // Check by re-emitting to TL text and comparing the directive presence.
    let reemitted = reparsed.to_tl_with_schemas();
    let has_root_array_orig = tl_text.contains("@root-array");
    let has_root_array_rt = reemitted.contains("@root-array");
    assert_eq!(
        has_root_array_orig, has_root_array_rt,
        "json-schema roundtrip @root-array directive mismatch",
    );
});
