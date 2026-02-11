//! Convert a JSON file to TeaLeaf compact and TOON formats.
//!
//! Usage: cargo run -p accuracy-benchmark --example convert_formats -- <input.json> <output_dir>

use accuracy_benchmark::tasks::{convert_json_to_tl, convert_json_to_toon};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input.json> <output_dir>", args[0]);
        std::process::exit(1);
    }

    let input = &args[1];
    let output_dir = std::path::Path::new(&args[2]);

    let json_str = std::fs::read_to_string(input)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", input, e));

    let stem = std::path::Path::new(input)
        .file_stem()
        .unwrap()
        .to_string_lossy();

    // TeaLeaf compact
    let tl = convert_json_to_tl(&json_str).expect("TL conversion failed");
    let tl_path = output_dir.join(format!("{}.tl", stem));
    std::fs::write(&tl_path, &tl).expect("Failed to write TL file");
    println!("Wrote {} ({} bytes)", tl_path.display(), tl.len());

    // TOON
    let json_val: serde_json::Value =
        serde_json::from_str(&json_str).expect("JSON parse failed");
    let toon = convert_json_to_toon(&json_val).expect("TOON conversion failed");
    let toon_path = output_dir.join(format!("{}.toon", stem));
    std::fs::write(&toon_path, &toon).expect("Failed to write TOON file");
    println!("Wrote {} ({} bytes)", toon_path.display(), toon.len());

    // Also report JSON size for comparison
    println!("JSON source: {} bytes", json_str.len());
}
