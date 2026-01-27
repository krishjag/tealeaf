//! Cross-provider comparison engine

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::metrics::{AccuracyMetrics, MetricWeights, MetricValue};
use crate::tasks::{BenchmarkTask, TaskResult, TaskStatus};

/// Result of analyzing a single task response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub task_id: String,
    pub provider: String,
    pub metrics: AccuracyMetrics,
    pub element_scores: Vec<super::metrics::ElementScore>,
}

/// Score for a provider on a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderScore {
    pub provider: String,
    pub composite_score: f64,
    pub breakdown: Vec<MetricValue>,
}

/// Result of comparing providers on a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub task_id: String,
    pub providers_ranked: Vec<String>,
    pub scores_by_provider: Vec<ProviderScore>,
    pub winner: Option<String>,
    pub margin: Option<f64>,
    pub notes: Option<String>,
}

/// Aggregated results across multiple tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResults {
    pub total_tasks: usize,
    pub wins_by_provider: HashMap<String, u32>,
    pub avg_scores_by_provider: HashMap<String, f64>,
    pub scores_by_category: HashMap<String, HashMap<String, f64>>,
    pub scores_by_complexity: HashMap<String, HashMap<String, f64>>,
}

/// Comparison engine for analyzing results across providers
pub struct ComparisonEngine {
    weights: MetricWeights,
}

impl ComparisonEngine {
    /// Create a new comparison engine with default weights
    pub fn new() -> Self {
        Self {
            weights: MetricWeights::default(),
        }
    }

    /// Create with custom weights
    pub fn with_weights(weights: MetricWeights) -> Self {
        Self { weights }
    }

    /// Analyze a single task result
    pub fn analyze_result(
        &self,
        task: &BenchmarkTask,
        result: &TaskResult,
    ) -> Option<AnalysisResult> {
        if result.status != TaskStatus::Success {
            return None;
        }

        let response = result.response.as_ref()?;

        let element_scores = super::metrics::score_elements(
            &response.content,
            &task.expected_elements,
        );

        let output_type = task.metadata.output_type.to_string();
        let metrics = super::metrics::analyze_response(
            response,
            &task.expected_elements,
            &output_type,
        );

        Some(AnalysisResult {
            task_id: task.metadata.id.clone(),
            provider: result.provider.clone(),
            metrics,
            element_scores,
        })
    }

    /// Compare responses from multiple providers for a single task
    pub fn compare_responses(
        &self,
        task: &BenchmarkTask,
        analysis_results: &HashMap<String, AnalysisResult>,
    ) -> ComparisonResult {
        let mut provider_scores: Vec<ProviderScore> = analysis_results
            .iter()
            .map(|(provider, result)| {
                let composite = result.metrics.composite_score_weighted(&self.weights);
                ProviderScore {
                    provider: provider.clone(),
                    composite_score: composite,
                    breakdown: result.metrics.to_breakdown(),
                }
            })
            .collect();

        // Sort by composite score descending
        provider_scores.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let providers_ranked: Vec<String> = provider_scores
            .iter()
            .map(|s| s.provider.clone())
            .collect();

        let winner = provider_scores.first().map(|s| s.provider.clone());

        let margin = if provider_scores.len() >= 2 {
            Some(provider_scores[0].composite_score - provider_scores[1].composite_score)
        } else {
            None
        };

        ComparisonResult {
            task_id: task.metadata.id.clone(),
            providers_ranked,
            scores_by_provider: provider_scores,
            winner,
            margin,
            notes: None,
        }
    }

    /// Aggregate comparison results across multiple tasks
    pub fn aggregate_comparisons(&self, comparisons: &[ComparisonResult]) -> AggregatedResults {
        let mut wins: HashMap<String, u32> = HashMap::new();
        let mut scores: HashMap<String, Vec<f64>> = HashMap::new();

        for comp in comparisons {
            // Count wins
            if let Some(winner) = &comp.winner {
                *wins.entry(winner.clone()).or_insert(0) += 1;
            }

            // Collect scores
            for ps in &comp.scores_by_provider {
                scores
                    .entry(ps.provider.clone())
                    .or_insert_with(Vec::new)
                    .push(ps.composite_score);
            }
        }

        // Calculate average scores
        let avg_scores: HashMap<String, f64> = scores
            .iter()
            .map(|(k, v)| {
                let avg = if v.is_empty() {
                    0.0
                } else {
                    v.iter().sum::<f64>() / v.len() as f64
                };
                (k.clone(), avg)
            })
            .collect();

        AggregatedResults {
            total_tasks: comparisons.len(),
            wins_by_provider: wins,
            avg_scores_by_provider: avg_scores,
            scores_by_category: HashMap::new(), // Would need task info
            scores_by_complexity: HashMap::new(), // Would need task info
        }
    }

