//! Scoring framework and rubrics

use regex::Regex;
use serde::{Deserialize, Serialize};

/// A scoring rubric for evaluating responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringRubric {
    pub name: String,
    pub description: String,
    pub criteria: Vec<QualityCriterion>,
}

/// A single quality criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCriterion {
    pub name: String,
    pub description: String,
    pub weight: f64,
    pub evaluator_type: EvaluatorType,
}

/// Type of evaluator to use
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvaluatorType {
    /// Check for presence of keywords
    Keywords { keywords: Vec<String>, threshold: f64 },
    /// Check for numeric values
    ContainsNumbers { min_count: usize },
    /// Check for structured output (headers, lists)
    StructuredOutput,
    /// Check for recommendation language
    Recommendations,
    /// Check for specific regex pattern
    Pattern { pattern: String },
    /// Custom evaluator (name-based lookup)
    Custom { name: String },
}

impl ScoringRubric {
    /// Create a new scoring rubric
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            criteria: Vec::new(),
        }
    }

    /// Add a criterion
    pub fn criterion(mut self, criterion: QualityCriterion) -> Self {
        self.criteria.push(criterion);
        self
    }

    /// Evaluate a response against this rubric
    pub fn evaluate(&self, response: &str) -> RubricScore {
        let criterion_scores: Vec<CriterionScore> = self
            .criteria
            .iter()
            .map(|c| {
                let score = evaluate_criterion(response, c);
                CriterionScore {
                    name: c.name.clone(),
                    score,
                    weight: c.weight,
                    weighted_score: score * c.weight,
                }
            })
            .collect();

        let total_weight: f64 = criterion_scores.iter().map(|c| c.weight).sum();
        let weighted_sum: f64 = criterion_scores.iter().map(|c| c.weighted_score).sum();

        let composite_score = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        RubricScore {
            rubric_name: self.name.clone(),
            composite_score,
            criterion_scores,
        }
    }
}

/// Score from a rubric evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricScore {
    pub rubric_name: String,
    pub composite_score: f64,
    pub criterion_scores: Vec<CriterionScore>,
}

/// Score for a single criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionScore {
    pub name: String,
    pub score: f64,
    pub weight: f64,
    pub weighted_score: f64,
}

/// Evaluate a single criterion
fn evaluate_criterion(response: &str, criterion: &QualityCriterion) -> f64 {
    match &criterion.evaluator_type {
        EvaluatorType::Keywords { keywords, threshold } => {
            evaluate_keywords(response, keywords, *threshold)
        }
        EvaluatorType::ContainsNumbers { min_count } => {
            evaluate_numbers(response, *min_count)
        }
        EvaluatorType::StructuredOutput => {
            evaluate_structure(response)
        }
        EvaluatorType::Recommendations => {
            evaluate_recommendations(response)
        }
        EvaluatorType::Pattern { pattern } => {
            evaluate_pattern(response, pattern)
        }
        EvaluatorType::Custom { name } => {
            evaluate_custom(response, name)
        }
    }
}

/// Evaluate keyword presence
fn evaluate_keywords(response: &str, keywords: &[String], threshold: f64) -> f64 {
    if keywords.is_empty() {
        return 1.0;
    }

    let response_lower = response.to_lowercase();
    let found_count = keywords
        .iter()
        .filter(|kw| response_lower.contains(&kw.to_lowercase()))
        .count();

    let ratio = found_count as f64 / keywords.len() as f64;

    if ratio >= threshold {
        1.0
    } else {
        ratio / threshold
    }
}

/// Evaluate numeric content
fn evaluate_numbers(response: &str, min_count: usize) -> f64 {
    let number_pattern = Regex::new(r"\d+\.?\d*").unwrap();
    let count = number_pattern.find_iter(response).count();

    if count >= min_count {
        1.0
    } else if min_count > 0 {
        count as f64 / min_count as f64
    } else {
        1.0
    }
}

/// Evaluate structured output
fn evaluate_structure(response: &str) -> f64 {
    let structure_markers = [
        "##", "###", "**", "- ", "* ", "1.", "2.", "3.",
        "```", "|", "---",
    ];

    let count = structure_markers
        .iter()
        .filter(|m| response.contains(*m))
        .count();

    (count as f64 / 5.0).min(1.0)
}

/// Evaluate recommendation language
fn evaluate_recommendations(response: &str) -> f64 {
    let rec_patterns = [
        "recommend", "should", "suggest", "consider", "advise",
        "action", "implement", "improve", "optimize", "prioritize",
        "next step", "immediate", "critical", "important", "propose",
    ];

    let response_lower = response.to_lowercase();
    let count = rec_patterns
        .iter()
        .filter(|p| response_lower.contains(*p))
        .count();

    (count as f64 / 4.0).min(1.0)
}

/// Evaluate regex pattern
fn evaluate_pattern(response: &str, pattern: &str) -> f64 {
    match Regex::new(pattern) {
        Ok(re) => {
            if re.is_match(response) {
                1.0
            } else {
                0.0
            }
        }
        Err(_) => 0.0,
    }
}

