#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Need at least some bytes for both document and path
    if data.len() < 4 {
        return;
    }

    // Use first byte as split point ratio
    let split = (data[0] as usize * data.len()) / 256;
    let split = split.clamp(1, data.len() - 1);

    let doc_bytes = &data[..split];
    let path_bytes = &data[split..];

    // Parse document from first portion
    let doc_str = match std::str::from_utf8(doc_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };
    let tl = match tealeaf::TeaLeaf::parse(doc_str) {
        Ok(tl) => tl,
        Err(_) => return,
    };

    // Build path string from second portion
    let path = match std::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Exercise TeaLeaf::get_path — must never panic
    let _ = tl.get_path(path);

    // Also exercise Value::get_path on each top-level value
    for (_key, value) in &tl.data {
        let _ = value.get_path(path);
    }
});
