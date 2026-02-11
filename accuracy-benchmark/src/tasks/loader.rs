//! Task loading from TeaLeaf and JSON files

use std::path::Path;
use serde::Deserialize;

use super::{BenchmarkTask, DataSource, ExpectedElement, TaskMetadata};
use super::categories::{Complexity, OutputType};

/// Error type for task loading
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Missing field: {0}")]
    MissingField(String),
}

/// Load tasks from a TeaLeaf file
pub fn load_tasks_from_file(path: impl AsRef<Path>) -> Result<Vec<BenchmarkTask>, LoadError> {
    let content = std::fs::read_to_string(path)?;
    load_tasks_from_string(&content)
}

/// Load tasks from a TeaLeaf string
pub fn load_tasks_from_string(content: &str) -> Result<Vec<BenchmarkTask>, LoadError> {
    // Parse using tealeaf-core
    let tl = tealeaf::TeaLeaf::parse(content)
        .map_err(|e| LoadError::Parse(e.to_string()))?;

    let mut tasks = Vec::new();

    // Look for "tasks" key containing a table
    if let Some(tasks_value) = tl.data.get("tasks") {
        if let Some(arr) = tasks_value.as_array() {
            for (idx, row) in arr.iter().enumerate() {
                let task = parse_task_from_value(row, idx)
                    .map_err(|e| LoadError::Parse(format!("Task {}: {}", idx, e)))?;
                tasks.push(task);
            }
        }
    }

    Ok(tasks)
}

/// Parse a single task from a TeaLeaf value
fn parse_task_from_value(value: &tealeaf::Value, _index: usize) -> Result<BenchmarkTask, String> {
    // Handle both object and tuple formats
    let obj = match value {
        tealeaf::Value::Object(o) => o.clone(),
        tealeaf::Value::Array(arr) if arr.len() >= 5 => {
            // Tuple format: (metadata, prompt, data_refs, expected_elements, grading_rubric, max_tokens, temperature?)
            return parse_task_from_tuple(arr);
        }
        _ => return Err(format!("Expected object or tuple, got {:?}", value)),
    };

    // Parse from object format
    let metadata = parse_metadata(&obj)?;

    let prompt = obj
        .get("prompt_template")
        .or_else(|| obj.get("prompt"))
        .and_then(|v| v.as_str())
        .ok_or("Missing prompt_template")?
        .to_string();

    let max_tokens = obj
        .get("max_tokens")
        .and_then(|v| v.as_int())
        .unwrap_or(2048) as u32;

    let temperature = obj
        .get("temperature")
        .and_then(|v| v.as_float())
        .map(|f| f as f32);

    let expected_elements = parse_expected_elements(&obj)?;

    Ok(BenchmarkTask {
        metadata,
        prompt_template: prompt.clone(),
        prompt,
        max_tokens,
        temperature,
        expected_elements,
        data_context: Vec::new(),
        grading_rubric: obj.get("grading_rubric").and_then(|v| v.as_str()).map(String::from),
        data_source: DataSource::None,
    })
}

/// Parse task from tuple format
fn parse_task_from_tuple(arr: &[tealeaf::Value]) -> Result<BenchmarkTask, String> {
    // Format: (metadata_tuple, prompt, data_refs, expected_elements, grading_rubric?, max_tokens, temperature?)
    let metadata = parse_metadata_from_tuple(arr.first().ok_or("Missing metadata")?)?;

    let prompt = arr
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("Missing prompt")?
        .to_string();

    let expected_elements = if let Some(tealeaf::Value::Array(elems)) = arr.get(3) {
        elems
            .iter()
            .filter_map(|e| parse_expected_element(e).ok())
            .collect()
    } else {
        Vec::new()
    };

    let max_tokens = arr
        .get(5)
        .and_then(|v| v.as_int())
        .unwrap_or(2048) as u32;

    let temperature = arr
        .get(6)
        .and_then(|v| v.as_float())
        .map(|f| f as f32);

    Ok(BenchmarkTask {
        metadata,
        prompt_template: prompt.clone(),
        prompt,
        max_tokens,
        temperature,
        expected_elements,
        data_context: Vec::new(),
        grading_rubric: arr.get(4).and_then(|v| v.as_str()).map(String::from),
        data_source: DataSource::None,
    })
}

