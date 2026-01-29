//! TeaLeaf CLI

use std::env;
use std::process;

use tealeaf::{TeaLeaf, Reader};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let result = match args[1].as_str() {
        "compile" => cmd_compile(&args[2..]),
        "decompile" => cmd_decompile(&args[2..]),
        "info" => cmd_info(&args[2..]),
        "validate" => cmd_validate(&args[2..]),
        "to-json" => cmd_to_json(&args[2..]),
        "from-json" => cmd_from_json(&args[2..]),
        "tlbx-to-json" => cmd_tlbx_to_json(&args[2..]),
        "json-to-tlbx" => cmd_json_to_tlbx(&args[2..]),
        "help" | "--help" | "-h" => { print_usage(); Ok(()) }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn print_usage() {
    println!("TeaLeaf v2.0 - Schema-aware document format");
    println!();
    println!("Usage: tealeaf <command> [options]");
    println!();
    println!("Commands:");
    println!("  compile <input.tl> -o <output.tlbx>       Compile text to binary");
    println!("  decompile <input.tlbx> -o <output.tl>     Decompile binary to text");
    println!("  info <file.tl|file.tlbx>                  Show file info (auto-detects format)");
    println!("  validate <file.tl>                        Validate text format");
    println!();
    println!("JSON Conversion:");
    println!("  to-json <input.tl> [-o <output.json>]     Convert TeaLeaf text to JSON");
    println!("  from-json <input.json> -o <output.tl>     Convert JSON to TeaLeaf text");
    println!("  tlbx-to-json <input.tlbx> [-o <out.json>] Convert TeaLeaf binary to JSON");
    println!("  json-to-tlbx <input.json> -o <out.tlbx>   Convert JSON to TeaLeaf binary");
    println!();
    println!("  help                                      Show this help");
}

fn cmd_compile(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 || args[1] != "-o" {
        eprintln!("Usage: tealeaf compile <input.tl> -o <output.tlbx>");
        process::exit(1);
    }

    let input = &args[0];
    let output = &args[2];

    println!("Compiling {} -> {}", input, output);

    let doc = TeaLeaf::load(input)?;
    doc.compile(output, true)?;
    
    let in_size = std::fs::metadata(input)?.len();
    let out_size = std::fs::metadata(output)?.len();
    let ratio = out_size as f64 / in_size as f64 * 100.0;
    
    println!("Done: {} bytes -> {} bytes ({:.1}%)", in_size, out_size, ratio);
    Ok(())
}

fn cmd_decompile(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 || args[1] != "-o" {
        eprintln!("Usage: tealeaf decompile <input.tlbx> -o <output.tl>");
        process::exit(1);
    }

    let input = &args[0];
    let output = &args[2];

    println!("Decompiling {} -> {}", input, output);
    
    let reader = Reader::open(input)?;
    let mut out = String::new();
    
    // Write schemas
    for schema in &reader.schemas {
        out.push_str("@struct ");
        out.push_str(&schema.name);
        out.push_str(" (");
        for (i, field) in schema.fields.iter().enumerate() {
            if i > 0 { out.push_str(", "); }
            out.push_str(&field.name);
            out.push_str(": ");
            out.push_str(&format!("{}", field.field_type));
        }
        out.push_str(")\n");
    }
    out.push('\n');
    
    // Write data
    for key in reader.keys() {
        out.push_str(key);
        out.push_str(": ");
        let value = reader.get(key)?;
        write_value(&mut out, &value, 0);
        out.push('\n');
    }
    
    std::fs::write(output, out)?;
    println!("Done");
    Ok(())
}

fn cmd_info(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        eprintln!("Usage: tealeaf info <file.tl|file.tlbx>");
        process::exit(1);
    }

    let input = &args[0];
    let file_size = std::fs::metadata(input)?.len();

    // Auto-detect format: try reading first 4 bytes for magic
    let is_binary = {
        let mut file = std::fs::File::open(input)?;
        let mut magic = [0u8; 4];
        use std::io::Read;
        if file.read_exact(&mut magic).is_ok() {
            &magic == b"TLBX"
        } else {
            false
        }
    };

    println!("File: {}", input);
    println!("Size: {} bytes", file_size);

    if is_binary {
        println!("Format: Binary (.tlbx)");
        println!();

        let reader = Reader::open(input)?;

        println!("Schemas: {}", reader.schemas.len());
        for schema in &reader.schemas {
            println!("  {} ({} fields)", schema.name, schema.fields.len());
        }
        println!();
        println!("Sections: {}", reader.keys().len());
        for key in reader.keys() {
            println!("  {}", key);
        }
    } else {
        println!("Format: Text (.tl)");
        println!();

        match TeaLeaf::load(input) {
            Ok(doc) => {
                println!("Schemas: {}", doc.schemas.len());
                for (name, schema) in &doc.schemas {
                    println!("  {} ({} fields)", name, schema.fields.len());
                }
                println!();
                println!("Keys: {}", doc.data.len());
                for key in doc.data.keys() {
                    println!("  {}", key);
                }
            }
            Err(e) => {
                println!("Parse Error: {}", e);
                process::exit(1);
            }
        }
    }

    Ok(())
}

