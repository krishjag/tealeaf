# TeaLeaf Accuracy Benchmark Suite

A benchmark suite for evaluating LLM providers' ability to analyze structured data across three formats: **TeaLeaf**, **JSON**, and **TOON**.

## Overview

This benchmark suite:
1. Takes JSON source data from various [business domains](#data-sources)
2. Converts it to TeaLeaf, JSON, and TOON formats
3. Sends analysis prompts to multiple LLM providers
4. Evaluates and compares the responses across formats and [generates charts](#charts)

## Latest Results (14 Tasks, 7 Domains)

*SEC EDGAR, CDC PLACES, BLS JOLTS, WCCA court cases, NVD CVEs, USDA food products, NYC PLUTO data. Claude Sonnet 4.5 and GPT-5.2.*

### Accuracy

| | TL | JSON | TOON |
|---|---|---|---|
| **Anthropic accuracy** | 0.944 | 0.949 | 0.949 |
| **OpenAI accuracy** | 0.937 | 0.941 | 0.940 |
| **Input token savings** | **-51%** | baseline | **-20%** |

### Per-Domain Accuracy (TL Format)

| Domain | Anthropic | OpenAI |
|--------|-----------|--------|
| Legal | 0.965 | 0.972 |
| HR / Labor | 0.961 | 0.951 |
| Finance | 0.960 | 0.892 |
| Healthcare | 0.945 | 0.949 |
| Technology | 0.933 | 0.961 |
| Real Estate | 0.919 | 0.928 |
| Retail | 0.926 | 0.909 |

### Input Token Savings (vs JSON Baseline)

| Provider | TL Input | JSON Input | TOON Input | TL vs JSON | TOON vs JSON |
|----------|----------|------------|------------|------------|--------------|
| Anthropic | 421,141 | 864,232 | 678,626 | **-51.3%** | -21.5% |
| OpenAI | 362,918 | 734,984 | 593,902 | **-50.6%** | -19.2% |

### Key Findings

- **~51% input token savings** with TL vs JSON on real-world data
- **~20% input token savings** with TOON vs JSON on the same data
- **No accuracy loss** -- scores within noise across all three formats (all providers 0.93-0.95)
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

## Data Sources

All input data is real-world, sourced from US government public-domain APIs and public records. Each domain has one JSON data file shared by two benchmark tasks.

| Domain | File | Description | Records | Size |
|--------|------|-------------|---------|------|
| Finance | [sec_edgar_2025_q4.json](tasks/finance/data/sec_edgar_2025_q4.json) | SEC EDGAR 10-K annual filings (XBRL). 4 companies (Apple, Visa, Costco, Qualcomm), 5 financial statements each (Balance Sheet, Income Statement, Cash Flows, Comprehensive Income, Stockholders Equity), 80-127 line items per company. | 4 companies, 399 line items | 191 KB |
| Healthcare | [places_wa_health_2023.json](tasks/healthcare/data/places_wa_health_2023.json) | CDC PLACES county-level health indicators (BRFSS). 10 Washington State counties (King through Garfield), 33 health measures per county with crude prevalence, confidence intervals, and population data. | 10 counties, 330 measures | 107 KB |
| HR / Labor | [jolts_2025.json](tasks/hr/data/jolts_2025.json) | BLS JOLTS monthly labor turnover survey. 12 industries (Total nonfarm through Government), 12 months of 2025, 6 metrics each (job openings, hires, total separations, quits, layoffs, other separations) as levels and rates. | 12 industries, 144 month-records | 97 KB |
| Legal | [wcca_cases.json](tasks/legal/data/wcca_cases.json) | Wisconsin Circuit Court (WCCA) criminal case data. 1 felony case spanning 2015-2025 with 2 charges, nested sentencing structures, 231 docket entries, defendant info, and financial receivables. | 1 case, 231 docket entries | 79 KB |
| Technology | [nvd_cves_linux_kernel_2024-03-01_to_2024-03-31.json](tasks/technology/data/nvd_cves_linux_kernel_2024-03-01_to_2024-03-31.json) | NIST NVD Linux kernel CVEs (March 2024). 40 vulnerability records with CVSS v3.1 scoring, CWE classifications (10 unique), affected version ranges (CPE), and categorized references. 12 HIGH, 28 MEDIUM severity. | 40 CVEs, 10 CWE types | 333 KB |
| Retail | [usda_branded_breakfast_cereal.json](tasks/retail/data/usda_branded_breakfast_cereal.json) | USDA FoodData Central branded foods. 25 breakfast cereal products with full nutritional profiles: 697 nutrient entries across 34 unique nutrients, derivation codes, percent daily values, serving sizes, and ingredient lists. | 25 products, 697 nutrient entries | 484 KB |
| Real Estate | [pluto_manhattan.json](tasks/real_estate/data/pluto_manhattan.json) | NYC PLUTO Manhattan tax lots. 30 highest-assessed-value properties, ~76 fields each covering zoning, building dimensions, area breakdowns, assessed values, FAR, ownership, and geographic coordinates. | 30 properties, ~76 fields each | 59 KB |

**Total:** ~1.35 MB of JSON source data across 7 domains, driving 14 benchmark tasks (2 per domain).

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
| `errors/` | Error details for failed tasks (only if errors occurred) |

Response files are named `{source}-{task-id}-{provider}-{format}.txt` (e.g., `real-fin-001-anthropic-tl.txt`).

### analysis.tl Structure

The `analysis.tl` output uses `@struct` schema definitions for self-documenting positional tables:

```tl
# Schema definitions
@struct api_response (task_id: string, provider: string, model: string?,
    input_tokens: int, output_tokens: int, latency_ms: int,
    http_status: int, retry_count: int, response_length: int,
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
evidence/2026-02-14-01/
  README.md          # Run details, results summary, methodology
  summary.json       # Aggregate results
  analysis.tl        # Full per-task results (api_response, analysis_result, format_accuracy, format_tokens)
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
