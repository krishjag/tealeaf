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

fn compile_read_and_check(tl: &tealeaf::TeaLeaf, path: &std::path::Path, compress: bool) {
    if tl.compile(path, compress).is_err() {
        return;
    }
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return,
    };
    let reader = match tealeaf::Reader::from_bytes(bytes) {
        Ok(r) => r,
        Err(_) => {
            // Writer produced output that Reader rejects — that's a bug
            panic!("Reader failed to load writer-produced binary (compress={})", compress);
        }
    };

    // Invariant: every key the reader returns must match the original value
    let reader_keys = reader.keys();
    for key in &reader_keys {
        let reader_val = match reader.get(key) {
            Ok(v) => v,
            Err(_) => {
                panic!("Reader.get failed for key '{}' in writer-produced binary", key);
            }
        };
        if let Some(original) = tl.get(key) {
            assert!(
                values_equal(original, &reader_val),
                "binary roundtrip value mismatch for key '{}' (compress={})",
                key, compress,
            );
        } else {
            panic!(
                "reader returned key '{}' not in original (compress={})",
                key, compress,
            );
        }
    }

    // Reverse invariant: every original key must exist in reader output
    // (catches writer bugs that silently drop sections)
    for key in tl.data.keys() {
        if !reader_keys.iter().any(|k| k == key) {
            panic!(
                "binary roundtrip dropped key '{}' (compress={})",
                key, compress,
            );
        }
    }
}

fuzz_target!(|data: &str| {
    let tl = match tealeaf::TeaLeaf::parse(data) {
        Ok(tl) => tl,
        Err(_) => return,
    };

    let tmp = match tempfile::NamedTempFile::new() {
        Ok(t) => t,
        Err(_) => return,
    };
    let path = tmp.path().to_path_buf();

    compile_read_and_check(&tl, &path, false);
    compile_read_and_check(&tl, &path, true);
});
