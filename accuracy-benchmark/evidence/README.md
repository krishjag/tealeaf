# Accuracy Benchmark Evidence

This directory contains dated evidence packages from real-world benchmark runs comparing **TeaLeaf (TL)**, **JSON**, and **TOON** formats. Each dated folder is a self-contained snapshot with prompts, responses, analysis, and charts.

## Evidence Runs

| Date | Models | Tasks | Domains | TL Input Savings | Notes |
|------|--------|-------|---------|-----------------|-------|
| [2026-02-14-01](2026-02-14-01/) | Sonnet 4.5, GPT-5.2 | 14 | 7 | ~51% vs JSON | Full 7-domain run, max_tokens 8192, 84/84 API calls successful, 0 errors |

## Structure

Each dated folder contains:

```
YYYY-MM-DD/
  README.md        # Run details, key results, methodology
  summary.json     # Aggregate scores and category breakdown
  analysis.tl      # Full per-task results (TeaLeaf format)
  prompts/         # Exact prompts sent to LLMs (no API calls needed to regenerate)
  responses/       # Raw LLM responses
  charts/          # Generated visualizations
```

## Adding New Evidence

1. Run the benchmark:
   ```bash
   cargo run --release -p accuracy-benchmark -- run --compare-formats --data-source real --save-responses -o accuracy-benchmark/results
   ```

2. Create a dated evidence folder and collect artifacts:
   ```bash
   DATE=$(date +%Y-%m-%d)
   mkdir -p accuracy-benchmark/evidence/$DATE

   # Dump prompts
   cargo run --release -p accuracy-benchmark -- dump-prompts --data-source real -o accuracy-benchmark/evidence/$DATE/prompts

   # Copy run results (use the run ID from step 1)
   RUN_ID=<run-id>
   cp accuracy-benchmark/results/$RUN_ID/analysis.tl accuracy-benchmark/evidence/$DATE/
   cp accuracy-benchmark/results/$RUN_ID/summary.json accuracy-benchmark/evidence/$DATE/
   cp -r accuracy-benchmark/results/$RUN_ID/responses accuracy-benchmark/evidence/$DATE/

   # Generate charts
   python accuracy-benchmark/scripts/generate_charts.py --results-dir accuracy-benchmark/results/$RUN_ID --output-dir accuracy-benchmark/evidence/$DATE/charts
   ```

3. Update this README's evidence runs table.