    /// Aggregate with task metadata for category/complexity breakdown
    pub fn aggregate_with_tasks(
        &self,
        comparisons: &[ComparisonResult],
        tasks: &[BenchmarkTask],
    ) -> AggregatedResults {
        let mut base = self.aggregate_comparisons(comparisons);

        // Build task lookup
        let task_map: HashMap<&str, &BenchmarkTask> = tasks
            .iter()
            .map(|t| (t.metadata.id.as_str(), t))
            .collect();

        // Category scores
        let mut category_scores: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();
        let mut complexity_scores: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();

        for comp in comparisons {
            if let Some(task) = task_map.get(comp.task_id.as_str()) {
                let category = &task.metadata.category;
                let complexity = format!("{:?}", task.metadata.complexity);

                for ps in &comp.scores_by_provider {
                    // Category
                    category_scores
                        .entry(category.clone())
                        .or_insert_with(HashMap::new)
                        .entry(ps.provider.clone())
                        .or_insert_with(Vec::new)
                        .push(ps.composite_score);

                    // Complexity
                    complexity_scores
                        .entry(complexity.clone())
                        .or_insert_with(HashMap::new)
                        .entry(ps.provider.clone())
                        .or_insert_with(Vec::new)
                        .push(ps.composite_score);
                }
            }
        }

        // Average category scores
        base.scores_by_category = category_scores
            .into_iter()
            .map(|(cat, providers)| {
                let avg_map: HashMap<String, f64> = providers
                    .into_iter()
                    .map(|(p, scores)| {
                        let avg = if scores.is_empty() {
                            0.0
                        } else {
                            scores.iter().sum::<f64>() / scores.len() as f64
                        };
                        (p, avg)
                    })
                    .collect();
                (cat, avg_map)
            })
            .collect();

        // Average complexity scores
        base.scores_by_complexity = complexity_scores
            .into_iter()
            .map(|(comp, providers)| {
                let avg_map: HashMap<String, f64> = providers
                    .into_iter()
                    .map(|(p, scores)| {
                        let avg = if scores.is_empty() {
                            0.0
                        } else {
                            scores.iter().sum::<f64>() / scores.len() as f64
                        };
                        (p, avg)
                    })
                    .collect();
                (comp, avg_map)
            })
            .collect();

        base
    }
}

impl Default for ComparisonEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Display impl for OutputType
impl std::fmt::Display for crate::tasks::OutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            crate::tasks::OutputType::Summary => write!(f, "summary"),
            crate::tasks::OutputType::Calculation => write!(f, "calculation"),
            crate::tasks::OutputType::Analysis => write!(f, "analysis"),
            crate::tasks::OutputType::Prediction => write!(f, "prediction"),
            crate::tasks::OutputType::Recommendation => write!(f, "recommendation"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_comparisons() {
        let comparisons = vec![
            ComparisonResult {
                task_id: "T1".to_string(),
                providers_ranked: vec!["A".to_string(), "B".to_string()],
                scores_by_provider: vec![
                    ProviderScore {
                        provider: "A".to_string(),
                        composite_score: 0.9,
                        breakdown: vec![],
                    },
                    ProviderScore {
                        provider: "B".to_string(),
                        composite_score: 0.8,
                        breakdown: vec![],
                    },
                ],
                winner: Some("A".to_string()),
                margin: Some(0.1),
                notes: None,
            },
            ComparisonResult {
                task_id: "T2".to_string(),
                providers_ranked: vec!["B".to_string(), "A".to_string()],
                scores_by_provider: vec![
                    ProviderScore {
                        provider: "B".to_string(),
                        composite_score: 0.85,
                        breakdown: vec![],
                    },
                    ProviderScore {
                        provider: "A".to_string(),
                        composite_score: 0.75,
                        breakdown: vec![],
                    },
                ],
                winner: Some("B".to_string()),
                margin: Some(0.1),
                notes: None,
            },
        ];

        let engine = ComparisonEngine::new();
        let aggregated = engine.aggregate_comparisons(&comparisons);

        assert_eq!(aggregated.total_tasks, 2);
        assert_eq!(aggregated.wins_by_provider.get("A"), Some(&1));
        assert_eq!(aggregated.wins_by_provider.get("B"), Some(&1));
    }
}
