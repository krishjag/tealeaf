#![no_main]
use libfuzzer_sys::fuzz_target;

/// Recursively walk a Value tree to force full decoding of nested structures.
fn walk(v: &tealeaf::Value) {
    match v {
        tealeaf::Value::Array(arr) => arr.iter().for_each(walk),
        tealeaf::Value::Object(obj) => obj.values().for_each(walk),
        tealeaf::Value::Map(pairs) => pairs.iter().for_each(|(k, v)| { walk(k); walk(v); }),
        tealeaf::Value::Tagged(_, inner) => walk(inner),
        _ => {}
    }
}

fuzz_target!(|data: &[u8]| {
    // Fuzz the binary reader — must never panic
    if let Ok(reader) = tealeaf::Reader::from_bytes(data.to_vec()) {
        for key in reader.keys() {
            if let Ok(val) = reader.get(key) {
                walk(&val);
            }
        }
    }
});
