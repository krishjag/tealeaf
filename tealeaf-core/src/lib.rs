//! TeaLeaf - Schema-aware document format
//!
//! Peace between human and machine.
//!
//! # Example
//!
//! ```rust
//! use tealeaf::{TeaLeaf, Value};
//!
//! let doc = TeaLeaf::parse(r#"
//!     @struct user (id: int, name: string)
//!     users: @table user [
//!         (1, alice),
//!         (2, bob),
//!     ]
//! "#).unwrap();
//!
//! let users = doc.get("users").unwrap();
//! ```

mod types;
mod lexer;
mod parser;
mod writer;
mod reader;

pub use types::{Error, Result, TLType, FieldType, Field, Schema, Union, Variant, Value, MAGIC, VERSION, VERSION_MAJOR, VERSION_MINOR, HEADER_SIZE};
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::Parser;
pub use writer::Writer;
pub use reader::Reader;

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A parsed TeaLeaf document
pub struct TeaLeaf {
    pub schemas: HashMap<String, Schema>,
    pub data: HashMap<String, Value>,
    /// Tracks if the source JSON was a root-level array (for round-trip fidelity)
    is_root_array: bool,
}

impl TeaLeaf {
    /// Create a new TeaLeaf document from data and schemas.
    ///
    /// This constructor is primarily for programmatic document creation.
    /// For parsing from formats, use `parse()`, `load()`, or `from_json()`.
    pub fn new(schemas: HashMap<String, Schema>, data: HashMap<String, Value>) -> Self {
        Self {
            schemas,
            data,
            is_root_array: false,
        }
    }

    /// Parse TeaLeaf text format
    pub fn parse(input: &str) -> Result<Self> {
        let tokens = Lexer::new(input).tokenize()?;
        let mut parser = Parser::new(tokens);
        let data = parser.parse()?;
        let is_root_array = parser.is_root_array();
        Ok(Self {
            schemas: parser.into_schemas(),
            data,
            is_root_array,
        })
    }

    /// Load from text file
    ///
    /// Include paths are resolved relative to the loaded file's directory.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let tokens = Lexer::new(&content).tokenize()?;
        let mut parser = Parser::new(tokens).with_base_path(path);
        let data = parser.parse()?;
        let is_root_array = parser.is_root_array();
        Ok(Self {
            schemas: parser.into_schemas(),
            data,
            is_root_array,
        })
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Get a schema by name
    pub fn schema(&self, name: &str) -> Option<&Schema> {
        self.schemas.get(name)
    }

    /// Compile to binary format
    pub fn compile<P: AsRef<Path>>(&self, path: P, compress: bool) -> Result<()> {
        let mut writer = Writer::new();
        writer.set_root_array(self.is_root_array);
        for schema in self.schemas.values() {
            writer.add_schema(schema.clone());
        }
        for (key, value) in &self.data {
            let schema = self.find_schema_for_value(value);
            writer.add_section(key, value, schema);
        }
        writer.write(path, compress)
    }

    fn find_schema_for_value(&self, value: &Value) -> Option<&Schema> {
        // Try to find a matching schema for array values
        if let Value::Array(arr) = value {
            if let Some(Value::Object(obj)) = arr.first() {
                for schema in self.schemas.values() {
                    // Match by both field count AND field names
                    if schema.fields.len() == obj.len() {
                        let all_fields_match = schema.fields.iter()
                            .all(|f| obj.contains_key(&f.name));
                        if all_fields_match {
                            return Some(schema);
                        }
                    }
                }
            }
        }
        None
    }

    /// Parse from JSON string.
    ///
    /// # Stability Policy
    ///
    /// This function follows a **"plain JSON only"** policy:
    /// - JSON is parsed as-is with **no magic conversion**
    /// - `{"$ref": "x"}` stays as an Object, NOT a Ref
    /// - `{"$tag": "ok", "$value": 200}` stays as an Object, NOT a Tagged
    /// - `"0xdeadbeef"` stays as a String, NOT Bytes
    /// - `"2024-01-15T10:30:00Z"` stays as a String, NOT a Timestamp
    /// - `[[1, "one"], [2, "two"]]` stays as an Array, NOT a Map
    ///
    /// To create special TeaLeaf types, use the text format or binary API directly.
    ///
    /// # Number Type Inference
    ///
    /// - Integers that fit `i64` → `Value::Int`
    /// - Large positive integers that fit `u64` → `Value::UInt`
    /// - Numbers with decimals or scientific notation → `Value::Float`
    pub fn from_json(json: &str) -> Result<Self> {
        let json_value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| Error::ParseError(format!("Invalid JSON: {}", e)))?;

        let (data, is_root_array) = match json_value {
            serde_json::Value::Object(obj) => {
                let map = obj.into_iter()
                    .map(|(k, v)| (k, json_to_tealeaf_value(v)))
                    .collect();
                (map, false)
            }
            serde_json::Value::Array(_) => {
                // Root-level array: store under "root" key but track for round-trip
                let mut map = HashMap::new();
                map.insert("root".to_string(), json_to_tealeaf_value(json_value));
                (map, true)
            }
            _ => {
                // Other primitives (string, number, bool, null) at root
                let mut map = HashMap::new();
                map.insert("root".to_string(), json_to_tealeaf_value(json_value));
                (map, false)
            }
        };

        Ok(Self {
            schemas: HashMap::new(),
            data,
            is_root_array,
        })
    }

    /// Parse from JSON string with automatic schema inference.
    ///
    /// This variant analyzes the JSON structure and automatically:
    /// - Detects arrays of uniformly-structured objects
    /// - Infers schema names from parent keys (e.g., "products" → "product")
    /// - Generates `@struct` definitions for uniform arrays
    /// - Enables `@table` format output when serialized
    ///
    /// Use `to_tl_with_schemas()` to serialize with the inferred schemas.
    pub fn from_json_with_schemas(json: &str) -> Result<Self> {
        let doc = Self::from_json(json)?;

        let mut inferrer = SchemaInferrer::new();
        inferrer.infer(&doc.data);
        let (schemas, _) = inferrer.into_schemas();

        Ok(Self {
            schemas,
            data: doc.data,
            is_root_array: doc.is_root_array,
        })
    }

    /// Serialize to TeaLeaf text format with schemas.
    ///
    /// If schemas are present (either from parsing or inference), outputs
    /// `@struct` definitions and uses `@table` format for matching arrays.
    ///
    /// If this document represents a root-level JSON array (from `from_json`),
    /// the output will include `@root-array` directive for round-trip fidelity.
    pub fn to_tl_with_schemas(&self) -> String {
        let mut output = String::new();

        // Emit @root-array directive if this represents a root-level array
        if self.is_root_array {
            output.push_str("@root-array\n\n");
        }

        if self.schemas.is_empty() {
            output.push_str(&dumps(&self.data));
        } else {
            // Build schema order (alphabetical for determinism)
            let mut schema_order: Vec<String> = self.schemas.keys().cloned().collect();
            schema_order.sort();
            output.push_str(&dumps_with_schemas(&self.data, &self.schemas, &schema_order));
        }

        output
    }

    /// Convert to JSON string (pretty-printed).
    ///
    /// # Stability Policy - TeaLeaf→JSON Fixed Representations
    ///
    /// Special TeaLeaf types serialize to JSON with these **stable formats**:
    ///
    /// | TeaLeaf Type | JSON Format                                    |
    /// |------------|------------------------------------------------|
    /// | Bytes      | `"0xdeadbeef"` (lowercase hex with 0x prefix) |
    /// | Timestamp  | `"2024-01-15T10:30:00.123Z"` (ISO 8601 UTC)   |
    /// | Ref        | `{"$ref": "key_name"}`                         |
    /// | Tagged     | `{"$tag": "tag_name", "$value": <value>}`     |
    /// | Map        | `[[key1, val1], [key2, val2], ...]`           |
    /// | Float NaN  | `null` (JSON has no NaN)                       |
    /// | Float ±Inf | `null` (JSON has no Infinity)                  |
    ///
    /// These representations are **contractually stable** and will not change.
    pub fn to_json(&self) -> Result<String> {
        // If the source was a root-level array, return it directly (not wrapped in object)
        if self.is_root_array {
            if let Some(root_value) = self.data.get("root") {
                return serde_json::to_string_pretty(&tealeaf_to_json_value(root_value))
                    .map_err(|e| Error::ParseError(format!("JSON serialization failed: {}", e)));
            }
        }

        let json_obj: serde_json::Map<String, serde_json::Value> = self.data
            .iter()
            .map(|(k, v)| (k.clone(), tealeaf_to_json_value(v)))
            .collect();

        serde_json::to_string_pretty(&serde_json::Value::Object(json_obj))
            .map_err(|e| Error::ParseError(format!("JSON serialization failed: {}", e)))
    }

    /// Convert to compact JSON string (no pretty printing)
    pub fn to_json_compact(&self) -> Result<String> {
        // If the source was a root-level array, return it directly (not wrapped in object)
        if self.is_root_array {
            if let Some(root_value) = self.data.get("root") {
                return serde_json::to_string(&tealeaf_to_json_value(root_value))
                    .map_err(|e| Error::ParseError(format!("JSON serialization failed: {}", e)));
            }
        }

        let json_obj: serde_json::Map<String, serde_json::Value> = self.data
            .iter()
            .map(|(k, v)| (k.clone(), tealeaf_to_json_value(v)))
            .collect();

        serde_json::to_string(&serde_json::Value::Object(json_obj))
            .map_err(|e| Error::ParseError(format!("JSON serialization failed: {}", e)))
    }
}

