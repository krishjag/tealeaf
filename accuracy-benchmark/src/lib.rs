//! Accuracy Benchmark Suite for PAX Format
//!
//! This crate provides a comprehensive benchmark suite for evaluating LLM
//! accuracy across multiple providers using PAX-formatted analysis tasks.
//!
//! # Features
//!
//! - 50 analysis tasks across 10 business domains
//! - Support for Anthropic (Claude), OpenAI, and xAI (Grok) providers
//! - Automated response analysis with accuracy metrics
//! - Cross-provider comparison and ranking
//! - PAX and JSON result output
//!
//! # Example
//!
//! ```no_run
//! use accuracy_benchmark::{
//!     providers::create_all_providers,
//!     runner::{Executor, ExecutorConfig},
//!     tasks::BenchmarkTask,
//!     analysis::ComparisonEngine,
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create providers from environment
//!     let providers = create_all_providers();
//!
//!     // Create executor
//!     let executor = Executor::new(providers, ExecutorConfig::default());
//!
//!     // Create a simple task
//!     let task = BenchmarkTask::new(
//!         "TEST-001",
//!         "test",
//!         "Analyze the following data and provide insights."
//!     );
//!
//!     // Execute
//!     let results = executor.execute_task(&task).await;
//!
//!     // Analyze results
//!     let engine = ComparisonEngine::new();
//!     // ... process results
//! }
//! ```

pub mod analysis;
pub mod config;
pub mod providers;
pub mod reporting;
pub mod runner;
pub mod tasks;

pub use config::{Config, DataFormat};

/// Prelude module for common imports
pub mod prelude {
    pub use crate::analysis::{
        AccuracyMetrics, AggregatedResults, AnalysisResult, ComparisonEngine, ComparisonResult,
    };
    pub use crate::config::{Config, DataFormat};
    pub use crate::providers::{
        create_all_providers, create_providers, CompletionRequest, CompletionResponse,
        LLMProvider, Message, ProviderError, ProviderResult,
    };
    pub use crate::reporting::{print_console_report, JsonSummary, PaxWriter};
    pub use crate::runner::{Executor, ExecutorConfig};
    pub use crate::tasks::{
        BenchmarkTask, Complexity, Domain, OutputType, TaskMetadata, TaskResult, TaskStatus,
    };
}
