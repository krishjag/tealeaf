//! TeaLeaf format output for benchmark results

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::analysis::{AggregatedResults, AnalysisResult, ComparisonResult};
use crate::config::DataFormat;
use crate::tasks::{TaskResult, TaskResultKey, TaskStatus};

/// Write benchmark results in TeaLeaf format
pub struct TLWriter;

impl TLWriter {
    /// Write run results to a TeaLeaf file
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
        format_aggregated: Option<&HashMap<DataFormat, AggregatedResults>>,
        format_token_usage: Option<&HashMap<(String, DataFormat), (u32, u32)>>,
        format_task_results: Option<&HashMap<TaskResultKey, TaskResult>>,
        format_analysis_results: Option<&HashMap<DataFormat, Vec<HashMap<String, AnalysisResult>>>>,
        format_comparisons: Option<&HashMap<DataFormat, Vec<ComparisonResult>>>,
    ) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;

        let has_format_comparison = format_task_results.is_some();

        // Write header comment
        writeln!(file, "# Accuracy Benchmark Results")?;
        writeln!(file, "# Generated: {}", Utc::now())?;
        writeln!(file)?;

        // Write schema definitions for table types
        write_schemas(&mut file, has_format_comparison)?;

        // Write run metadata
        writeln!(file, "run_metadata: {{")?;
        writeln!(file, "    run_id: \"{}\",", run_id)?;
        writeln!(file, "    started_at: {},", format_timestamp(&started_at))?;
        writeln!(file, "    completed_at: {},", format_timestamp(&completed_at))?;
        writeln!(file, "    total_tasks: {},", task_results.len())?;
        writeln!(file, "    providers: [{}]", providers.join(", "))?;
        writeln!(file, "}}")?;
        writeln!(file)?;

        // Write task results — format-tagged when format comparison data available
        writeln!(file, "# Task Results")?;
        if let Some(fmt_results) = format_task_results {
            write_task_results_with_format(&mut file, fmt_results)?;
        } else {
            write_task_results(&mut file, task_results)?;
        }
        writeln!(file)?;

        // Write analysis results — format-tagged when available
        writeln!(file, "# Analysis Results")?;
        if let Some(fmt_analysis) = format_analysis_results {
            write_analysis_results_with_format(&mut file, fmt_analysis)?;
        } else {
            write_analysis_results(&mut file, analysis_results)?;
        }
        writeln!(file)?;

        // Write comparisons — format-tagged when available
        writeln!(file, "# Comparisons")?;
        if let Some(fmt_comparisons) = format_comparisons {
            write_comparisons_with_format(&mut file, fmt_comparisons)?;
        } else {
            write_comparisons(&mut file, comparisons)?;
        }
        writeln!(file)?;

        // Write summary
        writeln!(file, "# Summary")?;
        write_summary(&mut file, aggregated)?;

        // Write format comparison aggregates if available
        if let (Some(fmt_agg), Some(fmt_tokens)) = (format_aggregated, format_token_usage) {
            writeln!(file)?;
            writeln!(file, "# Format Comparison")?;
            write_format_accuracy(&mut file, providers, fmt_agg)?;
            writeln!(file)?;
            write_format_tokens(&mut file, providers, fmt_tokens)?;
        }

        Ok(())
    }

    /// Write just the summary to a TeaLeaf file
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

fn write_schemas(file: &mut std::fs::File, include_format_comparison: bool) -> std::io::Result<()> {
    writeln!(file, "# Schema definitions")?;
    if include_format_comparison {
        // Format-tagged schemas (include format field)
        writeln!(
            file,
            "@struct api_response (task_id: string, provider: string, format: string, model: string?, \
             input_tokens: int, output_tokens: int, latency_ms: int, \
             http_status: int, retry_count: int, response_length: int, \
             timestamp: timestamp, status: string)"
        )?;
        writeln!(
            file,
            "@struct analysis_result (task_id: string, provider: string, format: string, \
             completeness: float, relevance: float, coherence: float, \
             factual_accuracy: float)"
        )?;
        writeln!(
            file,
            "@struct comparison_result (task_id: string, format: string, providers_ranked: []string, \
             winner: string?, margin: float?)"
        )?;
        writeln!(
            file,
            "@struct format_accuracy (provider: string, format: string, \
             avg_score: float, wins: int)"
        )?;
        writeln!(
            file,
            "@struct format_tokens (provider: string, format: string, \
             input_tokens: int, output_tokens: int, total_tokens: int)"
        )?;
    } else {
        // Legacy schemas (no format field)
        writeln!(
            file,
            "@struct api_response (task_id: string, provider: string, model: string?, \
             input_tokens: int, output_tokens: int, latency_ms: int, \
             http_status: int, retry_count: int, response_length: int, \
             timestamp: timestamp, status: string)"
        )?;
        writeln!(
            file,
            "@struct analysis_result (task_id: string, provider: string, \
             completeness: float, relevance: float, coherence: float, \
             factual_accuracy: float)"
        )?;
        writeln!(
            file,
            "@struct comparison_result (task_id: string, providers_ranked: []string, \
             winner: string?, margin: float?)"
        )?;
    }
    writeln!(file)?;
    Ok(())
}

