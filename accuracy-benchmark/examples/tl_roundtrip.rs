//! Roundtrip TeaLeaf: JSON -> TL (compact) -> JSON, then compare.
//!
//! Usage: cargo run -p accuracy-benchmark --example tl_roundtrip -- <original.json> <file.tl>

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <original.json> <file.tl>", args[0]);
        std::process::exit(1);
    }

    let original_path = &args[1];
    let tl_path = &args[2];

    // Load original JSON
    let original_str = std::fs::read_to_string(original_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", original_path, e));
    let original: serde_json::Value = serde_json::from_str(&original_str)
        .unwrap_or_else(|e| panic!("Failed to parse original JSON: {}", e));

    // Load TL and convert back to JSON via tealeaf-core
    let tl_str = std::fs::read_to_string(tl_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", tl_path, e));
    let doc = tealeaf::TeaLeaf::parse(&tl_str)
        .unwrap_or_else(|e| panic!("Failed to parse TL: {}", e));
    let rt_json_str = doc.to_json()
        .unwrap_or_else(|e| panic!("Failed to convert TL to JSON: {}", e));
    let roundtripped: serde_json::Value = serde_json::from_str(&rt_json_str)
        .unwrap_or_else(|e| panic!("Failed to parse roundtripped JSON: {}", e));

    // Compare
    if original == roundtripped {
        println!("PASS: roundtripped JSON is identical to original");
    } else {
        println!("DIFF: roundtripped JSON differs from original\n");
        let mut diff_count = 0;
        report_diffs(&original, &roundtripped, "$", &mut diff_count);
        println!("\nTotal differences: {}", diff_count);

        // Write roundtripped JSON for manual inspection
        let rt_path = tl_path.replace(".tl", "_roundtripped.json");
        let rt_json = serde_json::to_string_pretty(&roundtripped).unwrap();
        std::fs::write(&rt_path, &rt_json).unwrap();
        println!("Roundtripped JSON written to: {}", rt_path);
    }
}

/// Walk two JSON values and report where they diverge.
fn report_diffs(a: &serde_json::Value, b: &serde_json::Value, path: &str, count: &mut usize) {
    use serde_json::Value::*;
    match (a, b) {
        (Object(ma), Object(mb)) => {
            for key in ma.keys().chain(mb.keys()).collect::<std::collections::BTreeSet<_>>() {
                let p = format!("{}.{}", path, key);
                match (ma.get(key), mb.get(key)) {
                    (Some(va), Some(vb)) => report_diffs(va, vb, &p, count),
                    (Some(_), None) => { *count += 1; println!("  MISSING in roundtrip: {}", p); }
                    (None, Some(_)) => { *count += 1; println!("  EXTRA in roundtrip:   {}", p); }
                    (None, None) => {}
                }
            }
        }
        (Array(va), Array(vb)) => {
            if va.len() != vb.len() {
                *count += 1;
                println!("  LENGTH MISMATCH at {}: original={}, roundtripped={}", path, va.len(), vb.len());
            }
            for (i, (ea, eb)) in va.iter().zip(vb.iter()).enumerate() {
                report_diffs(ea, eb, &format!("{}[{}]", path, i), count);
            }
        }
        (Number(na), Number(nb)) => {
            let fa = na.as_f64().unwrap_or(f64::NAN);
            let fb = nb.as_f64().unwrap_or(f64::NAN);
            if (fa - fb).abs() > 1e-6 * fa.abs().max(1.0) {
                *count += 1;
                println!("  NUMBER DIFF at {}: {} vs {}", path, na, nb);
            } else if na != nb {
                println!("  NUMBER REPR at {}: {} vs {} (values equal)", path, na, nb);
            }
        }
        _ => {
            if a != b {
                *count += 1;
                let a_s = format!("{}", a);
                let b_s = format!("{}", b);
                let a_short = if a_s.len() > 80 { &a_s[..80] } else { &a_s };
                let b_short = if b_s.len() > 80 { &b_s[..80] } else { &b_s };
                println!("  DIFF at {}: {} vs {}", path, a_short, b_short);
            }
        }
    }
}
