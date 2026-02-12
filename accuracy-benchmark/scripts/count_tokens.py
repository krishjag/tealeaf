#!/usr/bin/env python3
"""
Count input tokens for accuracy benchmark prompts using the Anthropic token counting API.

Replicates the exact same prompts sent during live benchmark runs (across TeaLeaf,
JSON, and TOON formats) but calls the token counting endpoint instead of completions,
giving accurate token counts at zero cost.

Prerequisites:
    pip install anthropic
    export ANTHROPIC_API_KEY=sk-...

    # Build the benchmark binary (used to generate prompts):
    cargo build --release -p accuracy-benchmark

Usage (run from repository root):
    # Count tokens for real-world SEC EDGAR data (default)
    python accuracy-benchmark/scripts/count_tokens.py

    # Count tokens for synthetic data
    python accuracy-benchmark/scripts/count_tokens.py --data-source synthetic

    # Use a specific model for tokenization
    python accuracy-benchmark/scripts/count_tokens.py --model claude-opus-4-6

    # Use pre-generated prompt files (skip generation step)
    python accuracy-benchmark/scripts/count_tokens.py --prompts-dir results/prompts/real
"""

import argparse
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

try:
    import anthropic
except ImportError:
    print("Error: 'anthropic' package required. Install with: pip install anthropic")
    sys.exit(1)


FORMATS = ["tl", "json", "toon"]


def generate_prompts(data_source: str, output_dir: Path) -> None:
    """Generate prompt files using the benchmark's dump-prompts command."""
    cmd = [
        "cargo", "run", "-p", "accuracy-benchmark", "--release", "--",
        "dump-prompts",
        "--data-source", data_source,
        "--output", str(output_dir),
    ]
    print(f"Generating prompts via: {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error generating prompts:\n{result.stderr}")
        sys.exit(1)
    # Show generation summary (last line)
    lines = result.stdout.strip().splitlines()
    if lines:
        print(lines[-1])


def extract_prompt(filepath: Path) -> str:
    """Extract the actual prompt content from a dump-prompts file.

    dump-prompts files have a metadata header followed by the prompt:
        === API Request: FIN-001 (TEALEAF format) ===
        Task ID:     FIN-001
        ...
        === PROMPT ===

        <actual prompt text>
    """
    text = filepath.read_text(encoding="utf-8")
    marker = "=== PROMPT ==="
    idx = text.find(marker)
    if idx == -1:
        return text  # Fallback: use entire file
    return text[idx + len(marker) :].lstrip("\n")


def parse_filename(filename: str) -> tuple[str, str]:
    """Parse task ID and format from prompt filename.

    Filenames follow the pattern: {task-id-lowercase}-{format}.txt
    e.g., 'fin-001-tl.txt' -> ('FIN-001', 'tl')
    """
    stem = filename.removesuffix(".txt")
    for fmt in FORMATS:
        suffix = f"-{fmt}"
        if stem.endswith(suffix):
            task_id = stem[: -len(suffix)].upper()
            return task_id, fmt
    return stem.upper(), "unknown"


def count_tokens(client: anthropic.Anthropic, model: str, prompt: str) -> int:
    """Count input tokens for a prompt using the Anthropic token counting API."""
    response = client.messages.count_tokens(
        model=model,
        messages=[{"role": "user", "content": prompt}],
    )
    return response.input_tokens


