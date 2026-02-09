# Accuracy Benchmark Results
# Generated: 2026-02-05 15:29:42.396938400 UTC

run_metadata: {
    run_id: "20260205-152419",
    started_at: 2026-02-05T15:24:19Z,
    completed_at: 2026-02-05T15:29:42Z,
    total_tasks: 12,
    providers: [anthropic, openai]
}

# Task Results
responses: @table api_response [
    (FIN-001, openai, "gpt-5.2-2025-12-11", 315, 490, 6742, 2026-02-05T15:24:38Z, success),
    (FIN-001, anthropic, "claude-sonnet-4-5-20250929", 396, 1083, 12309, 2026-02-05T15:24:31Z, success),
    (FIN-002, openai, "gpt-5.2-2025-12-11", 199, 777, 10934, 2026-02-05T15:25:27Z, success),
    (FIN-002, anthropic, "claude-sonnet-4-5-20250929", 248, 1751, 23399, 2026-02-05T15:25:16Z, success),
    (RET-001, anthropic, "claude-sonnet-4-5-20250929", 229, 1087, 16921, 2026-02-05T15:24:36Z, success),
    (RET-001, openai, "gpt-5.2-2025-12-11", 176, 524, 8553, 2026-02-05T15:24:44Z, success),
    (RET-002, openai, "gpt-5.2-2025-12-11", 183, 1064, 21415, 2026-02-05T15:25:47Z, success),
    (RET-002, anthropic, "claude-sonnet-4-5-20250929", 241, 2288, 38158, 2026-02-05T15:25:26Z, success),
    (HLT-001, anthropic, "claude-sonnet-4-5-20250929", 321, 747, 11564, 2026-02-05T15:24:47Z, success),
    (HLT-001, openai, "gpt-5.2-2025-12-11", 253, 398, 8630, 2026-02-05T15:24:56Z, success),
    (TEC-001, anthropic, "claude-sonnet-4-5-20250929", 214, 1964, 39131, 2026-02-05T15:24:58Z, success),
    (TEC-001, openai, "gpt-5.2-2025-12-11", 153, 1083, 21682, 2026-02-05T15:25:20Z, success),
    (MKT-001, openai, "gpt-5.2-2025-12-11", 173, 706, 10470, 2026-02-05T15:25:03Z, success),
    (MKT-001, anthropic, "claude-sonnet-4-5-20250929", 220, 1447, 21077, 2026-02-05T15:24:52Z, success),
    (LOG-001, anthropic, "claude-sonnet-4-5-20250929", 892, 2785, 42584, 2026-02-05T15:25:58Z, success),
    (LOG-001, openai, "gpt-5.2-2025-12-11", 802, 1127, 22171, 2026-02-05T15:26:21Z, success),
    (HR-001, openai, "gpt-5.2-2025-12-11", 654, 1284, 20131, 2026-02-05T15:27:13Z, success),
    (HR-001, anthropic, "claude-sonnet-4-5-20250929", 750, 4206, 63331, 2026-02-05T15:26:53Z, success),
    (MFG-001, anthropic, "claude-sonnet-4-5-20250929", 831, 4249, 63576, 2026-02-05T15:27:02Z, success),
    (MFG-001, openai, "gpt-5.2-2025-12-11", 725, 1423, 22763, 2026-02-05T15:27:25Z, success),
    (RE-001, openai, "gpt-5.2-2025-12-11", 626, 1721, 29040, 2026-02-05T15:26:19Z, success),
    (RE-001, anthropic, "claude-sonnet-4-5-20250929", 730, 3000, 51408, 2026-02-05T15:25:49Z, success),
    (LEG-001, openai, "gpt-5.2-2025-12-11", 609, 1499, 25897, 2026-02-05T15:26:44Z, success),
    (LEG-001, anthropic, "claude-sonnet-4-5-20250929", 721, 3295, 52568, 2026-02-05T15:26:18Z, success),
]

