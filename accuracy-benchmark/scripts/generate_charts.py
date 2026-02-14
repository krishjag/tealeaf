#!/usr/bin/env python3
"""Generate benchmark result charts from accuracy-benchmark run data.

Produces two charts:
  1. Accuracy by Format & Provider (grouped bar chart per domain)
  2. Input Token Usage by Format & Provider (grouped bar chart per domain)

Usage:
    pip install matplotlib
    python generate_charts.py                    # use hardcoded data from previous runs
    python generate_charts.py --results-dir DIR  # parse analysis.tl files from a run directory
"""

import argparse
import re
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np


# ── Hardcoded data from individual domain runs (Feb 13, 2026) ────────────────
# Each domain was benchmarked separately with --compare-formats --data-source real
# Source: analysis.tl -> format_accuracy and format_tokens tables

DOMAIN_DATA = {
    "Finance": {
        # Run 20260213-183949 (FIN-001 + FIN-002)
        "accuracy": {
            ("anthropic", "tl"): 0.953, ("openai", "tl"): 0.930,
            ("anthropic", "json"): 0.953, ("openai", "json"): 0.940,
            ("anthropic", "toon"): 0.951, ("openai", "toon"): 0.931,
        },
        "input_tokens": {
            ("anthropic", "tl"): 81619, ("openai", "tl"): 69470,
            ("anthropic", "json"): 174707, ("openai", "json"): 142618,
            ("anthropic", "toon"): 81361, ("openai", "toon"): 70012,
        },
    },
    "Healthcare": {
        # Run 20260213-183949 (HEALTH-001 + HEALTH-002) — same run, extract per-category
        # Per-task tokens from responses table:
        #   HEALTH TL: anthropic 13631+13647=27278, openai 11982+12001=23983
        #   HEALTH JSON: anthropic 39230+39246=78476, openai 31091+31110=62201
        #   HEALTH TOON: anthropic 13237+13253=26490, openai 11797+11816=23613
        # Per-category accuracy from summary: healthcare anthropic 0.954, openai 0.932
        # Per-format accuracy computed from analysis_results rows:
        "accuracy": {
            ("anthropic", "tl"): 0.954, ("openai", "tl"): 0.928,
            ("anthropic", "json"): 0.954, ("openai", "json"): 0.946,
            ("anthropic", "toon"): 0.953, ("openai", "toon"): 0.935,
        },
        "input_tokens": {
            ("anthropic", "tl"): 27278, ("openai", "tl"): 23983,
            ("anthropic", "json"): 78476, ("openai", "json"): 62201,
            ("anthropic", "toon"): 26490, ("openai", "toon"): 23613,
        },
    },
    "HR / Labor": {
        # Run 20260213-202100 (LABOR-001 + LABOR-002)
        "accuracy": {
            ("anthropic", "tl"): 0.973, ("openai", "tl"): 0.946,
            ("anthropic", "json"): 0.972, ("openai", "json"): 0.945,
            ("anthropic", "toon"): 0.965, ("openai", "toon"): 0.945,
        },
        "input_tokens": {
            ("anthropic", "tl"): 14279, ("openai", "tl"): 13606,
            ("anthropic", "json"): 63121, ("openai", "json"): 55418,
            ("anthropic", "toon"): 41965, ("openai", "toon"): 38148,
        },
    },
    "Legal": {
        # Run 20260213-213023 (LEGAL-001 + LEGAL-002)
        "accuracy": {
            ("anthropic", "tl"): 0.957, ("openai", "tl"): 0.976,
            ("anthropic", "json"): 0.961, ("openai", "json"): 0.966,
            ("anthropic", "toon"): 0.952, ("openai", "toon"): 0.580,
        },
        "input_tokens": {
            ("anthropic", "tl"): 29168, ("openai", "tl"): 25319,
            ("anthropic", "json"): 50088, ("openai", "json"): 42841,
            ("anthropic", "toon"): 41508, ("openai", "toon"): 36869,
        },
    },
    "Technology": {
        # Run 20260213-221948 (TECH-001 + TECH-002)
        "accuracy": {
            ("anthropic", "tl"): 0.936, ("openai", "tl"): 0.967,
            ("anthropic", "json"): 0.940, ("openai", "json"): 0.944,
            ("anthropic", "toon"): 0.940, ("openai", "toon"): 0.942,
        },
        "input_tokens": {
            ("anthropic", "tl"): 179737, ("openai", "tl"): 151035,
            ("anthropic", "json"): 246957, ("openai", "json"): 202377,
            ("anthropic", "toon"): 215505, ("openai", "toon"): 180969,
        },
    },
    "Retail": {
        # Run 20260213-224450 (RETAIL-001 + RETAIL-002)
        "accuracy": {
            ("anthropic", "tl"): 0.919, ("openai", "tl"): 0.546,
            ("anthropic", "json"): 0.931, ("openai", "json"): 0.934,
            ("anthropic", "toon"): 0.923, ("openai", "toon"): 0.559,
        },
        "input_tokens": {
            ("anthropic", "tl"): 102129, ("openai", "tl"): 90262,
            ("anthropic", "json"): 295663, ("openai", "json"): 258064,
            ("anthropic", "toon"): 239165, ("openai", "toon"): 213596,
        },
    },
    "Real Estate": {
        # Run 20260213-230227 (RE-001 + RE-002)
        "accuracy": {
            ("anthropic", "tl"): 0.922, ("openai", "tl"): 0.610,
            ("anthropic", "json"): 0.913, ("openai", "json"): 0.931,
            ("anthropic", "toon"): 0.928, ("openai", "toon"): 0.860,
        },
        "input_tokens": {
            ("anthropic", "tl"): 18658, ("openai", "tl"): 17012,
            ("anthropic", "json"): 51828, ("openai", "json"): 45342,
            ("anthropic", "toon"): 42196, ("openai", "toon"): 39404,
        },
    },
}