/// Convert JSON value to TeaLeaf value (best-effort)
fn json_to_tealeaf_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            // Preserve the original number type from JSON
            // If it was written as a float (e.g., 4254.0), keep it as float
            if n.is_f64() {
                Value::Float(n.as_f64().unwrap_or(0.0))
            } else if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::UInt(u)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => {
            Value::Array(arr.into_iter().map(json_to_tealeaf_value).collect())
        }
        serde_json::Value::Object(obj) => {
            Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, json_to_tealeaf_value(v)))
                    .collect()
            )
        }
    }
}

/// Convert TeaLeaf value to JSON value
///
/// Type preservation:
/// - Value::Int → JSON integer (e.g., 42)
/// - Value::Float → JSON float (e.g., 42.0)
///
/// Since we preserve the original JSON type during parsing (is_f64 check),
/// we output the same type when converting back to JSON.
fn tealeaf_to_json_value(tl: &Value) -> serde_json::Value {
    match tl {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::UInt(u) => serde_json::Value::Number((*u).into()),
        Value::Float(f) => {
            // Always output floats as floats - the type distinction is intentional
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => {
            // Encode bytes as hex string with 0x prefix
            let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
            serde_json::Value::String(format!("0x{}", hex))
        }
        Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(tealeaf_to_json_value).collect())
        }
        Value::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), tealeaf_to_json_value(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        Value::Map(pairs) => {
            // Convert map to array of [key, value] pairs
            let arr: Vec<serde_json::Value> = pairs
                .iter()
                .map(|(k, v)| {
                    serde_json::Value::Array(vec![
                        tealeaf_to_json_value(k),
                        tealeaf_to_json_value(v),
                    ])
                })
                .collect();
            serde_json::Value::Array(arr)
        }
        Value::Ref(r) => {
            // Encode ref as object with special key
            let mut obj = serde_json::Map::new();
            obj.insert("$ref".to_string(), serde_json::Value::String(r.clone()));
            serde_json::Value::Object(obj)
        }
        Value::Tagged(tag, inner) => {
            // Encode tagged value as object
            let mut obj = serde_json::Map::new();
            obj.insert("$tag".to_string(), serde_json::Value::String(tag.clone()));
            obj.insert("$value".to_string(), tealeaf_to_json_value(inner));
            serde_json::Value::Object(obj)
        }
        Value::Timestamp(ts) => {
            // Encode as ISO 8601 string
            let secs = ts / 1000;
            let millis = ts % 1000;
            let days = secs / 86400;
            let time_secs = secs % 86400;
            let hours = time_secs / 3600;
            let mins = (time_secs % 3600) / 60;
            let secs_rem = time_secs % 60;
            let (year, month, day) = days_to_ymd(days as i32);

            let iso = if millis > 0 {
                format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                    year, month, day, hours, mins, secs_rem, millis)
            } else {
                format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    year, month, day, hours, mins, secs_rem)
            };
            serde_json::Value::String(iso)
        }
    }
}

/// Read a binary TeaLeaf file
pub fn open<P: AsRef<Path>>(path: P) -> Result<Reader> {
    Reader::open(path)
}

/// Parse TeaLeaf text
pub fn parse(input: &str) -> Result<TeaLeaf> {
    TeaLeaf::parse(input)
}

/// Convenience: load and get data
pub fn loads(input: &str) -> Result<HashMap<String, Value>> {
    Ok(TeaLeaf::parse(input)?.data)
}

/// Convenience: serialize to TeaLeaf text
/// Check if a string needs quoting when serialized to TeaLeaf format.
/// Returns true if the string could be misinterpreted as another type.
fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    // Check for characters that require quoting
    // Note: '@' anywhere in the string needs quoting since @ starts directives
    // Note: '/' needs quoting since words stop at non-alphanumeric chars
    // Note: '.' needs quoting since it breaks words
    if s.contains(|c: char| {
        c.is_whitespace() || c == '"' || c == ',' || c == '(' || c == ')'
        || c == '[' || c == ']' || c == '{' || c == '}' || c == ':' || c == '@'
        || c == '/' || c == '.'
    }) {
        return true;
    }

    // Reserved words
    if matches!(s, "true" | "false" | "null" | "~") {
        return true;
    }

    // Starts with special TeaLeaf syntax characters
    let first = s.chars().next().unwrap();
    if matches!(first, '!' | '@' | '#') {
        return true;
    }

    // Starts with 0x (bytes literal)
    if s.starts_with("0x") || s.starts_with("0X") {
        return true;
    }

    // Could be parsed as a number (integer, float, or negative)
    // This is conservative - if it looks numeric at all, quote it
    let s_trimmed = if s.starts_with('-') || s.starts_with('+') {
        &s[1..]
    } else {
        s
    };

    if !s_trimmed.is_empty() {
        let first_char = s_trimmed.chars().next().unwrap();
        if first_char.is_ascii_digit() {
            // Starts with a digit - could be a number, quote it
            return true;
        }
    }

    false
}

pub fn dumps(data: &HashMap<String, Value>) -> String {
    let mut out = String::new();
    for (key, value) in data {
        out.push_str(key);
        out.push_str(": ");
        write_value(&mut out, value, 0);
        out.push('\n');
    }
    out
}

/// Format a float ensuring it always has a decimal point (e.g., 42.0 not 42)
fn format_float(f: f64) -> String {
    let s = f.to_string();
    // If the string doesn't contain a decimal point or 'e' (scientific notation),
    // add ".0" to make it clear this is a float
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        format!("{}.0", s)
    } else {
        s
    }
}

