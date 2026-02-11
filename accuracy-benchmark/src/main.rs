//! Accuracy Benchmark CLI

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use accuracy_benchmark::{
    analysis::{ComparisonEngine, AnalysisResult},
    config::{Config, DataFormat},
    providers::{create_all_providers_with_config, create_providers_with_config, LLMProvider},
    reporting::{print_console_report, JsonSummary, TLWriter},
    runner::{Executor, ExecutorConfig},
    tasks::{load_tasks_from_directory, load_tasks_from_file, load_tasks_from_json_file, BenchmarkTask, TaskResult},
};

/// Data source selection for benchmark tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum DataSource {
    /// Use small synthetic datasets (default, always available)
    Synthetic,
    /// Use real-world datasets (requires download via processing scripts)
    Real,
}

#[derive(Parser)]
#[command(name = "accuracy-benchmark")]
#[command(about = "Accuracy benchmark suite for TeaLeaf format across LLM providers")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute benchmark suite
    Run {
        /// Comma-separated provider list (default: all available)
        #[arg(short, long)]
        providers: Option<String>,

        /// Comma-separated task categories to run (default: all)
        #[arg(long)]
        categories: Option<String>,

        /// Path to task definitions (file or directory)
        #[arg(short, long)]
        tasks: Option<PathBuf>,

        /// Number of parallel requests per provider
        #[arg(long, default_value = "3")]
        parallel: usize,

        /// Output directory for results
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Compare TeaLeaf vs JSON format performance
        #[arg(long)]
        compare_formats: bool,

        /// Use synthetic or real-world data files
        #[arg(long, value_enum, default_value = "synthetic")]
        data_source: DataSource,

        /// Save raw API responses to individual files in the output directory
        #[arg(long)]
        save_responses: bool,
    },

    /// Analyze existing results
    Analyze {
        /// Path to results directory or TeaLeaf file
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Generate reports from results
    Report {
        /// Path to results directory
        #[arg(short, long)]
        input: PathBuf,

        /// Output format (console, json, tl, html)
        #[arg(short, long, default_value = "console")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List available tasks
    ListTasks {
        /// Path to task definitions
        #[arg(short, long)]
        tasks: Option<PathBuf>,

        /// Use synthetic or real-world data files
        #[arg(long, value_enum, default_value = "synthetic")]
        data_source: DataSource,
    },

    /// Generate sample configuration
    InitConfig {
        /// Output path for configuration file
        #[arg(short, long, default_value = "config/models.toml")]
        output: PathBuf,
    },

    /// Dump all task prompts (both TL and JSON formats) to text files for review
    DumpPrompts {
        /// Output directory for prompt files
        #[arg(short, long, default_value = "results/prompts")]
        output: PathBuf,

        /// Path to task definitions (file or directory)
        #[arg(short, long)]
        tasks: Option<PathBuf>,

        /// Use synthetic or real-world data files
        #[arg(long, value_enum, default_value = "synthetic")]
        data_source: DataSource,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new("accuracy_benchmark=debug,info")
    } else {
        EnvFilter::new("accuracy_benchmark=info,warn")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    match cli.command {
        Commands::Run {
            providers,
            categories,
            tasks,
            parallel,
            output,
            compare_formats,
            data_source,
            save_responses,
        } => {
            run_benchmark(providers, categories, tasks, parallel, output, compare_formats, data_source, save_responses).await?;
        }

        Commands::Analyze { input } => {
            analyze_results(input)?;
        }

        Commands::Report {
            input,
            format,
            output,
        } => {
            generate_report(input, &format, output)?;
        }

        Commands::ListTasks { tasks, data_source } => {
            list_tasks(tasks, data_source)?;
        }

        Commands::InitConfig { output } => {
            init_config(output)?;
        }

        Commands::DumpPrompts { output, tasks, data_source } => {
            dump_prompts(output, tasks, data_source)?;
        }
    }

    Ok(())
}

async fn run_benchmark(
    providers_arg: Option<String>,
    categories_arg: Option<String>,
    tasks_path: Option<PathBuf>,
    parallel: usize,
    output_dir: Option<PathBuf>,
    compare_formats: bool,
    data_source: DataSource,
    save_responses: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let started_at = Utc::now();
    let run_id = started_at.format("%Y%m%d-%H%M%S").to_string();

    println!("=== Accuracy Benchmark Suite ===");
    println!("Run ID: {}", run_id);
    println!("Data:   {:?}", data_source);
    if compare_formats {
        println!("Mode: Format Comparison (TeaLeaf vs JSON vs TOON)");
    }
    println!();

    // Load model/provider configuration
    let model_config = Config::load_or_default();

    // Create providers (config applies RPM/TPM limits, model, thinking settings)
    let providers: Vec<Arc<dyn LLMProvider + Send + Sync>> = if let Some(names) = providers_arg {
        let names: Vec<&str> = names.split(',').map(|s| s.trim()).collect();
        create_providers_with_config(&names, &model_config)?
    } else {
        create_all_providers_with_config(&model_config)
    };

    if providers.is_empty() {
        eprintln!("Error: No providers available. Set API keys in environment.");
        eprintln!("  ANTHROPIC_API_KEY for Anthropic/Claude");
        eprintln!("  OPENAI_API_KEY for OpenAI");
        std::process::exit(1);
    }

    let provider_names: Vec<String> = providers.iter().map(|p| p.name().to_string()).collect();
    println!("Providers: {}", provider_names.join(", "));

    // Load tasks
    let tasks = load_tasks(tasks_path, categories_arg, data_source)?;

    if tasks.is_empty() {
        eprintln!("Error: No tasks to run");
        std::process::exit(1);
    }

    println!("Tasks: {}", tasks.len());
    println!();

    // Create executor
    let config = ExecutorConfig {
        parallel_requests: parallel,
        compare_formats,
        ..Default::default()
    };
    let executor = Executor::new(providers.clone(), config);

    // Execute tasks
    println!("Running benchmark...");

    // Use format-aware execution when comparing formats
    let (task_results, format_comparison_results) = if compare_formats {
        let format_results = executor.execute_tasks_with_formats(&tasks).await;

        // Convert to legacy format for TeaLeaf results (for backward compatible reporting)
        let mut legacy_results: Vec<HashMap<String, TaskResult>> = Vec::new();
        for task in &tasks {
            let mut task_map: HashMap<String, TaskResult> = HashMap::new();
            for provider in &providers {
                let tl_key = accuracy_benchmark::tasks::TaskResultKey::new(
                    &task.metadata.id,
                    provider.name(),
                    DataFormat::TL,
                );
                if let Some(result) = format_results.get(&tl_key) {
                    task_map.insert(provider.name().to_string(), result.clone());
                }
            }
            legacy_results.push(task_map);
        }
        (legacy_results, Some(format_results))
    } else {
        (executor.execute_tasks(&tasks).await, None)
    };

    // Analyze results
    println!("Analyzing results...");
    let engine = ComparisonEngine::new();

    let mut analysis_results: Vec<HashMap<String, AnalysisResult>> = Vec::new();
    let mut comparisons = Vec::new();

    for (task, results) in tasks.iter().zip(task_results.iter()) {
        // Analyze each provider's response
        let mut task_analysis: HashMap<String, AnalysisResult> = HashMap::new();

        for (provider, result) in results {
            if let Some(analysis) = engine.analyze_result(task, result) {
                task_analysis.insert(provider.clone(), analysis);
            }
        }

        if !task_analysis.is_empty() {
            let comparison = engine.compare_responses(task, &task_analysis);
            comparisons.push(comparison);
        }

        analysis_results.push(task_analysis);
    }

    // Aggregate results
    let aggregated = engine.aggregate_with_tasks(&comparisons, &tasks);

    // If format comparison enabled, analyze results for each format
    let all_formats = DataFormat::all();
    let format_aggregated: HashMap<DataFormat, _> = if let Some(ref format_results) = format_comparison_results {
        let mut agg_map = HashMap::new();
        for &fmt in &all_formats {
            let mut fmt_comparisons = Vec::new();
            for task in &tasks {
                let mut task_analysis: HashMap<String, AnalysisResult> = HashMap::new();
                for provider in &providers {
                    let key = accuracy_benchmark::tasks::TaskResultKey::new(
                        &task.metadata.id,
                        provider.name(),
                        fmt,
                    );
                    if let Some(result) = format_results.get(&key) {
                        if let Some(analysis) = engine.analyze_result(task, result) {
                            task_analysis.insert(provider.name().to_string(), analysis);
                        }
                    }
                }
                if !task_analysis.is_empty() {
                    let comparison = engine.compare_responses(task, &task_analysis);
                    fmt_comparisons.push(comparison);
                }
            }
            agg_map.insert(fmt, engine.aggregate_with_tasks(&fmt_comparisons, &tasks));
        }
        agg_map
    } else {
        HashMap::new()
    };

    // Print per-format results
    if compare_formats {
        for &fmt in &all_formats {
            if let Some(agg) = format_aggregated.get(&fmt) {
                println!("\n=== {} Format Results ===", fmt.as_str().to_uppercase());
                print_console_report(agg);
            }
        }
    } else {
        print_console_report(&aggregated);
    }

    // Print format comparison if enabled
    if compare_formats {
        if let Some(ref format_results) = format_comparison_results {
            // Collect successful results for pairing
            let successful: std::collections::HashSet<_> = format_results
                .iter()
                .filter(|(_, r)| r.response.is_some())
                .map(|(k, _)| (k.task_id.clone(), k.provider.clone(), k.format))
                .collect();

            // Count tasks where ALL formats succeeded per provider
            let mut paired_task_counts: HashMap<String, usize> = HashMap::new();
            for provider in &provider_names {
                let count = tasks.iter().filter(|task| {
                    all_formats.iter().all(|&fmt| {
                        successful.contains(&(task.metadata.id.clone(), provider.clone(), fmt))
                    })
                }).count();
                paired_task_counts.insert(provider.clone(), count);
            }

            // Calculate token usage — only count tasks where ALL formats succeeded
            let mut token_usage: HashMap<(String, DataFormat), (u32, u32)> = HashMap::new();
            for (key, result) in format_results {
                if let Some(ref response) = result.response {
                    let all_succeeded = all_formats.iter().all(|&fmt| {
                        successful.contains(&(key.task_id.clone(), key.provider.clone(), fmt))
                    });
                    if all_succeeded {
                        let entry = token_usage
                            .entry((key.provider.clone(), key.format))
                            .or_insert((0, 0));
                        entry.0 += response.input_tokens;
                        entry.1 += response.output_tokens;
                    }
                }
            }

            let num_tasks = tasks.len();

            // === Accuracy Comparison ===
            println!("\n=== Format Comparison: Accuracy ===");
            println!("(Scores averaged across tasks where all formats succeeded)");
            println!("{:-<80}", "");
            print!("{:<12} {:>6}", "Provider", "Pairs");
            for fmt in &all_formats {
                print!(" {:>12}", format!("{} Score", fmt.as_str().to_uppercase()));
            }
            println!();
            println!("{:-<80}", "");

            for provider in &provider_names {
                let pairs = paired_task_counts.get(provider).copied().unwrap_or(0);
                print!("{:<12} {:>5}/{}", provider, pairs, num_tasks);
                for fmt in &all_formats {
                    let score = format_aggregated.get(fmt)
                        .and_then(|agg| agg.avg_scores_by_provider.get(provider).copied())
                        .unwrap_or(0.0);
                    print!(" {:>12.3}", score);
                }
                println!();
            }
            println!("{:-<80}", "");

            // === Token Comparison (vs JSON baseline) ===
            println!("\n=== Format Comparison: Tokens (vs JSON baseline) ===");
            println!("(Only tasks where all formats succeeded)");
            println!("{:-<96}", "");
            print!("{:<12}", "Provider");
            for fmt in &all_formats {
                print!(" {:>13}", format!("{} Tokens", fmt.as_str().to_uppercase()));
            }
            for fmt in &all_formats {
                if *fmt != DataFormat::Json {
                    print!(" {:>11}", format!("{} vs JSON", fmt.as_str().to_uppercase()));
                }
            }
            println!();
            println!("{:-<96}", "");

            for provider in &provider_names {
                print!("{:<12}", provider);
                let json_total = {
                    let (i, o) = token_usage.get(&(provider.clone(), DataFormat::Json)).copied().unwrap_or((0, 0));
                    i + o
                };
                for &fmt in &all_formats {
                    let (i, o) = token_usage.get(&(provider.clone(), fmt)).copied().unwrap_or((0, 0));
                    print!(" {:>13}", i + o);
                }
                for &fmt in &all_formats {
                    if fmt != DataFormat::Json {
                        let (i, o) = token_usage.get(&(provider.clone(), fmt)).copied().unwrap_or((0, 0));
                        let total = i + o;
                        let pct = if json_total > 0 {
                            ((total as f64 - json_total as f64) / json_total as f64) * 100.0
                        } else {
                            0.0
                        };
                        print!(" {:>+10.1}%", pct);
                    }
                }
                println!();
            }
            println!("{:-<96}", "");

            // === Input Token Breakdown ===
            println!("\n=== Input Token Breakdown (vs JSON baseline) ===");
            println!("{:-<96}", "");
            print!("{:<12}", "Provider");
            for fmt in &all_formats {
                print!(" {:>11}", format!("{} In", fmt.as_str().to_uppercase()));
            }
            for fmt in &all_formats {
                if *fmt != DataFormat::Json {
                    print!(" {:>11}", format!("{} vs JSON", fmt.as_str().to_uppercase()));
                }
            }
            println!();
            println!("{:-<96}", "");

            for provider in &provider_names {
                print!("{:<12}", provider);
                let json_in = token_usage.get(&(provider.clone(), DataFormat::Json)).copied().unwrap_or((0, 0)).0;
                for &fmt in &all_formats {
                    let (i, _) = token_usage.get(&(provider.clone(), fmt)).copied().unwrap_or((0, 0));
                    print!(" {:>11}", i);
                }
                for &fmt in &all_formats {
                    if fmt != DataFormat::Json {
                        let (i, _) = token_usage.get(&(provider.clone(), fmt)).copied().unwrap_or((0, 0));
                        let pct = if json_in > 0 {
                            ((i as f64 - json_in as f64) / json_in as f64) * 100.0
                        } else {
                            0.0
                        };
                        print!(" {:>+10.1}%", pct);
                    }
                }
                println!();
            }
            println!("{:-<96}", "");

            // === Key Findings ===
            println!("\nKey Findings:");
            for provider in &provider_names {
                let json_score = format_aggregated.get(&DataFormat::Json)
                    .and_then(|a| a.avg_scores_by_provider.get(provider).copied())
                    .unwrap_or(0.0);
                let json_in = token_usage.get(&(provider.clone(), DataFormat::Json)).copied().unwrap_or((0, 0)).0;
                let json_total = {
                    let (i, o) = token_usage.get(&(provider.clone(), DataFormat::Json)).copied().unwrap_or((0, 0));
                    (i + o) as f64
                };

                // Find best format by accuracy and by token savings
                let mut best_accuracy_fmt = DataFormat::Json;
                let mut best_accuracy = json_score;
                let mut best_savings_fmt = DataFormat::Json;
                let mut best_savings_pct: f64 = 0.0;

                for &fmt in &all_formats {
                    let score = format_aggregated.get(&fmt)
                        .and_then(|a| a.avg_scores_by_provider.get(provider).copied())
                        .unwrap_or(0.0);
                    if score > best_accuracy {
                        best_accuracy = score;
                        best_accuracy_fmt = fmt;
                    }
                    let (i, o) = token_usage.get(&(provider.clone(), fmt)).copied().unwrap_or((0, 0));
                    let total = (i + o) as f64;
                    let savings = if json_total > 0.0 { (1.0 - total / json_total) * 100.0 } else { 0.0 };
                    if savings > best_savings_pct {
                        best_savings_pct = savings;
                        best_savings_fmt = fmt;
                    }
                }

                // Accuracy verdict
                let accuracy_diff = best_accuracy - json_score;
                let accuracy_verdict = if accuracy_diff > 0.02 {
                    format!("{} +{:.1}% accuracy vs JSON", best_accuracy_fmt.as_str().to_uppercase(), accuracy_diff * 100.0)
                } else {
                    "all formats comparable accuracy".to_string()
                };

                // Token verdict
                let (best_in, _) = token_usage.get(&(provider.clone(), best_savings_fmt)).copied().unwrap_or((0, 0));
                let input_savings = if json_in > 0 { (1.0 - best_in as f64 / json_in as f64) * 100.0 } else { 0.0 };
                let token_verdict = if best_savings_pct > 3.0 {
                    format!("{} saves {:.0}% total tokens ({:.0}% on input) vs JSON",
                        best_savings_fmt.as_str().to_uppercase(), best_savings_pct, input_savings)
                } else {
                    "similar token usage across formats".to_string()
                };

                println!("  {}: {}, {}", provider, accuracy_verdict, token_verdict);
            }
            println!();
        }
    }

    // Save results
    let completed_at = Utc::now();
    let output_base = output_dir.unwrap_or_else(|| PathBuf::from("accuracy-benchmark/results/runs"));
    let run_dir = output_base.join(&run_id);
    std::fs::create_dir_all(&run_dir)?;

    // Write TeaLeaf results
    let tl_path = run_dir.join("analysis.tl");
    TLWriter::write_run_results(
        &tl_path,
        &run_id,
        started_at,
        completed_at,
        &provider_names,
        &task_results,
        &analysis_results,
        &comparisons,
        &aggregated,
    )?;
    println!("\nTeaLeaf results written to: {}", tl_path.display());

    // Write JSON summary
    let json_path = run_dir.join("summary.json");
    let summary = JsonSummary::from_aggregated(&run_id, &aggregated, "analysis.tl");
    summary.write_to_file(&json_path)?;
    println!("JSON summary written to: {}", json_path.display());

    // Save raw API responses if requested
    if save_responses {
        let responses_dir = run_dir.join("responses");
        std::fs::create_dir_all(&responses_dir)?;
        let mut count = 0;

        if let Some(ref format_results) = format_comparison_results {
            // Format comparison mode: save per task/provider/format
            for (key, result) in format_results {
                if let Some(ref response) = result.response {
                    let filename = format!(
                        "{}-{}-{}.txt",
                        key.task_id.to_lowercase(),
                        key.provider,
                        key.format.as_str()
                    );
                    std::fs::write(responses_dir.join(&filename), &response.content)?;
                    count += 1;
                }
            }
        } else {
            // Single-format mode: save per task/provider
            for task_map in &task_results {
                for (provider, result) in task_map {
                    if let Some(ref response) = result.response {
                        let filename = format!(
                            "{}-{}.txt",
                            result.task_id.to_lowercase(),
                            provider
                        );
                        std::fs::write(responses_dir.join(&filename), &response.content)?;
                        count += 1;
                    }
                }
            }
        }

        println!("API responses written to: {} ({} files)", responses_dir.display(), count);
    }

    Ok(())
}

fn load_tasks(
    path: Option<PathBuf>,
    categories_arg: Option<String>,
    data_source: DataSource,
) -> Result<Vec<BenchmarkTask>, Box<dyn std::error::Error>> {
    // Parse category filter
    let category_filter: Option<Vec<String>> = categories_arg.map(|s| {
        s.split(',')
            .map(|c| c.trim().to_lowercase())
            .collect()
    });

    let mut tasks = if let Some(path) = path {
        // Load from user-specified path
        if path.is_dir() {
            load_tasks_from_directory(&path)?
        } else if path.extension().map(|e| e == "json").unwrap_or(false) {
            load_tasks_from_json_file(&path)?
        } else {
            load_tasks_from_file(&path)?
        }
    } else {
        // Load from built-in task definition files
        let tasks_dir = PathBuf::from("accuracy-benchmark/tasks");
        let def_file = match data_source {
            DataSource::Synthetic => tasks_dir.join("synthetic.json"),
            DataSource::Real => tasks_dir.join("real.json"),
        };
        load_tasks_from_json_file(&def_file)?
    };

    // Filter by category
    if let Some(categories) = category_filter {
        tasks.retain(|t| categories.contains(&t.metadata.category.to_lowercase()));
    }

    Ok(tasks)
}

fn analyze_results(input: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Analyzing results from: {}", input.display());
    // TODO: Implement result analysis
    println!("Analysis feature not yet implemented");
    Ok(())
}

fn generate_report(
    input: PathBuf,
    format: &str,
    _output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating {} report from: {}", format, input.display());
    // TODO: Implement report generation
    println!("Report generation not yet implemented");
    Ok(())
}

fn list_tasks(tasks_path: Option<PathBuf>, data_source: DataSource) -> Result<(), Box<dyn std::error::Error>> {
    let tasks = load_tasks(tasks_path, None, data_source)?;

    println!("Available Tasks ({}):", tasks.len());
    println!("{:-<60}", "");

    for task in &tasks {
        println!(
            "  {} | {} | {:?} | {:?}",
            task.metadata.id,
            task.metadata.category,
            task.metadata.complexity,
            task.metadata.output_type
        );
    }

    Ok(())
}

fn init_config(output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();

    // Ensure parent directory exists
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    config.save_toml(&output)?;
    println!("Configuration written to: {}", output.display());
    Ok(())
}

fn dump_prompts(
    output_dir: PathBuf,
    tasks_path: Option<PathBuf>,
    data_source: DataSource,
) -> Result<(), Box<dyn std::error::Error>> {
    let tasks = load_tasks(tasks_path, None, data_source)?;

    if tasks.is_empty() {
        eprintln!("Error: No tasks to dump");
        std::process::exit(1);
    }

    std::fs::create_dir_all(&output_dir)?;

    println!("=== Dumping API Request Prompts ===");
    println!("Tasks: {}", tasks.len());
    println!("Output: {}", output_dir.display());
    println!();

    for task in &tasks {
        for format in DataFormat::all() {
            let mut task_clone = task.clone();
            match task_clone.prepare_prompt_with_format(format) {
                Ok(()) => {
                    let filename = format!(
                        "{}-{}.txt",
                        task.metadata.id.to_lowercase(),
                        format.as_str()
                    );
                    let filepath = output_dir.join(&filename);

                    // Build the full request content with metadata header
                    let content = format!(
                        "=== API Request: {} ({} format) ===\n\
                         Task ID:     {}\n\
                         Category:    {}\n\
                         Complexity:  {:?}\n\
                         Output Type: {:?}\n\
                         Format:      {}\n\
                         Max Tokens:  {}\n\
                         Temperature: {}\n\
                         {}\n\
                         === PROMPT ===\n\n\
                         {}",
                        task.metadata.id,
                        format.as_str().to_uppercase(),
                        task.metadata.id,
                        task.metadata.category,
                        task.metadata.complexity,
                        task.metadata.output_type,
                        format.as_str().to_uppercase(),
                        task.max_tokens,
                        task.temperature
                            .map(|t| format!("{}", t))
                            .unwrap_or_else(|| "none".to_string()),
                        "=".repeat(50),
                        task_clone.prompt,
                    );

                    std::fs::write(&filepath, &content)?;
                    println!(
                        "  [{}] {} ({}) -> {}",
                        format.as_str().to_uppercase(),
                        task.metadata.id,
                        task.metadata.category,
                        filename
                    );
                }
                Err(e) => {
                    eprintln!(
                        "  [ERROR] {} ({} format): {}",
                        task.metadata.id,
                        format.as_str(),
                        e
                    );
                }
            }
        }
    }

    println!("\nDone. {} files written to {}", tasks.len() * 2, output_dir.display());
    Ok(())
}
