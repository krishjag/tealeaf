"""
Validate API-reported input tokens against tiktoken counts.

This script:
1. Reads the dumped prompt files (from `accuracy-benchmark dump-prompts`)
2. Tokenizes each prompt with tiktoken (OpenAI's tokenizer)
3. Compares against API-reported input_tokens from analysis.tl
4. Computes data-only token savings by tokenizing instruction vs data separately

Usage:
    pip install tiktoken
    python accuracy-benchmark/scripts/validate_tokens.py

Prerequisites:
    cargo run -p accuracy-benchmark -- dump-prompts -o accuracy-benchmark/results/prompts
"""

import os
import re
import sys
from pathlib import Path

try:
    import tiktoken
except ImportError:
    print("Error: tiktoken not installed. Run: pip install tiktoken")
    sys.exit(1)


def safe_print(text: str) -> None:
    """Print text with control characters stripped (satisfies CodeQL log-injection)."""
    sanitized = text.replace("\r", "").replace("\n", "")
    print(sanitized)  # noqa: log-injection

# Paths relative to repo root
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
PROMPTS_DIR = REPO_ROOT / "accuracy-benchmark" / "results" / "prompts"
ANALYSIS_TL = REPO_ROOT / "accuracy-benchmark" / "results" / "sample" / "analysis.tl"

# Task IDs in order
TASK_IDS = [
    "FIN-001", "FIN-002", "RET-001", "RET-002", "HLT-001", "TEC-001",
    "MKT-001", "LOG-001", "HR-001", "MFG-001", "RE-001", "LEG-001",
]

# Known prompt templates (instruction text only, without {tl_data} placeholder).
# These are extracted from the prompt files by splitting at the data boundary.
# The instruction text is identical between TL and JSON runs of the same task.

def extract_prompt(filepath: Path) -> str:
    """Extract the actual prompt from a dump file (after '=== PROMPT ===')."""
    text = filepath.read_text(encoding="utf-8")
    marker = "=== PROMPT ===\n\n"
    idx = text.find(marker)
    if idx == -1:
        raise ValueError(f"No prompt marker found in {filepath}")
    return text[idx + len(marker):]


def split_instruction_and_data(tl_prompt: str, json_prompt: str) -> tuple[str, str, str]:
    """Split a prompt into instruction text and data payload.

    The instruction text is identical between TL and JSON prompts.
    We find the longest common prefix and suffix to isolate the data.

    Returns (instruction_prefix, tl_data, json_data)
    where instruction = prefix + suffix (the parts that are the same).
    """
    # Find common prefix
    prefix_len = 0
    for a, b in zip(tl_prompt, json_prompt):
        if a == b:
            prefix_len += 1
        else:
            break

    # Find common suffix
    suffix_len = 0
    for a, b in zip(reversed(tl_prompt), reversed(json_prompt)):
        if a == b:
            suffix_len += 1
        else:
            break

    # Ensure prefix/suffix don't overlap
    if prefix_len + suffix_len > min(len(tl_prompt), len(json_prompt)):
        suffix_len = min(len(tl_prompt), len(json_prompt)) - prefix_len

    instruction_prefix = tl_prompt[:prefix_len]
    instruction_suffix = tl_prompt[len(tl_prompt) - suffix_len:] if suffix_len > 0 else ""

    tl_data = tl_prompt[prefix_len:len(tl_prompt) - suffix_len] if suffix_len > 0 else tl_prompt[prefix_len:]
    json_data = json_prompt[prefix_len:len(json_prompt) - suffix_len] if suffix_len > 0 else json_prompt[prefix_len:]

    return instruction_prefix + instruction_suffix, tl_data, json_data