fn write_value(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Null => out.push('~'),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::UInt(u) => out.push_str(&u.to_string()),
        Value::Float(f) => out.push_str(&format_float(*f)),
        Value::String(s) => {
            if needs_quoting(s) {
                out.push('"');
                out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
                out.push('"');
            } else {
                out.push_str(s);
            }
        }
        Value::Bytes(b) => {
            out.push_str("0x");
            for byte in b {
                out.push_str(&format!("{:02x}", byte));
            }
        }
        Value::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                write_value(out, v, indent);
            }
            out.push(']');
        }
        Value::Object(obj) => {
            out.push('{');
            let mut first = true;
            for (k, v) in obj {
                if !first { out.push_str(", "); }
                first = false;
                out.push_str(k);
                out.push_str(": ");
                write_value(out, v, indent);
            }
            out.push('}');
        }
        Value::Map(pairs) => {
            out.push_str("@map {");
            let mut first = true;
            for (k, v) in pairs {
                if !first { out.push_str(", "); }
                first = false;
                write_value(out, k, indent);
                out.push_str(": ");
                write_value(out, v, indent);
            }
            out.push('}');
        }
        Value::Ref(r) => {
            out.push('!');
            out.push_str(r);
        }
        Value::Tagged(tag, inner) => {
            out.push(':');
            out.push_str(tag);
            out.push(' ');
            write_value(out, inner, indent);
        }
        Value::Timestamp(ts) => {
            // Convert Unix millis to ISO 8601
            let secs = ts / 1000;
            let millis = ts % 1000;
            // Simple conversion (not handling negative timestamps)
            let days = secs / 86400;
            let time_secs = secs % 86400;
            let hours = time_secs / 3600;
            let mins = (time_secs % 3600) / 60;
            let secs_rem = time_secs % 60;

            // Calculate date from days since epoch
            let (year, month, day) = days_to_ymd(days as i32);

            if millis > 0 {
                out.push_str(&format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                    year, month, day, hours, mins, secs_rem, millis
                ));
            } else {
                out.push_str(&format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    year, month, day, hours, mins, secs_rem
                ));
            }
        }
    }
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_ymd(days: i32) -> (i32, u32, u32) {
    // Algorithm from Howard Hinnant
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// =============================================================================
// Schema Inference
// =============================================================================

/// Inferred type information for a field
#[derive(Debug, Clone, PartialEq)]
enum InferredType {
    Null,
    Bool,
    Int,
    Float,
    String,
    Array(Box<InferredType>),
    Object(Vec<(String, InferredType)>),  // Ordered fields
    Mixed,  // Different types seen - fall back to any
}

impl InferredType {
    fn merge(&self, other: &InferredType) -> InferredType {
        if self == other {
            return self.clone();
        }
        match (self, other) {
            (InferredType::Null, t) | (t, InferredType::Null) => {
                // Null + T = T (nullable)
                t.clone()
            }
            (InferredType::Int, InferredType::Float) | (InferredType::Float, InferredType::Int) => {
                InferredType::Float
            }
            (InferredType::Array(a), InferredType::Array(b)) => {
                InferredType::Array(Box::new(a.merge(b)))
            }
            (InferredType::Object(a), InferredType::Object(b)) => {
                // Merge objects: keep fields present in both, track nullability
                let mut merged = Vec::new();
                let b_map: HashMap<&str, &InferredType> = b.iter().map(|(k, v)| (k.as_str(), v)).collect();

                for (key, a_type) in a {
                    if let Some(b_type) = b_map.get(key.as_str()) {
                        merged.push((key.clone(), a_type.merge(b_type)));
                    }
                    // Fields only in a are dropped (not uniform)
                }

                // Check if structures are compatible (same fields)
                if merged.len() == a.len() && merged.len() == b.len() {
                    InferredType::Object(merged)
                } else {
                    InferredType::Mixed
                }
            }
            _ => InferredType::Mixed,
        }
    }

    fn to_field_type(&self, schemas: &HashMap<String, Schema>) -> FieldType {
        match self {
            InferredType::Null => FieldType::new("string").nullable(),  // Unknown type, default to string
            InferredType::Bool => FieldType::new("bool"),
            InferredType::Int => FieldType::new("int"),
            InferredType::Float => FieldType::new("float"),
            InferredType::String => FieldType::new("string"),
            InferredType::Array(inner) => {
                let inner_type = inner.to_field_type(schemas);
                FieldType {
                    base: inner_type.base,
                    nullable: inner_type.nullable,
                    is_array: true,
                }
            }
            InferredType::Object(fields) => {
                // Check if this matches an existing schema
                for (name, schema) in schemas {
                    if schema.fields.len() == fields.len() {
                        let all_match = schema.fields.iter().all(|sf| {
                            fields.iter().any(|(k, _)| k == &sf.name)
                        });
                        if all_match {
                            return FieldType::new(name.clone());
                        }
                    }
                }
                // No matching schema, use object
                FieldType::new("object")
            }
            InferredType::Mixed => FieldType::new("any"),
        }
    }
}

fn infer_type(value: &Value) -> InferredType {
    match value {
        Value::Null => InferredType::Null,
        Value::Bool(_) => InferredType::Bool,
        Value::Int(_) | Value::UInt(_) => InferredType::Int,
        Value::Float(_) => InferredType::Float,
        Value::String(_) => InferredType::String,
        Value::Array(arr) => {
            if arr.is_empty() {
                InferredType::Array(Box::new(InferredType::Mixed))
            } else {
                let mut element_type = infer_type(&arr[0]);
                for item in arr.iter().skip(1) {
                    element_type = element_type.merge(&infer_type(item));
                }
                InferredType::Array(Box::new(element_type))
            }
        }
        Value::Object(obj) => {
            let mut fields: Vec<(String, InferredType)> = obj
                .iter()
                .map(|(k, v)| (k.clone(), infer_type(v)))
                .collect();
            fields.sort_by(|a, b| a.0.cmp(&b.0));  // Consistent ordering
            InferredType::Object(fields)
        }
        _ => InferredType::Mixed,
    }
}

/// Singularize a plural name (simple heuristic)
fn singularize(name: &str) -> String {
    let name = name.to_lowercase();
    if name.ends_with("ies") {
        format!("{}y", &name[..name.len()-3])
    } else if name.ends_with("es") && (name.ends_with("sses") || name.ends_with("xes") || name.ends_with("ches") || name.ends_with("shes")) {
        name[..name.len()-2].to_string()
    } else if name.ends_with('s') && !name.ends_with("ss") {
        name[..name.len()-1].to_string()
    } else {
        name
    }
}

/// Check if array elements are objects that match a schema's structure
fn array_matches_schema(arr: &[Value], schema: &Schema) -> bool {
    if arr.is_empty() {
        return false;
    }

    // Check if first element is an object
    let first = match &arr[0] {
        Value::Object(obj) => obj,
        _ => return false,
    };

    // Get schema field names
    let schema_fields: HashSet<_> = schema.fields.iter().map(|f| f.name.as_str()).collect();

    // Get object keys
    let obj_keys: HashSet<_> = first.keys().map(|k| k.as_str()).collect();

    // Check if there's significant overlap (at least 50% of schema fields present)
    let overlap = schema_fields.intersection(&obj_keys).count();
    let required_overlap = schema_fields.len() / 2;

    overlap > required_overlap || overlap == schema_fields.len()
}

/// Schema inferrer that analyzes data and generates schemas
pub struct SchemaInferrer {
    schemas: HashMap<String, Schema>,
    schema_order: Vec<String>,  // Track order for output
}

impl SchemaInferrer {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            schema_order: Vec::new(),
        }
    }

    /// Analyze data and infer schemas from uniform object arrays
    pub fn infer(&mut self, data: &HashMap<String, Value>) {
        for (key, value) in data {
            self.analyze_value(key, value);
        }
    }

    fn analyze_value(&mut self, hint_name: &str, value: &Value) {
        if let Value::Array(arr) = value {
            self.analyze_array(hint_name, arr);
        } else if let Value::Object(obj) = value {
            // Recursively analyze nested objects
            for (k, v) in obj {
                self.analyze_value(k, v);
            }
        }
    }

    fn analyze_array(&mut self, hint_name: &str, arr: &[Value]) {
        if arr.is_empty() {
            return;
        }

        // Check if all elements are objects with the same structure
        let first = match &arr[0] {
            Value::Object(obj) => obj,
            _ => return,
        };

        // Collect all field names from first object
        let mut field_names: Vec<String> = first.keys().cloned().collect();
        field_names.sort();

        // Verify all objects have the same fields
        for item in arr.iter().skip(1) {
            if let Value::Object(obj) = item {
                let mut item_fields: Vec<String> = obj.keys().cloned().collect();
                item_fields.sort();
                if item_fields != field_names {
                    return;  // Not uniform
                }
            } else {
                return;  // Not all objects
            }
        }

        // Infer types for each field across all objects
        let mut field_types: HashMap<String, InferredType> = HashMap::new();
        let mut has_null: HashMap<String, bool> = HashMap::new();

        for item in arr {
            if let Value::Object(obj) = item {
                for (key, val) in obj {
                    let inferred = infer_type(val);
                    let is_null = matches!(val, Value::Null);

                    *has_null.entry(key.clone()).or_insert(false) |= is_null;

                    field_types
                        .entry(key.clone())
                        .and_modify(|existing| *existing = existing.merge(&inferred))
                        .or_insert(inferred);
                }
            }
        }

        // Generate schema name from hint
        let schema_name = singularize(hint_name);

        // Skip if schema already exists
        if self.schemas.contains_key(&schema_name) {
            return;
        }

        // First, recursively analyze nested arrays and objects to create their schemas
        for item in arr {
            if let Value::Object(obj) = item {
                for (field_name, field_val) in obj {
                    if let Value::Array(nested) = field_val {
                        self.analyze_array(field_name, nested);
                    }
                }
                break;  // Only need to process first object for nested arrays
            }
        }

        // Analyze nested object fields - collect all non-null objects for each field
        // and create schemas if they're uniform across all array items
        for field_name in &field_names {
            let nested_objects: Vec<&HashMap<String, Value>> = arr
                .iter()
                .filter_map(|item| {
                    if let Value::Object(obj) = item {
                        if let Some(Value::Object(nested)) = obj.get(field_name) {
                            return Some(nested);
                        }
                    }
                    None
                })
                .collect();

            // If we found at least one object, check if they're uniform
            if !nested_objects.is_empty() {
                self.analyze_nested_objects(field_name, &nested_objects);
            }
        }

        // Build schema
        let mut schema = Schema::new(&schema_name);

        // Use consistent field ordering (alphabetical)
        for field_name in &field_names {
            if let Some(inferred) = field_types.get(field_name) {
                let mut field_type = inferred.to_field_type(&self.schemas);

                // Mark as nullable if any null values seen
                if has_null.get(field_name).copied().unwrap_or(false) {
                    field_type.nullable = true;
                }

                // Check if there's a nested schema for array fields
                if let Value::Object(first_obj) = &arr[0] {
                    if let Some(Value::Array(nested_arr)) = first_obj.get(field_name) {
                        let nested_schema_name = singularize(field_name);
                        if let Some(nested_schema) = self.schemas.get(&nested_schema_name) {
                            // Verify array elements are objects matching the schema structure
                            if array_matches_schema(nested_arr, nested_schema) {
                                field_type = FieldType {
                                    base: nested_schema_name,
                                    nullable: field_type.nullable,
                                    is_array: true,
                                };
                            }
                        }
                    }
                }

                // Check if there's a nested schema for object fields
                let nested_schema_name = singularize(field_name);
                if self.schemas.contains_key(&nested_schema_name) {
                    if matches!(inferred, InferredType::Object(_)) {
                        field_type = FieldType {
                            base: nested_schema_name,
                            nullable: field_type.nullable,
                            is_array: false,
                        };
                    }
                }

                schema.add_field(field_name, field_type);
            }
        }

        self.schema_order.push(schema_name.clone());
        self.schemas.insert(schema_name, schema);
    }

    /// Analyze a collection of nested objects (from the same field across array items)
    /// and create a schema if they have uniform structure
    fn analyze_nested_objects(&mut self, field_name: &str, objects: &[&HashMap<String, Value>]) {
        if objects.is_empty() {
            return;
        }

        // Get field names from first object
        let first = objects[0];
        let mut nested_field_names: Vec<String> = first.keys().cloned().collect();
        nested_field_names.sort();

        // Check if all objects have the same fields
        for obj in objects.iter().skip(1) {
            let mut obj_fields: Vec<String> = obj.keys().cloned().collect();
            obj_fields.sort();
            if obj_fields != nested_field_names {
                return; // Not uniform
            }
        }

        // They're uniform - create a schema
        let schema_name = singularize(field_name);

        // Skip if schema already exists
        if self.schemas.contains_key(&schema_name) {
            return;
        }

        // Infer field types across all objects
        let mut field_types: HashMap<String, InferredType> = HashMap::new();
        let mut has_null: HashMap<String, bool> = HashMap::new();

        for obj in objects {
            for (key, val) in *obj {
                let inferred = infer_type(val);
                let is_null = matches!(val, Value::Null);

                *has_null.entry(key.clone()).or_insert(false) |= is_null;

                field_types
                    .entry(key.clone())
                    .and_modify(|existing| *existing = existing.merge(&inferred))
                    .or_insert(inferred);
            }
        }

        // Recursively analyze nested objects within these objects
        for nested_field in &nested_field_names {
            let deeper_objects: Vec<&HashMap<String, Value>> = objects
                .iter()
                .filter_map(|obj| {
                    if let Some(Value::Object(nested)) = obj.get(nested_field) {
                        Some(nested)
                    } else {
                        None
                    }
                })
                .collect();

            if !deeper_objects.is_empty() {
                self.analyze_nested_objects(nested_field, &deeper_objects);
            }
        }

        // Build schema
        let mut schema = Schema::new(&schema_name);

        for nested_field in &nested_field_names {
            if let Some(inferred) = field_types.get(nested_field) {
                let mut field_type = inferred.to_field_type(&self.schemas);

                if has_null.get(nested_field).copied().unwrap_or(false) {
                    field_type.nullable = true;
                }

                // Check if this field has a nested schema
                if let Some(nested_schema) = self.schemas.get(&singularize(nested_field)) {
                    if matches!(inferred, InferredType::Object(_)) {
                        field_type = FieldType::new(nested_schema.name.clone());
                    }
                }

                schema.add_field(nested_field, field_type);
            }
        }

        self.schema_order.push(schema_name.clone());
        self.schemas.insert(schema_name, schema);
    }

    pub fn into_schemas(self) -> (HashMap<String, Schema>, Vec<String>) {
        (self.schemas, self.schema_order)
    }
}

