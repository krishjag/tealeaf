# Accuracy Benchmark

The accuracy benchmark suite evaluates LLM providers' ability to analyze structured data across three formats: **TeaLeaf**, **JSON**, and **TOON**. It converts JSON source data into each format, sends analysis prompts to multiple providers, and scores the responses.

## Overview

The workflow:

1. Loads task definitions from JSON files (`synthetic.json` or `real.json`)
2. Converts source data to TeaLeaf, JSON, and TOON formats
3. Sends analysis prompts to multiple LLM providers
4. Evaluates and compares the responses using a scoring framework

## Supported Providers

| Provider | Environment Variable | Model |
|----------|---------------------|-------|
| **Anthropic** | `ANTHROPIC_API_KEY` | Claude Sonnet 4.5 (Extended Thinking) |
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

- **Synthetic** (default) -- 12 tasks across 10 business domains with small, hand-crafted datasets. Definitions in `tasks/synthetic.json`.
- **Real** -- 2 complex financial analysis tasks using SEC EDGAR 10-K filing data (Apple, Visa, Costco, Qualcomm). Definitions in `tasks/real.json`.

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

| ID | Domain | Complexity | Description |
|----|--------|------------|-------------|
| FIN-001 | Finance | Complex | Balance sheet analysis, current ratio, profit margin, cross-company ranking |
| FIN-002 | Finance | Complex | Debt-to-equity, free cash flow, interest coverage, risk flags |

## Task Definition Format

Tasks are defined in JSON files -- no Rust code changes needed to add or modify tasks:

```json
{
  "version": "1.0",
  "tasks": [
    {
      "id": "FIN-001",
      "category": "finance",
      "complexity": "simple",
      "output_type": "calculation",
      "prompt_template": "Analyze the data (provided in {format_name} format):\n\n{data}\n\nCalculate totals.",
      "data_file": "finance/synthetic-data/financial_statement_simple.json",
      "expected_elements": [
        {"element_type": "metric", "description": "Total revenue", "required": true, "validation_pattern": "\\$[\\d,]+"}
      ]
    }
  ]
}
```

The `{data}` placeholder is replaced with the task data rendered in the current format (TeaLeaf, JSON, or TOON). The `{format_name}` placeholder is replaced with the human-readable format name ("TeaLeaf", "JSON", or "TOON").

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

Each task defines expected elements with optional regex validation:

- **Without pattern:** checks for keyword presence from description
- **With pattern:** validates using regex (e.g., `\$[\d,]+` for dollar amounts, `\d+\.?\d*%` for percentages)

### Scoring Rubrics

Different rubrics apply based on output type:

| Output Type | Key Criteria |
|-------------|--------------|
| **Calculation** | Numeric content (5+ numbers), structured output |
| **Analysis** | Depth, structure, evidence with data |
| **Recommendation** | Actionable language, prioritization, justification |
| **Summary** | Completeness, conciseness, organization |

## Three-Format Comparison Results

Run with `--compare-formats` to compare TeaLeaf vs JSON vs TOON input efficiency.

### Real-World Results (SEC EDGAR 10-K Data)

*4 companies, 399 line items, ~196KB JSON baseline. Claude Sonnet 4.5 and GPT-5.2.*

| | TL | JSON | TOON |
|---|---|---|---|
| **Anthropic accuracy** | 0.952 | 0.960 | 0.935 |
| **OpenAI accuracy** | 0.927 | 0.933 | 0.886 |
| **Input savings (Anthropic)** | -43.5% | baseline | -43.0% |
| **Input savings (OpenAI)** | -43.4% | baseline | -42.3% |

### Synthetic Results (12 Tasks, 10 Domains)

*Small datasets. Claude Sonnet 4.5 and GPT-5.2.*

| Provider | TL Score | JSON Score | Input Savings (TL) |
|----------|----------|------------|---------------------|
| **anthropic** | 0.988 | 0.978 | **-30.0%** |
| **openai** | 0.901 | 0.899 | **-31.3%** |

### Key Findings

- **~43% input token savings** on real-world data (TL and TOON both vs JSON)
- **~30% input token savings** on synthetic data (smaller datasets dilute savings)
- **No accuracy loss** -- scores within noise across all three formats
- TeaLeaf's advantage increases with nesting depth (schema inference + positional encoding)
- TOON achieves similar byte-level savings to TL via different notation trade-offs
- All results are captured in `analysis.tl` with `@struct` schemas and `@table` format comparison tables

