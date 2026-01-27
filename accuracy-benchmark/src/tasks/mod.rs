//! Task definitions and loading

pub mod categories;
pub mod loader;

pub use categories::{Complexity, Domain, OutputType};
pub use loader::{load_tasks_from_directory, load_tasks_from_file, load_tasks_from_string, LoadError};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::DataFormat;
use crate::providers::CompletionResponse;

/// Metadata for a benchmark task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub id: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub complexity: Complexity,
    pub output_type: OutputType,
    pub version: String,
}

/// An expected element in a task response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedElement {
    pub element_type: String,
    pub description: String,
    pub required: bool,
    pub validation_pattern: Option<String>,
}

/// A reference to data context for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReference {
    pub source_file: String,
    pub section: Option<String>,
    pub description: String,
}

/// Input data source for a benchmark task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    /// Inline JSON data (will be converted to PAX)
    InlineJson(serde_json::Value),
    /// Path to a JSON file (will be loaded and converted to PAX)
    JsonFile(String),
    /// Raw PAX data (already in PAX format)
    RawPax(String),
    /// No data - prompt is self-contained
    None,
}

impl Default for DataSource {
    fn default() -> Self {
        DataSource::None
    }
}

/// A benchmark task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub metadata: TaskMetadata,
    /// Prompt template with {pax_data} placeholder for data
    pub prompt_template: String,
    /// The actual prompt (filled in during execution)
    #[serde(skip)]
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub expected_elements: Vec<ExpectedElement>,
    pub data_context: Vec<DataReference>,
    pub grading_rubric: Option<String>,
    /// Data source (JSON to be converted to PAX)
    #[serde(default)]
    pub data_source: DataSource,
}

impl BenchmarkTask {
    /// Create a new benchmark task
    pub fn new(id: impl Into<String>, category: impl Into<String>, prompt_template: impl Into<String>) -> Self {
        let template = prompt_template.into();
        Self {
            metadata: TaskMetadata {
                id: id.into(),
                category: category.into(),
                subcategory: None,
                complexity: Complexity::Moderate,
                output_type: OutputType::Analysis,
                version: "1.0".to_string(),
            },
            prompt_template: template.clone(),
            prompt: template,
            max_tokens: 2048,
            temperature: Some(0.3),
            expected_elements: Vec::new(),
            data_context: Vec::new(),
            grading_rubric: None,
            data_source: DataSource::None,
        }
    }

    /// Create a task with JSON data that will be converted to PAX
    pub fn with_json_data(mut self, json: serde_json::Value) -> Self {
        self.data_source = DataSource::InlineJson(json);
        self
    }

    /// Create a task with a JSON file path
    pub fn with_json_file(mut self, path: impl Into<String>) -> Self {
        self.data_source = DataSource::JsonFile(path.into());
        self
    }

    /// Prepare the prompt by converting JSON data to PAX and inserting it
    pub fn prepare_prompt(&mut self) -> Result<(), String> {
        self.prepare_prompt_with_format(DataFormat::Pax)
    }

    /// Prepare the prompt with a specific data format (PAX or JSON)
    pub fn prepare_prompt_with_format(&mut self, format: DataFormat) -> Result<(), String> {
        let data = match format {
            DataFormat::Pax => self.get_data_as_pax()?,
            DataFormat::Json => self.get_data_as_json()?,
        };

        // Replace {pax_data} or {data} placeholder in template
        self.prompt = self.prompt_template
            .replace("{pax_data}", &data)
            .replace("{data}", &data);
        Ok(())
    }

    /// Get data in PAX format
    fn get_data_as_pax(&self) -> Result<String, String> {
        match &self.data_source {
            DataSource::InlineJson(json) => {
                // Convert JSON to PAX using pax-core
                let json_str = serde_json::to_string_pretty(json)
                    .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
                convert_json_to_pax(&json_str)
            }
            DataSource::JsonFile(path) => {
                // Load and convert JSON file
                let json_str = std::fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read JSON file {}: {}", path, e))?;
                convert_json_to_pax(&json_str)
            }
            DataSource::RawPax(pax) => Ok(pax.clone()),
            DataSource::None => Ok(String::new()),
        }
    }

    /// Get data in JSON format
    fn get_data_as_json(&self) -> Result<String, String> {
        match &self.data_source {
            DataSource::InlineJson(json) => {
                serde_json::to_string_pretty(json)
                    .map_err(|e| format!("Failed to serialize JSON: {}", e))
            }
            DataSource::JsonFile(path) => {
                std::fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read JSON file {}: {}", path, e))
            }
            DataSource::RawPax(pax) => {
                // PAX data stays as-is when JSON format requested
                // (alternative: could try to convert PAX to JSON, but complex)
                Ok(pax.clone())
            }
            DataSource::None => Ok(String::new()),
        }
    }

