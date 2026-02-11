//! Roundtrip TOON: JSON -> TOON -> JSON, then compare.
//!
//! Usage: cargo run -p accuracy-benchmark --example toon_roundtrip -- <original.json> <file.toon>

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <original.json> <file.toon>", args[0]);
        std::process::exit(1);
    }

    let original_path = &args[1];
    let toon_path = &args[2];

    // Load original JSON
    let original_str = std::fs::read_to_string(original_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", original_path, e));
    let original: serde_json::Value = serde_json::from_str(&original_str)
        .unwrap_or_else(|e| panic!("Failed to parse original JSON: {}", e));

    // Load TOON and decode back to JSON
    let toon_str = std::fs::read_to_string(toon_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", toon_path, e));
    // Try multiple decode strategies
    let roundtripped: serde_json::Value = match toon_format::decode_default(&toon_str) {
        Ok(v) => v,
        Err(e1) => {
            eprintln!("decode_default failed: {}", e1);
            eprintln!("Trying decode_no_coerce...");
            match toon_format::decode_no_coerce(&toon_str) {
                Ok(v) => v,
                Err(e2) => {
                    eprintln!("decode_no_coerce failed: {}", e2);
                    eprintln!("Trying decode_strict...");
                    toon_format::decode_strict(&toon_str)
                        .unwrap_or_else(|e3| panic!("All decode attempts failed.\n  default: {}\n  no_coerce: {}\n  strict: {}", e1, e2, e3))
                }
            }
        }
    };

    // Compare
    if original == roundtripped {
        println!("PASS: roundtripped JSON is identical to original");
    } else {
        println!("FAIL: roundtripped JSON differs from original\n");
        report_diffs(&original, &roundtripped, "$");

        // Write roundtripped JSON for manual inspection
        let rt_path = toon_path.replace(".toon", "_roundtripped.json");
        let rt_json = serde_json::to_string_pretty(&roundtripped).unwrap();
        std::fs::write(&rt_path, &rt_json).unwrap();
        println!("\nRoundtripped JSON written to: {}", rt_path);
    }
}

/// Walk two JSON values and report where they diverge.
fn report_diffs(a: &serde_json::Value, b: &serde_json::Value, path: &str) {
    use serde_json::Value::*;
    match (a, b) {
        (Object(ma), Object(mb)) => {
            for key in ma.keys().chain(mb.keys()).collect::<std::collections::BTreeSet<_>>() {
                let p = format!("{}.{}", path, key);
                match (ma.get(key), mb.get(key)) {
                    (Some(va), Some(vb)) => report_diffs(va, vb, &p),
                    (Some(_), None) => println!("  MISSING in roundtrip: {}", p),
                    (None, Some(_)) => println!("  EXTRA in roundtrip:   {}", p),
                    (None, None) => {}
                }
            }
        }
        (Array(va), Array(vb)) => {
            if va.len() != vb.len() {
                println!("  LENGTH MISMATCH at {}: original={}, roundtripped={}", path, va.len(), vb.len());
            }
            for (i, (ea, eb)) in va.iter().zip(vb.iter()).enumerate() {
                report_diffs(ea, eb, &format!("{}[{}]", path, i));
            }
        }
        (Number(na), Number(nb)) => {
            // Compare numerically: both as f64
            let fa = na.as_f64().unwrap_or(f64::NAN);
            let fb = nb.as_f64().unwrap_or(f64::NAN);
            if (fa - fb).abs() > 1e-6 * fa.abs().max(1.0) {
                println!("  NUMBER DIFF at {}: {} vs {}", path, na, nb);
            } else if na != nb {
                // Same value but different representation (e.g., 0 vs 0.0)
                println!("  NUMBER REPR at {}: {} vs {} (values equal)", path, na, nb);
            }
        }
        _ => {
            if a != b {
                let a_s = format!("{}", a);
                let b_s = format!("{}", b);
                let a_short = if a_s.len() > 80 { &a_s[..80] } else { &a_s };
                let b_short = if b_s.len() > 80 { &b_s[..80] } else { &b_s };
                println!("  DIFF at {}: {} vs {}", path, a_short, b_short);
            }
        }
    }
}
