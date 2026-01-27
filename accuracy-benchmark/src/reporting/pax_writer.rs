//! PAX format output for benchmark results

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::analysis::{AggregatedResults, AnalysisResult, ComparisonResult};
use crate::tasks::{TaskResult, TaskStatus};

/// Write benchmark results in PAX format
pub struct PaxWriter;

impl PaxWriter {
    /// Write run results to a PAX file
    pub fn write_run_results(
        path: impl AsRef<Path>,
        run_id: &str,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        providers: &[String],
        task_results: &[HashMap<String, TaskResult>],
        analysis_results: &[HashMap<String, AnalysisResult>],
        comparisons: &[ComparisonResult],
        aggregated: &AggregatedResults,
    ) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;

        // Write header comment
        writeln!(file, "# Accuracy Benchmark Results")?;
        writeln!(file, "# Generated: {}", Utc::now())?;
        writeln!(file)?;

        // Write run metadata
        writeln!(file, "run_metadata: {{")?;
        writeln!(file, "    run_id: \"{}\",", run_id)?;
        writeln!(file, "    started_at: {},", format_timestamp(&started_at))?;
        writeln!(file, "    completed_at: {},", format_timestamp(&completed_at))?;
        writeln!(file, "    total_tasks: {},", task_results.len())?;
        writeln!(file, "    providers: [{}]", providers.join(", "))?;
        writeln!(file, "}}")?;
        writeln!(file)?;

        // Write response summary
        writeln!(file, "# Task Results")?;
        write_task_results(&mut file, task_results)?;
        writeln!(file)?;

        // Write analysis results
        writeln!(file, "# Analysis Results")?;
        write_analysis_results(&mut file, analysis_results)?;
        writeln!(file)?;

        // Write comparisons
        writeln!(file, "# Comparisons")?;
        write_comparisons(&mut file, comparisons)?;
        writeln!(file)?;

        // Write summary
        writeln!(file, "# Summary")?;
        write_summary(&mut file, aggregated)?;

        Ok(())
    }

    /// Write just the summary to a PAX file
    pub fn write_summary(
        path: impl AsRef<Path>,
        aggregated: &AggregatedResults,
    ) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;

        writeln!(file, "# Accuracy Benchmark Summary")?;
        writeln!(file, "# Generated: {}", Utc::now())?;
        writeln!(file)?;

        write_summary(&mut file, aggregated)?;

        Ok(())
    }
}

fn format_timestamp(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn write_task_results(
    file: &mut std::fs::File,
    task_results: &[HashMap<String, TaskResult>],
) -> std::io::Result<()> {
    writeln!(file, "responses: @table api_response [")?;

    for task_map in task_results {
        for (provider, result) in task_map {
            let status = match result.status {
                TaskStatus::Success => "success",
                TaskStatus::Error => "error",
                TaskStatus::Timeout => "timeout",
                TaskStatus::RateLimited => "rate_limited",
            };

            if let Some(response) = &result.response {
                writeln!(
                    file,
                    "    ({}, {}, \"{}\", {}, {}, {}, {}, {}),",
                    result.task_id,
                    provider,
                    response.model,
                    response.input_tokens,
                    response.output_tokens,
                    response.latency_ms,
                    format_timestamp(&result.timestamp),
                    status
                )?;
            } else {
                writeln!(
                    file,
                    "    ({}, {}, ~, 0, 0, 0, {}, {}),",
                    result.task_id,
                    provider,
                    format_timestamp(&result.timestamp),
                    status
                )?;
            }
        }
    }

    writeln!(file, "]")?;
    Ok(())
}

fn write_analysis_results(
    file: &mut std::fs::File,
    analysis_results: &[HashMap<String, AnalysisResult>],
) -> std::io::Result<()> {
    writeln!(file, "analysis_results: @table analysis_result [")?;

    for task_map in analysis_results {
        for (provider, result) in task_map {
            writeln!(
                file,
                "    ({}, {}, {:.3}, {:.3}, {:.3}, {:.3}),",
                result.task_id,
                provider,
                result.metrics.completeness,
                result.metrics.relevance,
                result.metrics.coherence,
                result.metrics.factual_accuracy
            )?;
        }
    }

    writeln!(file, "]")?;
    Ok(())
}

fn write_comparisons(
    file: &mut std::fs::File,
    comparisons: &[ComparisonResult],
) -> std::io::Result<()> {
    writeln!(file, "comparisons: @table comparison_result [")?;

    for comp in comparisons {
        let winner = comp.winner.as_deref().unwrap_or("~");
        let margin = comp.margin.map(|m| format!("{:.3}", m)).unwrap_or_else(|| "~".to_string());

        writeln!(
            file,
            "    ({}, [{}], {}, {}),",
            comp.task_id,
            comp.providers_ranked.join(", "),
            winner,
            margin
        )?;
    }

    writeln!(file, "]")?;
    Ok(())
}

fn write_summary(
    file: &mut std::fs::File,
    aggregated: &AggregatedResults,
) -> std::io::Result<()> {
    writeln!(file, "summary: {{")?;
    writeln!(file, "    total_tasks: {},", aggregated.total_tasks)?;

    // Wins
    writeln!(file, "    wins: {{")?;
    for (provider, wins) in &aggregated.wins_by_provider {
        writeln!(file, "        {}: {},", provider, wins)?;
    }
    writeln!(file, "    }},")?;

    // Average scores
    writeln!(file, "    avg_scores: {{")?;
    for (provider, score) in &aggregated.avg_scores_by_provider {
        writeln!(file, "        {}: {:.3},", provider, score)?;
    }
    writeln!(file, "    }},")?;

    // Category breakdown (if available)
    if !aggregated.scores_by_category.is_empty() {
        writeln!(file, "    by_category: {{")?;
        for (category, providers) in &aggregated.scores_by_category {
            writeln!(file, "        {}: {{", category)?;
            for (provider, score) in providers {
                writeln!(file, "            {}: {:.3},", provider, score)?;
            }
            writeln!(file, "        }},")?;
        }
        writeln!(file, "    }},")?;
    }

    // Complexity breakdown (if available)
    if !aggregated.scores_by_complexity.is_empty() {
        writeln!(file, "    by_complexity: {{")?;
        for (complexity, providers) in &aggregated.scores_by_complexity {
            writeln!(file, "        {}: {{", complexity)?;
            for (provider, score) in providers {
                writeln!(file, "            {}: {:.3},", provider, score)?;
            }
            writeln!(file, "        }},")?;
        }
        writeln!(file, "    }}")?;
    }

    writeln!(file, "}}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        let dt = chrono::DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(format_timestamp(&dt), "2024-01-15T10:30:00Z");
    }
}
