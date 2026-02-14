# Accuracy Benchmark Evidence — 2026-02-14

Real-world benchmark results comparing **TeaLeaf (TL)**, **JSON**, and **TOON** formats across two LLM providers.

## Run Details

| Field | Value |
|-------|-------|
| **Run ID** | `20260214-033446` |
| **Date** | February 14, 2026 |
| **Models** | Claude Sonnet 4.5 (`claude-sonnet-4-5-20250929`), GPT-5.2 (`gpt-5.2-2025-12-11`) |
| **Data Source** | Real-world (SEC EDGAR 10-K filings, CDC PLACES health data, BLS JOLTS labor data, WCCA court cases, NVD CVEs, USDA food products, NYC PLUTO property data) |
| **Tasks** | 14 tasks across 7 domains |
| **API Calls** | 84 (14 tasks x 3 formats x 2 providers) |

## Domains

| Domain | Tasks | Data Source |
|--------|-------|-------------|
| Finance | FIN-001, FIN-002 | SEC EDGAR 10-K annual filings |
| Healthcare | HEALTH-001, HEALTH-002 | CDC PLACES county health indicators |
| HR / Labor | HR-001, HR-002 | BLS JOLTS labor turnover survey |
| Legal | LEGAL-001, LEGAL-002 | WCCA Wisconsin Circuit Court cases |
| Technology | TECH-001, TECH-002 | NVD Linux kernel CVEs |
| Retail | RETAIL-001, RETAIL-002 | USDA FoodData Central branded foods |
| Real Estate | RE-001, RE-002 | NYC PLUTO Manhattan tax lots |

## Key Results

### Token Efficiency (Input Tokens vs JSON)

| Provider | TL vs JSON | TOON vs JSON |
|----------|-----------|-------------|
| Anthropic | **-51.3%** | -21.5% |
| OpenAI | **-50.6%** | -19.2% |

### Accuracy (averaged across all 14 tasks)

| Provider | TL | JSON | TOON |
|----------|-----|------|------|
| Anthropic | 0.948 | 0.948 | 0.947 |
| OpenAI | 0.831 | 0.888 | 0.890 |

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

All three formats produce comparable accuracy scores for Anthropic. OpenAI shows some variance with lower scores on Legal and Retail TL tasks. TeaLeaf achieves ~51% input token savings with no accuracy loss on Anthropic.

## Directory Structure

```
2026-02-14/
  README.md          # This file
  summary.json       # Aggregate results and category breakdown
  analysis.tl        # Full results in TeaLeaf format (per-task scores, token counts, comparisons)
  prompts/           # 42 files: exact prompts sent to each LLM
    {source}-{task}-{format}.txt    (e.g., real-fin-001-tl.txt)
  responses/         # 84 files: raw LLM responses
    {source}-{task}-{provider}-{format}.txt  (e.g., real-fin-001-anthropic-tl.txt)
  charts/            # Generated visualizations
    accuracy_by_format.png        # Accuracy comparison across all domains
    input_tokens_by_format.png    # Token usage comparison across all domains
    token_savings_tl_vs_json.png  # TL and TOON savings vs JSON baseline
```

## Methodology

1. **Identical content**: Each task uses the same underlying data, encoded in three formats (TL, JSON, TOON)
2. **Same instructions**: Task prompts differ only in the data format section; system prompts and questions are identical
3. **Independent scoring**: An LLM judge scores each response on four dimensions (completeness, relevance, coherence, factual accuracy) on a 0-1 scale
4. **Cross-provider comparison**: Both Anthropic and OpenAI models are tested on every task/format combination

## Reproducing

```bash
# Run the full benchmark (requires ANTHROPIC_API_KEY and OPENAI_API_KEY)
cargo run --release -p accuracy-benchmark -- run --compare-formats --data-source real --save-responses -o accuracy-benchmark/results

# Dump prompts only (no API calls)
cargo run --release -p accuracy-benchmark -- dump-prompts --data-source real -o accuracy-benchmark/evidence/prompts

# Generate charts from results
python accuracy-benchmark/scripts/generate_charts.py --results-dir accuracy-benchmark/results/<run-id>
```
