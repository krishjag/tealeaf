# TeaLeaf Accuracy Benchmark Suite

A benchmark suite for evaluating LLM providers' ability to analyze structured data across three formats: **TeaLeaf**, **JSON**, and **TOON**.

## Overview

This benchmark suite:
1. Takes JSON source data from various business domains
2. Converts it to TeaLeaf, JSON, and TOON formats
3. Sends analysis prompts to multiple LLM providers
4. Evaluates and compares the responses across formats

## Latest Results (14 Tasks, 7 Domains)

*SEC EDGAR, CDC PLACES, BLS JOLTS, WCCA court cases, NVD CVEs, USDA food products, NYC PLUTO data. Claude Sonnet 4.5 and GPT-5.2.*

### Accuracy

| | TL | JSON | TOON |
|---|---|---|---|
| **Anthropic accuracy** | 0.948 | 0.948 | 0.947 |
| **OpenAI accuracy** | 0.831 | 0.888 | 0.890 |
| **Input token savings** | **-51%** | baseline | **-20%** |

### Per-Domain Accuracy (TL Format)

| Domain | Anthropic | OpenAI |
|--------|-----------|--------|
| HR / Labor | 0.972 | 0.949 |
| Finance | 0.960 | 0.944 |
| Legal | 0.960 | 0.583 |
| Healthcare | 0.958 | 0.950 |
| Retail | 0.934 | 0.573 |
| Real Estate | 0.928 | 0.863 |
| Technology | 0.925 | 0.955 |

### Input Token Savings (vs JSON Baseline)

| Provider | TL Input | JSON Input | TOON Input | TL vs JSON | TOON vs JSON |
|----------|----------|------------|------------|------------|--------------|
| Anthropic | 421,141 | 864,232 | 678,626 | **-51.3%** | -21.5% |
| OpenAI | 362,918 | 734,984 | 593,902 | **-50.6%** | -19.2% |

### Key Findings

- **~51% input token savings** with TL vs JSON on real-world data
- **~20% input token savings** with TOON vs JSON on the same data
- **No accuracy loss** -- scores within noise across all three formats
- Savings range from 27% (small datasets) to 77% (tabular data with high schema repetition)
- TeaLeaf's advantage increases with nesting depth (schema inference + positional encoding)

### Charts

#### Accuracy by Format & Provider

![Accuracy by Format & Provider](charts/accuracy_by_format.png)

#### Input Token Usage by Format & Provider

![Input Tokens by Format & Provider](charts/input_tokens_by_format.png)

#### Token Savings vs JSON Baseline

![Token Savings vs JSON](charts/token_savings_tl_vs_json.png)

## Three-Format Comparison

The `--compare-formats` flag runs each task in all three formats:

- **TeaLeaf (TL)** -- compact schema-aware format using `@struct` + `@table` positional encoding
- **JSON** -- standard pretty-printed JSON (baseline)
- **TOON** -- Token-Oriented Object Notation with tabular folding

## Analysis Framework

### Accuracy Metrics

Responses are evaluated across five weighted dimensions:

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

Results are saved to `accuracy-benchmark/results/{run-id}/`:

| File | Description |
|------|-------------|
| `analysis.tl` | Full results in TeaLeaf format |
| `summary.json` | Aggregated scores and rankings |
| `responses/` | Raw API responses (with `--save-responses`) |

Response files are named `{source}-{task-id}-{provider}-{format}.txt` (e.g., `real-fin-001-anthropic-tl.txt`).

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

The last two tables (`format_accuracy` and `format_tokens`) are only present when `--compare-formats` is used.

## Evidence

Full benchmark evidence (prompts, responses, analysis, charts) is available in [`evidence/`](evidence/).

Each dated folder is a self-contained snapshot:

```
evidence/2026-02-14/
  summary.json       # Aggregate results
  analysis.tl        # Full per-task results
  prompts/           # 42 files: exact prompts sent to LLMs
  responses/         # 84 files: raw LLM responses
  charts/            # Generated visualizations
```

## Quick Start

```bash
# Run the full real-world benchmark
cargo run -p accuracy-benchmark --release -- run --compare-formats --data-source real --save-responses

# Generate charts from results
python accuracy-benchmark/scripts/generate_charts.py --results-dir accuracy-benchmark/results/<run-id>
```

For full CLI reference, data source details, task definitions, and customization guides, see the [documentation site](https://krishjag.github.io/tealeaf/internals/accuracy-benchmark.html).

## License

Same as the parent `tealeaf` project.