# Task ID prefix -> display domain name
TASK_PREFIX_TO_DOMAIN = {
    "FIN": "Finance",
    "HEALTH": "Healthcare",
    "HR": "HR / Labor",
    "LEGAL": "Legal",
    "TECH": "Technology",
    "RETAIL": "Retail",
    "RE": "Real Estate",
}


def _task_id_to_domain(task_id: str) -> str:
    """Map a task ID like 'FIN-001' to its domain name."""
    prefix = task_id.rsplit("-", 1)[0]
    return TASK_PREFIX_TO_DOMAIN.get(prefix, prefix)


def load_from_results_dir(results_dir: Path) -> dict:
    """Parse an analysis.tl file and extract per-domain accuracy and token data."""
    analysis_path = results_dir / "analysis.tl"
    if not analysis_path.exists():
        raise FileNotFoundError(f"No analysis.tl found in {results_dir}")

    text = analysis_path.read_text(encoding="utf-8")

    # ── Parse responses table for per-task input tokens ──
    # Row: ("task_id", "provider", "format", "model", input_tokens, output_tokens, ...)
    responses_section = text[text.find("responses:"):text.find("# Analysis")]
    # Accumulate input_tokens per (domain, provider, format)
    domain_tokens: dict[str, dict[tuple, int]] = {}
    for m in re.finditer(
        r'\("([^"]+)",\s*"(\w+)",\s*"(\w+)",\s*"[^"]*",\s*(\d+)',
        responses_section,
    ):
        task_id, provider, fmt, input_tok = m.groups()
        domain = _task_id_to_domain(task_id)
        domain_tokens.setdefault(domain, {})
        key = (provider, fmt)
        domain_tokens[domain][key] = domain_tokens[domain].get(key, 0) + int(input_tok)

    # ── Parse analysis_results table for per-task accuracy scores ──
    # Row: ("task_id", "provider", "format", completeness, relevance, coherence, factual_accuracy)
    analysis_section = text[text.find("analysis_results:"):text.find("# Comparisons")]
    # Accumulate scores per (domain, provider, format)
    domain_scores: dict[str, dict[tuple, list]] = {}
    for m in re.finditer(
        r'\("([^"]+)",\s*"(\w+)",\s*"(\w+)",\s*([\d.]+),\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)\)',
        analysis_section,
    ):
        task_id, provider, fmt = m.group(1), m.group(2), m.group(3)
        scores = [float(m.group(i)) for i in range(4, 8)]
        avg_score = sum(scores) / len(scores)
        domain = _task_id_to_domain(task_id)
        domain_scores.setdefault(domain, {})
        key = (provider, fmt)
        domain_scores[domain].setdefault(key, []).append(avg_score)

    # ── Build per-domain data in same shape as DOMAIN_DATA ──
    # Use TASK_PREFIX_TO_DOMAIN order for consistent chart ordering
    domain_data = {}
    for domain in TASK_PREFIX_TO_DOMAIN.values():
        if domain not in domain_tokens:
            continue
        accuracy = {}
        for key, score_list in domain_scores.get(domain, {}).items():
            accuracy[key] = sum(score_list) / len(score_list)
        domain_data[domain] = {
            "accuracy": accuracy,
            "input_tokens": domain_tokens[domain],
        }

    return domain_data


