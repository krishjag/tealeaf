# Accuracy Benchmark

The accuracy benchmark suite evaluates LLM providers' ability to analyze structured data in TeaLeaf format. It sends analysis prompts with TeaLeaf-formatted business data to multiple providers and scores the responses.

## Overview

The workflow:

1. Takes JSON data from various business domains
2. Converts it to TeaLeaf format using `tealeaf-core`
3. Sends analysis prompts to multiple LLM providers
4. Evaluates and compares the responses using a scoring framework

## Supported Providers

| Provider | Environment Variable | Model |
|----------|---------------------|-------|
| **Anthropic** | `ANTHROPIC_API_KEY` | Claude Opus 4.5 (Extended Thinking) |
| **OpenAI** | `OPENAI_API_KEY` | GPT-5.2 |

## Installation

### Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/krishjag/tealeaf/releases):

| Platform | Architecture | File |
|----------|--------------|------|
| Windows | x64 | `accuracy-benchmark-windows-x64.zip` |
| Windows | ARM64 | `accuracy-benchmark-windows-arm64.zip` |
| macOS | Intel | `accuracy-benchmark-macos-x64.tar.gz` |
| macOS | Apple Silicon | `accuracy-benchmark-macos-arm64.tar.gz` |
| Linux | x64 | `accuracy-benchmark-linux-x64.tar.gz` |
| Linux | ARM64 | `accuracy-benchmark-linux-arm64.tar.gz` |
| Linux | x64 (static) | `accuracy-benchmark-linux-musl-x64.tar.gz` |

### Build from Source

```bash
cargo build -p accuracy-benchmark --release

# Or run directly
cargo run -p accuracy-benchmark -- --help
```

## Usage

```bash
# Run with all available providers
cargo run -p accuracy-benchmark -- run

# Run with specific providers
cargo run -p accuracy-benchmark -- run --providers anthropic,openai

# Run specific categories only
cargo run -p accuracy-benchmark -- run --categories finance,retail

# Compare TeaLeaf vs JSON format performance
cargo run -p accuracy-benchmark -- run --compare-formats

# Verbose output
cargo run -p accuracy-benchmark -- -v run

# List available tasks
cargo run -p accuracy-benchmark -- list-tasks

# Generate configuration template
cargo run -p accuracy-benchmark -- init-config -o my-config.json
```

## Benchmark Tasks

The suite includes 12 tasks across 10 business domains:

| ID | Domain | Complexity | Output Type |
|----|--------|------------|-------------|
| FIN-001 | Finance | Simple | Calculation |
| FIN-002 | Finance | Moderate | Calculation |
| RET-001 | Retail | Simple | Summary |
| RET-002 | Retail | Complex | Recommendation |
| HLT-001 | Healthcare | Simple | Summary |
| TEC-001 | Technology | Moderate | Analysis |
| MKT-001 | Marketing | Moderate | Calculation |
| LOG-001 | Logistics | Moderate | Analysis |
| HR-001 | HR | Moderate | Analysis |
| MFG-001 | Manufacturing | Moderate | Calculation |
| RE-001 | Real Estate | Complex | Recommendation |
| LEG-001 | Legal | Complex | Analysis |

### Data Sources

Each task specifies input data in one of two ways:

**Inline JSON:**
```rust
BenchmarkTask::new("FIN-001", "finance", "Analyze this data:\n\n{tl_data}")
    .with_json_data(serde_json::json!({
        "revenue": 1000000,
        "expenses": 750000
    }))
```

**JSON file reference:**
```rust
BenchmarkTask::new("LOG-001", "logistics", "Analyze this data:\n\n{tl_data}")
    .with_json_file("tasks/logistics/data/shipments.json")
```

The `{tl_data}` placeholder in the prompt template is replaced with TeaLeaf-formatted data before sending to the LLM.

## Analysis Framework

### Accuracy Metrics

Responses are evaluated across five dimensions:

| Metric | Weight | Description |
|--------|--------|-------------|
| **Completeness** | 25% | Were all expected elements addressed? |
| **Relevance** | 25% | How relevant is the response to the task? |
| **Coherence** | 20% | Is the response well-structured? |
| **Factual Accuracy** | 20% | Do values match validation patterns? |
| **Actionability** | 10% | For recommendations -- are they actionable? |

### Element Detection

Each task defines expected elements that should appear in the response:

```rust
// Keyword presence check
.expect("metric", "Total revenue calculation", true)

// Regex pattern validation
.expect_with_pattern("metric", "Percentage value", true, r"\d+\.?\d*%")
```

- **Without pattern:** checks for keyword presence from description
- **With pattern:** validates using regex (e.g., `\$[\d,]+` for dollar amounts)

### Scoring Rubrics

Different rubrics apply based on output type:

| Output Type | Key Criteria |
|-------------|--------------|
| **Calculation** | Numeric content (5+ numbers), structured output |
| **Analysis** | Depth, structure, evidence with data |
| **Recommendation** | Actionable language, prioritization, justification |
| **Summary** | Completeness, conciseness, organization |

### Coherence Checks

- Structure markers: `##`, `###`, `**`, `-`, numbered lists
- Paragraph breaks (3+ paragraphs preferred)
- Reasonable length (100-2000 words)

### Actionability Keywords

For recommendation tasks, these keywords are detected:

- recommend, should, suggest, consider, advise
- action, implement, improve, optimize, prioritize
- next step, immediate, critical, important