/// Parse metadata from object
fn parse_metadata(obj: &indexmap::IndexMap<String, tealeaf::Value>) -> Result<TaskMetadata, String> {
    let metadata_obj = obj
        .get("metadata")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_else(|| obj.clone());

    let id = metadata_obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?
        .to_string();

    let category = metadata_obj
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or("Missing category")?
        .to_string();

    let complexity = metadata_obj
        .get("complexity")
        .and_then(|v| v.as_str())
        .unwrap_or("moderate")
        .parse()
        .unwrap_or(Complexity::Moderate);

    let output_type = metadata_obj
        .get("expected_output_type")
        .or_else(|| metadata_obj.get("output_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("analysis")
        .parse()
        .unwrap_or(OutputType::Analysis);

    Ok(TaskMetadata {
        id,
        category,
        subcategory: metadata_obj.get("subcategory").and_then(|v| v.as_str()).map(String::from),
        complexity,
        output_type,
        version: metadata_obj.get("version").and_then(|v| v.as_str()).unwrap_or("1.0").to_string(),
    })
}

/// Parse metadata from tuple format
fn parse_metadata_from_tuple(value: &tealeaf::Value) -> Result<TaskMetadata, String> {
    let arr = value.as_array().ok_or("Metadata must be tuple/array")?;

    let id = arr.first().and_then(|v| v.as_str()).ok_or("Missing id")?.to_string();
    let category = arr.get(1).and_then(|v| v.as_str()).ok_or("Missing category")?.to_string();
    let subcategory = arr.get(2).and_then(|v| v.as_str()).map(String::from);
    let complexity = arr
        .get(3)
        .and_then(|v| v.as_str())
        .unwrap_or("moderate")
        .parse()
        .unwrap_or(Complexity::Moderate);
    let output_type = arr
        .get(4)
        .and_then(|v| v.as_str())
        .unwrap_or("analysis")
        .parse()
        .unwrap_or(OutputType::Analysis);
    let version = arr.get(5).and_then(|v| v.as_str()).unwrap_or("1.0").to_string();

    Ok(TaskMetadata {
        id,
        category,
        subcategory,
        complexity,
        output_type,
        version,
    })
}

/// Parse expected elements from object
fn parse_expected_elements(obj: &indexmap::IndexMap<String, tealeaf::Value>) -> Result<Vec<ExpectedElement>, String> {
    let elements = match obj.get("expected_elements") {
        Some(tealeaf::Value::Array(arr)) => arr,
        _ => return Ok(Vec::new()),
    };

    elements
        .iter()
        .map(parse_expected_element)
        .collect()
}

/// Parse a single expected element
fn parse_expected_element(value: &tealeaf::Value) -> Result<ExpectedElement, String> {
    match value {
        tealeaf::Value::Object(obj) => {
            let element_type = obj
                .get("element_type")
                .or_else(|| obj.get("type"))
                .and_then(|v| v.as_str())
                .ok_or("Missing element_type")?
                .to_string();

            let description = obj
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or("Missing description")?
                .to_string();

            let required = obj
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let validation_pattern = obj
                .get("validation_pattern")
                .and_then(|v| v.as_str())
                .map(String::from);

            Ok(ExpectedElement {
                element_type,
                description,
                required,
                validation_pattern,
            })
        }
        tealeaf::Value::Array(arr) if arr.len() >= 3 => {
            // Tuple format: (type, description, required, pattern?)
            let element_type = arr[0].as_str().ok_or("Missing element_type")?.to_string();
            let description = arr[1].as_str().ok_or("Missing description")?.to_string();
            let required = arr[2].as_bool().unwrap_or(true);
            let validation_pattern = arr.get(3).and_then(|v| v.as_str()).map(String::from);

            Ok(ExpectedElement {
                element_type,
                description,
                required,
                validation_pattern,
            })
        }
        _ => Err(format!("Invalid expected element format: {:?}", value)),
    }
}

// ── JSON task definition loading ──────────────────────────────────────────

/// Top-level JSON task definition file
#[derive(Debug, Deserialize)]
struct TaskDefinitionFile {
    #[allow(dead_code)]
    #[serde(default)]
    version: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    description: Option<String>,
    tasks: Vec<TaskDefinition>,
}

/// A single task definition in the JSON file (flat, human-friendly schema)
#[derive(Debug, Deserialize)]
struct TaskDefinition {
    id: String,
    category: String,
    #[serde(default)]
    subcategory: Option<String>,
    #[serde(default = "default_complexity")]
    complexity: Complexity,
    #[serde(default = "default_output_type")]
    output_type: OutputType,
    prompt_template: String,
    /// Path to data file, relative to the task definition file's directory
    #[serde(default)]
    data_file: Option<String>,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
    #[serde(default = "default_temperature")]
    temperature: Option<f32>,
    #[serde(default)]
    expected_elements: Vec<ExpectedElement>,
    #[serde(default)]
    grading_rubric: Option<String>,
}

fn default_complexity() -> Complexity { Complexity::Moderate }
fn default_output_type() -> OutputType { OutputType::Analysis }
fn default_max_tokens() -> u32 { 2048 }
fn default_temperature() -> Option<f32> { Some(0.3) }

impl TaskDefinition {
    /// Convert to a `BenchmarkTask`, resolving `data_file` relative to `base_dir`.
    fn into_benchmark_task(self, base_dir: &Path) -> BenchmarkTask {
        let data_source = match self.data_file {
            Some(relative) => {
                let full = base_dir.join(&relative);
                DataSource::JsonFile(full.to_string_lossy().to_string())
            }
            None => DataSource::None,
        };

        let template = self.prompt_template;
        BenchmarkTask {
            metadata: TaskMetadata {
                id: self.id,
                category: self.category,
                subcategory: self.subcategory,
                complexity: self.complexity,
                output_type: self.output_type,
                version: "1.0".to_string(),
            },
            prompt_template: template.clone(),
            prompt: template,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            expected_elements: self.expected_elements,
            data_context: Vec::new(),
            grading_rubric: self.grading_rubric,
            data_source,
        }
    }
}

/// Load task definitions from a JSON file.
///
/// Data file paths in the definitions are resolved relative to the JSON file's
/// parent directory.
pub fn load_tasks_from_json_file(path: impl AsRef<Path>) -> Result<Vec<BenchmarkTask>, LoadError> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    let base_dir = path.parent().unwrap_or(Path::new("."));
    load_tasks_from_json_str(&content, base_dir)
}

/// Load task definitions from a JSON string.
///
/// `base_dir` is used to resolve relative `data_file` paths.
pub fn load_tasks_from_json_str(
    content: &str,
    base_dir: &Path,
) -> Result<Vec<BenchmarkTask>, LoadError> {
    let file: TaskDefinitionFile = serde_json::from_str(content)
        .map_err(|e| LoadError::Parse(format!("JSON parse error: {}", e)))?;

    Ok(file
        .tasks
        .into_iter()
        .map(|td| td.into_benchmark_task(base_dir))
        .collect())
}

/// Load all tasks from a directory
pub fn load_tasks_from_directory(dir: impl AsRef<Path>) -> Result<Vec<BenchmarkTask>, LoadError> {
    let mut all_tasks = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let result = match ext {
            "tl" => load_tasks_from_file(&path),
            "json" => load_tasks_from_json_file(&path),
            _ => continue,
        };

        match result {
            Ok(tasks) => all_tasks.extend(tasks),
            Err(e) => {
                tracing::warn!("Failed to load tasks from {:?}: {}", path, e);
            }
        }
    }

    Ok(all_tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_complexity() {
        assert_eq!("simple".parse::<Complexity>().unwrap(), Complexity::Simple);
        assert_eq!("moderate".parse::<Complexity>().unwrap(), Complexity::Moderate);
        assert_eq!("complex".parse::<Complexity>().unwrap(), Complexity::Complex);
    }

    #[test]
    fn test_parse_domain() {
        use super::super::categories::Domain;
        assert_eq!("finance".parse::<Domain>().unwrap(), Domain::Finance);
        assert_eq!("Healthcare".parse::<Domain>().unwrap(), Domain::Healthcare);
        assert_eq!("RETAIL".parse::<Domain>().unwrap(), Domain::Retail);
    }

    #[test]
    fn test_load_json_basic() {
        let json = r#"{
            "version": "1.0",
            "tasks": [
                {
                    "id": "TEST-001",
                    "category": "test",
                    "complexity": "simple",
                    "output_type": "calculation",
                    "prompt_template": "Analyze {data}",
                    "data_file": "test/data.json",
                    "expected_elements": [
                        {"element_type": "metric", "description": "Result", "required": true}
                    ]
                }
            ]
        }"#;

        let tasks = load_tasks_from_json_str(json, Path::new("/base")).unwrap();
        assert_eq!(tasks.len(), 1);
        let t = &tasks[0];
        assert_eq!(t.metadata.id, "TEST-001");
        assert_eq!(t.metadata.category, "test");
        assert_eq!(t.metadata.complexity, Complexity::Simple);
        assert_eq!(t.metadata.output_type, OutputType::Calculation);
        assert_eq!(t.prompt_template, "Analyze {data}");
        assert_eq!(t.max_tokens, 2048);
        assert_eq!(t.temperature, Some(0.3));
        assert_eq!(t.expected_elements.len(), 1);
        assert!(t.expected_elements[0].required);
        // data_file resolved relative to base_dir
        match &t.data_source {
            DataSource::JsonFile(p) => assert!(p.contains("test") && p.contains("data.json")),
            _ => panic!("Expected JsonFile data source"),
        }
    }

    #[test]
    fn test_load_json_defaults() {
        let json = r#"{
            "tasks": [
                {
                    "id": "MIN-001",
                    "category": "minimal",
                    "prompt_template": "Do something"
                }
            ]
        }"#;

        let tasks = load_tasks_from_json_str(json, Path::new(".")).unwrap();
        assert_eq!(tasks.len(), 1);
        let t = &tasks[0];
        assert_eq!(t.metadata.complexity, Complexity::Moderate);
        assert_eq!(t.metadata.output_type, OutputType::Analysis);
        assert_eq!(t.max_tokens, 2048);
        assert_eq!(t.temperature, Some(0.3));
        assert!(t.expected_elements.is_empty());
        assert!(matches!(t.data_source, DataSource::None));
    }

    #[test]
    fn test_load_json_with_validation_pattern() {
        let json = r#"{
            "tasks": [{
                "id": "PAT-001",
                "category": "test",
                "prompt_template": "test",
                "expected_elements": [
                    {"element_type": "metric", "description": "pct", "required": true, "validation_pattern": "\\d+%"}
                ]
            }]
        }"#;

        let tasks = load_tasks_from_json_str(json, Path::new(".")).unwrap();
        assert_eq!(tasks[0].expected_elements[0].validation_pattern.as_deref(), Some("\\d+%"));
    }
}