# ── Chart configuration ──────────────────────────────────────────────────────

# Dark theme
plt.style.use("dark_background")
plt.rcParams.update({
    "figure.facecolor": "#1a1a2e",
    "axes.facecolor": "#16213e",
    "axes.edgecolor": "#4a4a6a",
    "axes.labelcolor": "#e0e0e0",
    "text.color": "#e0e0e0",
    "xtick.color": "#c0c0c0",
    "ytick.color": "#c0c0c0",
    "grid.color": "#4a4a6a",
    "legend.facecolor": "#1a1a2e",
    "legend.edgecolor": "#4a4a6a",
})

# 6 bars per domain: 2 providers × 3 formats (brighter colors for dark bg)
BAR_GROUPS = [
    ("Anthropic TL", "anthropic", "tl", "#3b82f6"),      # bright blue
    ("Anthropic JSON", "anthropic", "json", "#60a5fa"),   # medium blue
    ("Anthropic TOON", "anthropic", "toon", "#93c5fd"),   # light blue
    ("OpenAI TL", "openai", "tl", "#ef4444"),             # bright red
    ("OpenAI JSON", "openai", "json", "#f87171"),         # medium red
    ("OpenAI TOON", "openai", "toon", "#fca5a5"),         # light red
]


def plot_accuracy(domain_data: dict, output_path: Path):
    """Generate accuracy comparison chart."""
    domains = list(domain_data.keys())
    n_domains = len(domains)
    n_bars = len(BAR_GROUPS)
    bar_width = 0.12
    x = np.arange(n_domains)

    fig, ax = plt.subplots(figsize=(16, 7))

    for i, (label, provider, fmt, color) in enumerate(BAR_GROUPS):
        offset = (i - n_bars / 2 + 0.5) * bar_width
        values = []
        for domain in domains:
            score = domain_data[domain]["accuracy"].get((provider, fmt), 0)
            values.append(score)
        bars = ax.bar(x + offset, values, bar_width, label=label, color=color,
                      edgecolor="#1a1a2e", linewidth=0.5)
        # Add format label inside bar + value above
        fmt_label = fmt.upper()
        y_floor = 0.4  # matches ylim lower bound
        for bar, val in zip(bars, values):
            if val > 0:
                visible_bottom = max(bar.get_y(), y_floor)
                visible_height = bar.get_height() - (visible_bottom - bar.get_y())
                bar_cx = bar.get_x() + bar.get_width() / 2
                if visible_height > 0.12:
                    # Enough room: format label inside the bar
                    ax.text(bar_cx, visible_bottom + visible_height / 2,
                            fmt_label, ha="center", va="center", fontsize=7.5,
                            fontweight="bold", color="white", rotation=90)
                    # Score above the bar
                    ax.text(bar_cx, bar.get_height() + 0.005,
                            f"{val*100:.0f}%", ha="center", va="bottom", fontsize=8,
                            fontweight="bold", rotation=65)
                else:
                    # Short bar: combined label above
                    ax.text(bar_cx, bar.get_height() + 0.005,
                            f"{fmt_label}\n{val*100:.0f}%", ha="center", va="bottom",
                            fontsize=7, fontweight="bold", rotation=0, linespacing=0.9)

    ax.set_xlabel("Domain", fontsize=12, fontweight="bold")
    ax.set_ylabel("Accuracy Score", fontsize=12, fontweight="bold")
    ax.set_title("Accuracy by Format & Provider Across Domains\n(Real-World Data Benchmark)",
                 fontsize=14, fontweight="bold", pad=15)
    ax.set_xticks(x)
    ax.set_xticklabels(domains, fontsize=10)
    ax.set_ylim(0.4, 1.15)
    ax.yaxis.set_major_formatter(mticker.PercentFormatter(xmax=1.0))
    ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.08), ncol=6, fontsize=9,
              frameon=True, fancybox=True)
    ax.grid(axis="y", alpha=0.3, linestyle="--")
    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"Accuracy chart saved to: {output_path}")