    /// Check if this task has data that can be formatted
    pub fn has_data(&self) -> bool {
        !matches!(self.data_source, DataSource::None)
    }

    /// Set complexity
    pub fn with_complexity(mut self, complexity: Complexity) -> Self {
        self.metadata.complexity = complexity;
        self
    }

    /// Set output type
    pub fn with_output_type(mut self, output_type: OutputType) -> Self {
        self.metadata.output_type = output_type;
        self
    }

    /// Add an expected element
    pub fn expect(mut self, element_type: impl Into<String>, description: impl Into<String>, required: bool) -> Self {
        self.expected_elements.push(ExpectedElement {
            element_type: element_type.into(),
            description: description.into(),
            required,
            validation_pattern: None,
        });
        self
    }

    /// Add an expected element with validation pattern
    pub fn expect_with_pattern(
        mut self,
        element_type: impl Into<String>,
        description: impl Into<String>,
        required: bool,
        pattern: impl Into<String>,
    ) -> Self {
        self.expected_elements.push(ExpectedElement {
            element_type: element_type.into(),
            description: description.into(),
            required,
            validation_pattern: Some(pattern.into()),
        });
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Result of executing a task for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub provider: String,
    /// Data format used for this execution (PAX or JSON)
    #[serde(default = "default_format")]
    pub format: DataFormat,
    pub status: TaskStatus,
    pub response: Option<TaskResponse>,
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

fn default_format() -> DataFormat {
    DataFormat::Pax
}

/// Key for identifying task results by task, provider, and format
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskResultKey {
    pub task_id: String,
    pub provider: String,
    pub format: DataFormat,
}

impl TaskResultKey {
    pub fn new(task_id: impl Into<String>, provider: impl Into<String>, format: DataFormat) -> Self {
        Self {
            task_id: task_id.into(),
            provider: provider.into(),
            format,
        }
    }
}

/// Status of a task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Success,
    Error,
    Timeout,
    RateLimited,
}

/// Response data from a task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
    pub finish_reason: String,
}

impl From<CompletionResponse> for TaskResponse {
    fn from(resp: CompletionResponse) -> Self {
        Self {
            content: resp.content,
            model: resp.model,
            input_tokens: resp.input_tokens,
            output_tokens: resp.output_tokens,
            latency_ms: resp.latency_ms,
            finish_reason: resp.finish_reason,
        }
    }
}

impl TaskResult {
    /// Create a successful result
    pub fn success(task_id: String, provider: String, response: CompletionResponse) -> Self {
        Self::success_with_format(task_id, provider, response, DataFormat::Pax)
    }

    /// Create a successful result with specific format
    pub fn success_with_format(task_id: String, provider: String, response: CompletionResponse, format: DataFormat) -> Self {
        Self {
            task_id,
            provider,
            format,
            status: TaskStatus::Success,
            response: Some(response.into()),
            error_message: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a failure result
    pub fn failure(task_id: String, provider: String, error: String) -> Self {
        Self::failure_with_format(task_id, provider, error, DataFormat::Pax)
    }

    /// Create a failure result with specific format
    pub fn failure_with_format(task_id: String, provider: String, error: String, format: DataFormat) -> Self {
        Self {
            task_id,
            provider,
            format,
            status: TaskStatus::Error,
            response: None,
            error_message: Some(error),
            timestamp: Utc::now(),
        }
    }

    /// Check if the result was successful
    pub fn is_success(&self) -> bool {
        self.status == TaskStatus::Success
    }

    /// Get the result key
    pub fn key(&self) -> TaskResultKey {
        TaskResultKey::new(&self.task_id, &self.provider, self.format)
    }
}

/// Convert JSON string to PAX format using pax-core
pub fn convert_json_to_pax(json_str: &str) -> Result<String, String> {
    // Use pax-core's from_json_with_schemas to convert and infer schemas
    let pax = pax::Pax::from_json_with_schemas(json_str)
        .map_err(|e| format!("Failed to convert JSON to PAX: {}", e))?;

    // Convert back to PAX text format
    let pax_text = pax.to_pax_with_schemas();

    Ok(pax_text)
}

/// Convert JSON file to PAX format
pub fn convert_json_file_to_pax(path: &str) -> Result<String, String> {
    let json_str = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file {}: {}", path, e))?;
    convert_json_to_pax(&json_str)
}
