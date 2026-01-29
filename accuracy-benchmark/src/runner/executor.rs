//! Async task executor for running benchmark tasks

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;

use crate::config::DataFormat;
use crate::providers::{CompletionRequest, CompletionResponse, LLMProvider, Message, ProviderError};
use crate::tasks::{BenchmarkTask, TaskResult, TaskResultKey};

/// Configuration for the executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum parallel requests per provider
    pub parallel_requests: usize,
    /// Number of retries on failure
    pub retry_count: u32,
    /// Initial retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    pub max_retry_delay_ms: u64,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Enable format comparison (run tasks in both TeaLeaf and JSON formats)
    pub compare_formats: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            parallel_requests: 3,
            retry_count: 3,
            retry_delay_ms: 1000,
            max_retry_delay_ms: 60_000,
            timeout_ms: 120_000,
            compare_formats: false,
        }
    }
}

/// Executor for running benchmark tasks
pub struct Executor {
    config: ExecutorConfig,
    providers: Vec<Arc<dyn LLMProvider + Send + Sync>>,
    semaphore: Arc<Semaphore>,
}

impl Executor {
    /// Create a new executor
    pub fn new(
        providers: Vec<Arc<dyn LLMProvider + Send + Sync>>,
        config: ExecutorConfig,
    ) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.parallel_requests * providers.len()));
        Self {
            config,
            providers,
            semaphore,
        }
    }

    /// Execute a single task against all providers (TeaLeaf format only, legacy interface)
    pub async fn execute_task(
        &self,
        task: &BenchmarkTask,
    ) -> HashMap<String, TaskResult> {
        // Convert to the new format-aware results
        let format_results = self.execute_task_with_formats(task).await;

        // Extract only TeaLeaf results for backward compatibility
        let mut results = HashMap::new();
        for (key, result) in format_results {
            if key.format == DataFormat::TL {
                results.insert(key.provider, result);
            }
        }
        results
    }

    /// Execute a single task against all providers with format comparison
    pub async fn execute_task_with_formats(
        &self,
        task: &BenchmarkTask,
    ) -> HashMap<TaskResultKey, TaskResult> {
        let mut results = HashMap::new();

        // Determine which formats to test
        let formats = if self.config.compare_formats && task.has_data() {
            DataFormat::all()
        } else {
            vec![DataFormat::TL]
        };

        for format in formats {
            // Prepare the task with the specific format
            let mut prepared_task = task.clone();
            if let Err(e) = prepared_task.prepare_prompt_with_format(format) {
                // If preparation fails, return error for all providers
                for provider in &self.providers {
                    let key = TaskResultKey::new(&task.metadata.id, provider.name(), format);
                    results.insert(
                        key,
                        TaskResult::failure_with_format(
                            task.metadata.id.clone(),
                            provider.name().to_string(),
                            format!("Failed to prepare task with {} format: {}", format, e),
                            format,
                        ),
                    );
                }
                continue;
            }

            for provider in &self.providers {
                let result = self.execute_task_for_provider_with_format(
                    &prepared_task,
                    provider.clone(),
                    format,
                ).await;
                let key = TaskResultKey::new(&task.metadata.id, provider.name(), format);
                results.insert(key, result);
            }
        }

        results
    }

    /// Execute a task for a specific provider (legacy, TeaLeaf format)
    #[allow(dead_code)]
    async fn execute_task_for_provider(
        &self,
        task: &BenchmarkTask,
        provider: Arc<dyn LLMProvider + Send + Sync>,
    ) -> TaskResult {
        self.execute_task_for_provider_with_format(task, provider, DataFormat::TL).await
    }

    /// Execute a task for a specific provider with a specific format
    async fn execute_task_for_provider_with_format(
        &self,
        task: &BenchmarkTask,
        provider: Arc<dyn LLMProvider + Send + Sync>,
        format: DataFormat,
    ) -> TaskResult {
        let _permit = self.semaphore.acquire().await.unwrap();

        let mut last_error = None;
        let mut delay = self.config.retry_delay_ms;

        for attempt in 0..=self.config.retry_count {
            if attempt > 0 {
                tracing::info!(
                    "Retry {} for task {} on {} ({})",
                    attempt,
                    task.metadata.id,
                    provider.name(),
                    format
                );
                sleep(Duration::from_millis(delay)).await;
                delay = (delay * 2).min(self.config.max_retry_delay_ms);
            }

            match self.try_execute(task, &provider).await {
                Ok(response) => {
                    return TaskResult::success_with_format(
                        task.metadata.id.clone(),
                        provider.name().to_string(),
                        response,
                        format,
                    );
                }
                Err(ProviderError::RateLimited { retry_after_ms }) => {
                    tracing::warn!(
                        "Rate limited on {} ({}), waiting {}ms",
                        provider.name(),
                        format,
                        retry_after_ms
                    );
                    sleep(Duration::from_millis(retry_after_ms)).await;
                    last_error = Some(ProviderError::RateLimited { retry_after_ms });
                }
                Err(e) => {
                    tracing::error!(
                        "Error on {} for task {} ({}): {}",
                        provider.name(),
                        task.metadata.id,
                        format,
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        TaskResult::failure_with_format(
            task.metadata.id.clone(),
            provider.name().to_string(),
            last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string()),
            format,
        )
    }

    /// Try to execute a task (single attempt)
    async fn try_execute(
        &self,
        task: &BenchmarkTask,
        provider: &Arc<dyn LLMProvider + Send + Sync>,
    ) -> Result<CompletionResponse, ProviderError> {
        let request = CompletionRequest::new(
            vec![Message::user(&task.prompt)],
            task.max_tokens,
        )
        .with_temperature(task.temperature.unwrap_or(0.3));

        let timeout = Duration::from_millis(self.config.timeout_ms);

        match tokio::time::timeout(timeout, provider.complete(&request)).await {
            Ok(result) => result,
            Err(_) => Err(ProviderError::Timeout {
                timeout_ms: self.config.timeout_ms,
            }),
        }
    }

    /// Execute multiple tasks in parallel (legacy, returns provider-keyed results)
    pub async fn execute_tasks(
        &self,
        tasks: &[BenchmarkTask],
    ) -> Vec<HashMap<String, TaskResult>> {
        let mut handles = Vec::new();

        for task in tasks {
            let task = task.clone();
            let executor = self.clone_for_task();

            let handle = tokio::spawn(async move {
                executor.execute_task(&task).await
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!("Task execution panicked: {}", e);
                    results.push(HashMap::new());
                }
            }
        }

        results
    }

    /// Execute multiple tasks in parallel with format comparison
    pub async fn execute_tasks_with_formats(
        &self,
        tasks: &[BenchmarkTask],
    ) -> HashMap<TaskResultKey, TaskResult> {
        let mut handles = Vec::new();

        for task in tasks {
            let task = task.clone();
            let executor = self.clone_for_task();

            let handle = tokio::spawn(async move {
                executor.execute_task_with_formats(&task).await
            });

            handles.push(handle);
        }

        let mut all_results = HashMap::new();
        for handle in handles {
            match handle.await {
                Ok(task_results) => {
                    all_results.extend(task_results);
                }
                Err(e) => {
                    tracing::error!("Task execution panicked: {}", e);
                }
            }
        }

        all_results
    }

    /// Clone the executor for spawning tasks
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            providers: self.providers.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}

/// Progress callback for tracking execution
pub trait ProgressCallback: Send + Sync {
    fn on_task_start(&self, task_id: &str, provider: &str);
    fn on_task_complete(&self, task_id: &str, provider: &str, success: bool);
    fn on_progress(&self, completed: usize, total: usize);
}

/// Default no-op progress callback
pub struct NoOpProgress;

impl ProgressCallback for NoOpProgress {
    fn on_task_start(&self, _task_id: &str, _provider: &str) {}
    fn on_task_complete(&self, _task_id: &str, _provider: &str, _success: bool) {}
    fn on_progress(&self, _completed: usize, _total: usize) {}
}

/// Console progress callback
pub struct ConsoleProgress;

impl ProgressCallback for ConsoleProgress {
    fn on_task_start(&self, task_id: &str, provider: &str) {
        println!("  Starting {} on {}...", task_id, provider);
    }

    fn on_task_complete(&self, task_id: &str, provider: &str, success: bool) {
        let status = if success { "OK" } else { "FAILED" };
        println!("  {} {} on {}: {}", status, task_id, provider, status);
    }

    fn on_progress(&self, completed: usize, total: usize) {
        println!("Progress: {}/{} tasks complete", completed, total);
    }
}
