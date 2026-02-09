//! Accuracy metrics calculation

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::tasks::{ExpectedElement, TaskResponse};

/// Accuracy metrics for a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyMetrics {
    /// Did it address all expected elements? (0.0 - 1.0)
    pub completeness: f64,
    /// How relevant is the response to the task? (0.0 - 1.0)
    pub relevance: f64,
    /// Is the response logically structured? (0.0 - 1.0)
    pub coherence: f64,
    /// For verifiable facts (0.0 - 1.0)
    pub factual_accuracy: f64,
    /// For recommendations (0.0 - 1.0)
    pub actionability: f64,
}

impl AccuracyMetrics {
    /// Calculate composite score with default weights
    pub fn composite_score(&self) -> f64 {
        self.composite_score_weighted(&MetricWeights::default())
    }

    /// Calculate composite score with custom weights
    pub fn composite_score_weighted(&self, weights: &MetricWeights) -> f64 {
        let total_weight = weights.completeness
            + weights.relevance
            + weights.coherence
            + weights.factual_accuracy
            + weights.actionability;

        if total_weight == 0.0 {
            return 0.0;
        }

        (self.completeness * weights.completeness
            + self.relevance * weights.relevance
            + self.coherence * weights.coherence
            + self.factual_accuracy * weights.factual_accuracy
            + self.actionability * weights.actionability)
            / total_weight
    }

    /// Convert to breakdown format
    pub fn to_breakdown(&self) -> Vec<MetricValue> {
        vec![
            MetricValue::new("completeness", self.completeness),
            MetricValue::new("relevance", self.relevance),
            MetricValue::new("coherence", self.coherence),
            MetricValue::new("factual_accuracy", self.factual_accuracy),
            MetricValue::new("actionability", self.actionability),
        ]
    }
}

impl Default for AccuracyMetrics {
    fn default() -> Self {
        Self {
            completeness: 0.0,
            relevance: 0.0,
            coherence: 0.0,
            factual_accuracy: 0.0,
            actionability: 0.0,
        }
    }
}

/// Weights for metric calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricWeights {
    pub completeness: f64,
    pub relevance: f64,
    pub coherence: f64,
    pub factual_accuracy: f64,
    pub actionability: f64,
}

impl Default for MetricWeights {
    fn default() -> Self {
        Self {
            completeness: 0.25,
            relevance: 0.25,
            coherence: 0.20,
            factual_accuracy: 0.20,
            actionability: 0.10,
        }
    }
}

/// A single metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub metric: String,
    pub value: f64,
}

impl MetricValue {
    pub fn new(metric: impl Into<String>, value: f64) -> Self {
        Self {
            metric: metric.into(),
            value,
        }
    }
}

/// Score for an individual expected element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementScore {
    pub element_type: String,
    pub found: bool,
    pub quality_score: f64,
    pub notes: Option<String>,
}

/// Analyze a response against expected elements
pub fn analyze_response(
    response: &TaskResponse,
    expected_elements: &[ExpectedElement],
    task_type: &str,
) -> AccuracyMetrics {
    let content = &response.content;

    // Calculate completeness based on expected elements
    let completeness = calculate_completeness(content, expected_elements);

    // Calculate relevance
    let relevance = calculate_relevance(content, expected_elements);

    // Calculate coherence
    let coherence = analyze_coherence(content);

    // Calculate factual accuracy (based on validation patterns)
    let factual_accuracy = verify_facts(content, expected_elements);

    // Calculate actionability for recommendations
    let actionability = if task_type == "recommendation" {
        score_actionability(content)
    } else {
        1.0 // N/A for non-recommendation tasks
    };

    AccuracyMetrics {
        completeness,
        relevance,
        coherence,
        factual_accuracy,
        actionability,
    }
}

/// Check if an element is present in the response
pub fn detect_element(response: &str, element: &ExpectedElement) -> bool {
    // If there's a validation pattern, use it
    if let Some(pattern) = &element.validation_pattern {
        if let Ok(re) = Regex::new(pattern) {
            return re.is_match(response);
        }
    }

    // Otherwise, check for keywords from description
    let keywords: Vec<&str> = element.description
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .take(5)
        .collect();

    let response_lower = response.to_lowercase();
    keywords.iter().any(|kw| response_lower.contains(&kw.to_lowercase()))
}

/// Calculate completeness score
fn calculate_completeness(content: &str, expected_elements: &[ExpectedElement]) -> f64 {
    if expected_elements.is_empty() {
        return 1.0;
    }

    let found_count = expected_elements
        .iter()
        .filter(|e| detect_element(content, e))
        .count();

    found_count as f64 / expected_elements.len() as f64
}