def main():
    parser = argparse.ArgumentParser(
        description="Count input tokens for accuracy benchmark prompts"
    )
    parser.add_argument(
        "--data-source",
        choices=["synthetic", "real"],
        default="real",
        help="Data source for tasks (default: real)",
    )
    parser.add_argument(
        "--model",
        default="claude-sonnet-4-5-20250929",
        help="Model for token counting (default: claude-sonnet-4-5-20250929)",
    )
    parser.add_argument(
        "--prompts-dir",
        type=Path,
        help="Directory with pre-generated prompt files (skips generation)",
    )
    args = parser.parse_args()

    # --- Generate or locate prompt files ---
    if args.prompts_dir:
        prompts_dir = args.prompts_dir
        if not prompts_dir.exists():
            print(f"Error: prompts directory not found: {prompts_dir}")
            sys.exit(1)
    else:
        prompts_dir = Path(tempfile.mkdtemp(prefix="benchmark-prompts-"))
        generate_prompts(args.data_source, prompts_dir)

    # --- Discover prompt files ---
    prompt_files = sorted(prompts_dir.glob("*.txt"))
    if not prompt_files:
        print(f"No prompt files found in {prompts_dir}")
        sys.exit(1)

    # --- Count tokens ---
    client = anthropic.Anthropic()
    results: dict[tuple[str, str], int] = {}

    print(f"\nCounting tokens (model: {args.model})...\n")
    print(f"{'Task':<12} {'Format':<8} {'Tokens':>10}")
    print("-" * 32)

    for filepath in prompt_files:
        task_id, fmt = parse_filename(filepath.name)
        if fmt == "unknown":
            continue
        prompt = extract_prompt(filepath)
        tokens = count_tokens(client, args.model, prompt)
        results[(task_id, fmt)] = tokens
        print(f"{task_id:<12} {fmt:<8} {tokens:>10,}")

    # --- Summary table ---
    task_ids = sorted(set(t for t, _ in results))
    if not task_ids:
        print("\nNo results to summarize.")
        return

    print(f"\n{'=' * 86}")
    print(f"{'Task':<12}", end="")
    for fmt in FORMATS:
        print(f" {fmt.upper():>10}", end="")
    print(f" {'TL vs JSON':>12} {'TOON vs JSON':>13} {'TL vs TOON':>12}")
    print("-" * 86)

    totals: dict[str, int] = defaultdict(int)
    for task_id in task_ids:
        print(f"{task_id:<12}", end="")
        json_tokens = results.get((task_id, "json"), 0)
        for fmt in FORMATS:
            tokens = results.get((task_id, fmt), 0)
            totals[fmt] += tokens
            print(f" {tokens:>10,}", end="")
        tl_tokens = results.get((task_id, "tl"), 0)
        toon_tokens = results.get((task_id, "toon"), 0)
        tl_pct = ((tl_tokens - json_tokens) / json_tokens * 100) if json_tokens else 0
        toon_pct = (
            (toon_tokens - json_tokens) / json_tokens * 100 if json_tokens else 0
        )
        tl_toon_pct = (
            (tl_tokens - toon_tokens) / toon_tokens * 100 if toon_tokens else 0
        )
        print(f" {tl_pct:>+11.1f}% {toon_pct:>+12.1f}% {tl_toon_pct:>+11.1f}%")

    print("-" * 86)
    json_total = totals["json"]
    toon_total = totals["toon"]
    print(f"{'TOTAL':<12}", end="")
    for fmt in FORMATS:
        print(f" {totals[fmt]:>10,}", end="")
    tl_pct = ((totals["tl"] - json_total) / json_total * 100) if json_total else 0
    toon_pct = ((toon_total - json_total) / json_total * 100) if json_total else 0
    tl_toon_pct = ((totals["tl"] - toon_total) / toon_total * 100) if toon_total else 0
    print(f" {tl_pct:>+11.1f}% {toon_pct:>+12.1f}% {tl_toon_pct:>+11.1f}%")
    print(f"{'=' * 86}")

    # --- Data-only token estimate ---
    # The prompt template text is constant across formats; only the {data} portion
    # differs. Estimate data-only tokens by subtracting the minimum (which
    # approximates the shared instruction overhead).
    if len(FORMATS) > 1 and all(totals[f] > 0 for f in FORMATS):
        print(f"\nEstimated data-only savings (subtracting shared instruction overhead):")
        min_per_task: dict[str, int] = {}
        for task_id in task_ids:
            min_per_task[task_id] = min(
                results.get((task_id, f), 0) for f in FORMATS
            )
        # Assume instruction tokens ~ min tokens across formats (conservative)
        # A tighter estimate would require counting tokens of the template alone
        for fmt in FORMATS:
            if fmt == "json":
                continue
            fmt_data = sum(
                results.get((t, fmt), 0) - min_per_task[t] for t in task_ids
            )
            json_data = sum(
                results.get((t, "json"), 0) - min_per_task[t] for t in task_ids
            )
            if json_data > 0:
                pct = (fmt_data - json_data) / json_data * 100
                print(
                    f"  {fmt.upper()} data tokens vs JSON data tokens: {pct:+.1f}%"
                    f"  ({fmt_data:,} vs {json_data:,})"
                )

    print(f"\nPrompt files: {prompts_dir}")


if __name__ == "__main__":
    main()