def parse_api_tokens(analysis_path: Path) -> dict[str, dict[str, int]]:
    """Parse API-reported input tokens from analysis.tl.

    Returns {task_id: {provider: input_tokens}}
    """
    text = analysis_path.read_text(encoding="utf-8")
    tokens = {}

    # Parse the responses table: (task_id, provider, model, input_tokens, output_tokens, ...)
    for match in re.finditer(
        r'\((\w+-\d+),\s*(\w+),\s*"[^"]+",\s*(\d+),\s*(\d+)',
        text
    ):
        task_id = match.group(1)
        provider = match.group(2)
        input_tokens = int(match.group(3))
        if task_id not in tokens:
            tokens[task_id] = {}
        tokens[task_id][provider] = input_tokens

    return tokens


def main():
    # Check prerequisites
    if not PROMPTS_DIR.exists():
        print(f"Error: Prompts directory not found: {PROMPTS_DIR}")
        print("Run: cargo run -p accuracy-benchmark -- dump-prompts -o accuracy-benchmark/results/prompts")
        sys.exit(1)

    # Load tiktoken encoder for OpenAI models
    # gpt-5.2 likely uses cl100k_base or o200k_base; try o200k_base first (GPT-4o/5 family)
    try:
        enc = tiktoken.encoding_for_model("gpt-4o")  # o200k_base, same family as gpt-5.x
    except KeyError:
        enc = tiktoken.get_encoding("o200k_base")
    print(f"Tokenizer: {enc.name}")

    # Parse API-reported tokens if available
    api_tokens = {}
    if ANALYSIS_TL.exists():
        api_tokens = parse_api_tokens(ANALYSIS_TL)
        print(f"Loaded API token counts from: {ANALYSIS_TL.name}")
    else:
        print(f"Note: No analysis.tl found at {ANALYSIS_TL}, skipping API comparison")

    print()

    # Process each task
    total_tl_data_tokens = 0
    total_json_data_tokens = 0
    total_instruction_tokens = 0
    results = []

    print(f"{'Task':<10} {'TL Prompt':>10} {'JSON Prompt':>12} {'Instruct.':>10} {'TL Data':>10} {'JSON Data':>10} {'Data Sav.':>10}", end="")
    if api_tokens:
        print(f"  {'API (OAI)':>10} {'Diff':>8} {'Diff %':>8}", end="")
    print()
    print("-" * (72 + (30 if api_tokens else 0)))

    for task_id in TASK_IDS:
        tl_file = PROMPTS_DIR / f"{task_id.lower()}-tl.txt"
        json_file = PROMPTS_DIR / f"{task_id.lower()}-json.txt"

        if not tl_file.exists() or not json_file.exists():
            print(f"{task_id:<10} MISSING")
            continue

        tl_prompt = extract_prompt(tl_file)
        json_prompt = extract_prompt(json_file)

        # Tokenize full prompts
        tl_tokens = len(enc.encode(tl_prompt))
        json_tokens = len(enc.encode(json_prompt))

        # Split into instruction and data, then tokenize separately
        instruction_text, tl_data, json_data = split_instruction_and_data(tl_prompt, json_prompt)
        instruction_tokens = len(enc.encode(instruction_text))
        tl_data_tokens = len(enc.encode(tl_data))
        json_data_tokens = len(enc.encode(json_data))

        # Data savings
        data_savings_pct = (1 - tl_data_tokens / json_data_tokens) * 100 if json_data_tokens > 0 else 0

        total_tl_data_tokens += tl_data_tokens
        total_json_data_tokens += json_data_tokens
        total_instruction_tokens += instruction_tokens

        row = f"{task_id:<10} {tl_tokens:>10,} {json_tokens:>12,} {instruction_tokens:>10,} {tl_data_tokens:>10,} {json_data_tokens:>10,} {data_savings_pct:>9.1f}%"

        # Compare against API-reported tokens (OpenAI only, since tiktoken is OpenAI's tokenizer)
        if task_id in api_tokens and "openai" in api_tokens[task_id]:
            api_input = api_tokens[task_id]["openai"]
            diff = tl_tokens - api_input
            diff_pct = (diff / api_input * 100) if api_input > 0 else 0
            row += f"  {api_input:>10,} {diff:>+8,} {diff_pct:>+7.1f}%"

        safe_print(row)
        results.append({
            "task_id": task_id,
            "tl_tokens": tl_tokens,
            "json_tokens": json_tokens,
            "instruction_tokens": instruction_tokens,
            "tl_data_tokens": tl_data_tokens,
            "json_data_tokens": json_data_tokens,
            "data_savings_pct": data_savings_pct,
        })

    print("-" * (72 + (30 if api_tokens else 0)))

    # Totals
    total_data_savings = (1 - total_tl_data_tokens / total_json_data_tokens) * 100 if total_json_data_tokens > 0 else 0
    total_tl = sum(r["tl_tokens"] for r in results)
    total_json = sum(r["json_tokens"] for r in results)
    total_input_savings = (1 - total_tl / total_json) * 100 if total_json > 0 else 0

    print(f"{'TOTAL':<10} {total_tl:>10,} {total_json:>12,} {total_instruction_tokens:>10,} {total_tl_data_tokens:>10,} {total_json_data_tokens:>10,} {total_data_savings:>9.1f}%", end="")
    if api_tokens:
        api_total = sum(api_tokens.get(t, {}).get("openai", 0) for t in TASK_IDS)
        diff = total_tl - api_total
        diff_pct = (diff / api_total * 100) if api_total > 0 else 0
        print(f"  {api_total:>10,} {diff:>+8,} {diff_pct:>+7.1f}%", end="")
    print()

    # Compute median data savings
    savings_values = sorted(r["data_savings_pct"] for r in results)
    n = len(savings_values)
    if n % 2 == 0:
        median_savings = (savings_values[n // 2 - 1] + savings_values[n // 2]) / 2
    else:
        median_savings = savings_values[n // 2]

    # Compute median API diff % (per-task validation)
    api_diffs = []
    for r in results:
        task_id = r["task_id"]
        if task_id in api_tokens and "openai" in api_tokens[task_id]:
            api_input = api_tokens[task_id]["openai"]
            if api_input > 0:
                api_diffs.append((r["tl_tokens"] - api_input) / api_input * 100)
    api_diffs.sort()
    if api_diffs:
        na = len(api_diffs)
        median_api_diff = (api_diffs[na // 2 - 1] + api_diffs[na // 2]) / 2 if na % 2 == 0 else api_diffs[na // 2]
    else:
        median_api_diff = None

    print()
    print("=== Summary ===")
    print(f"Data-only token savings (weighted total):  {total_data_savings:.1f}%")
    print(f"Data-only token savings (median):          {median_savings:.1f}%")
    print(f"Total input token savings (TL vs JSON):    {total_input_savings:.1f}%")
    print(f"Instruction tokens (shared, per format):   {total_instruction_tokens:,}")
    print(f"TL data tokens:                            {total_tl_data_tokens:,}")
    print(f"JSON data tokens:                          {total_json_data_tokens:,}")

    if api_tokens:
        api_total = sum(api_tokens.get(t, {}).get("openai", 0) for t in TASK_IDS)
        diff_pct = abs(total_tl - api_total) / api_total * 100 if api_total > 0 else 0
        print(f"\nAPI vs tiktoken validation (OpenAI, TL format):")
        print(f"  API reported total:  {api_total:,}")
        print(f"  tiktoken total:      {total_tl:,}")
        print(f"  Aggregate diff:      {diff_pct:.1f}%")
        if median_api_diff is not None:
            print(f"  Median per-task diff: {median_api_diff:+.1f}%")
        if diff_pct <= 1.0:
            print(f"  Result:              PASS (within 1%)")
        elif diff_pct <= 5.0:
            print(f"  Result:              MARGINAL (within 5%)")
        else:
            print(f"  Result:              FAIL (>5% difference)")


if __name__ == "__main__":
    main()
