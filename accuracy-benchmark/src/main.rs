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
    providers::{create_all_providers, create_providers, LLMProvider},
    reporting::{print_console_report, JsonSummary, TLWriter},
    runner::{Executor, ExecutorConfig},
    tasks::{load_tasks_from_directory, load_tasks_from_file, BenchmarkTask, TaskResult},
};

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
    },

    /// Generate sample configuration
    InitConfig {
        /// Output path for configuration file
        #[arg(short, long, default_value = "config/models.toml")]
        output: PathBuf,
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
        } => {
            run_benchmark(providers, categories, tasks, parallel, output, compare_formats).await?;
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

        Commands::ListTasks { tasks } => {
            list_tasks(tasks)?;
        }

        Commands::InitConfig { output } => {
            init_config(output)?;
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
) -> Result<(), Box<dyn std::error::Error>> {
    let started_at = Utc::now();
    let run_id = started_at.format("%Y%m%d-%H%M%S").to_string();

    println!("=== Accuracy Benchmark Suite ===");
    println!("Run ID: {}", run_id);
    if compare_formats {
        println!("Mode: Format Comparison (TeaLeaf vs JSON)");
    }
    println!();

    // Create providers
    let providers: Vec<Arc<dyn LLMProvider + Send + Sync>> = if let Some(names) = providers_arg {
        let names: Vec<&str> = names.split(',').map(|s| s.trim()).collect();
        create_providers(&names)?
    } else {
        create_all_providers()
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
    let tasks = load_tasks(tasks_path, categories_arg)?;

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

    // If format comparison enabled, also analyze JSON results
    let json_aggregated = if let Some(ref format_results) = format_comparison_results {
        // Analyze JSON results separately
        let mut json_analysis_results: Vec<HashMap<String, AnalysisResult>> = Vec::new();
        let mut json_comparisons = Vec::new();

        for task in &tasks {
            let mut task_analysis: HashMap<String, AnalysisResult> = HashMap::new();

            for provider in &providers {
                let json_key = accuracy_benchmark::tasks::TaskResultKey::new(
                    &task.metadata.id,
                    provider.name(),
                    DataFormat::Json,
                );
                if let Some(result) = format_results.get(&json_key) {
                    if let Some(analysis) = engine.analyze_result(task, result) {
                        task_analysis.insert(provider.name().to_string(), analysis);
                    }
                }
            }

            if !task_analysis.is_empty() {
                let comparison = engine.compare_responses(task, &task_analysis);
                json_comparisons.push(comparison);
            }

            json_analysis_results.push(task_analysis);
        }

        Some(engine.aggregate_with_tasks(&json_comparisons, &tasks))
    } else {
        None
    };

    // Print results
    if compare_formats {
        println!("\n=== TeaLeaf Format Results ===");
    }
    print_console_report(&aggregated);

    // Print JSON format comparison if enabled
    if let Some(ref json_agg) = json_aggregated {
        println!("\n=== JSON Format Results ===");
        print_console_report(json_agg);

        // Calculate token usage by provider and format.
        // Only count tasks where BOTH formats succeeded for a given
        // (task_id, provider) pair — otherwise the totals are not comparable.
        let mut token_usage: HashMap<(String, DataFormat), (u32, u32)> = HashMap::new();
        if let Some(ref format_results) = format_comparison_results {
            // Collect successful results indexed by (task_id, provider, format)
            let successful: HashMap<_, _> = format_results
                .iter()
                .filter(|(_, r)| r.response.is_some())
                .map(|(k, r)| ((k.task_id.clone(), k.provider.clone(), k.format), r))
                .collect();

            for (key, result) in format_results {
                if let Some(ref response) = result.response {
                    // Find the counterpart format
                    let other_format = if key.format == DataFormat::TL {
                        DataFormat::Json
                    } else {
                        DataFormat::TL
                    };

                    // Only include if the other format also succeeded for this task+provider
                    if successful.contains_key(&(key.task_id.clone(), key.provider.clone(), other_format)) {
                        let entry = token_usage
                            .entry((key.provider.clone(), key.format))
                            .or_insert((0, 0));
                        entry.0 += response.input_tokens;
                        entry.1 += response.output_tokens;
                    }
                }
            }
        }

        // Count paired tasks per provider (where both formats succeeded)
        let mut paired_task_counts: HashMap<String, usize> = HashMap::new();
        if let Some(ref format_results) = format_comparison_results {
            let successful: std::collections::HashSet<_> = format_results
                .iter()
                .filter(|(_, r)| r.response.is_some())
                .map(|(&ref k, _)| (k.task_id.clone(), k.provider.clone(), k.format))
                .collect();

            for provider in &provider_names {
                let count = format_results
                    .keys()
                    .filter(|k| {
                        k.provider == *provider
                            && k.format == DataFormat::TL
                            && successful.contains(&(k.task_id.clone(), k.provider.clone(), DataFormat::TL))
                            && successful.contains(&(k.task_id.clone(), k.provider.clone(), DataFormat::Json))
                    })
                    .count();
                paired_task_counts.insert(provider.clone(), count);
            }
        }

        // Print format comparison summary
        println!("\n=== Format Comparison Summary ===");
        println!("(Token comparison uses only tasks where both formats succeeded)");
        println!("{:-<108}", "");
        println!("{:<12} {:>6} {:>13} {:>11} {:>8} {:>15} {:>13} {:>9} {:>9}",
            "Provider", "Pairs", "TeaLeaf Score", "JSON Score", "Diff", "TeaLeaf Tokens", "JSON Tokens", "Diff", "Diff %");
        println!("{:-<108}", "");

        for provider in &provider_names {
            let tl_score = aggregated.avg_scores_by_provider.get(provider).copied().unwrap_or(0.0);
            let json_score = json_agg.avg_scores_by_provider.get(provider).copied().unwrap_or(0.0);
            let score_diff = tl_score - json_score;
            let pairs = paired_task_counts.get(provider).copied().unwrap_or(0);

            // Get token counts (already filtered to paired tasks only)
            let (tl_in, tl_out) = token_usage
                .get(&(provider.clone(), DataFormat::TL))
                .copied()
                .unwrap_or((0, 0));
            let (json_in, json_out) = token_usage
                .get(&(provider.clone(), DataFormat::Json))
                .copied()
                .unwrap_or((0, 0));
            let tl_total = tl_in + tl_out;
            let json_total = json_in + json_out;
            let token_diff = tl_total as i64 - json_total as i64;

            // Calculate token percentage difference (negative = TeaLeaf uses fewer tokens)
            let token_diff_pct = if json_total > 0 {
                ((tl_total as f64 - json_total as f64) / json_total as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "{:<12} {:>6} {:>13.3} {:>11.3} {:>+8.3} {:>15} {:>13} {:>+9} {:>+8.1}%",
                provider, format!("{}/12", pairs), tl_score, json_score, score_diff, tl_total, json_total, token_diff, token_diff_pct
            );
        }
        println!("{:-<108}", "");

        // Print token breakdown (input vs output)
        println!("\nToken Breakdown (Input / Output):");
        println!("{:-<78}", "");
        println!("{:<12} {:>19} {:>15} {:>14} {:>14}",
            "Provider", "TeaLeaf (in/out)", "JSON (in/out)", "In Diff %", "Out Diff %");
        println!("{:-<78}", "");

        for provider in &provider_names {
            let (tl_in, tl_out) = token_usage
                .get(&(provider.clone(), DataFormat::TL))
                .copied()
                .unwrap_or((0, 0));
            let (json_in, json_out) = token_usage
                .get(&(provider.clone(), DataFormat::Json))
                .copied()
                .unwrap_or((0, 0));

            let in_diff_pct = if json_in > 0 {
                ((tl_in as f64 - json_in as f64) / json_in as f64) * 100.0
            } else {
                0.0
            };
            let out_diff_pct = if json_out > 0 {
                ((tl_out as f64 - json_out as f64) / json_out as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "{:<12} {:>8} / {:<8} {:>6} / {:<6} {:>+13.1}% {:>+13.1}%",
                provider, tl_in, tl_out, json_in, json_out, in_diff_pct, out_diff_pct
            );
        }
        println!("{:-<78}", "");

        // Print summary interpretation
        println!("\nKey Findings:");
        for provider in &provider_names {
            let tl_score = aggregated.avg_scores_by_provider.get(provider).copied().unwrap_or(0.0);
            let json_score = json_agg.avg_scores_by_provider.get(provider).copied().unwrap_or(0.0);
            let score_diff = tl_score - json_score;

            let (tl_in, tl_out) = token_usage
                .get(&(provider.clone(), DataFormat::TL))
                .copied()
                .unwrap_or((0, 0));
            let (json_in, json_out) = token_usage
                .get(&(provider.clone(), DataFormat::Json))
                .copied()
                .unwrap_or((0, 0));
            let tl_total = tl_in + tl_out;
            let json_total = json_in + json_out;

            let total_diff_pct = if json_total > 0 {
                ((tl_total as f64 - json_total as f64) / json_total as f64) * 100.0
            } else {
                0.0
            };
            let input_diff_pct = if json_in > 0 {
                ((tl_in as f64 - json_in as f64) / json_in as f64) * 100.0
            } else {
                0.0
            };

            // Accuracy verdict
            let accuracy_verdict = if score_diff > 0.02 {
                format!("TeaLeaf +{:.1}% accuracy", score_diff * 100.0)
            } else if score_diff < -0.02 {
                format!("JSON +{:.1}% accuracy", -score_diff * 100.0)
            } else {
                "comparable accuracy".to_string()
            };

            // Token verdict
            let token_verdict = if total_diff_pct < -3.0 {
                format!("TeaLeaf saves {:.0}% total tokens ({:.0}% on input)", -total_diff_pct, -input_diff_pct)
            } else if total_diff_pct > 3.0 {
                format!("JSON saves {:.0}% total tokens", total_diff_pct)
            } else if input_diff_pct < -10.0 {
                format!("TeaLeaf saves {:.0}% on input tokens", -input_diff_pct)
            } else {
                "similar token usage".to_string()
            };

            println!("  {}: {}, {}", provider, accuracy_verdict, token_verdict);
        }
        println!();
    }

    // Save results
    let completed_at = Utc::now();
    let output_base = output_dir.unwrap_or_else(|| PathBuf::from("results/runs"));
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

    Ok(())
}

fn load_tasks(
    path: Option<PathBuf>,
    categories_arg: Option<String>,
) -> Result<Vec<BenchmarkTask>, Box<dyn std::error::Error>> {
    // Parse category filter
    let category_filter: Option<Vec<String>> = categories_arg.map(|s| {
        s.split(',')
            .map(|c| c.trim().to_lowercase())
            .collect()
    });

    // Load tasks from path or use built-in tasks
    let mut tasks = if let Some(path) = path {
        if path.is_dir() {
            load_tasks_from_directory(&path)?
        } else {
            load_tasks_from_file(&path)?
        }
    } else {
        // Use default sample tasks
        create_sample_tasks()
    };

    // Filter by category
    if let Some(categories) = category_filter {
        tasks.retain(|t| categories.contains(&t.metadata.category.to_lowercase()));
    }

    Ok(tasks)
}

fn create_sample_tasks() -> Vec<BenchmarkTask> {
    // Sample JSON data for tasks
    let financial_json = serde_json::json!({
        "company": "TechCorp Inc",
        "period": "Q3 2024",
        "currency": "USD",
        "income_statement": {
            "revenue": [
                {"account": "Product Sales", "amount": 4200000},
                {"account": "Service Revenue", "amount": 800000}
            ],
            "expenses": [
                {"account": "Cost of Goods Sold", "amount": 2000000},
                {"account": "Operating Expenses", "amount": 1500000},
                {"account": "Tax Expense", "amount": 387500}
            ],
            "other_income": [
                {"account": "Interest Income", "amount": 50000}
            ]
        },
        "balance_sheet": {
            "assets": [
                {"account": "Cash", "amount": 2000000},
                {"account": "Accounts Receivable", "amount": 1500000},
                {"account": "Inventory", "amount": 800000}
            ],
            "liabilities": [
                {"account": "Accounts Payable", "amount": 600000},
                {"account": "Long-term Debt", "amount": 1000000}
            ]
        }
    });

    let portfolio_json = serde_json::json!({
        "portfolio": {
            "owner": "John Smith",
            "holdings": [
                {"symbol": "AAPL", "shares": 100, "purchase_price": 150.00, "current_price": 175.00, "dividend_yield": 0.005},
                {"symbol": "GOOGL", "shares": 50, "purchase_price": 2500.00, "current_price": 2800.00},
                {"symbol": "MSFT", "shares": 75, "purchase_price": 300.00, "current_price": 350.00, "dividend_yield": 0.008}
            ],
            "cash_balance": 10000.00
        }
    });

    let sales_json = serde_json::json!({
        "period": "2024-W03",
        "daily_sales": [
            {"day": "Monday", "orders": 150, "revenue": 12500},
            {"day": "Tuesday", "orders": 175, "revenue": 15200},
            {"day": "Wednesday", "orders": 140, "revenue": 11800},
            {"day": "Thursday", "orders": 190, "revenue": 18500},
            {"day": "Friday", "orders": 220, "revenue": 22000},
            {"day": "Saturday", "orders": 280, "revenue": 28500},
            {"day": "Sunday", "orders": 195, "revenue": 19200}
        ]
    });

    let customers_json = serde_json::json!({
        "customers": [
            {"id": "A", "orders": 12, "total_spend": 2400, "days_since_last_order": 5},
            {"id": "B", "orders": 2, "total_spend": 150, "days_since_last_order": 90},
            {"id": "C", "orders": 8, "total_spend": 1800, "days_since_last_order": 15},
            {"id": "D", "orders": 1, "total_spend": 50, "days_since_last_order": 180},
            {"id": "E", "orders": 20, "total_spend": 5500, "days_since_last_order": 2}
        ]
    });

    let patient_json = serde_json::json!({
        "patient": {
            "name": "John Doe",
            "age": 45,
            "gender": "Male",
            "blood_type": "O+"
        },
        "conditions": [
            {"name": "Type 2 Diabetes", "diagnosed": "2019-03-15", "status": "active"},
            {"name": "Hypertension", "diagnosed": "2020-08-22", "status": "active"}
        ],
        "medications": [
            {"name": "Metformin", "dosage": "500mg", "frequency": "twice daily"},
            {"name": "Lisinopril", "dosage": "10mg", "frequency": "daily"}
        ],
        "allergies": [
            {"allergen": "Penicillin", "severity": "severe"},
            {"allergen": "Sulfa drugs", "severity": "moderate"}
        ],
        "vitals": {
            "blood_pressure": "135/85",
            "blood_glucose": 145
        }
    });

    let server_json = serde_json::json!({
        "server": {
            "hostname": "web-prod-01",
            "metrics": {
                "cpu_avg": 85,
                "cpu_peak": 98,
                "memory_pct": 78,
                "disk_pct": 92,
                "network_mbps": 450,
                "network_capacity_mbps": 1000
            },
            "performance": {
                "requests_per_min": 15000,
                "error_rate_pct": 2.5,
                "latency_p99_ms": 850,
                "latency_target_ms": 500
            }
        }
    });

    let campaigns_json = serde_json::json!({
        "campaigns": [
            {
                "name": "Campaign A",
                "channel": "Social Media",
                "spend": 5000,
                "impressions": 500000,
                "clicks": 15000,
                "conversions": 300,
                "revenue": 18000
            },
            {
                "name": "Campaign B",
                "channel": "Email",
                "spend": 1000,
                "sent": 50000,
                "opens": 12500,
                "clicks": 2500,
                "conversions": 200,
                "revenue": 12000
            }
        ]
    });

    vec![
        // Finance tasks - with JSON data converted to TeaLeaf
        BenchmarkTask::new(
            "FIN-001",
            "finance",
            "You are a financial analyst. Analyze the following quarterly financial data (provided in TeaLeaf format) and extract key metrics:\n\n{tl_data}\n\nProvide:\n1. Total revenue (sum all revenue sources)\n2. Gross profit and margin\n3. Operating income\n4. Net income\n5. Profit margin percentage\n6. Total assets and liabilities\n7. Shareholders' equity"
        )
        .with_json_data(financial_json)
        .with_complexity(accuracy_benchmark::tasks::Complexity::Simple)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Calculation)
        .expect_with_pattern("metric", "Total revenue calculation", true, r"\$[\d,]+")
        .expect("metric", "Gross profit calculation", true)
        .expect("metric", "Net income calculation", true),

        BenchmarkTask::new(
            "FIN-002",
            "finance",
            "You are an investment analyst. Analyze the following portfolio data (provided in TeaLeaf format) and calculate key metrics:\n\n{tl_data}\n\nCalculate:\n1. Total portfolio value (including cash)\n2. Total cost basis\n3. Total unrealized gain/loss\n4. Portfolio return percentage\n5. Allocation percentages for each holding\n6. Weighted average dividend yield"
        )
        .with_json_data(portfolio_json)
        .with_complexity(accuracy_benchmark::tasks::Complexity::Moderate)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Calculation)
        .expect_with_pattern("metric", "Portfolio value", true, r"\$[\d,]+")
        .expect("metric", "Unrealized gain", true)
        .expect("metric", "Return percentage", true),

        // Retail tasks - with JSON data
        BenchmarkTask::new(
            "RET-001",
            "retail",
            "You are a retail analyst. Analyze the following weekly sales data (provided in TeaLeaf format) and provide key metrics:\n\n{tl_data}\n\nProvide:\n1. Total weekly revenue\n2. Total orders\n3. Average order value\n4. Best and worst performing days\n5. Day-over-day trends and patterns"
        )
        .with_json_data(sales_json)
        .with_complexity(accuracy_benchmark::tasks::Complexity::Simple)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Summary)
        .expect_with_pattern("metric", "Total revenue", true, r"\$[\d,]+")
        .expect("insight", "Best performing day", true)
        .expect("insight", "Trend analysis", false),

        BenchmarkTask::new(
            "RET-002",
            "retail",
            "You are a customer analytics expert. Analyze the following customer data (provided in TeaLeaf format) and perform customer segmentation:\n\n{tl_data}\n\nPerform RFM (Recency, Frequency, Monetary) analysis and:\n1. Calculate RFM scores for each customer\n2. Segment each customer (Champions, Loyal, At-Risk, Lost, etc.)\n3. Identify high-value customers\n4. Recommend retention strategies for each segment"
        )
        .with_json_data(customers_json)
        .with_complexity(accuracy_benchmark::tasks::Complexity::Complex)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Recommendation)
        .expect("analysis", "Customer segmentation", true)
        .expect("insight", "High-value identification", true)
        .expect("recommendation", "Retention strategy", true),

        // Healthcare tasks - with JSON data
        BenchmarkTask::new(
            "HLT-001",
            "healthcare",
            "You are a clinical data analyst. Extract and summarize key information from the following patient record (provided in TeaLeaf format):\n\n{tl_data}\n\nProvide:\n1. Patient demographics summary\n2. Active conditions count and list\n3. Current medications with potential interactions\n4. Allergy alerts with severity levels\n5. Recent vitals assessment"
        )
        .with_json_data(patient_json)
        .with_complexity(accuracy_benchmark::tasks::Complexity::Simple)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Summary)
        .expect("summary", "Demographics", true)
        .expect("summary", "Conditions", true)
        .expect("alert", "Allergy information", true),

        // Technology tasks - with JSON data
        BenchmarkTask::new(
            "TEC-001",
            "technology",
            "You are a DevOps engineer. Analyze the following server metrics (provided in TeaLeaf format) and identify issues:\n\n{tl_data}\n\nProvide:\n1. Critical issues requiring immediate action\n2. Warning-level concerns\n3. Resource utilization summary\n4. Performance assessment vs targets\n5. Prioritized recommended actions"
        )
        .with_json_data(server_json)
        .with_complexity(accuracy_benchmark::tasks::Complexity::Moderate)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Analysis)
        .expect("analysis", "Critical issues", true)
        .expect("analysis", "Resource assessment", true)
        .expect("recommendation", "Action items", true),

        // Marketing tasks - with JSON data
        BenchmarkTask::new(
            "MKT-001",
            "marketing",
            "You are a marketing analyst. Analyze the following campaign data (provided in TeaLeaf format) and calculate ROI metrics:\n\n{tl_data}\n\nCalculate for each campaign:\n1. CTR (Click-through rate)\n2. Conversion rate\n3. CPA (Cost per acquisition)\n4. ROAS (Return on ad spend)\n5. ROI percentage\n\nThen compare the campaigns and recommend budget allocation."
        )
        .with_json_data(campaigns_json)
        .with_complexity(accuracy_benchmark::tasks::Complexity::Moderate)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Calculation)
        .expect_with_pattern("metric", "CTR calculation", true, r"\d+\.?\d*%")
        .expect("metric", "ROAS calculation", true)
        .expect("metric", "ROI calculation", true),

        // Logistics task - using JSON file
        BenchmarkTask::new(
            "LOG-001",
            "logistics",
            "You are a logistics analyst. Analyze the following shipment data (provided in TeaLeaf format) and calculate delivery metrics:\n\n{tl_data}\n\nProvide:\n1. Total shipments and delivery status breakdown\n2. On-time delivery rate percentage\n3. Average shipment cost and cost per kg\n4. Carrier performance comparison\n5. Warehouse utilization summary\n6. Recommendations for improving delivery performance"
        )
        .with_json_file("accuracy-benchmark/tasks/logistics/data/shipments.json")
        .with_complexity(accuracy_benchmark::tasks::Complexity::Moderate)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Analysis)
        .expect_with_pattern("metric", "On-time delivery rate", true, r"\d+\.?\d*%")
        .expect("analysis", "Carrier comparison", true)
        .expect("recommendation", "Performance improvement", false),

        // HR task - using JSON file
        BenchmarkTask::new(
            "HR-001",
            "hr",
            "You are an HR analyst. Analyze the following employee data (provided in TeaLeaf format) and provide workforce insights:\n\n{tl_data}\n\nProvide:\n1. Headcount by department\n2. Average salary by department and role level\n3. Performance score distribution\n4. Tenure analysis (average years at company)\n5. Attrition analysis and trends\n6. Cost of benefits per employee"
        )
        .with_json_file("accuracy-benchmark/tasks/hr/data/employees.json")
        .with_complexity(accuracy_benchmark::tasks::Complexity::Moderate)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Analysis)
        .expect("metric", "Headcount breakdown", true)
        .expect("metric", "Average salary", true)
        .expect("insight", "Attrition analysis", true),

        // Manufacturing task - using JSON file
        BenchmarkTask::new(
            "MFG-001",
            "manufacturing",
            "You are a manufacturing analyst. Analyze the following production data (provided in TeaLeaf format) and calculate OEE metrics:\n\n{tl_data}\n\nProvide:\n1. Total production volume (planned vs actual)\n2. Production efficiency percentage\n3. Defect rate by product\n4. Equipment utilization analysis\n5. Downtime analysis and causes\n6. Material stock levels vs reorder points"
        )
        .with_json_file("accuracy-benchmark/tasks/manufacturing/data/production.json")
        .with_complexity(accuracy_benchmark::tasks::Complexity::Moderate)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Calculation)
        .expect_with_pattern("metric", "Production efficiency", true, r"\d+\.?\d*%")
        .expect("metric", "Defect rate", true)
        .expect("analysis", "Equipment utilization", true),

        // Real Estate task - using JSON file
        BenchmarkTask::new(
            "RE-001",
            "real_estate",
            "You are a real estate analyst. Analyze the following property and market data (provided in TeaLeaf format) and provide market insights:\n\n{tl_data}\n\nProvide:\n1. Active listings summary by property type\n2. Average price per square foot by type\n3. Days on market analysis\n4. Transaction volume and average sale price\n5. Market conditions assessment (buyer's vs seller's market)\n6. Investment recommendations based on the data"
        )
        .with_json_file("accuracy-benchmark/tasks/real_estate/data/properties.json")
        .with_complexity(accuracy_benchmark::tasks::Complexity::Complex)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Recommendation)
        .expect("metric", "Price per sqft", true)
        .expect("analysis", "Market conditions", true)
        .expect("recommendation", "Investment guidance", true),

        // Legal task - using JSON file
        BenchmarkTask::new(
            "LEG-001",
            "legal",
            "You are a legal analyst. Analyze the following contract portfolio (provided in TeaLeaf format) and assess compliance and risk:\n\n{tl_data}\n\nProvide:\n1. Contract portfolio summary by type\n2. Total contract value by category\n3. Risk level distribution\n4. Compliance status overview\n5. Upcoming renewals and expirations\n6. Active disputes and financial exposure\n7. Recommendations for risk mitigation"
        )
        .with_json_file("accuracy-benchmark/tasks/legal/data/contracts.json")
        .with_complexity(accuracy_benchmark::tasks::Complexity::Complex)
        .with_output_type(accuracy_benchmark::tasks::OutputType::Analysis)
        .expect("summary", "Contract portfolio", true)
        .expect("analysis", "Risk assessment", true)
        .expect("recommendation", "Risk mitigation", true),
    ]
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

fn list_tasks(tasks_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let tasks = load_tasks(tasks_path, None)?;

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