# Analysis Results
analysis_results: @table analysis_result [
    (FIN-001, openai, 0.667, 0.625, 0.943, 0.000),
    (FIN-001, anthropic, 1.000, 1.000, 1.000, 1.000),
    (FIN-002, openai, 1.000, 1.000, 0.971, 1.000),
    (FIN-002, anthropic, 1.000, 1.000, 1.000, 1.000),
    (RET-001, anthropic, 1.000, 1.000, 1.000, 1.000),
    (RET-001, openai, 1.000, 0.833, 0.971, 1.000),
    (RET-002, openai, 1.000, 0.500, 0.914, 1.000),
    (RET-002, anthropic, 1.000, 1.000, 1.000, 1.000),
    (HLT-001, openai, 1.000, 0.750, 0.971, 1.000),
    (HLT-001, anthropic, 1.000, 0.750, 1.000, 1.000),
    (TEC-001, openai, 1.000, 0.833, 1.000, 1.000),
    (TEC-001, anthropic, 1.000, 0.833, 1.000, 1.000),
    (MKT-001, openai, 0.667, 0.250, 1.000, 1.000),
    (MKT-001, anthropic, 1.000, 1.000, 1.000, 1.000),
    (LOG-001, openai, 1.000, 0.857, 1.000, 1.000),
    (LOG-001, anthropic, 1.000, 1.000, 1.000, 1.000),
    (HR-001, anthropic, 1.000, 1.000, 1.000, 1.000),
    (HR-001, openai, 1.000, 0.833, 0.943, 1.000),
    (MFG-001, openai, 1.000, 1.000, 0.943, 1.000),
    (MFG-001, anthropic, 1.000, 1.000, 1.000, 1.000),
    (RE-001, anthropic, 1.000, 0.833, 1.000, 1.000),
    (RE-001, openai, 1.000, 0.833, 0.971, 1.000),
    (LEG-001, openai, 1.000, 0.833, 0.971, 1.000),
    (LEG-001, anthropic, 1.000, 1.000, 1.000, 1.000),
]

# Comparisons
comparisons: @table comparison_result [
    (FIN-001, [anthropic, openai], anthropic, 0.389),
    (FIN-002, [anthropic, openai], anthropic, 0.006),
    (RET-001, [anthropic, openai], anthropic, 0.047),
    (RET-002, [anthropic, openai], anthropic, 0.175),
    (HLT-001, [anthropic, openai], anthropic, 0.006),
    (TEC-001, [openai, anthropic], openai, 0.000),
    (MKT-001, [anthropic, openai], anthropic, 0.271),
    (LOG-001, [anthropic, openai], anthropic, 0.036),
    (HR-001, [anthropic, openai], anthropic, 0.053),
    (MFG-001, [anthropic, openai], anthropic, 0.011),
    (RE-001, [anthropic, openai], anthropic, 0.006),
    (LEG-001, [anthropic, openai], anthropic, 0.047),
]

# Summary
summary: {
    total_tasks: 12,
    wins: {
        anthropic: 11,
        openai: 1,
    },
    avg_scores: {
        anthropic: 0.988,
        openai: 0.901,
    },
    by_category: {
        logistics: {
            anthropic: 1.000,
            openai: 0.964,
        },
        technology: {
            anthropic: 0.958,
            openai: 0.958,
        },
        hr: {
            openai: 0.947,
            anthropic: 1.000,
        },
        healthcare: {
            openai: 0.932,
            anthropic: 0.938,
        },
        finance: {
            anthropic: 1.000,
            openai: 0.803,
        },
        retail: {
            anthropic: 1.000,
            openai: 0.889,
        },
        legal: {
            anthropic: 1.000,
            openai: 0.953,
        },
        manufacturing: {
            anthropic: 1.000,
            openai: 0.989,
        },
        marketing: {
            openai: 0.729,
            anthropic: 1.000,
        },
        real_estate: {
            anthropic: 0.958,
            openai: 0.953,
        },
    },
    by_complexity: {
        Simple: {
            openai: 0.832,
            anthropic: 0.979,
        },
        Moderate: {
            openai: 0.930,
            anthropic: 0.993,
        },
        Complex: {
            anthropic: 0.986,
            openai: 0.910,
        },
    }
}
