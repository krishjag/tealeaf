//! Results reporting

pub mod pax_writer;

pub use pax_writer::PaxWriter;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::analysis::AggregatedResults;

/// JSON summary export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSummary {
    pub run_id: String,
    pub timestamp: String,
    pub total_tasks: usize,
    pub provider_rankings: Vec<ProviderRanking>,
    pub category_breakdown: HashMap<String, CategoryLeader>,
    pub detailed_results_file: String,
}

/// Provider ranking in summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRanking {
    pub provider: String,
    pub wins: u32,
    pub avg_score: f64,
}

/// Category leader info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryLeader {
    pub leader: String,
    pub margin: f64,
}

impl JsonSummary {
    /// Create from aggregated results
    pub fn from_aggregated(
        run_id: impl Into<String>,
        aggregated: &AggregatedResults,
        detailed_file: impl Into<String>,
    ) -> Self {
        let mut rankings: Vec<ProviderRanking> = aggregated
            .avg_scores_by_provider
            .iter()
            .map(|(provider, &score)| {
                let wins = *aggregated.wins_by_provider.get(provider).unwrap_or(&0);
                ProviderRanking {
                    provider: provider.clone(),
                    wins,
                    avg_score: score,
                }
            })
            .collect();

        // Sort by avg_score descending
        rankings.sort_by(|a, b| {
            b.avg_score
                .partial_cmp(&a.avg_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Calculate category leaders
        let category_breakdown: HashMap<String, CategoryLeader> = aggregated
            .scores_by_category
            .iter()
            .filter_map(|(category, providers)| {
                let mut sorted: Vec<_> = providers.iter().collect();
                sorted.sort_by(|a, b| {
                    b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal)
                });

                if sorted.len() >= 2 {
                    let leader = sorted[0];
                    let second = sorted[1];
                    Some((
                        category.clone(),
                        CategoryLeader {
                            leader: leader.0.clone(),
                            margin: leader.1 - second.1,
                        },
                    ))
                } else if let Some(leader) = sorted.first() {
                    Some((
                        category.clone(),
                        CategoryLeader {
                            leader: leader.0.clone(),
                            margin: 0.0,
                        },
                    ))
                } else {
                    None
                }
            })
            .collect();

        Self {
            run_id: run_id.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            total_tasks: aggregated.total_tasks,
            provider_rankings: rankings,
            category_breakdown,
            detailed_results_file: detailed_file.into(),
        }
    }

    /// Write to JSON file
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }
}

/// Generate a console report
pub fn print_console_report(aggregated: &AggregatedResults) {
    println!("\n=== Accuracy Benchmark Results ===\n");
    println!("Total Tasks: {}\n", aggregated.total_tasks);

    // Provider rankings
    println!("Provider Rankings:");
    println!("{:-<50}", "");

    let mut rankings: Vec<_> = aggregated.avg_scores_by_provider.iter().collect();
    rankings.sort_by(|a, b| {
        b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal)
    });

    for (i, (provider, score)) in rankings.iter().enumerate() {
        let wins = aggregated.wins_by_provider.get(*provider).unwrap_or(&0);
        println!(
            "  {}. {} - Avg Score: {:.3}, Wins: {}",
            i + 1,
            provider,
            score,
            wins
        );
    }

    // Category breakdown
    if !aggregated.scores_by_category.is_empty() {
        println!("\nScores by Category:");
        println!("{:-<50}", "");

        for (category, providers) in &aggregated.scores_by_category {
            println!("  {}:", category);
            let mut sorted: Vec<_> = providers.iter().collect();
            sorted.sort_by(|a, b| {
                b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            for (provider, score) in sorted {
                println!("    {}: {:.3}", provider, score);
            }
        }
    }

    // Complexity breakdown
    if !aggregated.scores_by_complexity.is_empty() {
        println!("\nScores by Complexity:");
        println!("{:-<50}", "");

        for (complexity, providers) in &aggregated.scores_by_complexity {
            println!("  {}:", complexity);
            let mut sorted: Vec<_> = providers.iter().collect();
            sorted.sort_by(|a, b| {
                b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            for (provider, score) in sorted {
                println!("    {}: {:.3}", provider, score);
            }
        }
    }

    println!("\n{:=<50}", "");
}