// === Legacy writers (no format field) ===

fn write_task_results(
    file: &mut std::fs::File,
    task_results: &[HashMap<String, TaskResult>],
) -> std::io::Result<()> {
    writeln!(file, "responses: @table api_response [")?;

    for task_map in task_results {
        for (provider, result) in task_map {
            let status = status_str(result.status);
            if let Some(response) = &result.response {
                writeln!(
                    file,
                    "    (\"{}\", \"{}\", \"{}\", {}, {}, {}, {}, {}, {}, {}, \"{}\"),",
                    result.task_id, provider, response.model,
                    response.input_tokens, response.output_tokens, response.latency_ms,
                    response.http_status, result.retry_count, response.response_length,
                    format_timestamp(&result.timestamp), status
                )?;
            } else {
                writeln!(
                    file,
                    "    (\"{}\", \"{}\", ~, 0, 0, 0, 0, {}, 0, {}, \"{}\"),",
                    result.task_id, provider,
                    result.retry_count,
                    format_timestamp(&result.timestamp), status
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
                "    (\"{}\", \"{}\", {:.3}, {:.3}, {:.3}, {:.3}),",
                result.task_id, provider,
                result.metrics.completeness, result.metrics.relevance,
                result.metrics.coherence, result.metrics.factual_accuracy
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
        write_comparison_row(file, comp, None)?;
    }

    writeln!(file, "]")?;
    Ok(())
}

// === Format-tagged writers ===

fn write_task_results_with_format(
    file: &mut std::fs::File,
    format_results: &HashMap<TaskResultKey, TaskResult>,
) -> std::io::Result<()> {
    writeln!(file, "responses: @table api_response [")?;

    // Sort by format then task_id then provider for deterministic output
    let mut entries: Vec<_> = format_results.iter().collect();
    entries.sort_by(|(ka, _), (kb, _)| {
        ka.format.as_str().cmp(kb.format.as_str())
            .then_with(|| ka.task_id.cmp(&kb.task_id))
            .then_with(|| ka.provider.cmp(&kb.provider))
    });

    for (key, result) in entries {
        let status = status_str(result.status);
        let fmt = key.format.as_str();
        if let Some(response) = &result.response {
            writeln!(
                file,
                "    (\"{}\", \"{}\", \"{}\", \"{}\", {}, {}, {}, {}, {}, {}, {}, \"{}\"),",
                result.task_id, key.provider, fmt, response.model,
                response.input_tokens, response.output_tokens, response.latency_ms,
                response.http_status, result.retry_count, response.response_length,
                format_timestamp(&result.timestamp), status
            )?;
        } else {
            writeln!(
                file,
                "    (\"{}\", \"{}\", \"{}\", ~, 0, 0, 0, 0, {}, 0, {}, \"{}\"),",
                result.task_id, key.provider, fmt,
                result.retry_count,
                format_timestamp(&result.timestamp), status
            )?;
        }
    }

    writeln!(file, "]")?;
    Ok(())
}

fn write_analysis_results_with_format(
    file: &mut std::fs::File,
    format_analysis: &HashMap<DataFormat, Vec<HashMap<String, AnalysisResult>>>,
) -> std::io::Result<()> {
    writeln!(file, "analysis_results: @table analysis_result [")?;

    for &fmt in &DataFormat::all() {
        if let Some(tasks) = format_analysis.get(&fmt) {
            for task_map in tasks {
                for (provider, result) in task_map {
                    writeln!(
                        file,
                        "    (\"{}\", \"{}\", \"{}\", {:.3}, {:.3}, {:.3}, {:.3}),",
                        result.task_id, provider, fmt.as_str(),
                        result.metrics.completeness, result.metrics.relevance,
                        result.metrics.coherence, result.metrics.factual_accuracy
                    )?;
                }
            }
        }
    }

    writeln!(file, "]")?;
    Ok(())
}

fn write_comparisons_with_format(
    file: &mut std::fs::File,
    format_comparisons: &HashMap<DataFormat, Vec<ComparisonResult>>,
) -> std::io::Result<()> {
    writeln!(file, "comparisons: @table comparison_result [")?;

    for &fmt in &DataFormat::all() {
        if let Some(comparisons) = format_comparisons.get(&fmt) {
            for comp in comparisons {
                write_comparison_row(file, comp, Some(fmt))?;
            }
        }
    }

    writeln!(file, "]")?;
    Ok(())
}

// === Shared helpers ===

fn status_str(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Success => "success",
        TaskStatus::Error => "error",
        TaskStatus::Timeout => "timeout",
        TaskStatus::RateLimited => "rate_limited",
    }
}

fn write_comparison_row(
    file: &mut std::fs::File,
    comp: &ComparisonResult,
    format: Option<DataFormat>,
) -> std::io::Result<()> {
    let margin = comp.margin.map(|m| format!("{:.3}", m)).unwrap_or_else(|| "~".to_string());
    let providers_quoted: Vec<String> = comp.providers_ranked.iter()
        .map(|p| format!("\"{}\"", p))
        .collect();
    let winner_str = match &comp.winner {
        Some(w) => format!("\"{}\"", w),
        None => "~".to_string(),
    };

    if let Some(fmt) = format {
        writeln!(
            file,
            "    (\"{}\", \"{}\", [{}], {}, {}),",
            comp.task_id, fmt.as_str(),
            providers_quoted.join(", "), winner_str, margin
        )?;
    } else {
        writeln!(
            file,
            "    (\"{}\", [{}], {}, {}),",
            comp.task_id,
            providers_quoted.join(", "), winner_str, margin
        )?;
    }
    Ok(())
}

fn write_format_accuracy(
    file: &mut std::fs::File,
    providers: &[String],
    format_aggregated: &HashMap<DataFormat, AggregatedResults>,
) -> std::io::Result<()> {
    writeln!(file, "format_accuracy: @table format_accuracy [")?;
    for &fmt in &DataFormat::all() {
        if let Some(agg) = format_aggregated.get(&fmt) {
            for provider in providers {
                let score = agg.avg_scores_by_provider.get(provider).copied().unwrap_or(0.0);
                let wins = agg.wins_by_provider.get(provider).copied().unwrap_or(0);
                writeln!(
                    file,
                    "    (\"{}\", \"{}\", {:.3}, {}),",
                    provider, fmt.as_str(), score, wins,
                )?;
            }
        }
    }
    writeln!(file, "]")?;
    Ok(())
}

fn write_format_tokens(
    file: &mut std::fs::File,
    providers: &[String],
    token_usage: &HashMap<(String, DataFormat), (u32, u32)>,
) -> std::io::Result<()> {
    writeln!(file, "format_tokens: @table format_tokens [")?;
    for &fmt in &DataFormat::all() {
        for provider in providers {
            let (input, output) = token_usage
                .get(&(provider.clone(), fmt))
                .copied()
                .unwrap_or((0, 0));
            writeln!(
                file,
                "    (\"{}\", \"{}\", {}, {}, {}),",
                provider, fmt.as_str(), input, output, input + output,
            )?;
        }
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

    writeln!(file, "    wins: {{")?;
    for (provider, wins) in &aggregated.wins_by_provider {
        writeln!(file, "        {}: {},", provider, wins)?;
    }
    writeln!(file, "    }},")?;

    writeln!(file, "    avg_scores: {{")?;
    for (provider, score) in &aggregated.avg_scores_by_provider {
        writeln!(file, "        {}: {:.3},", provider, score)?;
    }
    writeln!(file, "    }},")?;

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
