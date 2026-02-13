//! TeaLeaf CLI

use std::path::{Path, PathBuf};
use std::process;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use tealeaf::{FormatOptions, Reader, TeaLeaf};

#[derive(Parser)]
#[command(
    name = "tealeaf",
    version,
    about = "Schema-aware data format with human-readable text and compact binary",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile text format (.tl) to binary (.tlbx)
    Compile {
        /// Input .tl file
        input: PathBuf,
        /// Output .tlbx file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Decompile binary (.tlbx) to text format (.tl)
    Decompile {
        /// Input .tlbx file
        input: PathBuf,
        /// Output .tl file
        #[arg(short, long)]
        output: PathBuf,
        /// Omit insignificant whitespace for token-efficient output
        #[arg(long)]
        compact: bool,
        /// Write whole-number floats as integers (e.g. 42.0 becomes 42)
        #[arg(long)]
        compact_floats: bool,
    },

    /// Show file info (auto-detects text/binary format)
    Info {
        /// Input .tl or .tlbx file
        input: PathBuf,
    },

    /// Validate a text format (.tl) file
    Validate {
        /// Input .tl file
        input: PathBuf,
    },

    /// Convert TeaLeaf text (.tl) to JSON
    ToJson {
        /// Input .tl file
        input: PathBuf,
        /// Output .json file (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert JSON to TeaLeaf text (.tl) with schema inference
    FromJson {
        /// Input .json file
        input: PathBuf,
        /// Output .tl file
        #[arg(short, long)]
        output: PathBuf,
        /// Omit insignificant whitespace for token-efficient output
        #[arg(long)]
        compact: bool,
        /// Write whole-number floats as integers (e.g. 42.0 becomes 42)
        #[arg(long)]
        compact_floats: bool,
    },

    /// Convert TeaLeaf binary (.tlbx) to JSON
    TlbxToJson {
        /// Input .tlbx file
        input: PathBuf,
        /// Output .json file (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert JSON to TeaLeaf binary (.tlbx)
    JsonToTlbx {
        /// Input .json file
        input: PathBuf,
        /// Output .tlbx file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp
                | clap::error::ErrorKind::DisplayVersion => process::exit(0),
                _ => process::exit(1),
            }
        }
    };

    let result = match cli.command {
        Commands::Compile { ref input, ref output } => cmd_compile(input, output),
        Commands::Decompile { ref input, ref output, compact, compact_floats } =>
            cmd_decompile(input, output, compact, compact_floats),
        Commands::Info { ref input } => cmd_info(input),
        Commands::Validate { ref input } => cmd_validate(input),
        Commands::ToJson { ref input, ref output } => cmd_to_json(input, output.as_deref()),
        Commands::FromJson { ref input, ref output, compact, compact_floats } =>
            cmd_from_json(input, output, compact, compact_floats),
        Commands::TlbxToJson { ref input, ref output } => cmd_tlbx_to_json(input, output.as_deref()),
        Commands::JsonToTlbx { ref input, ref output } => cmd_json_to_tlbx(input, output),
        Commands::Completions { shell } => {
            generate(shell, &mut Cli::command(), "tealeaf", &mut std::io::stdout());
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn cmd_compile(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Compiling {} -> {}", input.display(), output.display());

    let doc = TeaLeaf::load(input)?;
    doc.compile(output, true)?;

    let in_size = std::fs::metadata(input)?.len();
    let out_size = std::fs::metadata(output)?.len();
    let ratio = out_size as f64 / in_size as f64 * 100.0;

    println!("Done: {} bytes -> {} bytes ({:.1}%)", in_size, out_size, ratio);
    Ok(())
}

fn cmd_decompile(input: &Path, output: &Path, compact: bool, compact_floats: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut flags = Vec::new();
    if compact { flags.push("compact"); }
    if compact_floats { flags.push("compact-floats"); }
    let flag_str = if flags.is_empty() { String::new() } else { format!(" ({})", flags.join(", ")) };
    println!("Decompiling {} -> {}{}", input.display(), output.display(), flag_str);

    let reader = Reader::open(input)?;
    let doc = TeaLeaf::from_reader(&reader)?;
    let mut opts = if compact { FormatOptions::compact() } else { FormatOptions::default() };
    if compact_floats { opts = opts.with_compact_floats(); }
    let tl_text = doc.to_tl_with_options(&opts);

    std::fs::write(output, tl_text)?;
    println!("Done");
    Ok(())
}

fn cmd_info(input: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

    println!("File: {}", input.display());
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

fn cmd_validate(input: &Path) -> Result<(), Box<dyn std::error::Error>> {
    match TeaLeaf::load(input) {
        Ok(doc) => {
            println!("\u{2713} Valid");
            println!("  Schemas: {}", doc.schemas.len());
            println!("  Keys: {}", doc.data.len());
        }
        Err(e) => {
            println!("\u{2717} Invalid: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

fn cmd_to_json(input: &Path, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let doc = TeaLeaf::load(input)?;
    let json = doc.to_json()?;

    match output {
        Some(path) => {
            std::fs::write(path, &json)?;
            println!("Converted {} -> {}", input.display(), path.display());
        }
        None => {
            println!("{}", json);
        }
    }

    Ok(())
}

fn cmd_from_json(input: &Path, output: &Path, compact: bool, compact_floats: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut flags = Vec::new();
    if compact { flags.push("compact"); }
    if compact_floats { flags.push("compact-floats"); }
    let flag_str = if flags.is_empty() { String::new() } else { format!(" ({})", flags.join(", ")) };
    println!("Converting {} -> {}{}", input.display(), output.display(), flag_str);

    let json_content = std::fs::read_to_string(input)?;

    let doc = TeaLeaf::from_json_with_schemas(&json_content)?;
    if !doc.schemas.is_empty() {
        println!("Inferred {} schema(s):", doc.schemas.len());
        for (name, schema) in &doc.schemas {
            println!("  @struct {} ({} fields)", name, schema.fields.len());
        }
    }
    let mut opts = if compact { FormatOptions::compact() } else { FormatOptions::default() };
    if compact_floats { opts = opts.with_compact_floats(); }
    let tl_text = doc.to_tl_with_options(&opts);

    std::fs::write(output, tl_text)?;
    println!("Done");

    Ok(())
}

fn cmd_tlbx_to_json(input: &Path, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let reader = Reader::open(input)?;
    let doc = TeaLeaf::from_reader(&reader)?;
    let json = doc.to_json()?;

    match output {
        Some(path) => {
            std::fs::write(path, &json)?;
            println!("Converted {} -> {}", input.display(), path.display());
        }
        None => {
            println!("{}", json);
        }
    }

    Ok(())
}

fn cmd_json_to_tlbx(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Converting {} -> {}", input.display(), output.display());

    let json_content = std::fs::read_to_string(input)?;
    let doc = TeaLeaf::from_json(&json_content)?;
    doc.compile(output, true)?;

    let in_size = std::fs::metadata(input)?.len();
    let out_size = std::fs::metadata(output)?.len();
    let ratio = out_size as f64 / in_size as f64 * 100.0;

    println!("Done: {} bytes -> {} bytes ({:.1}%)", in_size, out_size, ratio);
    Ok(())
}