> **Sample Results:** Reference benchmark results are available in [`accuracy-benchmark/results/`](https://github.com/krishjag/tealeaf/tree/main/accuracy-benchmark/results) in the repository.

## Output Files

Results are saved to `accuracy-benchmark/results/runs/{run-id}/`:

| File | Description |
|------|-------------|
| `analysis.tl` | Full results in TeaLeaf format with schema definitions |
| `summary.json` | Aggregated scores and rankings |
| `responses/` | Raw API responses (with `--save-responses`) |

Response files are named `{task-id}-{provider}-{format}.txt` in format comparison mode, or `{task-id}-{provider}.txt` in single-format mode.

### analysis.tl Structure

The `analysis.tl` output uses `@struct` schema definitions for self-documenting positional tables:

```tl
# Schema definitions
@struct api_response (task_id: string, provider: string, model: string?,
    input_tokens: int, output_tokens: int, latency_ms: int,
    timestamp: timestamp, status: string)
@struct analysis_result (task_id: string, provider: string,
    completeness: float, relevance: float, coherence: float,
    factual_accuracy: float)
@struct comparison_result (task_id: string, providers_ranked: []string,
    winner: string?, margin: float?)

# When --compare-formats is used, format comparison schemas are also included:
@struct format_accuracy (provider: string, format: string,
    avg_score: float, wins: int)
@struct format_tokens (provider: string, format: string,
    input_tokens: int, output_tokens: int, total_tokens: int)
```

The file contains these sections:

| Section | Description |
|---------|-------------|
| `run_metadata` | Run ID, timestamps, task count, providers |
| `responses` | `@table api_response` -- per-task API response details |
| `analysis_results` | `@table analysis_result` -- per-task accuracy metrics |
| `comparisons` | `@table comparison_result` -- provider rankings per task |
| `summary` | Aggregated wins, scores, category/complexity breakdowns |
| `format_accuracy` | `@table format_accuracy` -- per-provider accuracy by format (TL/JSON/TOON) |
| `format_tokens` | `@table format_tokens` -- per-provider token usage by format (TL/JSON/TOON) |

The last two tables (`format_accuracy` and `format_tokens`) are only present when `--compare-formats` is used. They capture the full three-format comparison (TeaLeaf vs JSON vs TOON) in structured form, enabling downstream analysis of format trade-offs.

## Adding Custom Tasks

Create a JSON definition file:

```json
{
  "tasks": [
    {
      "id": "CUSTOM-001",
      "category": "custom",
      "complexity": "moderate",
      "output_type": "analysis",
      "prompt_template": "Analyze (provided in {format_name} format):\n\n{data}\n\nProvide summary.",
      "data_file": "custom/data/my_data.json",
      "expected_elements": [
        {"element_type": "summary", "description": "Overview", "required": true}
      ]
    }
  ]
}
```

Then run:

```bash
cargo run -p accuracy-benchmark -- run --tasks path/to/custom-tasks.json
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
│   ├── main.rs              # CLI (clap), benchmark orchestration
│   ├── lib.rs               # Library exports
│   ├── config.rs            # Configuration, DataFormat enum
│   ├── providers/           # LLM provider clients
│   ├── tasks/               # Task loading and execution
│   │   ├── mod.rs           # BenchmarkTask, format conversion
│   │   └── loader.rs        # JSON + TeaLeaf file loaders
│   ├── runner/              # Parallel execution + rate limiting
│   ├── analysis/            # Scoring and comparison
│   └── reporting/           # TeaLeaf + JSON output
├── config/models.toml       # Provider/model configuration
├── tasks/
│   ├── synthetic.json       # 12 synthetic task definitions
│   ├── real.json            # 2 real-world task definitions
│   ├── finance/
│   │   ├── synthetic-data/  # Hand-crafted datasets
│   │   └── data/            # SEC EDGAR data + processing script
│   ├── retail/synthetic-data/
│   ├── healthcare/synthetic-data/
│   └── ...                  # 10 domain directories
├── results/runs/            # Archived benchmark runs
└── Cargo.toml
```