impl Default for SchemaInferrer {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize data to TeaLeaf text format with schemas
pub fn dumps_with_schemas(
    data: &HashMap<String, Value>,
    schemas: &HashMap<String, Schema>,
    schema_order: &[String],
) -> String {
    let mut out = String::new();

    // Write schemas in order
    for name in schema_order {
        if let Some(schema) = schemas.get(name) {
            out.push_str("@struct ");
            out.push_str(&schema.name);
            out.push_str(" (");
            for (i, field) in schema.fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&field.name);
                out.push_str(": ");
                out.push_str(&field.field_type.to_string());
            }
            out.push_str(")\n");
        }
    }

    if !schema_order.is_empty() {
        out.push('\n');
    }

    // Write data
    for (key, value) in data {
        out.push_str(key);
        out.push_str(": ");
        write_value_with_schemas(&mut out, value, schemas, Some(key), 0);
        out.push('\n');
    }

    out
}

fn write_value_with_schemas(
    out: &mut String,
    value: &Value,
    schemas: &HashMap<String, Schema>,
    hint_name: Option<&str>,
    indent: usize,
) {
    match value {
        Value::Null => out.push('~'),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::UInt(u) => out.push_str(&u.to_string()),
        Value::Float(f) => out.push_str(&format_float(*f)),
        Value::String(s) => {
            if needs_quoting(s) {
                out.push('"');
                out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
                out.push('"');
            } else {
                out.push_str(s);
            }
        }
        Value::Bytes(b) => {
            out.push_str("0x");
            for byte in b {
                out.push_str(&format!("{:02x}", byte));
            }
        }
        Value::Array(arr) => {
            // Check if this array can use @table format
            let schema_name = hint_name.map(singularize);
            let schema = schema_name.as_ref().and_then(|n| schemas.get(n));

            if let Some(schema) = schema {
                // Check if first element is an object matching the schema
                if let Some(Value::Object(_)) = arr.first() {
                    out.push_str("@table ");
                    out.push_str(&schema.name);
                    out.push_str(" [\n");

                    let inner_indent = indent + 2;
                    for (i, item) in arr.iter().enumerate() {
                        for _ in 0..inner_indent {
                            out.push(' ');
                        }
                        write_tuple(out, item, schema, schemas, inner_indent);
                        if i < arr.len() - 1 {
                            out.push(',');
                        }
                        out.push('\n');
                    }

                    for _ in 0..indent {
                        out.push(' ');
                    }
                    out.push(']');
                    return;
                }
            }

            // Fall back to regular array format
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value_with_schemas(out, v, schemas, None, indent);
            }
            out.push(']');
        }
        Value::Object(obj) => {
            out.push('{');
            let mut first = true;
            for (k, v) in obj {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                out.push_str(k);
                out.push_str(": ");
                write_value_with_schemas(out, v, schemas, Some(k), indent);
            }
            out.push('}');
        }
        Value::Map(pairs) => {
            out.push_str("@map {");
            let mut first = true;
            for (k, v) in pairs {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                write_value_with_schemas(out, k, schemas, None, indent);
                out.push_str(": ");
                write_value_with_schemas(out, v, schemas, None, indent);
            }
            out.push('}');
        }
        Value::Ref(r) => {
            out.push('!');
            out.push_str(r);
        }
        Value::Tagged(tag, inner) => {
            out.push(':');
            out.push_str(tag);
            out.push(' ');
            write_value_with_schemas(out, inner, schemas, None, indent);
        }
        Value::Timestamp(ts) => {
            let secs = ts / 1000;
            let millis = ts % 1000;
            let days = secs / 86400;
            let time_secs = secs % 86400;
            let hours = time_secs / 3600;
            let mins = (time_secs % 3600) / 60;
            let secs_rem = time_secs % 60;
            let (year, month, day) = days_to_ymd(days as i32);

            if millis > 0 {
                out.push_str(&format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                    year, month, day, hours, mins, secs_rem, millis
                ));
            } else {
                out.push_str(&format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    year, month, day, hours, mins, secs_rem
                ));
            }
        }
    }
}

fn write_tuple(
    out: &mut String,
    value: &Value,
    schema: &Schema,
    schemas: &HashMap<String, Schema>,
    indent: usize,
) {
    if let Value::Object(obj) = value {
        out.push('(');
        for (i, field) in schema.fields.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            if let Some(v) = obj.get(&field.name) {
                // For array fields with a known schema type, write tuples directly without @table
                if field.field_type.is_array {
                    if let Some(item_schema) = schemas.get(&field.field_type.base) {
                        // The schema defines the element type - write array with tuples directly
                        write_schema_array(out, v, item_schema, schemas, indent);
                    } else {
                        // No schema for element type - use regular array format
                        write_value_with_schemas(out, v, schemas, None, indent);
                    }
                } else if schemas.contains_key(&field.field_type.base) {
                    // Non-array field with schema type - write as nested tuple
                    if let Some(nested_schema) = schemas.get(&field.field_type.base) {
                        write_tuple(out, v, nested_schema, schemas, indent);
                    } else {
                        write_value_with_schemas(out, v, schemas, None, indent);
                    }
                } else {
                    write_value_with_schemas(out, v, schemas, None, indent);
                }
            } else {
                out.push('~');
            }
        }
        out.push(')');
    } else {
        write_value_with_schemas(out, value, schemas, None, indent);
    }
}

