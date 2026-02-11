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
        "--version" | "-V" => { println!("tealeaf {}", env!("CARGO_PKG_VERSION")); Ok(()) }
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
    println!("TeaLeaf v{} - Schema-aware data format", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage: tealeaf <command> [options]");
    println!();
    println!("Commands:");
    println!("  compile <input.tl> -o <output.tlbx>              Compile text to binary");
    println!("  decompile <input.tlbx> -o <output.tl> [--compact] Decompile binary to text");
    println!("  info <file.tl|file.tlbx>                         Show file info (auto-detects format)");
    println!("  validate <file.tl>                               Validate text format");
    println!();
    println!("JSON Conversion:");
    println!("  to-json <input.tl> [-o <output.json>]            Convert TeaLeaf text to JSON");
    println!("  from-json <input.json> -o <output.tl> [--compact] Convert JSON to TeaLeaf text");
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
        eprintln!("Usage: tealeaf decompile <input.tlbx> -o <output.tl> [--compact]");
        process::exit(1);
    }

    let input = &args[0];
    let output = &args[2];
    let compact = args.get(3).map_or(false, |a| a == "--compact");

    println!("Decompiling {} -> {}{}", input, output, if compact { " (compact)" } else { "" });

    let reader = Reader::open(input)?;
    let doc = TeaLeaf::from_reader(&reader)?;
    let tl_text = if compact {
        doc.to_tl_with_schemas_compact()
    } else {
        doc.to_tl_with_schemas()
    };

    std::fs::write(output, tl_text)?;
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
        if !reader.unions.is_empty() {
            println!("Unions: {}", reader.unions.len());
            for union_def in &reader.unions {
                println!("  {} ({} variants)", union_def.name, union_def.variants.len());
            }
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
                if !doc.unions.is_empty() {
                    println!("Unions: {}", doc.unions.len());
                    for (name, union_def) in &doc.unions {
                        println!("  {} ({} variants)", name, union_def.variants.len());
                    }
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
        eprintln!("Usage: tealeaf from-json <input.json> -o <output.tl> [--compact]");
        process::exit(1);
    }

    let input = &args[0];
    let output = &args[2];
    let compact = args.get(3).map_or(false, |a| a == "--compact");

    println!("Converting {} -> {}{}", input, output, if compact { " (compact)" } else { "" });

    let json_content = std::fs::read_to_string(input)?;

    let doc = TeaLeaf::from_json_with_schemas(&json_content)?;
    if !doc.schemas.is_empty() {
        println!("Inferred {} schema(s):", doc.schemas.len());
        for (name, schema) in &doc.schemas {
            println!("  @struct {} ({} fields)", name, schema.fields.len());
        }
    }
    let tl_text = if compact {
        doc.to_tl_with_schemas_compact()
    } else {
        doc.to_tl_with_schemas()
    };

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
    let doc = TeaLeaf::from_reader(&reader)?;
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

