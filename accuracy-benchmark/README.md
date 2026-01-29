# TeaLeaf Accuracy Benchmark Suite

A comprehensive benchmark suite for evaluating LLM providers' ability to analyze structured data in TeaLeaf format.

## Overview

This benchmark suite:
1. Takes JSON data from various business domains
2. Converts it to TeaLeaf format using `tealeaf-core`
3. Sends analysis prompts to multiple LLM providers
4. Evaluates and compares the responses

## Supported Providers

| Provider | Environment Variable | Models |
|----------|---------------------|--------|
| **Anthropic** | `ANTHROPIC_API_KEY` | Claude Opus 4.5 (Extended Thinking) |
| **OpenAI** | `OPENAI_API_KEY` | GPT-5.2 |

## Installation

### Pre-built Binaries (Recommended)

Download the latest release for your platform from the [GitHub Releases](https://github.com/krishjag/tealeaf/releases) page:

| Platform | Architecture | File |
|----------|--------------|------|
| **Windows** | x64 | `accuracy-benchmark-windows-x64.zip` |
| **Windows** | ARM64 | `accuracy-benchmark-windows-arm64.zip` |
| **macOS** | Intel | `accuracy-benchmark-macos-x64.tar.gz` |
| **macOS** | Apple Silicon | `accuracy-benchmark-macos-arm64.tar.gz` |
| **Linux** | x64 | `accuracy-benchmark-linux-x64.tar.gz` |
| **Linux** | ARM64 | `accuracy-benchmark-linux-arm64.tar.gz` |
| **Linux** | x64 (static) | `accuracy-benchmark-linux-musl-x64.tar.gz` |

```bash
# Example: Download and install on Linux x64
curl -LO https://github.com/krishjag/tealeaf/releases/latest/download/accuracy-benchmark-linux-x64.tar.gz
tar -xzf accuracy-benchmark-linux-x64.tar.gz
sudo mv accuracy-benchmark /usr/local/bin/

# Example: Download and install on macOS Apple Silicon
curl -LO https://github.com/krishjag/tealeaf/releases/latest/download/accuracy-benchmark-macos-arm64.tar.gz
tar -xzf accuracy-benchmark-macos-arm64.tar.gz
sudo mv accuracy-benchmark /usr/local/bin/
```

### Build from Source

```bash
# Build the benchmark suite
cargo build -p accuracy-benchmark --release

# Or run directly
cargo run -p accuracy-benchmark -- --help
```

## Usage

### Run Benchmark

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
```

### List Available Tasks

```bash
cargo run -p accuracy-benchmark -- list-tasks
```

### Generate Configuration

```bash
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

## JSON to TeaLeaf Workflow

Each task can specify input data in two ways:

### Inline JSON Data

```rust
BenchmarkTask::new("FIN-001", "finance", "Analyze this data:\n\n{tl_data}")
    .with_json_data(serde_json::json!({
        "revenue": 1000000,
        "expenses": 750000
    }))
```

### JSON File Reference

```rust
BenchmarkTask::new("LOG-001", "logistics", "Analyze this data:\n\n{tl_data}")
    .with_json_file("tasks/logistics/data/shipments.json")
```

The `{tl_data}` placeholder in the prompt template is replaced with the TeaLeaf-formatted data before sending to the LLM.

## Analysis Framework

Responses are evaluated using multiple metrics:

### Accuracy Metrics

| Metric | Weight | Description |
|--------|--------|-------------|
| **Completeness** | 25% | Were all expected elements addressed? |
| **Relevance** | 25% | How relevant is the response to the task? |
| **Coherence** | 20% | Is the response well-structured? |
| **Factual Accuracy** | 20% | Do values match validation patterns? |
| **Actionability** | 10% | For recommendations - are they actionable? |

### Element Detection

Each task defines expected elements that should appear in the response:

```rust
.expect("metric", "Total revenue calculation", true)
.expect_with_pattern("metric", "Percentage value", true, r"\d+\.?\d*%")
```

- **Without pattern**: Checks for keyword presence from description
- **With pattern**: Validates using regex (e.g., `\$[\d,]+` for dollar amounts)

### Scoring Rubrics

Different rubrics are applied based on task output type:

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

## Example Run

Run with `--compare-formats` to compare TeaLeaf vs JSON performance:

```bash
cargo run -p accuracy-benchmark -- run --compare-formats
```

### Format Comparison Summary

| Provider | TeaLeaf Score | JSON Score | Accuracy Diff | TeaLeaf Input | JSON Input | Input Savings |
|----------|---------------|------------|---------------|---------------|------------|---------------|
| **anthropic** | 0.970 | 0.983 | -0.013 | 4,939 | 8,251 | **-40.1%** |
| **openai** | 0.925 | 0.905 | +0.021 | 4,866 | 7,076 | **-31.2%** |

*Input tokens = data sent to the model. Output tokens vary by model verbosity and are not shown.*

### Key Findings

| Provider | Accuracy | Input Token Efficiency |
|----------|----------|------------------------|
| **anthropic** | Comparable | TeaLeaf uses 40% fewer input tokens |
| **openai** | TeaLeaf +2.1% better | TeaLeaf uses 31% fewer input tokens |

**Bottom Line:** TeaLeaf format consistently uses **30-40% fewer input tokens** than JSON due to its more compact structure, while maintaining comparable or better accuracy.

## Output Files

Results are saved in two formats:

### TeaLeaf Format (`analysis.tl`)

```
# Accuracy Benchmark Results
# Generated: 2026-01-27 19:53:08 UTC

run_metadata: {
    run_id: "20260127-195139",
    started_at: 2026-01-27T19:51:39Z,
    completed_at: 2026-01-27T19:53:08Z,
    total_tasks: 12,
    providers: [anthropic, openai]
}

# Task Results - (task_id, provider, model, input_tokens, output_tokens, latency_ms, timestamp, status)
responses: @table api_response [
    (FIN-001, openai, "gpt-4-turbo-2024-04-09", 314, 705, 31975, 2026-01-27T19:52:24Z, success),
    (FIN-001, anthropic, "claude-sonnet-4-20250514", 365, 462, 6957, 2026-01-27T19:51:46Z, success),
    (FIN-002, anthropic, "claude-sonnet-4-20250514", 217, 661, 9454, 2026-01-27T19:51:48Z, success),
    (FIN-002, openai, "gpt-4-turbo-2024-04-09", 198, 1040, 45155, 2026-01-27T19:52:40Z, success),
    ...
]

# Analysis Results - (task_id, provider, completeness, relevance, coherence, factual_accuracy)
analysis_results: @table analysis_result [
    (FIN-001, openai, 1.000, 1.000, 1.000, 1.000),
    (FIN-001, anthropic, 1.000, 0.625, 1.000, 1.000),
    (FIN-002, openai, 1.000, 1.000, 1.000, 1.000),
    (FIN-002, anthropic, 1.000, 1.000, 1.000, 1.000),
    (RET-001, openai, 0.667, 1.000, 1.000, 0.000),
    (RET-001, anthropic, 1.000, 1.000, 1.000, 1.000),
    ...
]

# Comparisons - (task_id, providers_ranked, winner, margin)
comparisons: @table comparison_result [
    (FIN-001, [openai, anthropic], openai, 0.094),
    (FIN-002, [openai, anthropic], openai, 0.000),
    (RET-001, [anthropic, openai], anthropic, 0.283),
    (RET-002, [anthropic, openai], anthropic, 0.100),
    ...
]

# Summary
summary: {
    total_tasks: 12,
    wins: {
        anthropic: 6,
        openai: 6,
    },
    avg_scores: {
        anthropic: 0.957,
        openai: 0.923,
    },
    by_category: {
        logistics: { anthropic: 1.000, openai: 0.964 },
        finance: { anthropic: 0.953, openai: 1.000 },
        retail: { anthropic: 1.000, openai: 0.808 },
        ...
    },
    by_complexity: {
        Complex: { anthropic: 0.986, openai: 0.939 },
        Simple: { anthropic: 0.946, openai: 0.883 },
        Moderate: { anthropic: 0.947, openai: 0.935 },
    }
}
```

### JSON Summary (`summary.json`)

```json
{
  "run_id": "20260127-195139",
  "timestamp": "2026-01-27T19:53:08.338356100+00:00",
  "total_tasks": 12,
  "provider_rankings": [
    {
      "provider": "anthropic",
      "wins": 6,
      "avg_score": 0.956512896825397
    },
    {
      "provider": "openai",
      "wins": 6,
      "avg_score": 0.9228968253968256
    }
  ],
  "category_breakdown": {
    "logistics": { "leader": "anthropic", "margin": 0.036 },
    "retail": { "leader": "anthropic", "margin": 0.192 },
    "finance": { "leader": "openai", "margin": 0.047 },
    "legal": { "leader": "anthropic", "margin": 0.042 },
    "hr": { "leader": "anthropic", "margin": 0.042 },
    "technology": { "leader": "openai", "margin": 0.0 },
    "marketing": { "leader": "openai", "margin": 0.006 },
    "real_estate": { "leader": "openai", "margin": 0.0 },
    "healthcare": { "leader": "anthropic", "margin": 0.0 },
    "manufacturing": { "leader": "openai", "margin": 0.0 }
  },
  "detailed_results_file": "analysis.tl"
}
```

## Directory Structure

```
accuracy-benchmark/
├── src/
│   ├── main.rs           # CLI interface
│   ├── lib.rs            # Library exports
│   ├── config.rs         # Configuration
│   ├── providers/        # LLM provider clients
│   │   ├── anthropic.rs
│   │   ├── openai.rs
│   │   └── traits.rs
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
│       ├── tl_writer.rs
│       └── json_export.rs
├── tasks/                # Sample data by domain
│   ├── finance/data/
│   ├── healthcare/data/
│   ├── retail/data/
│   └── ...
└── Cargo.toml
```

## Adding Custom Tasks

### 1. Create JSON Data File

```json
// tasks/custom/data/my_data.json
{
  "items": [
    {"id": "A", "value": 100},
    {"id": "B", "value": 200}
  ]
}
```

### 2. Define Task in Code

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

### 3. Or Load from TeaLeaf File

Create a `.tl` file with task definitions:

```
tasks: [
  {
    metadata: {id: CUSTOM-001, category: custom, complexity: moderate}
    prompt_template: "Analyze: {tl_data}"
    expected_elements: [
      {element_type: summary, description: "Overview", required: true}
    ]
  }
]
```

Then load:
```bash
cargo run -p accuracy-benchmark -- run --tasks path/to/tasks.tl
```

## Extending Providers

To add a new LLM provider:

1. Create `src/providers/newprovider.rs` implementing `LLMProvider` trait
2. Add to `src/providers/mod.rs`
3. Update `create_all_providers()` and `create_providers()`

```rust
#[async_trait]
impl LLMProvider for NewProviderClient {
    fn name(&self) -> &str { "newprovider" }

    async fn complete(&self, request: CompletionRequest) -> ProviderResult<CompletionResponse> {
        // Implementation
    }
}
```

## License

Same as the parent `tealeaf` project.