## Format Comparison Results

Run with `--compare-formats` to compare TeaLeaf vs JSON input efficiency:

| Provider | TeaLeaf Score | JSON Score | Accuracy Diff | TeaLeaf Input | JSON Input | Input Savings |
|----------|---------------|------------|---------------|---------------|------------|---------------|
| **anthropic** | 0.970 | 0.983 | -0.013 | 4,939 | 8,251 | **-40.1%** |
| **openai** | 0.925 | 0.905 | +0.021 | 4,866 | 7,076 | **-31.2%** |

*Input tokens = data sent to the model. Output tokens vary by model verbosity.*

**Key findings:**

| Provider | Accuracy | Input Token Efficiency |
|----------|----------|------------------------|
| **anthropic** | Comparable | TeaLeaf uses 40% fewer input tokens |
| **openai** | TeaLeaf +2.1% better | TeaLeaf uses 31% fewer input tokens |

TeaLeaf format consistently uses **30-40% fewer input tokens** than JSON due to its more compact structure, while maintaining comparable or better accuracy.

## Output Files

Results are saved in two formats:

### TeaLeaf Format (`analysis.tl`)

```tl
# Accuracy Benchmark Results
# Generated: 2026-01-27 19:53:08 UTC

run_metadata: {
    run_id: "20260127-195139",
    started_at: 2026-01-27T19:51:39Z,
    completed_at: 2026-01-27T19:53:08Z,
    total_tasks: 12,
    providers: [anthropic, openai]
}

responses: @table api_response [
    (FIN-001, openai, "gpt-4-turbo-2024-04-09", 314, 705, 31975, 2026-01-27T19:52:24Z, success),
    (FIN-001, anthropic, "claude-sonnet-4-20250514", 365, 462, 6957, 2026-01-27T19:51:46Z, success),
    ...
]

analysis_results: @table analysis_result [
    (FIN-001, openai, 1.000, 1.000, 1.000, 1.000),
    (FIN-001, anthropic, 1.000, 0.625, 1.000, 1.000),
    ...
]

comparisons: @table comparison_result [
    (FIN-001, [openai, anthropic], openai, 0.094),
    (RET-001, [anthropic, openai], anthropic, 0.283),
    ...
]

summary: {
    total_tasks: 12,
    wins: { anthropic: 6, openai: 6 },
    avg_scores: { anthropic: 0.957, openai: 0.923 },
    by_category: { ... },
    by_complexity: { ... }
}
```

### JSON Summary (`summary.json`)

```json
{
  "run_id": "20260127-195139",
  "total_tasks": 12,
  "provider_rankings": [
    { "provider": "anthropic", "wins": 6, "avg_score": 0.957 },
    { "provider": "openai", "wins": 6, "avg_score": 0.923 }
  ],
  "category_breakdown": {
    "retail": { "leader": "anthropic", "margin": 0.192 },
    "finance": { "leader": "openai", "margin": 0.047 },
    ...
  },
  "detailed_results_file": "analysis.tl"
}
```

## Adding Custom Tasks

### From JSON Data

```rust
BenchmarkTask::new(
    "CUSTOM-001",
    "custom_category",
    "Analyze this data:\n\n{tl_data}\n\nProvide summary and recommendations."
)
.with_json_file("tasks/custom/data/my_data.json")
.with_complexity(Complexity::Moderate)
.with_output_type(OutputType::Analysis)
.expect("summary", "Data overview", true)
.expect_with_pattern("metric", "Total value", true, r"\d+")
```

### From TeaLeaf File

```bash
cargo run -p accuracy-benchmark -- run --tasks path/to/tasks.tl
```

## Extending Providers

Implement the `LLMProvider` trait:

```rust
#[async_trait]
impl LLMProvider for NewProviderClient {
    fn name(&self) -> &str { "newprovider" }

    async fn complete(&self, request: CompletionRequest) -> ProviderResult<CompletionResponse> {
        // Implementation
    }
}
```

Then register in `src/providers/mod.rs` via `create_all_providers()` and `create_providers()`.

## Directory Structure

```
accuracy-benchmark/
├── src/
│   ├── main.rs           # CLI interface (clap)
│   ├── lib.rs            # Library exports
│   ├── config.rs         # Configuration management
│   ├── providers/        # LLM provider clients
│   │   ├── traits.rs     # LLMProvider trait
│   │   ├── anthropic.rs  # Claude implementation
│   │   └── openai.rs     # GPT implementation
│   ├── tasks/            # Task definitions
│   │   ├── mod.rs        # BenchmarkTask, DataSource
│   │   ├── categories.rs # Domain, Complexity, OutputType
│   │   └── loader.rs     # TeaLeaf file loader
│   ├── runner/           # Execution engine
│   │   ├── executor.rs   # Parallel task execution
│   │   └── rate_limiter.rs
│   ├── analysis/         # Response analysis
│   │   ├── metrics.rs    # AccuracyMetrics
│   │   ├── scoring.rs    # ScoringRubric
│   │   └── comparator.rs # Cross-provider comparison
│   └── reporting/        # Output generation
│       └── tl_writer.rs  # TeaLeaf format output
├── tasks/                # Sample data by domain
│   ├── finance/data/
│   ├── healthcare/data/
│   ├── retail/data/
│   └── ...
├── results/runs/         # Archived run results
└── Cargo.toml
```