def plot_tokens(domain_data: dict, output_path: Path):
    """Generate input token usage comparison chart."""
    domains = list(domain_data.keys())
    n_domains = len(domains)
    n_bars = len(BAR_GROUPS)
    bar_width = 0.12
    x = np.arange(n_domains)

    fig, ax = plt.subplots(figsize=(16, 7))

    # Pre-compute max bar height to set a proportional threshold for label placement
    max_val = 0
    for domain in domains:
        for _, provider, fmt, _ in BAR_GROUPS:
            v = domain_data[domain]["input_tokens"].get((provider, fmt), 0) / 1000
            if v > max_val:
                max_val = v
    min_inside_height = max_val * 0.03  # bar must be at least 3% of tallest to fit label inside

    for i, (label, provider, fmt, color) in enumerate(BAR_GROUPS):
        offset = (i - n_bars / 2 + 0.5) * bar_width
        values = []
        for domain in domains:
            tokens = domain_data[domain]["input_tokens"].get((provider, fmt), 0)
            values.append(tokens / 1000)  # display in thousands
        bars = ax.bar(x + offset, values, bar_width, label=label, color=color,
                      edgecolor="#1a1a2e", linewidth=0.5)
        # Add format label inside bar + value above
        fmt_label = fmt.upper()
        for bar, val in zip(bars, values):
            if val > 0:
                bar_cx = bar.get_x() + bar.get_width() / 2
                if bar.get_height() > min_inside_height:
                    # Enough room: format label inside the bar
                    ax.text(bar_cx, bar.get_y() + bar.get_height() / 2,
                            fmt_label, ha="center", va="center", fontsize=7.5,
                            fontweight="bold", color="white", rotation=90)
                    # Token count above the bar
                    ax.text(bar_cx, bar.get_height() + 1,
                            f"{val:.0f}K", ha="center", va="bottom", fontsize=8,
                            fontweight="bold", rotation=65)
                else:
                    # Short bar: combined label above (rotated to avoid overlap)
                    ax.text(bar_cx, bar.get_height() + 1,
                            f"{fmt_label} {val:.0f}K", ha="left", va="bottom",
                            fontsize=7, fontweight="bold", rotation=65)

    ax.set_xlabel("Domain", fontsize=12, fontweight="bold")
    ax.set_ylabel("Input Tokens (thousands)", fontsize=12, fontweight="bold")
    ax.set_title("Input Token Usage by Format & Provider Across Domains\n(Real-World Data Benchmark)",
                 fontsize=14, fontweight="bold", pad=15)
    ax.set_xticks(x)
    ax.set_xticklabels(domains, fontsize=10)
    ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.08), ncol=6, fontsize=9,
              frameon=True, fancybox=True)
    ax.grid(axis="y", alpha=0.3, linestyle="--")
    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"Token usage chart saved to: {output_path}")