fn cmd_validate(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        eprintln!("Usage: tealeaf validate <file.tl>");
        process::exit(1);
    }

    let input = &args[0];

    match TeaLeaf::load(input) {
        Ok(doc) => {
            println!("✓ Valid");
            println!("  Schemas: {}", doc.schemas.len());
            println!("  Keys: {}", doc.data.len());
        }
        Err(e) => {
            println!("✗ Invalid: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

fn cmd_to_json(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        eprintln!("Usage: tealeaf to-json <input.tl> [-o <output.json>]");
        process::exit(1);
    }

    let input = &args[0];
    let output = if args.len() >= 3 && args[1] == "-o" {
        Some(&args[2])
    } else {
        None
    };

    let doc = TeaLeaf::load(input)?;
    let json = doc.to_json()?;

    match output {
        Some(path) => {
            std::fs::write(path, &json)?;
            println!("Converted {} -> {}", input, path);
        }
        None => {
            println!("{}", json);
        }
    }

    Ok(())
}

fn cmd_from_json(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 || args[1] != "-o" {
        eprintln!("Usage: tealeaf from-json <input.json> -o <output.tl>");
        process::exit(1);
    }

    let input = &args[0];
    let output = &args[2];

    println!("Converting {} -> {}", input, output);

    let json_content = std::fs::read_to_string(input)?;

    let doc = TeaLeaf::from_json_with_schemas(&json_content)?;
    if !doc.schemas.is_empty() {
        println!("Inferred {} schema(s):", doc.schemas.len());
        for (name, schema) in &doc.schemas {
            println!("  @struct {} ({} fields)", name, schema.fields.len());
        }
    }
    let tl_text = doc.to_tl_with_schemas();

    std::fs::write(output, tl_text)?;
    println!("Done");

    Ok(())
}

fn cmd_tlbx_to_json(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        eprintln!("Usage: tealeaf tlbx-to-json <input.tlbx> [-o <output.json>]");
        process::exit(1);
    }

    let input = &args[0];
    let output = if args.len() >= 3 && args[1] == "-o" {
        Some(&args[2])
    } else {
        None
    };

    let reader = Reader::open(input)?;

    // Build JSON object from all sections
    let mut json_obj = serde_json::Map::new();
    for key in reader.keys() {
        let value = reader.get(key)?;
        json_obj.insert(key.to_string(), value_to_json(&value));
    }

    let json = serde_json::to_string_pretty(&serde_json::Value::Object(json_obj))?;

    match output {
        Some(path) => {
            std::fs::write(path, &json)?;
            println!("Converted {} -> {}", input, path);
        }
        None => {
            println!("{}", json);
        }
    }

    Ok(())
}

fn cmd_json_to_tlbx(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 || args[1] != "-o" {
        eprintln!("Usage: tealeaf json-to-tlbx <input.json> -o <output.tlbx>");
        process::exit(1);
    }

    let input = &args[0];
    let output = &args[2];

    println!("Converting {} -> {}", input, output);

    let json_content = std::fs::read_to_string(input)?;
    let doc = TeaLeaf::from_json(&json_content)?;
    doc.compile(output, true)?;

    let in_size = std::fs::metadata(input)?.len();
    let out_size = std::fs::metadata(output)?.len();
    let ratio = out_size as f64 / in_size as f64 * 100.0;

    println!("Done: {} bytes -> {} bytes ({:.1}%)", in_size, out_size, ratio);
    Ok(())
}

/// Convert TeaLeaf Value to serde_json::Value
fn value_to_json(value: &tealeaf::Value) -> serde_json::Value {
    use tealeaf::Value;

    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::UInt(u) => serde_json::Value::Number((*u).into()),
        Value::Float(f) => {
            // Always output floats as floats - the type distinction is intentional
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => {
            // Encode as hex string
            let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
            serde_json::Value::String(format!("0x{}", hex))
        }
        Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(value_to_json).collect())
        }
        Value::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        Value::Map(pairs) => {
            // Convert to array of [key, value] pairs
            let arr: Vec<serde_json::Value> = pairs
                .iter()
                .map(|(k, v)| serde_json::Value::Array(vec![
                    value_to_json(k),
                    value_to_json(v)
                ]))
                .collect();
            serde_json::Value::Array(arr)
        }
        Value::Ref(r) => {
            let mut obj = serde_json::Map::new();
            obj.insert("$ref".to_string(), serde_json::Value::String(r.clone()));
            serde_json::Value::Object(obj)
        }
        Value::Tagged(tag, inner) => {
            let mut obj = serde_json::Map::new();
            obj.insert("$tag".to_string(), serde_json::Value::String(tag.clone()));
            obj.insert("$value".to_string(), value_to_json(inner));
            serde_json::Value::Object(obj)
        }
        Value::Timestamp(ts) => {
            // Convert to ISO 8601 string
            let secs = ts / 1000;
            let millis = ts % 1000;
            let days = secs / 86400;
            let time_secs = secs % 86400;
            let hours = time_secs / 3600;
            let mins = (time_secs % 3600) / 60;
            let secs_rem = time_secs % 60;

            let (year, month, day) = days_to_ymd(days as i32);

            let iso = if millis > 0 {
                format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                    year, month, day, hours, mins, secs_rem, millis)
            } else {
                format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    year, month, day, hours, mins, secs_rem)
            };
            serde_json::Value::String(iso)
        }
    }
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_ymd(days: i32) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn write_value(out: &mut String, value: &tealeaf::Value, indent: usize) {
    use tealeaf::Value;
    
    match value {
        Value::Null => out.push('~'),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::UInt(u) => out.push_str(&u.to_string()),
        Value::Float(f) => out.push_str(&f.to_string()),
        Value::String(s) => {
            if s.contains(|c: char| c.is_whitespace() || c == '"' || c == ',') {
                out.push('"');
                out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
                out.push('"');
            } else {
                out.push_str(s);
            }
        }
        Value::Bytes(b) => {
            out.push_str("0x");
            for byte in b {
                out.push_str(&format!("{:02x}", byte));
            }
        }
        Value::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                write_value(out, v, indent);
            }
            out.push(']');
        }
        Value::Object(obj) => {
            out.push('{');
            let mut first = true;
            for (k, v) in obj {
                if !first { out.push_str(", "); }
                first = false;
                out.push_str(k);
                out.push_str(": ");
                write_value(out, v, indent);
            }
            out.push('}');
        }
        Value::Ref(r) => {
            out.push('!');
            out.push_str(r);
        }
        Value::Tagged(tag, inner) => {
            out.push(':');
            out.push_str(tag);
            out.push(' ');
            write_value(out, inner, indent);
        }
        Value::Map(pairs) => {
            out.push_str("@map {");
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                write_value(out, k, indent);
                out.push_str(": ");
                write_value(out, v, indent);
            }
            out.push('}');
        }
        Value::Timestamp(ts) => {
            // Convert Unix millis to ISO 8601
            let millis = ts % 1000;
            let secs = ts / 1000;
            let days = (secs / 86400) as i32 + 719468;
            let era = if days >= 0 { days } else { days - 146096 } / 146097;
            let doe = (days - era * 146097) as u32;
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let y = yoe as i32 + era * 400;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            let year = if m <= 2 { y + 1 } else { y };
            let day_secs = (secs % 86400) as u32;
            let hour = day_secs / 3600;
            let min = (day_secs % 3600) / 60;
            let sec = day_secs % 60;
            if millis != 0 {
                out.push_str(&format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z", year, m, d, hour, min, sec, millis));
            } else {
                out.push_str(&format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, m, d, hour, min, sec));
            }
        }
    }
}
