//! Task definitions and loading

pub mod categories;
pub mod loader;

pub use categories::{Complexity, Domain, OutputType};
pub use loader::{load_format_hints, load_tasks_from_directory, load_tasks_from_file, load_tasks_from_json_file, load_tasks_from_string, LoadError};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::DataFormat;
use crate::providers::CompletionResponse;

/// Per-format hint text loaded from `format_hints.json`
pub type FormatHints = HashMap<String, String>;

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
    /// Inline JSON data (will be converted to TeaLeaf)
    InlineJson(serde_json::Value),
    /// Path to a JSON file (will be loaded and converted to TeaLeaf)
    JsonFile(String),
    /// Raw TeaLeaf data (already in TeaLeaf format)
    RawTL(String),
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
    /// Prompt template with {tl_data} placeholder for data
    pub prompt_template: String,
    /// The actual prompt (filled in during execution)
    #[serde(skip)]
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub expected_elements: Vec<ExpectedElement>,
    pub data_context: Vec<DataReference>,
    pub grading_rubric: Option<String>,
    /// Data source (JSON to be converted to TeaLeaf)
    #[serde(default)]
    pub data_source: DataSource,
    /// Per-format hint flags, e.g. `{"tl": true}` to prepend a hint for TL prompts
    #[serde(default)]
    pub include_format_hint: HashMap<String, bool>,
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
            include_format_hint: HashMap::new(),
        }
    }

    /// Create a task with JSON data that will be converted to TeaLeaf
    pub fn with_json_data(mut self, json: serde_json::Value) -> Self {
        self.data_source = DataSource::InlineJson(json);
        self
    }

    /// Create a task with a JSON file path
    pub fn with_json_file(mut self, path: impl Into<String>) -> Self {
        self.data_source = DataSource::JsonFile(path.into());
        self
    }

    /// Prepare the prompt by converting JSON data to TeaLeaf and inserting it
    pub fn prepare_prompt(&mut self) -> Result<(), String> {
        self.prepare_prompt_with_format(DataFormat::TL, &HashMap::new())
    }

    /// Prepare the prompt with a specific data format (TeaLeaf, JSON, or TOON).
    /// If `format_hints` contains a non-empty string for this format and the task
    /// opts in via `include_format_hint`, the hint is prepended to the prompt.
    pub fn prepare_prompt_with_format(&mut self, format: DataFormat, format_hints: &FormatHints) -> Result<(), String> {
        let data = match format {
            DataFormat::TL => self.get_data_as_tl()?,
            DataFormat::Json => self.get_data_as_json()?,
            DataFormat::Toon => self.get_data_as_toon()?,
        };

        // Replace placeholders in template
        self.prompt = self.prompt_template
            .replace("{tl_data}", &data)
            .replace("{data}", &data)
            .replace("{format_name}", format.display_name());

        // Prepend format hint when the task opts in for this format
        let format_key = format.as_str();
        if self.include_format_hint.get(format_key).copied().unwrap_or(false) {
            if let Some(hint) = format_hints.get(format_key) {
                if !hint.is_empty() {
                    self.prompt = format!("{}\n\n{}", hint, self.prompt);
                }
            }
        }

        Ok(())
    }

    /// Get data in TeaLeaf format
    fn get_data_as_tl(&self) -> Result<String, String> {
        match &self.data_source {
            DataSource::InlineJson(json) => {
                // Convert JSON to TeaLeaf using tealeaf-core
                let json_str = serde_json::to_string_pretty(json)
                    .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
                convert_json_to_tl(&json_str)
            }
            DataSource::JsonFile(path) => {
                // Load and convert JSON file
                let json_str = std::fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read JSON file {}: {}", path, e))?;
                convert_json_to_tl(&json_str)
            }
            DataSource::RawTL(tl) => Ok(tl.clone()),
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
            DataSource::RawTL(tl) => {
                // TeaLeaf data stays as-is when JSON format requested
                // (alternative: could try to convert TeaLeaf to JSON, but complex)
                Ok(tl.clone())
            }
            DataSource::None => Ok(String::new()),
        }
    }

    /// Get data in TOON format
    fn get_data_as_toon(&self) -> Result<String, String> {
        match &self.data_source {
            DataSource::InlineJson(json) => {
                convert_json_to_toon(json)
            }
            DataSource::JsonFile(path) => {
                let json_str = std::fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read JSON file {}: {}", path, e))?;
                let json: serde_json::Value = serde_json::from_str(&json_str)
                    .map_err(|e| format!("Failed to parse JSON: {}", e))?;
                convert_json_to_toon(&json)
            }
            DataSource::RawTL(tl) => Ok(tl.clone()),
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
    /// Data format used for this execution (TeaLeaf or JSON)
    #[serde(default = "default_format")]
    pub format: DataFormat,
    pub status: TaskStatus,
    pub response: Option<TaskResponse>,
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
    /// Number of retry attempts before this result (0 = first attempt succeeded)
    #[serde(default)]
    pub retry_count: u32,
}

fn default_format() -> DataFormat {
    DataFormat::TL
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
    pub http_status: u16,
    pub response_length: usize,
}

impl From<CompletionResponse> for TaskResponse {
    fn from(resp: CompletionResponse) -> Self {
        let response_length = resp.content.len();
        Self {
            content: resp.content,
            model: resp.model,
            input_tokens: resp.input_tokens,
            output_tokens: resp.output_tokens,
            latency_ms: resp.latency_ms,
            finish_reason: resp.finish_reason,
            http_status: resp.http_status,
            response_length,
        }
    }
}

impl TaskResult {
    /// Create a successful result
    pub fn success(task_id: String, provider: String, response: CompletionResponse) -> Self {
        Self::success_with_format(task_id, provider, response, DataFormat::TL, 0)
    }

    /// Create a successful result with specific format and retry count
    pub fn success_with_format(task_id: String, provider: String, response: CompletionResponse, format: DataFormat, retry_count: u32) -> Self {
        Self {
            task_id,
            provider,
            format,
            status: TaskStatus::Success,
            response: Some(response.into()),
            error_message: None,
            timestamp: Utc::now(),
            retry_count,
        }
    }

    /// Create a failure result
    pub fn failure(task_id: String, provider: String, error: String) -> Self {
        Self::failure_with_format(task_id, provider, error, DataFormat::TL, 0)
    }

    /// Create a failure result with specific format and retry count
    pub fn failure_with_format(task_id: String, provider: String, error: String, format: DataFormat, retry_count: u32) -> Self {
        Self {
            task_id,
            provider,
            format,
            status: TaskStatus::Error,
            response: None,
            error_message: Some(error),
            timestamp: Utc::now(),
            retry_count,
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

/// Convert JSON string to TeaLeaf format using tealeaf-core
pub fn convert_json_to_tl(json_str: &str) -> Result<String, String> {
    // Use tealeaf-core's from_json_with_schemas to convert and infer schemas
    let tl = tealeaf::TeaLeaf::from_json_with_schemas(json_str)
        .map_err(|e| format!("Failed to convert JSON to TeaLeaf: {}", e))?;

    // Convert with compact whitespace and compact floats for maximum token savings
    let opts = tealeaf::FormatOptions::compact().with_compact_floats();
    let tl_text = tl.to_tl_with_options(&opts);

    Ok(tl_text)
}

/// Convert JSON file to TeaLeaf format
pub fn convert_json_file_to_tl(path: &str) -> Result<String, String> {
    let json_str = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file {}: {}", path, e))?;
    convert_json_to_tl(&json_str)
}

/// Convert JSON value to TOON format using toon-format
pub fn convert_json_to_toon(json: &serde_json::Value) -> Result<String, String> {
    toon_format::encode_default(json)
        .map_err(|e| format!("Failed to convert JSON to TOON: {}", e))
}