/// Calculate relevance score
fn calculate_relevance(content: &str, expected_elements: &[ExpectedElement]) -> f64 {
    if expected_elements.is_empty() {
        return 1.0;
    }

    // Count how many element keywords appear in the content
    let mut keyword_matches = 0;
    let mut total_keywords = 0;

    let content_lower = content.to_lowercase();

    for element in expected_elements {
        let keywords: Vec<&str> = element.description
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        total_keywords += keywords.len();
        keyword_matches += keywords
            .iter()
            .filter(|kw| content_lower.contains(&kw.to_lowercase()))
            .count();
    }

    if total_keywords == 0 {
        return 1.0;
    }

    keyword_matches as f64 / total_keywords as f64
}

/// Analyze coherence of the response
fn analyze_coherence(content: &str) -> f64 {
    let mut score = 0.5; // Base score

    // Check for structure markers
    let structure_markers = ["##", "###", "**", "- ", "1.", "2.", "3."];
    let structure_count = structure_markers
        .iter()
        .filter(|m| content.contains(*m))
        .count();

    score += (structure_count as f64 / structure_markers.len() as f64) * 0.2;

    // Check for paragraph breaks
    let paragraph_count = content.split("\n\n").count();
    if paragraph_count >= 3 {
        score += 0.15;
    }

    // Check for reasonable length
    let word_count = content.split_whitespace().count();
    if word_count >= 100 && word_count <= 2000 {
        score += 0.15;
    }

    score.min(1.0)
}

/// Verify facts using validation patterns
fn verify_facts(content: &str, expected_elements: &[ExpectedElement]) -> f64 {
    let elements_with_patterns: Vec<_> = expected_elements
        .iter()
        .filter(|e| e.validation_pattern.is_some())
        .collect();

    if elements_with_patterns.is_empty() {
        return 1.0; // No patterns to verify
    }

    let matched_count = elements_with_patterns
        .iter()
        .filter(|e| {
            if let Some(pattern) = &e.validation_pattern {
                Regex::new(pattern)
                    .map(|re| re.is_match(content))
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .count();

    matched_count as f64 / elements_with_patterns.len() as f64
}

/// Score actionability for recommendations
fn score_actionability(content: &str) -> f64 {
    let rec_patterns = [
        "recommend", "should", "suggest", "consider", "advise",
        "action", "implement", "improve", "optimize", "prioritize",
        "next step", "immediate", "critical", "important",
    ];

    let content_lower = content.to_lowercase();
    let count = rec_patterns
        .iter()
        .filter(|p| content_lower.contains(*p))
        .count();

    // Normalize to 0-1 range (expect at least 3 recommendation keywords)
    (count as f64 / 3.0).min(1.0)
}

/// Score element scores for an analysis
pub fn score_elements(content: &str, expected_elements: &[ExpectedElement]) -> Vec<ElementScore> {
    expected_elements
        .iter()
        .map(|element| {
            let found = detect_element(content, element);

            // Calculate quality based on how well the element is addressed
            let quality_score = if found {
                // Check if validation pattern matches (higher quality)
                if let Some(pattern) = &element.validation_pattern {
                    if Regex::new(pattern).map(|re| re.is_match(content)).unwrap_or(false) {
                        0.9 + (rand_quality() * 0.1) // 0.9-1.0
                    } else {
                        0.6 + (rand_quality() * 0.2) // 0.6-0.8
                    }
                } else {
                    0.7 + (rand_quality() * 0.2) // 0.7-0.9
                }
            } else {
                0.0
            };

            ElementScore {
                element_type: element.element_type.clone(),
                found,
                quality_score,
                notes: None,
            }
        })
        .collect()
}

// Simple deterministic "random" for quality variation
fn rand_quality() -> f64 {
    0.5 // Use fixed value for reproducibility
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_score() {
        let metrics = AccuracyMetrics {
            completeness: 0.8,
            relevance: 0.9,
            coherence: 0.7,
            factual_accuracy: 0.85,
            actionability: 0.75,
        };

        let score = metrics.composite_score();
        assert!(score > 0.7 && score < 0.9);
    }

    #[test]
    fn test_detect_element_with_pattern() {
        let element = ExpectedElement {
            element_type: "metric".to_string(),
            description: "Total revenue".to_string(),
            required: true,
            validation_pattern: Some(r"\$[\d,]+\.?\d*".to_string()),
        };

        assert!(detect_element("The total revenue is $1,234.56", &element));
        assert!(!detect_element("The total revenue is unknown", &element));
    }

    #[test]
    fn test_analyze_coherence() {
        let well_structured = "## Summary\n\n- Point 1\n- Point 2\n\n## Details\n\n**Important**: This is key.";
        let poorly_structured = "just some text without structure";

        let structured_score = analyze_coherence(well_structured);
        let unstructured_score = analyze_coherence(poorly_structured);

        assert!(structured_score > unstructured_score);
    }
}
