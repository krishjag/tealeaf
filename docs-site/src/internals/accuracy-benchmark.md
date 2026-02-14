# Accuracy Benchmark

The accuracy benchmark suite evaluates LLM providers' ability to analyze structured data across three formats: **TeaLeaf**, **JSON**, and **TOON**. It converts JSON source data into each format, sends analysis prompts to multiple providers, and scores the responses.

For the latest benchmark results, scoring analysis, and evidence packages, see the [accuracy-benchmark README](https://github.com/krishjag/tealeaf/tree/main/accuracy-benchmark).

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

## CLI Reference

### Run Benchmark

```bash
# Run with synthetic data (12 tasks, 10 domains)
cargo run -p accuracy-benchmark -- run

# Run with real-world data (14 tasks, 7 domains)
cargo run -p accuracy-benchmark -- run --data-source real

# Compare TeaLeaf vs JSON vs TOON format performance
cargo run -p accuracy-benchmark -- run --compare-formats

# Save raw API responses to files
cargo run -p accuracy-benchmark -- run --compare-formats --save-responses

# Run with specific providers
cargo run -p accuracy-benchmark -- run --providers anthropic,openai

# Run specific categories only
cargo run -p accuracy-benchmark -- run --categories finance,retail

# Run specific task IDs only
cargo run -p accuracy-benchmark -- run --task-ids RE-001,RE-002

# Run specific formats only (implies --compare-formats)
cargo run -p accuracy-benchmark -- run --formats tl,json

# Combine filters: specific tasks, providers, and formats
cargo run -p accuracy-benchmark -- run --task-ids RE-001 --providers openai --formats tl --data-source real

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

### Generate Charts

```bash
# From a specific run
python accuracy-benchmark/scripts/generate_charts.py --results-dir accuracy-benchmark/results/<run-id>

# Custom output directory
python accuracy-benchmark/scripts/generate_charts.py --results-dir accuracy-benchmark/results/<run-id> -o my-charts/
```

## Data Sources

The benchmark supports two data sources via `--data-source`:

### Synthetic (default)

12 tasks across 10 business domains with small, hand-crafted datasets. Task definitions in `tasks/synthetic.json`, data files in `tasks/{domain}/synthetic-data/`.

### Real

14 tasks across 7 business domains using real-world data sources. Task definitions in `tasks/real.json`.

| Domain | Tasks | Data Source |
|--------|-------|-------------|
| Finance | FIN-001, FIN-002 | SEC EDGAR 10-K annual filings |
| Healthcare | HEALTH-001, HEALTH-002 | Clinical trials, FDA drug data |
| HR / Labor | HR-001, HR-002 | Bureau of Labor Statistics |
| Legal | LEGAL-001, LEGAL-002 | Federal court filings |
| Technology | TECH-001, TECH-002 | Patent filings, FCC spectrum data |
| Retail | RETAIL-001, RETAIL-002 | Census retail trade data |
| Real Estate | RE-001, RE-002 | NYC PLUTO (Open Data) |

See the `tasks/` subdirectories for data provenance.

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

### Real-World Tasks (14)

| ID | Domain | Complexity | Output Type |
|----|--------|------------|-------------|
| FIN-001 | Finance | Complex | Calculation |
| FIN-002 | Finance | Complex | Analysis |
| HEALTH-001 | Healthcare | Complex | Analysis |
| HEALTH-002 | Healthcare | Complex | Summary |
| HR-001 | HR / Labor | Complex | Analysis |
| HR-002 | HR / Labor | Complex | Analysis |
| LEGAL-001 | Legal | Complex | Analysis |
| LEGAL-002 | Legal | Complex | Analysis |
| TECH-001 | Technology | Complex | Analysis |
| TECH-002 | Technology | Complex | Analysis |
| RETAIL-001 | Retail | Complex | Analysis |
| RETAIL-002 | Retail | Complex | Analysis |
| RE-001 | Real Estate | Complex | Analysis |
| RE-002 | Real Estate | Complex | Analysis |

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
| `include_format_hint` | no | `{}` | Per-format hint flags, e.g. `{"tl": true}`. Hint text is loaded from `format_hints.json` |

### Format Hints

The file `tasks/format_hints.json` maps format keys to hint text that is prepended to prompts when a task opts in via `include_format_hint`:

```json
{
  "tl": "Note: The data below uses TeaLeaf format. ...",
  "json": "",
  "toon": ""
}
```

When a task has `"include_format_hint": {"tl": true}`, the `tl` hint text is prepended to the prompt for TL-format runs. This is useful for tasks with wide schemas where the LLM benefits from a brief format explanation.

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
│   ├── real.json            # 14 real-world task definitions
│   ├── finance/
│   │   ├── synthetic-data/  # Small hand-crafted datasets
│   │   └── data/            # Real SEC EDGAR data + processing script
│   ├── retail/synthetic-data/
│   ├── healthcare/
│   ├── technology/
│   ├── marketing/synthetic-data/
│   ├── logistics/synthetic-data/
│   ├── hr/
│   ├── manufacturing/synthetic-data/
│   ├── real_estate/
│   └── legal/
├── evidence/                # Dated evidence packages
├── results/                 # Benchmark run outputs
└── Cargo.toml
```

## Output Files

Results are saved to `accuracy-benchmark/results/{run-id}/`:

| File | Description |
|------|-------------|
| `analysis.tl` | Full results in TeaLeaf format with schema definitions |
| `summary.json` | Aggregated scores and rankings |
| `responses/` | Raw API responses (with `--save-responses`) |

Response files are named `{source}-{task-id}-{provider}-{format}.txt` (e.g., `real-fin-001-anthropic-tl.txt`).
