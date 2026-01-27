//! Response analysis framework

pub mod comparator;
pub mod metrics;
pub mod scoring;

pub use comparator::{
    AggregatedResults, AnalysisResult, ComparisonEngine, ComparisonResult, ProviderScore,
};
pub use metrics::{
    AccuracyMetrics, ElementScore, MetricValue, MetricWeights, analyze_response, detect_element,
    score_elements,
};
pub use scoring::{
    CriterionScore, EvaluatorType, QualityCriterion, RubricScore, ScoringRubric,
    default_rubric_for_output_type,
};
