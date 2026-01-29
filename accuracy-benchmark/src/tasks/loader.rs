//! Task loading from TeaLeaf files

use std::path::Path;

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
fn parse_metadata(obj: &std::collections::HashMap<String, tealeaf::Value>) -> Result<TaskMetadata, String> {
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
fn parse_expected_elements(obj: &std::collections::HashMap<String, tealeaf::Value>) -> Result<Vec<ExpectedElement>, String> {
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

/// Load all tasks from a directory
pub fn load_tasks_from_directory(dir: impl AsRef<Path>) -> Result<Vec<BenchmarkTask>, LoadError> {
    let mut all_tasks = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "tl").unwrap_or(false) {
            match load_tasks_from_file(&path) {
                Ok(tasks) => all_tasks.extend(tasks),
                Err(e) => {
                    tracing::warn!("Failed to load tasks from {:?}: {}", path, e);
                }
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
}
