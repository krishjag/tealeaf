# Accuracy Benchmark Results
# Generated: 2026-02-12 00:43:55.936664700 UTC

# Schema definitions
@struct api_response (task_id: string, provider: string, format: string, model: string?, input_tokens: int, output_tokens: int, latency_ms: int, timestamp: timestamp, status: string)
@struct analysis_result (task_id: string, provider: string, format: string, completeness: float, relevance: float, coherence: float, factual_accuracy: float)
@struct comparison_result (task_id: string, format: string, providers_ranked: []string, winner: string?, margin: float?)
@struct format_accuracy (provider: string, format: string, avg_score: float, wins: int)
@struct format_tokens (provider: string, format: string, input_tokens: int, output_tokens: int, total_tokens: int)

run_metadata: {
    run_id: "20260212-004101",
    started_at: 2026-02-12T00:41:01Z,
    completed_at: 2026-02-12T00:43:55Z,
    total_tasks: 2,
    providers: [anthropic, openai]
}

# Task Results
responses: @table api_response [
    ("FIN-001", "anthropic", "json", "claude-sonnet-4-5-20250929", 48110, 1712, 24362, 2026-02-12T00:41:58Z, "success"),
    ("FIN-001", "openai", "json", "gpt-5.2-2025-12-11", 40202, 729, 9497, 2026-02-12T00:42:08Z, "success"),
    ("FIN-002", "anthropic", "json", "claude-sonnet-4-5-20250929", 48121, 2351, 35116, 2026-02-12T00:42:35Z, "success"),
    ("FIN-002", "openai", "json", "gpt-5.2-2025-12-11", 40215, 1341, 20358, 2026-02-12T00:42:55Z, "success"),
    ("FIN-001", "anthropic", "tl", "claude-sonnet-4-5-20250929", 27165, 1472, 23204, 2026-02-12T00:41:24Z, "success"),
    ("FIN-001", "openai", "tl", "gpt-5.2-2025-12-11", 22737, 592, 9361, 2026-02-12T00:41:34Z, "success"),
    ("FIN-002", "anthropic", "tl", "claude-sonnet-4-5-20250929", 27176, 2463, 37595, 2026-02-12T00:41:39Z, "success"),
    ("FIN-002", "openai", "tl", "gpt-5.2-2025-12-11", 22750, 1289, 21265, 2026-02-12T00:42:00Z, "success"),
    ("FIN-001", "anthropic", "toon", "claude-sonnet-4-5-20250929", 27430, 1470, 19631, 2026-02-12T00:42:27Z, "success"),
    ("FIN-001", "openai", "toon", "gpt-5.2-2025-12-11", 23193, 594, 8905, 2026-02-12T00:42:38Z, "success"),
    ("FIN-002", "anthropic", "toon", "claude-sonnet-4-5-20250929", 27441, 2612, 39732, 2026-02-12T00:43:35Z, "success"),
    ("FIN-002", "openai", "toon", "gpt-5.2-2025-12-11", 23206, 1481, 20233, 2026-02-12T00:43:55Z, "success"),
]

# Analysis Results
analysis_results: @table analysis_result [
    ("FIN-001", "anthropic", "tl", 1.000, 0.812, 1.000, 1.000),
    ("FIN-001", "openai", "tl", 1.000, 0.750, 1.000, 1.000),
    ("FIN-002", "anthropic", "tl", 1.000, 0.800, 1.000, 1.000),
    ("FIN-002", "openai", "tl", 1.000, 0.667, 1.000, 1.000),
    ("FIN-001", "openai", "json", 1.000, 0.750, 1.000, 1.000),
    ("FIN-001", "anthropic", "json", 1.000, 0.812, 1.000, 1.000),
    ("FIN-002", "anthropic", "json", 1.000, 0.867, 1.000, 1.000),
    ("FIN-002", "openai", "json", 1.000, 0.600, 1.000, 1.000),
    ("FIN-001", "openai", "toon", 0.800, 0.625, 0.971, 1.000),
    ("FIN-001", "anthropic", "toon", 1.000, 0.812, 1.000, 1.000),
    ("FIN-002", "openai", "toon", 1.000, 0.800, 1.000, 1.000),
    ("FIN-002", "anthropic", "toon", 1.000, 0.800, 1.000, 1.000),
]

# Comparisons
comparisons: @table comparison_result [
    ("FIN-001", "tl", ["anthropic", "openai"], "anthropic", 0.016),
    ("FIN-002", "tl", ["anthropic", "openai"], "anthropic", 0.033),
    ("FIN-001", "json", ["anthropic", "openai"], "anthropic", 0.016),
    ("FIN-002", "json", ["anthropic", "openai"], "anthropic", 0.067),
    ("FIN-001", "toon", ["anthropic", "openai"], "anthropic", 0.103),
    ("FIN-002", "toon", ["openai", "anthropic"], "openai", 0.000),
]

# Summary
summary: {
    total_tasks: 2,
    wins: {
        anthropic: 2,
    },
    avg_scores: {
        openai: 0.927,
        anthropic: 0.952,
    },
    by_category: {
        finance: {
            anthropic: 0.952,
            openai: 0.927,
        },
    },
    by_complexity: {
        Complex: {
            openai: 0.927,
            anthropic: 0.952,
        },
    }
}

# Format Comparison
format_accuracy: @table format_accuracy [
    ("anthropic", "tl", 0.952, 2),
    ("openai", "tl", 0.927, 0),
    ("anthropic", "json", 0.960, 2),
    ("openai", "json", 0.919, 0),
    ("anthropic", "toon", 0.952, 1),
    ("openai", "toon", 0.900, 1),
]

format_tokens: @table format_tokens [
    ("anthropic", "tl", 54341, 3935, 58276),
    ("openai", "tl", 45487, 1881, 47368),
    ("anthropic", "json", 96231, 4063, 100294),
    ("openai", "json", 80417, 2070, 82487),
    ("anthropic", "toon", 54871, 4082, 58953),
    ("openai", "toon", 46399, 2075, 48474),
]