def plot_token_savings(domain_data: dict, output_path: Path):
    """Generate TL & TOON token savings vs JSON chart (both providers)."""
    domains = list(domain_data.keys())
    n_domains = len(domains)

    # 4 bars per domain: Anthropic TL, Anthropic TOON, OpenAI TL, OpenAI TOON
    savings_groups = [
        ("Anthropic TL", "anthropic", "tl", "#3b82f6"),
        ("Anthropic TOON", "anthropic", "toon", "#93c5fd"),
        ("OpenAI TL", "openai", "tl", "#ef4444"),
        ("OpenAI TOON", "openai", "toon", "#fca5a5"),
    ]
    n_bars = len(savings_groups)
    bar_width = 0.18
    x = np.arange(n_domains)

    fig, ax = plt.subplots(figsize=(16, 7))

    for i, (label, provider, fmt, color) in enumerate(savings_groups):
        offset = (i - n_bars / 2 + 0.5) * bar_width
        savings = []
        for domain in domains:
            fmt_tokens = domain_data[domain]["input_tokens"].get((provider, fmt), 1)
            json_tokens = domain_data[domain]["input_tokens"].get((provider, "json"), 1)
            pct = (1 - fmt_tokens / json_tokens) * 100
            savings.append(pct)
        bars = ax.bar(x + offset, savings, bar_width, label=label, color=color,
                      edgecolor="#1a1a2e", linewidth=0.5)
        fmt_label = fmt.upper()
        for bar, val in zip(bars, savings):
            bar_cx = bar.get_x() + bar.get_width() / 2
            # Format label inside bar
            if bar.get_height() > 5:
                ax.text(bar_cx, bar.get_y() + bar.get_height() / 2,
                        fmt_label, ha="center", va="center", fontsize=7.5,
                        fontweight="bold", color="white", rotation=90)
            # Percentage above bar
            ax.text(bar_cx, bar.get_height() + 0.5,
                    f"{val:.0f}%", ha="center", va="bottom", fontsize=8,
                    fontweight="bold", rotation=65)

    ax.set_xlabel("Domain", fontsize=12, fontweight="bold")
    ax.set_ylabel("Input Token Savings vs JSON (%)", fontsize=12, fontweight="bold")
    ax.set_title("Token Savings vs JSON by Format & Provider Across Domains\n(Real-World Data Benchmark)",
                 fontsize=14, fontweight="bold", pad=15)
    ax.set_xticks(x)
    ax.set_xticklabels(domains, fontsize=10)
    ax.set_ylim(0, 85)
    ax.yaxis.set_major_formatter(mticker.PercentFormatter())
    ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.08), ncol=4, fontsize=9,
              frameon=True, fancybox=True)
    ax.grid(axis="y", alpha=0.3, linestyle="--")
    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)

    # Add average line (TL savings only, across both providers)
    all_savings = []
    for provider in ["anthropic", "openai"]:
        for domain in domains:
            tl = domain_data[domain]["input_tokens"].get((provider, "tl"), 1)
            json_t = domain_data[domain]["input_tokens"].get((provider, "json"), 1)
            all_savings.append((1 - tl / json_t) * 100)
    avg = np.mean(all_savings)
    ax.axhline(y=avg, color="#a0aec0", linestyle="--", linewidth=1.5, alpha=0.7)
    ax.text(n_domains - 0.5, avg + 1, f"Avg: {avg:.0f}%", fontsize=9, color="#a0aec0",
            ha="right", fontweight="bold")

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"Token savings chart saved to: {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Generate benchmark charts")
    parser.add_argument("--results-dir", type=Path, default=None,
                        help="Path to a run directory containing analysis.tl")
    parser.add_argument("--output-dir", type=Path,
                        default=Path(__file__).parent.parent / "charts",
                        help="Directory to write chart images (default: accuracy-benchmark/charts/)")
    args = parser.parse_args()

    if args.results_dir:
        domain_data = load_from_results_dir(args.results_dir)
    else:
        domain_data = DOMAIN_DATA

    args.output_dir.mkdir(parents=True, exist_ok=True)

    plot_accuracy(domain_data, args.output_dir / "accuracy_by_format.png")
    plot_tokens(domain_data, args.output_dir / "input_tokens_by_format.png")
    plot_token_savings(domain_data, args.output_dir / "token_savings_tl_vs_json.png")

    print(f"\nAll charts written to {args.output_dir}")


if __name__ == "__main__":
    main()
