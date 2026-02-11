# TeaLeaf Accuracy Benchmark Suite

A benchmark suite for evaluating LLM providers' ability to analyze structured data across three formats: **TeaLeaf**, **JSON**, and **TOON**.

## Overview

This benchmark suite:
1. Takes JSON source data from various business domains
2. Converts it to TeaLeaf, JSON, and TOON formats
3. Sends analysis prompts to multiple LLM providers
4. Evaluates and compares the responses across formats

## Supported Providers

| Provider | Environment Variable | Models |
|----------|---------------------|--------|
| **Anthropic** | `ANTHROPIC_API_KEY` | Claude Sonnet 4.5 (Extended Thinking) |
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
# Run with synthetic data (12 tasks, 10 domains)
cargo run -p accuracy-benchmark -- run

# Run with real-world SEC EDGAR data (2 tasks)
cargo run -p accuracy-benchmark -- run --data-source real

# Compare TeaLeaf vs JSON vs TOON format performance
cargo run -p accuracy-benchmark -- run --compare-formats

# Save raw API responses to files
cargo run -p accuracy-benchmark -- run --compare-formats --save-responses

# Run with specific providers
cargo run -p accuracy-benchmark -- run --providers anthropic,openai

# Run specific categories only
cargo run -p accuracy-benchmark -- run --categories finance,retail

# Custom output directory
cargo run -p accuracy-benchmark -- run -o my-results/

# Verbose output
cargo run -p accuracy-benchmark -- -v run
```

### Other Commands

```bash
# List available tasks
cargo run -p accuracy-benchmark -- list-tasks
cargo run -p accuracy-benchmark -- list-tasks --data-source real

# Dump rendered prompts (all 3 formats) without calling APIs
cargo run -p accuracy-benchmark -- dump-prompts --data-source synthetic --output prompts/