/// Evaluate using custom evaluator
fn evaluate_custom(response: &str, name: &str) -> f64 {
    match name {
        "length_check" => {
            let words = response.split_whitespace().count();
            if words >= 100 && words <= 2000 {
                1.0
            } else {
                0.5
            }
        }
        "paragraph_structure" => {
            let paragraphs = response.split("\n\n").count();
            if paragraphs >= 3 {
                1.0
            } else {
                paragraphs as f64 / 3.0
            }
        }
        _ => 0.5, // Unknown evaluator
    }
}

/// Create default rubrics for different task types
pub fn default_rubric_for_output_type(output_type: &str) -> ScoringRubric {
    match output_type.to_lowercase().as_str() {
        "summary" | "extraction" => {
            ScoringRubric::new("Summary Rubric", "Evaluates summary/extraction tasks")
                .criterion(QualityCriterion {
                    name: "Completeness".to_string(),
                    description: "Covers all key points".to_string(),
                    weight: 0.3,
                    evaluator_type: EvaluatorType::StructuredOutput,
                })
                .criterion(QualityCriterion {
                    name: "Conciseness".to_string(),
                    description: "Appropriate length".to_string(),
                    weight: 0.2,
                    evaluator_type: EvaluatorType::Custom { name: "length_check".to_string() },
                })
                .criterion(QualityCriterion {
                    name: "Structure".to_string(),
                    description: "Well-organized output".to_string(),
                    weight: 0.2,
                    evaluator_type: EvaluatorType::Custom { name: "paragraph_structure".to_string() },
                })
        }
        "calculation" | "aggregation" => {
            ScoringRubric::new("Calculation Rubric", "Evaluates calculation tasks")
                .criterion(QualityCriterion {
                    name: "Numeric Content".to_string(),
                    description: "Contains numeric results".to_string(),
                    weight: 0.4,
                    evaluator_type: EvaluatorType::ContainsNumbers { min_count: 5 },
                })
                .criterion(QualityCriterion {
                    name: "Structured Output".to_string(),
                    description: "Results clearly presented".to_string(),
                    weight: 0.3,
                    evaluator_type: EvaluatorType::StructuredOutput,
                })
        }
        "analysis" | "pattern" | "trend" => {
            ScoringRubric::new("Analysis Rubric", "Evaluates analysis tasks")
                .criterion(QualityCriterion {
                    name: "Depth".to_string(),
                    description: "Thorough analysis".to_string(),
                    weight: 0.3,
                    evaluator_type: EvaluatorType::Custom { name: "length_check".to_string() },
                })
                .criterion(QualityCriterion {
                    name: "Structure".to_string(),
                    description: "Well-organized".to_string(),
                    weight: 0.3,
                    evaluator_type: EvaluatorType::StructuredOutput,
                })
                .criterion(QualityCriterion {
                    name: "Evidence".to_string(),
                    description: "Supports claims with data".to_string(),
                    weight: 0.2,
                    evaluator_type: EvaluatorType::ContainsNumbers { min_count: 3 },
                })
        }
        "recommendation" | "decision" => {
            ScoringRubric::new("Recommendation Rubric", "Evaluates recommendation tasks")
                .criterion(QualityCriterion {
                    name: "Actionability".to_string(),
                    description: "Clear actionable recommendations".to_string(),
                    weight: 0.4,
                    evaluator_type: EvaluatorType::Recommendations,
                })
                .criterion(QualityCriterion {
                    name: "Structure".to_string(),
                    description: "Prioritized and organized".to_string(),
                    weight: 0.3,
                    evaluator_type: EvaluatorType::StructuredOutput,
                })
                .criterion(QualityCriterion {
                    name: "Justification".to_string(),
                    description: "Rationale provided".to_string(),
                    weight: 0.2,
                    evaluator_type: EvaluatorType::Custom { name: "paragraph_structure".to_string() },
                })
        }
        _ => {
            // Default general rubric
            ScoringRubric::new("General Rubric", "General evaluation")
                .criterion(QualityCriterion {
                    name: "Quality".to_string(),
                    description: "Overall quality".to_string(),
                    weight: 0.5,
                    evaluator_type: EvaluatorType::StructuredOutput,
                })
                .criterion(QualityCriterion {
                    name: "Completeness".to_string(),
                    description: "Response completeness".to_string(),
                    weight: 0.5,
                    evaluator_type: EvaluatorType::Custom { name: "length_check".to_string() },
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_evaluation() {
        let keywords = vec!["revenue".to_string(), "profit".to_string(), "cost".to_string()];
        let response = "The revenue increased while costs remained stable, improving profit margins.";

        let score = evaluate_keywords(response, &keywords, 0.8);
        assert_eq!(score, 1.0); // All keywords found
    }

    #[test]
    fn test_structure_evaluation() {
        let structured = "## Summary\n\n- Point 1\n- Point 2\n\n**Important**: Note here.";
        let unstructured = "This is just plain text without any structure.";

        let structured_score = evaluate_structure(structured);
        let unstructured_score = evaluate_structure(unstructured);

        assert!(structured_score > unstructured_score);
    }

    #[test]
    fn test_rubric_evaluation() {
        let rubric = default_rubric_for_output_type("analysis");
        let response = "## Analysis Results\n\n**Key Finding 1**: Revenue increased by 25%.\n\n**Key Finding 2**: Costs decreased by 10%.\n\nOverall, the data shows positive trends in profitability.";

        let score = rubric.evaluate(response);
        assert!(score.composite_score > 0.5);
    }
}