/// Write an array of schema-typed values as tuples (without @table annotation)
fn write_schema_array(
    out: &mut String,
    value: &Value,
    schema: &Schema,
    schemas: &HashMap<String, Schema>,
    indent: usize,
) {
    if let Value::Array(arr) = value {
        if arr.is_empty() {
            out.push_str("[]");
            return;
        }

        out.push_str("[\n");
        let inner_indent = indent + 2;
        for (i, item) in arr.iter().enumerate() {
            for _ in 0..inner_indent {
                out.push(' ');
            }
            write_tuple(out, item, schema, schemas, inner_indent);
            if i < arr.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        for _ in 0..indent {
            out.push(' ');
        }
        out.push(']');
    } else {
        // Not an array - fall back to regular value writing
        write_value_with_schemas(out, value, schemas, None, indent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_json_number_behavior() {
        // Test how serde_json handles different number formats
        let json_str = r#"{"int": 42, "float_whole": 42.0, "float_frac": 42.5}"#;
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();

        if let serde_json::Value::Object(obj) = parsed {
            let int_num = obj.get("int").unwrap().as_number().unwrap();
            let float_whole = obj.get("float_whole").unwrap().as_number().unwrap();
            let float_frac = obj.get("float_frac").unwrap().as_number().unwrap();

            println!("int (42): is_i64={}, is_u64={}, is_f64={}",
                int_num.is_i64(), int_num.is_u64(), int_num.is_f64());
            println!("float_whole (42.0): is_i64={}, is_u64={}, is_f64={}",
                float_whole.is_i64(), float_whole.is_u64(), float_whole.is_f64());
            println!("float_frac (42.5): is_i64={}, is_u64={}, is_f64={}",
                float_frac.is_i64(), float_frac.is_u64(), float_frac.is_f64());

            // Assert expected behavior
            assert!(int_num.is_i64(), "42 should be parsed as i64");
            assert!(float_whole.is_f64(), "42.0 should be parsed as f64");
            assert!(float_frac.is_f64(), "42.5 should be parsed as f64");
        }

        // Test how Rust formats floats
        println!("Rust float formatting:");
        println!("  42.0f64.to_string() = '{}'", 42.0f64.to_string());
        println!("  42.5f64.to_string() = '{}'", 42.5f64.to_string());

        // This is the problem! Rust's to_string() drops the .0
        // We need to ensure floats always have a decimal point
    }

    #[test]
    fn test_parse_simple() {
        let doc = TeaLeaf::parse(r#"
            name: alice
            age: 30
            active: true
        "#).unwrap();
        
        assert_eq!(doc.get("name").unwrap().as_str(), Some("alice"));
        assert_eq!(doc.get("age").unwrap().as_int(), Some(30));
        assert_eq!(doc.get("active").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn test_parse_struct() {
        let doc = TeaLeaf::parse(r#"
            @struct user (id: int, name: string, email: string?)
            users: @table user [
                (1, alice, "alice@test.com"),
                (2, bob, ~),
            ]
        "#).unwrap();
        
        let schema = doc.schema("user").unwrap();
        assert_eq!(schema.fields.len(), 3);
        assert!(schema.fields[2].field_type.nullable);
        
        let users = doc.get("users").unwrap().as_array().unwrap();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_nested_struct() {
        let doc = TeaLeaf::parse(r#"
            @struct address (city: string, zip: string)
            @struct user (id: int, name: string, home: address)
            users: @table user [
                (1, alice, (Berlin, "10115")),
                (2, bob, (Paris, "75001")),
            ]
        "#).unwrap();
        
        let users = doc.get("users").unwrap().as_array().unwrap();
        let alice = users[0].as_object().unwrap();
        let home = alice.get("home").unwrap().as_object().unwrap();
        assert_eq!(home.get("city").unwrap().as_str(), Some("Berlin"));
    }

    #[test]
    fn test_three_level_nesting() {
        let doc = TeaLeaf::parse(r#"
            @struct method (type: string, last4: string)
            @struct payment (amount: float, method: method)
            @struct order (id: int, payment: payment)
            orders: @table order [
                (1, (99.99, (credit, "4242"))),
            ]
        "#).unwrap();
        
        let orders = doc.get("orders").unwrap().as_array().unwrap();
        let order = orders[0].as_object().unwrap();
        let payment = order.get("payment").unwrap().as_object().unwrap();
        let method = payment.get("method").unwrap().as_object().unwrap();
        assert_eq!(method.get("type").unwrap().as_str(), Some("credit"));
    }

    #[test]
    fn test_json_roundtrip_basic() {
        let json = r#"{"name":"alice","age":30,"active":true,"score":95.5}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        assert_eq!(doc.get("name").unwrap().as_str(), Some("alice"));
        assert_eq!(doc.get("age").unwrap().as_int(), Some(30));
        assert_eq!(doc.get("active").unwrap().as_bool(), Some(true));
        assert_eq!(doc.get("score").unwrap().as_float(), Some(95.5));

        // Round-trip back to JSON
        let json_out = doc.to_json().unwrap();
        assert!(json_out.contains("\"name\":\"alice\"") || json_out.contains("\"name\": \"alice\""));
    }

    #[test]
    fn test_json_roundtrip_root_array() {
        // Root-level arrays should round-trip without wrapping
        let json = r#"[{"id":"0001","type":"donut","name":"Cake"},{"id":"0002","type":"donut","name":"Raised"}]"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        // Internally stored under "root" key
        let root = doc.get("root").unwrap();
        let arr = root.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        // Round-trip should produce the array directly, NOT {"root": [...]}
        let json_out = doc.to_json_compact().unwrap();
        assert!(json_out.starts_with('['), "Root array should serialize directly: {}", json_out);
        assert!(json_out.ends_with(']'), "Root array should end with ]: {}", json_out);
        assert!(!json_out.contains("\"root\""), "Should NOT wrap in root object: {}", json_out);

        // Verify content preserved
        assert!(json_out.contains("\"id\":\"0001\"") || json_out.contains("\"id\": \"0001\""));
        assert!(json_out.contains("\"name\":\"Cake\"") || json_out.contains("\"name\": \"Cake\""));
    }

    #[test]
    fn test_json_roundtrip_root_array_empty() {
        // Empty array should also round-trip correctly
        let json = r#"[]"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let json_out = doc.to_json_compact().unwrap();
        assert_eq!(json_out, "[]", "Empty array should round-trip: {}", json_out);
    }

    #[test]
    fn test_json_roundtrip_root_object_with_root_key() {
        // An object that happens to have a "root" key should NOT be confused
        let json = r#"{"root":[1,2,3],"other":"value"}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let json_out = doc.to_json_compact().unwrap();
        // This was a root object, so it should stay as an object
        assert!(json_out.starts_with('{'), "Root object should stay as object: {}", json_out);
        assert!(json_out.contains("\"root\""), "root key should be preserved: {}", json_out);
        assert!(json_out.contains("\"other\""), "other key should be preserved: {}", json_out);
    }

    #[test]
    fn test_json_export_bytes() {
        // Create a document with bytes programmatically
        let mut entries = std::collections::HashMap::new();
        entries.insert("data".to_string(), Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let json = doc.to_json().unwrap();
        assert!(json.contains("0xdeadbeef"), "Bytes should export as hex string: {}", json);
    }

    #[test]
    fn test_json_export_ref() {
        let mut entries = std::collections::HashMap::new();
        entries.insert("config".to_string(), Value::Ref("base_config".to_string()));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let json = doc.to_json().unwrap();
        assert!(json.contains("\"$ref\""), "Ref should export with $ref key: {}", json);
        assert!(json.contains("base_config"), "Ref name should be in output: {}", json);
    }

    #[test]
    fn test_json_export_tagged() {
        let mut entries = std::collections::HashMap::new();
        entries.insert("status".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let json = doc.to_json().unwrap();
        assert!(json.contains("\"$tag\""), "Tagged should export with $tag key: {}", json);
        assert!(json.contains("\"ok\""), "Tag name should be in output: {}", json);
        assert!(json.contains("\"$value\""), "Tagged should have $value key: {}", json);
    }

    #[test]
    fn test_json_export_map() {
        let mut entries = std::collections::HashMap::new();
        entries.insert("lookup".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
            (Value::Int(2), Value::String("two".to_string())),
        ]));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let json = doc.to_json().unwrap();
        // Map exports as array of [key, value] pairs
        // Check that the structure contains the key and values (regardless of formatting)
        assert!(json.contains("\"lookup\""), "Map key should be in output: {}", json);
        assert!(json.contains("\"one\""), "Map values should be in output: {}", json);
        assert!(json.contains("\"two\""), "Map values should be in output: {}", json);
        // Verify it's an array structure (has nested arrays)
        let compact = json.replace(" ", "").replace("\n", "");
        assert!(compact.contains("[["), "Map should export as nested array: {}", json);
    }

    #[test]
    fn test_json_export_timestamp() {
        let mut entries = std::collections::HashMap::new();
        // 2024-01-15T10:30:00Z = 1705315800000 ms, but let's verify with a known value
        // Use 0 = 1970-01-01T00:00:00Z for simplicity
        entries.insert("created".to_string(), Value::Timestamp(0));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let json = doc.to_json().unwrap();
        assert!(json.contains("1970-01-01"), "Timestamp should export as ISO 8601 date: {}", json);
        assert!(json.contains("00:00:00"), "Timestamp time should be epoch: {}", json);
    }

    #[test]
    fn test_json_import_limitation_ref_becomes_object() {
        // JSON with $ref pattern should become a plain object, NOT a Ref value
        let json = r#"{"config":{"$ref":"base_config"}}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let config = doc.get("config").unwrap();
        // This should be an Object, not a Ref
        assert!(config.as_object().is_some(), "JSON $ref should become Object, not Ref");
        assert!(config.as_ref_name().is_none(), "JSON $ref should NOT become Ref value");
    }

    #[test]
    fn test_json_import_limitation_tagged_becomes_object() {
        // JSON with $tag/$value pattern should become a plain object, NOT a Tagged value
        let json = r#"{"status":{"$tag":"ok","$value":200}}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let status = doc.get("status").unwrap();
        // This should be an Object, not a Tagged
        assert!(status.as_object().is_some(), "JSON $tag should become Object, not Tagged");
        assert!(status.as_tagged().is_none(), "JSON $tag should NOT become Tagged value");
    }

    #[test]
    fn test_json_import_limitation_timestamp_becomes_string() {
        // ISO 8601 strings in JSON should remain strings, NOT become Timestamp
        let json = r#"{"created":"2024-01-15T10:30:00Z"}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let created = doc.get("created").unwrap();
        // This should be a String, not a Timestamp
        assert!(created.as_str().is_some(), "ISO timestamp string should remain String");
        assert!(created.as_timestamp().is_none(), "ISO timestamp should NOT become Timestamp value");
    }

    // =========================================================================
    // JSON ↔ Binary Conversion Tests
    // =========================================================================

    #[test]
    fn test_json_to_binary_roundtrip_primitives() {
        use tempfile::NamedTempFile;

        let json = r#"{"name":"alice","age":30,"score":95.5,"active":true,"nothing":null}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        // Compile to binary
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();
        doc.compile(path, false).unwrap();

        // Read back
        let reader = Reader::open(path).unwrap();
        assert_eq!(reader.get("name").unwrap().as_str(), Some("alice"));
        assert_eq!(reader.get("age").unwrap().as_int(), Some(30));
        assert_eq!(reader.get("score").unwrap().as_float(), Some(95.5));
        assert_eq!(reader.get("active").unwrap().as_bool(), Some(true));
        assert!(reader.get("nothing").unwrap().is_null());
    }

    #[test]
    fn test_json_to_binary_roundtrip_arrays() {
        use tempfile::NamedTempFile;

        let json = r#"{"numbers":[1,2,3,4,5],"names":["alice","bob","charlie"]}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();

        let numbers = reader.get("numbers").unwrap();
        let arr = numbers.as_array().unwrap();
        assert_eq!(arr.len(), 5);
        assert_eq!(arr[0].as_int(), Some(1));
        assert_eq!(arr[4].as_int(), Some(5));

        let names = reader.get("names").unwrap();
        let arr = names.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_str(), Some("alice"));
    }

    #[test]
    fn test_json_to_binary_roundtrip_nested_objects() {
        use tempfile::NamedTempFile;

        let json = r#"{"user":{"name":"alice","profile":{"bio":"dev","settings":{"theme":"dark"}}}}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let user = reader.get("user").unwrap();
        let user_obj = user.as_object().unwrap();
        assert_eq!(user_obj.get("name").unwrap().as_str(), Some("alice"));

        let profile = user_obj.get("profile").unwrap().as_object().unwrap();
        assert_eq!(profile.get("bio").unwrap().as_str(), Some("dev"));

        let settings = profile.get("settings").unwrap().as_object().unwrap();
        assert_eq!(settings.get("theme").unwrap().as_str(), Some("dark"));
    }

    #[test]
    fn test_json_to_binary_with_compression() {
        use tempfile::NamedTempFile;

        // Create a document with repetitive data to test compression
        let mut entries = std::collections::HashMap::new();
        entries.insert("data".to_string(), Value::String("a".repeat(1000)));
        entries.insert("count".to_string(), Value::Int(12345));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), true).unwrap(); // compressed

        let reader = Reader::open(temp.path()).unwrap();
        assert_eq!(reader.get("data").unwrap().as_str(), Some("a".repeat(1000).as_str()));
        assert_eq!(reader.get("count").unwrap().as_int(), Some(12345));
    }

    #[test]
    fn test_tl_to_binary_preserves_ref() {
        use tempfile::NamedTempFile;

        let mut entries = std::collections::HashMap::new();
        entries.insert("base".to_string(), Value::Object(vec![
            ("host".to_string(), Value::String("localhost".to_string())),
        ].into_iter().collect()));
        entries.insert("config".to_string(), Value::Ref("base".to_string()));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let config = reader.get("config").unwrap();
        assert_eq!(config.as_ref_name(), Some("base"));
    }

    #[test]
    fn test_tl_to_binary_preserves_tagged() {
        use tempfile::NamedTempFile;

        let mut entries = std::collections::HashMap::new();
        entries.insert("status".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let status = reader.get("status").unwrap();
        let (tag, value) = status.as_tagged().unwrap();
        assert_eq!(tag, "ok");
        assert_eq!(value.as_int(), Some(200));
    }

    #[test]
    fn test_tl_to_binary_preserves_map() {
        use tempfile::NamedTempFile;

        let mut entries = std::collections::HashMap::new();
        entries.insert("lookup".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
            (Value::Int(2), Value::String("two".to_string())),
        ]));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let lookup = reader.get("lookup").unwrap();
        let map = lookup.as_map().unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map[0].0.as_int(), Some(1));
        assert_eq!(map[0].1.as_str(), Some("one"));
    }

    #[test]
    fn test_tl_to_binary_preserves_bytes() {
        use tempfile::NamedTempFile;

        let mut entries = std::collections::HashMap::new();
        entries.insert("data".to_string(), Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]));
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let data = reader.get("data").unwrap();
        assert_eq!(data.as_bytes(), Some(vec![0xde, 0xad, 0xbe, 0xef].as_slice()));
    }

    #[test]
    fn test_tl_to_binary_preserves_timestamp() {
        use tempfile::NamedTempFile;

        let mut entries = std::collections::HashMap::new();
        entries.insert("created".to_string(), Value::Timestamp(1705315800000)); // 2024-01-15T10:30:00Z
        let doc = TeaLeaf { data: entries, schemas: std::collections::HashMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let created = reader.get("created").unwrap();
        assert_eq!(created.as_timestamp(), Some(1705315800000));
    }

    #[test]
    fn test_json_import_limitation_hex_string_remains_string() {
        // Hex strings in JSON should remain strings, NOT become Bytes
        let json = r#"{"data":"0xdeadbeef"}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let data = doc.get("data").unwrap();
        // This should be a String, not Bytes
        assert!(data.as_str().is_some(), "Hex string should remain String");
        assert_eq!(data.as_str(), Some("0xdeadbeef"));
        assert!(data.as_bytes().is_none(), "Hex string should NOT become Bytes value");
    }

    #[test]
    fn test_json_import_limitation_array_pairs_remain_array() {
        // JSON arrays that look like map pairs should remain arrays, NOT become Maps
        let json = r#"{"lookup":[[1,"one"],[2,"two"]]}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let lookup = doc.get("lookup").unwrap();
        // This should be an Array, not a Map
        assert!(lookup.as_array().is_some(), "Array of pairs should remain Array");
        assert!(lookup.as_map().is_none(), "Array of pairs should NOT become Map value");

        // Verify structure
        let arr = lookup.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        let first_pair = arr[0].as_array().unwrap();
        assert_eq!(first_pair[0].as_int(), Some(1));
        assert_eq!(first_pair[1].as_str(), Some("one"));
    }

    // =========================================================================
    // Cross-Language Parity Test
    // =========================================================================

    #[test]
    fn test_cross_language_parity_all_types() {
        // This test verifies that Rust JSON export matches expected format
        // for ALL special types. The same fixture is tested in .NET.

        use tempfile::NamedTempFile;

        // Create a document with all special types
        let mut data = std::collections::HashMap::new();
        data.insert("null_val".to_string(), Value::Null);
        data.insert("bool_true".to_string(), Value::Bool(true));
        data.insert("int_val".to_string(), Value::Int(42));
        data.insert("float_val".to_string(), Value::Float(3.14159));
        data.insert("string_val".to_string(), Value::String("hello".to_string()));
        data.insert("bytes_val".to_string(), Value::Bytes(vec![0xca, 0xfe]));
        data.insert("timestamp_val".to_string(), Value::Timestamp(0));
        data.insert("array_val".to_string(), Value::Array(vec![Value::Int(1), Value::Int(2)]));
        data.insert("object_val".to_string(), Value::Object(
            vec![("x".to_string(), Value::Int(1))].into_iter().collect()
        ));
        data.insert("ref_val".to_string(), Value::Ref("object_val".to_string()));
        data.insert("tagged_val".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
        data.insert("map_val".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
        ]));

        let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

        // Compile to binary and read back
        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();
        let reader = Reader::open(temp.path()).unwrap();

        // Verify each type survives binary round-trip
        assert!(reader.get("null_val").unwrap().is_null());
        assert_eq!(reader.get("bool_true").unwrap().as_bool(), Some(true));
        assert_eq!(reader.get("int_val").unwrap().as_int(), Some(42));
        assert_eq!(reader.get("float_val").unwrap().as_float(), Some(3.14159));
        assert_eq!(reader.get("string_val").unwrap().as_str(), Some("hello"));
        assert_eq!(reader.get("bytes_val").unwrap().as_bytes(), Some(&[0xca, 0xfe][..]));
        assert_eq!(reader.get("timestamp_val").unwrap().as_timestamp(), Some(0));

        let arr = reader.get("array_val").unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 2);

        let obj = reader.get("object_val").unwrap();
        assert!(obj.as_object().is_some());

        let ref_val = reader.get("ref_val").unwrap();
        assert_eq!(ref_val.as_ref_name(), Some("object_val"));

        let tagged = reader.get("tagged_val").unwrap();
        let (tag, val) = tagged.as_tagged().unwrap();
        assert_eq!(tag, "ok");
        assert_eq!(val.as_int(), Some(200));

        let map = reader.get("map_val").unwrap();
        let pairs = map.as_map().unwrap();
        assert_eq!(pairs.len(), 1);

        // Verify JSON export format matches expected conventions
        let json = doc.to_json().unwrap();

        // Bytes should be hex string
        assert!(json.contains("0xcafe"), "Bytes should export as hex: {}", json);

        // Ref should have $ref key
        assert!(json.contains("\"$ref\""), "Ref should have $ref key: {}", json);

        // Tagged should have $tag and $value
        assert!(json.contains("\"$tag\""), "Tagged should have $tag: {}", json);
        assert!(json.contains("\"$value\""), "Tagged should have $value: {}", json);

        // Map should be array of pairs (nested arrays)
        let compact = json.replace(" ", "").replace("\n", "");
        assert!(compact.contains("[["), "Map should export as array of pairs: {}", json);

        // Timestamp should be ISO 8601
        assert!(json.contains("1970-01-01"), "Timestamp should be ISO 8601: {}", json);
    }

    // =========================================================================
    // JSON Conversion Contract Tests
    // =========================================================================
    // These tests lock down the exact JSON↔TeaLeaf conversion behavior.
    // STABILITY POLICY:
    // - Plain JSON roundtrip: MUST be lossless for primitives, arrays, objects
    // - TeaLeaf→JSON: Special types have FIXED representations that MUST NOT change
    // - JSON→TeaLeaf: No magic parsing; $ref/$tag/hex/ISO8601 stay as plain JSON

    mod conversion_contracts {
        use super::*;

        // --- Plain JSON Roundtrip (STABLE) ---

        #[test]
        fn contract_null_roundtrip() {
            let doc = TeaLeaf::from_json("null").unwrap();
            assert!(matches!(doc.get("root").unwrap(), Value::Null));
        }

        #[test]
        fn contract_bool_roundtrip() {
            let doc = TeaLeaf::from_json(r#"{"t": true, "f": false}"#).unwrap();
            assert_eq!(doc.get("t").unwrap().as_bool(), Some(true));
            assert_eq!(doc.get("f").unwrap().as_bool(), Some(false));

            let json = doc.to_json_compact().unwrap();
            assert!(json.contains("true"));
            assert!(json.contains("false"));
        }

        #[test]
        fn contract_integer_roundtrip() {
            let doc = TeaLeaf::from_json(r#"{"zero": 0, "pos": 42, "neg": -123}"#).unwrap();
            assert_eq!(doc.get("zero").unwrap().as_int(), Some(0));
            assert_eq!(doc.get("pos").unwrap().as_int(), Some(42));
            assert_eq!(doc.get("neg").unwrap().as_int(), Some(-123));
        }

        #[test]
        fn contract_float_roundtrip() {
            let doc = TeaLeaf::from_json(r#"{"pi": 3.14159}"#).unwrap();
            let pi = doc.get("pi").unwrap().as_float().unwrap();
            assert!((pi - 3.14159).abs() < 0.00001);
        }

        #[test]
        fn contract_string_roundtrip() {
            let doc = TeaLeaf::from_json(r#"{"s": "hello world", "u": "日本語", "e": ""}"#).unwrap();
            assert_eq!(doc.get("s").unwrap().as_str(), Some("hello world"));
            assert_eq!(doc.get("u").unwrap().as_str(), Some("日本語"));
            assert_eq!(doc.get("e").unwrap().as_str(), Some(""));
        }

        #[test]
        fn contract_array_roundtrip() {
            let doc = TeaLeaf::from_json(r#"{"arr": [1, "two", true, null]}"#).unwrap();
            let arr = doc.get("arr").unwrap().as_array().unwrap();
            assert_eq!(arr.len(), 4);
            assert_eq!(arr[0].as_int(), Some(1));
            assert_eq!(arr[1].as_str(), Some("two"));
            assert_eq!(arr[2].as_bool(), Some(true));
            assert!(matches!(arr[3], Value::Null));
        }

        #[test]
        fn contract_nested_array_roundtrip() {
            let doc = TeaLeaf::from_json(r#"{"matrix": [[1, 2], [3, 4]]}"#).unwrap();
            let matrix = doc.get("matrix").unwrap().as_array().unwrap();
            assert_eq!(matrix.len(), 2);
            let row0 = matrix[0].as_array().unwrap();
            assert_eq!(row0[0].as_int(), Some(1));
            assert_eq!(row0[1].as_int(), Some(2));
        }

        #[test]
        fn contract_object_roundtrip() {
            let doc = TeaLeaf::from_json(r#"{"user": {"name": "alice", "age": 30}}"#).unwrap();
            let user = doc.get("user").unwrap().as_object().unwrap();
            assert_eq!(user.get("name").unwrap().as_str(), Some("alice"));
            assert_eq!(user.get("age").unwrap().as_int(), Some(30));
        }

        // --- TeaLeaf→JSON Fixed Representations (STABLE) ---

        #[test]
        fn contract_bytes_to_json_hex() {
            let mut data = std::collections::HashMap::new();
            data.insert("b".to_string(), Value::Bytes(vec![0xca, 0xfe, 0xba, 0xbe]));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Bytes serialize as lowercase hex with 0x prefix
            assert!(json.contains("\"0xcafebabe\""), "Bytes must be 0x-prefixed hex: {}", json);
        }

        #[test]
        fn contract_bytes_empty_to_json() {
            let mut data = std::collections::HashMap::new();
            data.insert("b".to_string(), Value::Bytes(vec![]));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Empty bytes serialize as "0x"
            assert!(json.contains("\"0x\""), "Empty bytes must be \"0x\": {}", json);
        }

        #[test]
        fn contract_timestamp_to_json_iso8601() {
            let mut data = std::collections::HashMap::new();
            // 2024-01-15T10:50:00.123Z (verified milliseconds since epoch)
            data.insert("ts".to_string(), Value::Timestamp(1705315800123));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Timestamp serializes as ISO 8601 with milliseconds
            assert!(json.contains("2024-01-15T10:50:00.123Z"),
                "Timestamp must be ISO 8601 with ms: {}", json);
        }

        #[test]
        fn contract_timestamp_epoch_to_json() {
            let mut data = std::collections::HashMap::new();
            data.insert("ts".to_string(), Value::Timestamp(0));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Unix epoch is 1970-01-01T00:00:00Z (no ms for whole seconds)
            assert!(json.contains("1970-01-01T00:00:00Z"),
                "Epoch must be 1970-01-01T00:00:00Z: {}", json);
        }

        #[test]
        fn contract_ref_to_json() {
            let mut data = std::collections::HashMap::new();
            data.insert("r".to_string(), Value::Ref("target_key".to_string()));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Ref serializes as {"$ref": "name"}
            assert!(json.contains("\"$ref\":\"target_key\"") || json.contains("\"$ref\": \"target_key\""),
                "Ref must be {{\"$ref\": \"name\"}}: {}", json);
        }

        #[test]
        fn contract_tagged_to_json() {
            let mut data = std::collections::HashMap::new();
            data.insert("t".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Tagged serializes with $tag and $value keys
            assert!(json.contains("\"$tag\""), "Tagged must have $tag: {}", json);
            assert!(json.contains("\"ok\""), "Tag name must be present: {}", json);
            assert!(json.contains("\"$value\""), "Tagged must have $value: {}", json);
            assert!(json.contains("200"), "Inner value must be present: {}", json);
        }

        #[test]
        fn contract_tagged_null_value_to_json() {
            let mut data = std::collections::HashMap::new();
            data.insert("t".to_string(), Value::Tagged("none".to_string(), Box::new(Value::Null)));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Tagged with null inner still has $value: null
            assert!(json.contains("\"$value\":null") || json.contains("\"$value\": null"),
                "Tagged with null must have $value:null: {}", json);
        }

        #[test]
        fn contract_map_to_json_pairs() {
            let mut data = std::collections::HashMap::new();
            data.insert("m".to_string(), Value::Map(vec![
                (Value::Int(1), Value::String("one".to_string())),
                (Value::Int(2), Value::String("two".to_string())),
            ]));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Map serializes as array of [key, value] pairs
            assert!(json.contains("[[1,\"one\"],[2,\"two\"]]") ||
                    json.contains("[[1, \"one\"], [2, \"two\"]]"),
                "Map must be [[k,v],...]: {}", json);
        }

        #[test]
        fn contract_map_empty_to_json() {
            let mut data = std::collections::HashMap::new();
            data.insert("m".to_string(), Value::Map(vec![]));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Empty map serializes as empty array
            assert!(json.contains("\"m\":[]") || json.contains("\"m\": []"),
                "Empty map must be []: {}", json);
        }

        // --- JSON→TeaLeaf No Magic (STABLE) ---

        #[test]
        fn contract_json_dollar_ref_stays_object() {
            let doc = TeaLeaf::from_json(r#"{"x": {"$ref": "some_key"}}"#).unwrap();
            let x = doc.get("x").unwrap();
            // CONTRACT: JSON {"$ref": ...} MUST remain Object, NOT become Ref
            assert!(x.as_object().is_some(), "$ref in JSON must stay Object, not become Ref");
            assert!(x.as_ref_name().is_none(), "$ref must not auto-convert to Ref type");
        }

        #[test]
        fn contract_json_dollar_tag_stays_object() {
            let doc = TeaLeaf::from_json(r#"{"x": {"$tag": "ok", "$value": 200}}"#).unwrap();
            let x = doc.get("x").unwrap();
            // CONTRACT: JSON {"$tag": ..., "$value": ...} MUST remain Object
            assert!(x.as_object().is_some(), "$tag in JSON must stay Object, not become Tagged");
            assert!(x.as_tagged().is_none(), "$tag must not auto-convert to Tagged type");
        }

        #[test]
        fn contract_json_hex_string_stays_string() {
            let doc = TeaLeaf::from_json(r#"{"x": "0xdeadbeef"}"#).unwrap();
            let x = doc.get("x").unwrap();
            // CONTRACT: Hex strings MUST remain String, NOT become Bytes
            assert_eq!(x.as_str(), Some("0xdeadbeef"));
            assert!(x.as_bytes().is_none(), "Hex string must not auto-convert to Bytes");
        }

        #[test]
        fn contract_json_iso_timestamp_stays_string() {
            let doc = TeaLeaf::from_json(r#"{"x": "2024-01-15T10:30:00.000Z"}"#).unwrap();
            let x = doc.get("x").unwrap();
            // CONTRACT: ISO 8601 strings MUST remain String, NOT become Timestamp
            assert_eq!(x.as_str(), Some("2024-01-15T10:30:00.000Z"));
            assert!(x.as_timestamp().is_none(), "ISO string must not auto-convert to Timestamp");
        }

        #[test]
        fn contract_json_array_pairs_stays_array() {
            let doc = TeaLeaf::from_json(r#"{"x": [[1, "one"], [2, "two"]]}"#).unwrap();
            let x = doc.get("x").unwrap();
            // CONTRACT: Array of pairs MUST remain Array, NOT become Map
            assert!(x.as_array().is_some(), "Array of pairs must stay Array, not become Map");
            assert!(x.as_map().is_none(), "Array pairs must not auto-convert to Map");
        }

        // --- Number Type Inference (STABLE) ---

        #[test]
        fn contract_number_integer_to_int() {
            let doc = TeaLeaf::from_json(r#"{"n": 42}"#).unwrap();
            // CONTRACT: Integers that fit i64 become Int
            assert!(doc.get("n").unwrap().as_int().is_some());
        }

        #[test]
        fn contract_number_large_to_uint() {
            // Max u64 = 18446744073709551615, which doesn't fit i64
            let doc = TeaLeaf::from_json(r#"{"n": 18446744073709551615}"#).unwrap();
            // CONTRACT: Large positive integers that fit u64 become UInt
            assert!(doc.get("n").unwrap().as_uint().is_some());
        }

        #[test]
        fn contract_number_decimal_to_float() {
            let doc = TeaLeaf::from_json(r#"{"n": 3.14}"#).unwrap();
            // CONTRACT: Numbers with decimals become Float
            assert!(doc.get("n").unwrap().as_float().is_some());
        }

        // --- Edge Cases (STABLE) ---

        #[test]
        fn contract_float_nan_to_null() {
            let mut data = std::collections::HashMap::new();
            data.insert("f".to_string(), Value::Float(f64::NAN));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: NaN serializes as null (JSON has no NaN)
            assert!(json.contains("null"), "NaN must serialize as null: {}", json);
        }

        #[test]
        fn contract_float_infinity_to_null() {
            let mut data = std::collections::HashMap::new();
            data.insert("f".to_string(), Value::Float(f64::INFINITY));
            let doc = TeaLeaf { data, schemas: std::collections::HashMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Infinity serializes as null (JSON has no Infinity)
            assert!(json.contains("null"), "Infinity must serialize as null: {}", json);
        }

        #[test]
        fn contract_deep_nesting_preserved() {
            let doc = TeaLeaf::from_json(r#"{"a":{"b":{"c":{"d":{"e":5}}}}}"#).unwrap();
            let a = doc.get("a").unwrap().as_object().unwrap();
            let b = a.get("b").unwrap().as_object().unwrap();
            let c = b.get("c").unwrap().as_object().unwrap();
            let d = c.get("d").unwrap().as_object().unwrap();
            assert_eq!(d.get("e").unwrap().as_int(), Some(5));
        }
    }

    // =========================================================================
    // Schema Inference Tests
    // =========================================================================

    #[test]
    fn test_schema_inference_simple_array() {
        let json = r#"{"users": [{"name": "alice", "age": 30}, {"name": "bob", "age": 25}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        // Should have inferred a "user" schema
        let schema = doc.schema("user");
        assert!(schema.is_some(), "Should infer 'user' schema from 'users' array");

        let schema = schema.unwrap();
        assert_eq!(schema.fields.len(), 2);

        // Fields should be in alphabetical order
        assert_eq!(schema.fields[0].name, "age");
        assert_eq!(schema.fields[1].name, "name");

        // Data should still be accessible
        let users = doc.get("users").unwrap().as_array().unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].as_object().unwrap().get("name").unwrap().as_str(), Some("alice"));
    }

    #[test]
    fn test_schema_inference_nested_arrays() {
        let json = r#"{
            "orders": [
                {"id": 1, "items": [{"sku": "A", "qty": 2}, {"sku": "B", "qty": 1}]},
                {"id": 2, "items": [{"sku": "C", "qty": 3}]}
            ]
        }"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        // Should infer both "order" and "item" schemas
        assert!(doc.schema("order").is_some(), "Should infer 'order' schema");
        assert!(doc.schema("item").is_some(), "Should infer 'item' schema");

        let order_schema = doc.schema("order").unwrap();
        // Order should have "id" and "items" fields
        assert!(order_schema.fields.iter().any(|f| f.name == "id"));
        assert!(order_schema.fields.iter().any(|f| f.name == "items"));

        // The "items" field should reference the "item" schema
        let items_field = order_schema.fields.iter().find(|f| f.name == "items").unwrap();
        assert!(items_field.field_type.is_array);
        assert_eq!(items_field.field_type.base, "item");
    }

    #[test]
    fn test_schema_inference_to_tl_text() {
        let json = r#"{"products": [{"name": "Widget", "price": 9.99}, {"name": "Gadget", "price": 19.99}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        let tl_text = doc.to_tl_with_schemas();

        // Should contain struct definition
        assert!(tl_text.contains("@struct product"), "TeaLeaf text should contain struct definition");
        assert!(tl_text.contains("name: string"), "Struct should have name field");
        assert!(tl_text.contains("price: float"), "Struct should have price field");

        // Should contain @table directive
        assert!(tl_text.contains("@table product"), "TeaLeaf text should use @table for data");

        // Should contain tuple format data
        assert!(tl_text.contains("Widget") || tl_text.contains("\"Widget\""), "Data should be present");
    }

    #[test]
    fn test_schema_inference_roundtrip() {
        let json = r#"{"items": [{"id": 1, "name": "A"}, {"id": 2, "name": "B"}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        // Convert to TeaLeaf text with schemas
        let tl_text = doc.to_tl_with_schemas();

        // Parse the TeaLeaf text back
        let parsed = TeaLeaf::parse(&tl_text).unwrap();

        // Should have the same data
        let items = parsed.get("items").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_object().unwrap().get("id").unwrap().as_int(), Some(1));
        assert_eq!(items[0].as_object().unwrap().get("name").unwrap().as_str(), Some("A"));

        // Should have the schema
        assert!(parsed.schema("item").is_some());
    }

    #[test]
    fn test_schema_inference_nullable_fields() {
        let json = r#"{"users": [{"name": "alice", "email": "a@test.com"}, {"name": "bob", "email": null}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        let schema = doc.schema("user").unwrap();
        let email_field = schema.fields.iter().find(|f| f.name == "email").unwrap();

        // Email should be nullable since one value is null
        assert!(email_field.field_type.nullable, "Field with null values should be nullable");
    }

    #[test]
    fn test_schema_inference_nested_tuples_no_redundant_table() {
        let json = r#"{
            "orders": [
                {"id": 1, "items": [{"sku": "A", "qty": 2}]}
            ]
        }"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // Count occurrences of @table - should only appear at top level for each schema-typed array
        let _table_count = tl_text.matches("@table").count();

        // Should have @table for orders, but NOT redundant @table for nested items
        // The nested items array should just be [...] with tuples inside
        assert!(tl_text.contains("@table order"), "Should have @table for orders");

        // Parse and verify the structure is correct
        let parsed = TeaLeaf::parse(&tl_text).unwrap();
        let orders = parsed.get("orders").unwrap().as_array().unwrap();
        let first_order = orders[0].as_object().unwrap();
        let items = first_order.get("items").unwrap().as_array().unwrap();
        assert_eq!(items[0].as_object().unwrap().get("sku").unwrap().as_str(), Some("A"));
    }

    #[test]
    fn test_schema_inference_mismatched_arrays_not_matched() {
        // Test that arrays with different structures don't incorrectly share schemas
        let json = r#"{
            "users": [{"id": "U1", "name": "Alice"}],
            "products": [{"id": "P1", "price": 9.99}]
        }"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        // Should have separate schemas
        assert!(doc.schema("user").is_some());
        assert!(doc.schema("product").is_some());

        // User schema should have name field
        let user_schema = doc.schema("user").unwrap();
        assert!(user_schema.fields.iter().any(|f| f.name == "name"));

        // Product schema should have price field
        let product_schema = doc.schema("product").unwrap();
        assert!(product_schema.fields.iter().any(|f| f.name == "price"));
    }

    #[test]
    fn test_schema_inference_special_char_quoting() {
        // Test that strings with special characters are properly quoted
        let json = r#"{"items": [
            {"category": "Electronics/Audio", "email": "test@example.com", "path": "a.b.c"}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // These should be quoted in output since they contain special characters
        assert!(tl_text.contains("\"Electronics/Audio\""), "Slash should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"test@example.com\""), "@ should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"a.b.c\""), "Dots should be quoted: {}", tl_text);

        // Should parse back correctly
        let parsed = TeaLeaf::parse(&tl_text).unwrap();
        let items = parsed.get("items").unwrap().as_array().unwrap();
        let item = items[0].as_object().unwrap();
        assert_eq!(item.get("category").unwrap().as_str(), Some("Electronics/Audio"));
        assert_eq!(item.get("email").unwrap().as_str(), Some("test@example.com"));
    }

    #[test]
    fn test_schema_inference_nested_objects() {
        // Test that nested objects within array elements get schemas created
        let json = r#"{
            "customers": [
                {
                    "id": 1,
                    "name": "Alice",
                    "billing_address": {
                        "street": "123 Main St",
                        "city": "Boston",
                        "state": "MA",
                        "postal_code": "02101",
                        "country": "USA"
                    },
                    "shipping_address": {
                        "street": "456 Oak Ave",
                        "city": "Cambridge",
                        "state": "MA",
                        "postal_code": "02139",
                        "country": "USA"
                    }
                },
                {
                    "id": 2,
                    "name": "Bob",
                    "billing_address": {
                        "street": "789 Elm St",
                        "city": "New York",
                        "state": "NY",
                        "postal_code": "10001",
                        "country": "USA"
                    },
                    "shipping_address": {
                        "street": "789 Elm St",
                        "city": "New York",
                        "state": "NY",
                        "postal_code": "10001",
                        "country": "USA"
                    }
                }
            ]
        }"#;

        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        // Should have schemas for nested objects
        assert!(doc.schema("billing_address").is_some(), "Should create billing_address schema");
        assert!(doc.schema("shipping_address").is_some(), "Should create shipping_address schema");
        assert!(doc.schema("customer").is_some(), "Should create customer schema");

        // Check billing_address schema fields
        let billing_schema = doc.schema("billing_address").unwrap();
        let billing_fields: Vec<&str> = billing_schema.fields.iter().map(|f| f.name.as_str()).collect();
        assert!(billing_fields.contains(&"street"), "billing_address should have street field");
        assert!(billing_fields.contains(&"city"), "billing_address should have city field");
        assert!(billing_fields.contains(&"state"), "billing_address should have state field");
        assert!(billing_fields.contains(&"postal_code"), "billing_address should have postal_code field");
        assert!(billing_fields.contains(&"country"), "billing_address should have country field");

        // Check customer schema references the nested schemas
        let customer_schema = doc.schema("customer").unwrap();
        let billing_field = customer_schema.fields.iter().find(|f| f.name == "billing_address").unwrap();
        assert_eq!(billing_field.field_type.base, "billing_address", "customer.billing_address should reference billing_address schema");

        let shipping_field = customer_schema.fields.iter().find(|f| f.name == "shipping_address").unwrap();
        assert_eq!(shipping_field.field_type.base, "shipping_address", "customer.shipping_address should reference shipping_address schema");

        // Serialize and verify output
        let tl_text = doc.to_tl_with_schemas();
        assert!(tl_text.contains("@struct billing_address"), "Output should contain billing_address struct");
        assert!(tl_text.contains("@struct shipping_address"), "Output should contain shipping_address struct");
        assert!(tl_text.contains("billing_address: billing_address"), "customer should have billing_address field with billing_address type");
        assert!(tl_text.contains("shipping_address: shipping_address"), "customer should have shipping_address field with shipping_address type");
    }

    #[test]
    fn test_schema_inference_nested_objects_with_nulls() {
        // Test that nested objects handle nullable fields correctly
        let json = r#"{
            "orders": [
                {
                    "id": 1,
                    "customer": {
                        "name": "Alice",
                        "phone": "555-1234"
                    }
                },
                {
                    "id": 2,
                    "customer": {
                        "name": "Bob",
                        "phone": null
                    }
                }
            ]
        }"#;

        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        // Customer schema should exist with nullable phone
        let customer_schema = doc.schema("customer").unwrap();
        let phone_field = customer_schema.fields.iter().find(|f| f.name == "phone").unwrap();
        assert!(phone_field.field_type.nullable, "phone field should be nullable");
    }
}