# Generate configuration template
cargo run -p accuracy-benchmark -- init-config -o my-config.toml
```

## Data Sources

The benchmark supports two data sources via `--data-source`:

### Synthetic (default)

12 tasks across 10 business domains with small, hand-crafted datasets. Task definitions in `tasks/synthetic.json`, data files in `tasks/{domain}/synthetic-data/`.

### Real

2 complex financial analysis tasks using real SEC EDGAR 10-K filing data (Apple, Visa, Costco, Qualcomm). Task definitions in `tasks/real.json`, data in `tasks/finance/data/sec_edgar_2025_q4.json`.

See [tasks/finance/data/README.md](tasks/finance/data/README.md) for data provenance.

## Benchmark Tasks

### Synthetic Tasks (12)

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

### Real-World Tasks (2)

| ID | Domain | Complexity | Output Type | Description |
|----|--------|------------|-------------|-------------|
| FIN-001 | Finance | Complex | Calculation | Balance sheet analysis, current ratio, profit margin, cross-company ranking |
| FIN-002 | Finance | Complex | Analysis | Debt-to-equity, free cash flow, interest coverage, risk flags |

## Task Definition Format

Tasks are defined in JSON files (no Rust code changes needed):

```json
{
  "version": "1.0",
  "tasks": [
    {
      "id": "FIN-001",
      "category": "finance",
      "complexity": "simple",
      "output_type": "calculation",
      "prompt_template": "Analyze the following data (provided in {format_name} format):\n\n{data}\n\nCalculate totals.",
      "data_file": "finance/synthetic-data/financial_statement_simple.json",
      "expected_elements": [
        {"element_type": "metric", "description": "Total revenue", "required": true, "validation_pattern": "\\$[\\d,]+"}
      ]
    }
  ]
}
```

### Placeholders

| Placeholder | Substitution |
|-------------|-------------|
| `{data}` | The task data rendered in the current format (TeaLeaf, JSON, or TOON) |
| `{format_name}` | Human-readable format name: "TeaLeaf", "JSON", or "TOON" |

### Task Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `id` | yes | | Task identifier (e.g., "FIN-001") |
| `category` | yes | | Domain category |
| `complexity` | no | `moderate` | `simple`, `moderate`, or `complex` |
| `output_type` | no | `analysis` | `calculation`, `analysis`, `recommendation`, or `summary` |
| `prompt_template` | yes | | Prompt with `{data}` and optional `{format_name}` placeholders |
| `data_file` | no | | Path to JSON data file, relative to the definition file's directory |
| `max_tokens` | no | `2048` | Max response tokens |
| `temperature` | no | `0.3` | Sampling temperature |
| `expected_elements` | no | `[]` | Elements to detect in the response |
| `grading_rubric` | no | | Custom grading criteria |

## Three-Format Comparison

The `--compare-formats` flag runs each task in all three formats:

- **TeaLeaf (TL)** -- compact schema-aware format using `@struct` + `@table` positional encoding
- **JSON** -- standard pretty-printed JSON (baseline)
- **TOON** -- Token-Oriented Object Notation with tabular folding

### Real-World Results (SEC EDGAR 10-K Data)

*4 companies, 399 line items, ~196KB JSON baseline. Claude Sonnet 4.5 and GPT-5.2.*

| | TL | JSON | TOON |
|---|---|---|---|
| **Anthropic accuracy** | 0.943 | 0.960 | 0.952 |
| **OpenAI accuracy** | 0.935 | 0.935 | 0.892 |
| **Input savings (Anthropic)** | -41.9% | baseline | -43.0% |
| **Input savings (OpenAI)** | -41.5% | baseline | -42.3% |

### Synthetic Results (12 Tasks, 10 Domains)

*Small datasets. Claude Sonnet 4.5 and GPT-5.2.*

| Provider | TL Score | JSON Score | Input Savings (TL) |
|----------|----------|------------|---------------------|
| **anthropic** | 0.988 | 0.978 | **-30.0%** |
| **openai** | 0.901 | 0.899 | **-31.3%** |

### Key Findings

- **~42% input token savings** on real-world data (TL and TOON both vs JSON)
- **~30% input token savings** on synthetic data (smaller datasets dilute savings)
- **No accuracy loss** -- scores within noise across all three formats
- TeaLeaf's advantage increases with nesting depth (schema inference + positional encoding)
- TOON edges out TL by ~1% on tokenization despite being larger in bytes

## Analysis Framework

### Accuracy Metrics

| Metric | Weight | Description |
|--------|--------|-------------|
| **Completeness** | 25% | Were all expected elements addressed? |
| **Relevance** | 25% | How relevant is the response to the task? |
| **Coherence** | 20% | Is the response well-structured? |
| **Factual Accuracy** | 20% | Do values match validation patterns? |
| **Actionability** | 10% | For recommendations -- are they actionable? |

### Element Detection

Expected elements are defined in the task JSON:

```json
{"element_type": "metric", "description": "Total revenue", "required": true}
{"element_type": "metric", "description": "Percentage", "required": true, "validation_pattern": "\\d+\\.?\\d*%"}
```

- **Without pattern**: Checks for keyword presence from description
- **With pattern**: Validates using regex (e.g., `\$[\d,]+` for dollar amounts)

### Scoring Rubrics

| Output Type | Key Criteria |
|-------------|--------------|
| **Calculation** | Numeric content (5+ numbers), structured output |
| **Analysis** | Depth, structure, evidence with data |
| **Recommendation** | Actionable language, prioritization, justification |
| **Summary** | Completeness, conciseness, organization |

## Output Files

Results are saved to `accuracy-benchmark/results/runs/{run-id}/`:

| File | Description |
|------|-------------|
| `analysis.tl` | Full results in TeaLeaf format |
| `summary.json` | Aggregated scores and rankings |
| `responses/` | Raw API responses (with `--save-responses`) |

Response files are named `{task-id}-{provider}-{format}.txt` in format comparison mode, or `{task-id}-{provider}.txt` in single-format mode.

## Directory Structure

```
accuracy-benchmark/
├── src/
│   ├── main.rs              # CLI (clap), benchmark orchestration
│   ├── lib.rs               # Library exports
│   ├── config.rs            # Configuration, DataFormat enum
│   ├── providers/           # LLM provider clients
│   │   ├── traits.rs        # LLMProvider trait
│   │   ├── anthropic.rs     # Claude (Extended Thinking)
│   │   └── openai.rs        # GPT
│   ├── tasks/               # Task loading and execution
│   │   ├── mod.rs           # BenchmarkTask, DataSource, format conversion
│   │   ├── categories.rs    # Domain, Complexity, OutputType
│   │   └── loader.rs        # JSON + TeaLeaf file loaders
│   ├── runner/              # Execution engine
│   │   ├── executor.rs      # Parallel task execution (per-format)
│   │   └── rate_limiter.rs  # RPM/TPM rate limiting
│   ├── analysis/            # Response analysis
│   │   ├── metrics.rs       # AccuracyMetrics
│   │   ├── scoring.rs       # ScoringRubric
│   │   └── comparator.rs    # Cross-provider comparison
│   └── reporting/           # Output generation
│       ├── tl_writer.rs     # TeaLeaf format output
│       └── json_export.rs   # JSON summary
├── config/
│   └── models.toml          # Provider/model configuration
├── tasks/                   # Task definitions and data
│   ├── synthetic.json       # 12 synthetic task definitions
│   ├── real.json            # 2 real-world task definitions
│   ├── finance/
│   │   ├── synthetic-data/  # Small hand-crafted datasets
│   │   └── data/            # Real SEC EDGAR data + processing script
│   ├── retail/synthetic-data/
│   ├── healthcare/synthetic-data/
│   ├── technology/synthetic-data/
│   ├── marketing/synthetic-data/
│   ├── logistics/synthetic-data/
│   ├── hr/synthetic-data/
│   ├── manufacturing/synthetic-data/
│   ├── real_estate/synthetic-data/
│   └── legal/synthetic-data/
├── results/runs/            # Archived benchmark runs
└── Cargo.toml
```

## Adding Custom Tasks

### 1. Create a Data File

```json
// tasks/custom/data/my_data.json
{
  "items": [
    {"id": "A", "value": 100},
    {"id": "B", "value": 200}
  ]
}
```

### 2. Add Task to a JSON Definition File

```json
{
  "tasks": [
    {
      "id": "CUSTOM-001",
      "category": "custom",
      "complexity": "moderate",
      "output_type": "analysis",
      "prompt_template": "Analyze the following data (provided in {format_name} format):\n\n{data}\n\nProvide summary and recommendations.",
      "data_file": "custom/data/my_data.json",
      "expected_elements": [
        {"element_type": "summary", "description": "Data overview", "required": true},
        {"element_type": "metric", "description": "Total value", "required": true, "validation_pattern": "\\d+"}
      ]
    }
  ]
}
```

### 3. Run

```bash
# Load from custom file
cargo run -p accuracy-benchmark -- run --tasks path/to/custom-tasks.json

# Or place in the tasks/ directory and use --data-source
cargo run -p accuracy-benchmark -- list-tasks --tasks path/to/custom-tasks.json
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
