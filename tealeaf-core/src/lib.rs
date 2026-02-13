//! TeaLeaf - Schema-aware data format
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
pub mod convert;
pub mod builder;

pub use types::{Error, Result, TLType, FieldType, Field, Schema, Union, Variant, Value, ObjectMap, MAGIC, VERSION, VERSION_MAJOR, VERSION_MINOR, HEADER_SIZE, MAX_STRING_LENGTH, MAX_OBJECT_FIELDS, MAX_ARRAY_LENGTH};
pub use indexmap::IndexMap;
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::Parser;
pub use writer::Writer;
pub use reader::Reader;
pub use convert::{ToTeaLeaf, FromTeaLeaf, ConvertError, ToTeaLeafExt};
pub use builder::TeaLeafBuilder;

// Re-export derive macros when the "derive" feature is enabled
#[cfg(feature = "derive")]
pub use tealeaf_derive::{ToTeaLeaf, FromTeaLeaf};

use std::collections::HashSet;
use std::path::Path;

/// A parsed TeaLeaf document
pub struct TeaLeaf {
    pub schemas: IndexMap<String, Schema>,
    pub unions: IndexMap<String, Union>,
    pub data: IndexMap<String, Value>,
    /// Tracks if the source JSON was a root-level array (for round-trip fidelity)
    is_root_array: bool,
}

impl TeaLeaf {
    /// Create a new TeaLeaf document from data and schemas.
    ///
    /// This constructor is primarily for programmatic document creation.
    /// For parsing from formats, use `parse()`, `load()`, or `from_json()`.
    pub fn new(schemas: IndexMap<String, Schema>, data: IndexMap<String, Value>) -> Self {
        Self {
            schemas,
            unions: IndexMap::new(),
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
        let (schemas, unions) = parser.into_schemas_and_unions();
        Ok(Self {
            schemas,
            unions,
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
        let (schemas, unions) = parser.into_schemas_and_unions();
        Ok(Self {
            schemas,
            unions,
            data,
            is_root_array,
        })
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Navigate a dot-path expression to reach a deeply nested value.
    ///
    /// The first segment is used as the top-level document key;
    /// remaining segments are evaluated via [`Value::get_path`].
    ///
    /// Path syntax: `key.field[N].field`
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        if path.is_empty() {
            return None;
        }
        let bytes = path.as_bytes();
        // Find end of first segment (document key)
        let seg_end = bytes
            .iter()
            .position(|&b| b == b'.' || b == b'[')
            .unwrap_or(bytes.len());
        let first_key = &path[..seg_end];
        let root = self.data.get(first_key)?;

        if seg_end >= bytes.len() {
            return Some(root);
        }

        let mut remaining = seg_end;
        if bytes[remaining] == b'.' {
            remaining += 1;
        }
        root.get_path(&path[remaining..])
    }

    /// Get a schema by name
    pub fn schema(&self, name: &str) -> Option<&Schema> {
        self.schemas.get(name)
    }

    /// Get a union by name
    pub fn union(&self, name: &str) -> Option<&Union> {
        self.unions.get(name)
    }

    /// Compile to binary format
    pub fn compile<P: AsRef<Path>>(&self, path: P, compress: bool) -> Result<()> {
        let mut writer = Writer::new();
        writer.set_root_array(self.is_root_array);
        for (_, schema) in &self.schemas {
            writer.add_schema(schema.clone());
        }
        for (_, union_def) in &self.unions {
            writer.add_union(union_def.clone());
        }
        for (key, value) in &self.data {
            let schema = self.find_schema_for_value(value, key);
            writer.add_section(key, value, schema)?;
        }
        writer.write(path, compress)
    }

    fn find_schema_for_value(&self, value: &Value, key: &str) -> Option<&Schema> {
        // Try to find a matching schema for array values
        if let Value::Array(arr) = value {
            if arr.is_empty() {
                // For empty arrays, try name-based matching (singularize key → schema name)
                let singular = singularize(key);
                return self.schemas.values().find(|s| s.name.eq_ignore_ascii_case(&singular));
            }

            // Sample multiple elements: first, middle, last
            let sample_indices: Vec<usize> = {
                let mut indices = vec![0];
                if arr.len() > 2 { indices.push(arr.len() / 2); }
                if arr.len() > 1 { indices.push(arr.len() - 1); }
                indices
            };

            for schema in self.schemas.values() {
                let all_match = sample_indices.iter().all(|&i| {
                    if let Some(Value::Object(obj)) = arr.get(i) {
                        // All required (non-nullable) schema fields must be present
                        schema.fields.iter().all(|f| {
                            f.field_type.nullable || obj.contains_key(&f.name)
                        })
                        // All obj keys must be schema fields (no extra keys)
                        && obj.keys().all(|k| schema.fields.iter().any(|f| f.name == *k))
                    } else {
                        false
                    }
                });
                if all_match {
                    return Some(schema);
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
    /// - `"0xcafef00d"` stays as a String, NOT Bytes
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
                let mut map = IndexMap::new();
                map.insert("root".to_string(), json_to_tealeaf_value(json_value));
                (map, true)
            }
            _ => {
                // Other primitives (string, number, bool, null) at root
                let mut map = IndexMap::new();
                map.insert("root".to_string(), json_to_tealeaf_value(json_value));
                (map, false)
            }
        };

        Ok(Self {
            schemas: IndexMap::new(),
            unions: IndexMap::new(),
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
            unions: IndexMap::new(),
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
        self.to_tl_with_options(&FormatOptions::default())
    }

    /// Serialize to compact TeaLeaf text format with schema definitions.
    /// Removes insignificant whitespace (spaces after `:` and `,`, indentation,
    /// blank lines) while keeping the format parseable. Table rows remain one
    /// per line for readability.
    pub fn to_tl_with_schemas_compact(&self) -> String {
        self.to_tl_with_options(&FormatOptions::compact())
    }

    /// Serialize to TeaLeaf text format with custom formatting options.
    ///
    /// Use `FormatOptions::compact().with_compact_floats()` for maximum
    /// token savings (strips whitespace and `.0` from whole-number floats).
    pub fn to_tl_with_options(&self, opts: &FormatOptions) -> String {
        let mut output = String::new();

        if self.is_root_array {
            if opts.compact {
                output.push_str("@root-array\n");
            } else {
                output.push_str("@root-array\n\n");
            }
        }

        if self.schemas.is_empty() && self.unions.is_empty() {
            output.push_str(&dumps_with_options(&self.data, opts));
        } else {
            let schema_order: Vec<String> = self.schemas.keys().cloned().collect();
            let union_order: Vec<String> = self.unions.keys().cloned().collect();
            output.push_str(&dumps_with_schemas_with_options(
                &self.data, &self.schemas, &schema_order,
                &self.unions, &union_order, opts,
            ));
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
    /// | Bytes      | `"0xcafef00d"` (lowercase hex with 0x prefix) |
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

    /// Set whether the document represents a root-level array.
    pub fn set_root_array(&mut self, is_root_array: bool) {
        self.is_root_array = is_root_array;
    }

    /// Create a TeaLeaf document from a binary Reader.
    ///
    /// Reads all sections from the reader and carries schemas and unions through.
    pub fn from_reader(reader: &Reader) -> Result<Self> {
        let mut data = IndexMap::new();
        for key in reader.keys() {
            data.insert(key.to_string(), reader.get(key)?);
        }
        let schemas: IndexMap<String, Schema> = reader.schemas.iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect();
        let unions: IndexMap<String, Union> = reader.unions.iter()
            .map(|u| (u.name.clone(), u.clone()))
            .collect();
        let mut doc = Self {
            schemas,
            unions,
            data,
            is_root_array: reader.is_root_array(),
        };
        doc.set_root_array(reader.is_root_array());
        Ok(doc)
    }

    /// Create a TeaLeaf document from a single DTO.
    ///
    /// The DTO is placed under the given `key` in the document data map.
    /// Schemas are automatically collected from the DTO type.
    pub fn from_dto<T: convert::ToTeaLeaf>(key: &str, dto: &T) -> Self {
        let schemas = T::collect_schemas();
        let unions = T::collect_unions();
        let mut data = IndexMap::new();
        data.insert(key.to_string(), dto.to_tealeaf_value());
        let mut doc = Self::new(schemas, data);
        doc.unions = unions;
        doc
    }

    /// Create a TeaLeaf document from a slice of DTOs.
    ///
    /// The array is placed under the given `key` and schemas are
    /// collected from the element type.
    pub fn from_dto_array<T: convert::ToTeaLeaf>(key: &str, items: &[T]) -> Self {
        let schemas = T::collect_schemas();
        let unions = T::collect_unions();
        let mut data = IndexMap::new();
        let arr = Value::Array(items.iter().map(|i| i.to_tealeaf_value()).collect());
        data.insert(key.to_string(), arr);
        let mut doc = Self::new(schemas, data);
        doc.unions = unions;
        doc
    }

    /// Extract a DTO from this document by key.
    pub fn to_dto<T: convert::FromTeaLeaf>(&self, key: &str) -> Result<T> {
        let value = self
            .get(key)
            .ok_or_else(|| Error::MissingField(key.to_string()))?;
        T::from_tealeaf_value(value).map_err(|e| e.into())
    }

    /// Extract all values under a key as `Vec<T>`.
    pub fn to_dto_vec<T: convert::FromTeaLeaf>(&self, key: &str) -> Result<Vec<T>> {
        let value = self
            .get(key)
            .ok_or_else(|| Error::MissingField(key.to_string()))?;
        let arr = value
            .as_array()
            .ok_or_else(|| Error::ParseError("Expected array".into()))?;
        arr.iter()
            .map(|v| T::from_tealeaf_value(v).map_err(|e| e.into()))
            .collect()
    }
}

/// Convert JSON value to TeaLeaf value (best-effort)
fn json_to_tealeaf_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::UInt(u)
            } else {
                let raw = n.to_string();
                // Pure integer that doesn't fit i64/u64 → preserve exactly
                if !raw.contains('.') && !raw.contains('e') && !raw.contains('E') {
                    Value::JsonNumber(raw)
                } else {
                    match n.as_f64() {
                        Some(f) if f.is_finite() => Value::Float(f),
                        _ => Value::JsonNumber(raw),
                    }
                }
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
/// - Value::UInt → JSON integer (e.g., 18446744073709551615)
/// - Value::Float → JSON float (e.g., 42.0)
///
/// Integer types are tried first during JSON import (i64, then u64) so that
/// values within 64-bit range stay exact. Only true floats fall through to f64.
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
        Value::Timestamp(ts, tz) => {
            serde_json::Value::String(format_timestamp_millis(*ts, *tz))
        }
        Value::JsonNumber(s) => {
            s.parse::<serde_json::Number>()
                .map(serde_json::Value::Number)
                .unwrap_or_else(|_| serde_json::Value::String(s.clone()))
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
pub fn loads(input: &str) -> Result<IndexMap<String, Value>> {
    Ok(TeaLeaf::parse(input)?.data)
}

/// Convenience: serialize to TeaLeaf text
/// Check if a string needs quoting when serialized to TeaLeaf format.
/// Returns true if the string could be misinterpreted as another type.
fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    // Reserved words, null literal, and float literals the lexer would interpret
    if matches!(s, "true" | "false" | "null" | "~" | "NaN" | "inf" | "Infinity") {
        return true;
    }

    // Whitelist approach: only allow [a-zA-Z0-9_-.] unquoted (ASCII only).
    // Matches spec grammar: name = (letter | "_") { letter | digit | "_" | "-" | "." }
    // Any other character (Unicode digits, whitespace, punctuation, etc.)
    // requires quoting to ensure safe round-trip through the parser.
    // Note: '-' is excluded here because strings starting with '-' are caught
    // by the sign-character check below, and mid-string '-' in identifiers
    // like "foo-bar" is safe only when the first char is a letter.
    if s.contains(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.') {
        return true;
    }

    // Must start with letter or underscore per grammar: name = (letter | "_") { ... }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return true;
    }

    // Starts with 0x/0b (hex/binary literal prefix)
    if s.starts_with("0x") || s.starts_with("0X") || s.starts_with("0b") || s.starts_with("0B") {
        return true;
    }

    // Starts with sign character — always quote to avoid parser ambiguity
    // (parser may try to interpret as a signed number).
    if s.starts_with('-') || s.starts_with('+') {
        return true;
    }

    // Starts with a digit — could be parsed as a number
    if first.is_ascii_digit() {
        return true;
    }

    false
}

/// Write a key to the output, quoting if necessary for safe round-trip.
fn write_key(out: &mut String, key: &str) {
    if needs_quoting(key) {
        out.push('"');
        out.push_str(&escape_string(key));
        out.push('"');
    } else {
        out.push_str(key);
    }
}

/// Write a map key per spec grammar: `map_key = string | name | integer`.
/// Int/UInt are written as-is. String values use `write_key` for quoting.
/// Other value types (Null, Bool, Float, etc.) are coerced to quoted strings
/// so that the text format always round-trips through the parser.
fn write_map_key(out: &mut String, key: &Value) {
    match key {
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::UInt(u) => out.push_str(&u.to_string()),
        Value::String(s) => write_key(out, s),
        // Coerce non-spec key types to quoted strings for text format safety
        Value::Null => out.push_str("\"~\""),
        Value::Bool(b) => { out.push('"'); out.push_str(if *b { "true" } else { "false" }); out.push('"'); }
        Value::Float(f) => { out.push('"'); out.push_str(&f.to_string()); out.push('"'); }
        Value::JsonNumber(s) => { out.push('"'); out.push_str(s); out.push('"'); }
        Value::Timestamp(ts, tz) => { out.push('"'); out.push_str(&format_timestamp_millis(*ts, *tz)); out.push('"'); }
        Value::Bytes(b) => {
            out.push_str("\"0x");
            for byte in b { out.push_str(&format!("{:02x}", byte)); }
            out.push('"');
        }
        Value::Ref(r) => { out.push('"'); out.push('!'); out.push_str(r); out.push('"'); }
        Value::Tagged(tag, _) => { out.push('"'); out.push(':'); out.push_str(tag); out.push('"'); }
        Value::Array(_) | Value::Object(_) | Value::Map(_) => out.push_str("\"\""),
    }
}

/// Options controlling TeaLeaf text output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    /// Remove insignificant whitespace (spaces after `:` and `,`, indentation, blank lines).
    pub compact: bool,
    /// Emit whole-number floats without `.0` suffix (e.g., `42.0` → `42`).
    /// Saves characters/tokens but changes float→int type on re-parse.
    pub compact_floats: bool,
}

impl FormatOptions {
    /// Pretty-printed output (default).
    pub fn pretty() -> Self {
        Self { compact: false, compact_floats: false }
    }

    /// Compact output (whitespace stripped).
    pub fn compact() -> Self {
        Self { compact: true, compact_floats: false }
    }

    /// Enable compact float formatting (strip `.0` from whole-number floats).
    pub fn with_compact_floats(mut self) -> Self {
        self.compact_floats = true;
        self
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self::pretty()
    }
}

pub fn dumps(data: &IndexMap<String, Value>) -> String {
    dumps_inner(data, &FormatOptions::default())
}

/// Serialize data to compact TeaLeaf text format (no schemas).
/// Removes insignificant whitespace for token-efficient output.
pub fn dumps_compact(data: &IndexMap<String, Value>) -> String {
    dumps_inner(data, &FormatOptions::compact())
}

/// Serialize data to TeaLeaf text format with custom options (no schemas).
pub fn dumps_with_options(data: &IndexMap<String, Value>, opts: &FormatOptions) -> String {
    dumps_inner(data, opts)
}

fn dumps_inner(data: &IndexMap<String, Value>, opts: &FormatOptions) -> String {
    let mut out = String::new();
    for (key, value) in data {
        write_key(&mut out, key);
        out.push_str(kv_sep(opts.compact));
        write_value(&mut out, value, 0, opts);
        out.push('\n');
    }
    out
}

/// Returns ", " in pretty mode, "," in compact mode
#[inline]
fn sep(compact: bool) -> &'static str {
    if compact { "," } else { ", " }
}

/// Returns ": " in pretty mode, ":" in compact mode.
#[inline]
fn kv_sep(compact: bool) -> &'static str {
    if compact { ":" } else { ": " }
}

/// Escape a string for TeaLeaf text output.
/// Handles: \\ \" \n \t \r \b \f and \uXXXX for other control characters.
fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if c.is_control() => {
                // Other control characters use \uXXXX
                for unit in c.encode_utf16(&mut [0u16; 2]) {
                    out.push_str(&format!("\\u{:04x}", unit));
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Format a float ensuring it always has a decimal point or uses scientific notation.
/// Rust's f64::to_string() expands large/small values (e.g., 6.022e23 becomes
/// "602200000000000000000000"), which would be reparsed as an integer and overflow.
/// We use scientific notation for values outside a safe range.
fn format_float(f: f64, compact_floats: bool) -> String {
    // Handle non-finite values with keywords the lexer recognizes
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f.is_sign_positive() { "inf".to_string() } else { "-inf".to_string() };
    }

    let s = f.to_string();
    if s.contains('.') || s.contains('e') || s.contains('E') {
        // Already has decimal point or scientific notation — safe as-is
        s
    } else {
        // to_string() produced an integer-looking string (no '.' or 'e').
        // For large values, use scientific notation to avoid i64 overflow on re-parse.
        // For small values, append ".0" unless compact_floats is enabled.
        let digits = s.trim_start_matches('-').len();
        if digits > 15 {
            format!("{:e}", f)
        } else if compact_floats {
            s
        } else {
            format!("{}.0", s)
        }
    }
}

fn write_value(out: &mut String, value: &Value, indent: usize, opts: &FormatOptions) {
    match value {
        Value::Null => out.push('~'),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::UInt(u) => out.push_str(&u.to_string()),
        Value::JsonNumber(s) => out.push_str(s),
        Value::Float(f) => out.push_str(&format_float(*f, opts.compact_floats)),
        Value::String(s) => {
            if needs_quoting(s) {
                out.push('"');
                out.push_str(&escape_string(s));
                out.push('"');
            } else {
                out.push_str(s);
            }
        }
        Value::Bytes(b) => {
            out.push_str("b\"");
            for byte in b {
                out.push_str(&format!("{:02x}", byte));
            }
            out.push('"');
        }
        Value::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 { out.push_str(sep(opts.compact)); }
                write_value(out, v, indent, opts);
            }
            out.push(']');
        }
        Value::Object(obj) => {
            out.push('{');
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 { out.push_str(sep(opts.compact)); }
                write_key(out, k);
                out.push_str(kv_sep(opts.compact));
                write_value(out, v, indent, opts);
            }
            out.push('}');
        }
        Value::Map(pairs) => {
            out.push_str(if opts.compact { "@map{" } else { "@map {" });
            let mut first = true;
            for (k, v) in pairs {
                if !first { out.push_str(sep(opts.compact)); }
                first = false;
                // Map keys are restricted to string | name | integer per spec.
                // Write Int/UInt directly; convert other types to quoted strings.
                write_map_key(out, k);
                out.push_str(kv_sep(opts.compact));
                write_value(out, v, indent, opts);
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
            write_value(out, inner, indent, opts);
        }
        Value::Timestamp(ts, tz) => {
            out.push_str(&format_timestamp_millis(*ts, *tz));
        }
    }
}

/// Format a Unix-millis timestamp as an ISO 8601 string with timezone offset.
/// Handles negative timestamps (pre-epoch dates) correctly using Euclidean division.
/// Years outside [0000, 9999] are clamped to the boundary per spec (4-digit years only).
/// When tz_offset_minutes is 0, emits 'Z' suffix. Otherwise emits +HH:MM or -HH:MM.
fn format_timestamp_millis(ts: i64, tz_offset_minutes: i16) -> String {
    // Clamp to representable ISO 8601 range (years 0000-9999).
    // Year 0000-01-01T00:00:00Z = -62167219200000 ms
    // Year 9999-12-31T23:59:59.999Z = 253402300799999 ms
    const MIN_TS: i64 = -62_167_219_200_000;
    const MAX_TS: i64 = 253_402_300_799_999;
    let ts = ts.clamp(MIN_TS, MAX_TS);

    // Apply timezone offset to get local time for display
    let local_ts = ts + (tz_offset_minutes as i64) * 60_000;
    let local_ts = local_ts.clamp(MIN_TS, MAX_TS);

    let secs = local_ts.div_euclid(1000);
    let millis = local_ts.rem_euclid(1000);
    let days = secs.div_euclid(86400);
    let time_secs = secs.rem_euclid(86400);
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let secs_rem = time_secs % 60;
    let (year, month, day) = days_to_ymd(days);

    let tz_suffix = if tz_offset_minutes == 0 {
        "Z".to_string()
    } else {
        let sign = if tz_offset_minutes > 0 { '+' } else { '-' };
        let abs = tz_offset_minutes.unsigned_abs();
        format!("{}{:02}:{:02}", sign, abs / 60, abs % 60)
    };

    if millis > 0 {
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}{}",
            year, month, day, hours, mins, secs_rem, millis, tz_suffix)
    } else {
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
            year, month, day, hours, mins, secs_rem, tz_suffix)
    }
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from Howard Hinnant (extended to i64 for extreme timestamps)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
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
                // Merge objects: keep fields present in both (intersection).
                // Fields only in one side are dropped — the schema inference
                // uses union across all objects separately, so the type merge
                // only needs the common fields to identify the schema.
                let mut merged = Vec::new();
                let b_map: IndexMap<&str, &InferredType> = b.iter().map(|(k, v)| (k.as_str(), v)).collect();

                for (key, a_type) in a {
                    if let Some(b_type) = b_map.get(key.as_str()) {
                        merged.push((key.clone(), a_type.merge(b_type)));
                    }
                }

                if merged.is_empty() {
                    InferredType::Mixed
                } else {
                    InferredType::Object(merged)
                }
            }
            _ => InferredType::Mixed,
        }
    }

    fn to_field_type(&self, schemas: &IndexMap<String, Schema>) -> FieldType {
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
                // Check if this matches an existing schema.
                // Allow nullable schema fields to be absent (union-based inference
                // may produce schemas with more fields than the type intersection).
                let field_names: HashSet<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
                for (name, schema) in schemas {
                    if object_matches_schema(&field_names, schema) {
                        return FieldType::new(name.clone());
                    }
                }
                // No matching schema — use "any" (not "object", which is a
                // value-only type rejected by the parser in schema definitions)
                FieldType::new("any")
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
            let fields: Vec<(String, InferredType)> = obj
                .iter()
                .map(|(k, v)| (k.clone(), infer_type(v)))
                .collect();
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
    } else if name.len() > 1 && name.ends_with('s') && !name.ends_with("ss") {
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

/// Check if an object's keys match a schema, allowing nullable fields to be absent.
fn object_matches_schema(obj_keys: &HashSet<&str>, schema: &Schema) -> bool {
    // Every non-nullable schema field must be present
    schema.fields.iter().all(|f| f.field_type.nullable || obj_keys.contains(f.name.as_str()))
    // Every object key must be a schema field
    && obj_keys.iter().all(|k| schema.fields.iter().any(|f| f.name == *k))
}

/// Schema inferrer that analyzes data and generates schemas
pub struct SchemaInferrer {
    schemas: IndexMap<String, Schema>,
    schema_order: Vec<String>,  // Track order for output
}

impl SchemaInferrer {
    pub fn new() -> Self {
        Self {
            schemas: IndexMap::new(),
            schema_order: Vec::new(),
        }
    }

    /// Analyze data and infer schemas from uniform object arrays
    pub fn infer(&mut self, data: &IndexMap<String, Value>) {
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

        // Collect field names as union across all objects.
        // Fields not present in every object will be marked nullable.
        // Use the most-complete object's field ordering as canonical schema order.
        let mut field_count: IndexMap<String, usize> = IndexMap::new();
        let total_objects = arr.len();
        let mut most_fields_idx = 0;
        let mut most_fields_len = 0;

        for (i, item) in arr.iter().enumerate() {
            if let Value::Object(obj) = item {
                for key in obj.keys() {
                    *field_count.entry(key.clone()).or_insert(0) += 1;
                }
                if obj.len() > most_fields_len {
                    most_fields_len = obj.len();
                    most_fields_idx = i;
                }
            } else {
                return; // Not all objects
            }
        }

        // Use the most-complete object's field ordering, then append any remaining
        // fields from other objects (preserves the representative object's key order)
        let field_names: Vec<String> = if let Value::Object(repr) = &arr[most_fields_idx] {
            let mut names: Vec<String> = repr.keys().cloned().collect();
            for key in field_count.keys() {
                if !repr.contains_key(key) {
                    names.push(key.clone());
                }
            }
            names
        } else {
            field_count.keys().cloned().collect()
        };

        // Skip schema inference if fields are empty, any field name is empty,
        // or the schema name itself needs quoting (it appears unquoted in
        // `@struct name(...)` and `@table name [...]`).
        // Field names that need quoting are fine — they get quoted in the
        // @struct definition, e.g. `@struct root("@type":string, name:string)`.
        if field_names.is_empty()
            || field_names.iter().any(|n| n.is_empty())
            || needs_quoting(hint_name)
        {
            return;
        }

        // Require at least 1 field present in ALL objects (shared structure)
        let common_count = field_count.values().filter(|&&c| c == total_objects).count();
        if common_count == 0 {
            return;
        }

        // Infer types for each field across all objects
        let mut field_types: IndexMap<String, InferredType> = IndexMap::new();
        let mut has_null: IndexMap<String, bool> = IndexMap::new();

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

        // Mark fields not present in all objects as nullable
        for (field, count) in &field_count {
            if *count < total_objects {
                *has_null.entry(field.clone()).or_insert(false) = true;
            }
        }

        // Generate schema name from hint
        let schema_name = singularize(hint_name);

        // Skip if schema already exists
        if self.schemas.contains_key(&schema_name) {
            return;
        }

        // Recursively analyze nested fields in field order (depth-first).
        // Single pass processes arrays and objects as encountered, matching
        // the derive path's field-declaration-order traversal.
        // Use representative values from any object (not just first) since
        // optional fields may be absent from some objects.
        for field_name in &field_names {
            let representative = arr.iter().find_map(|item| {
                if let Value::Object(obj) = item {
                    obj.get(field_name)
                } else {
                    None
                }
            });
            match representative {
                Some(Value::Array(_)) => {
                    // Collect ALL nested array items from ALL parent objects.
                    // A single representative's array may not have all field
                    // variations (e.g., distribution objects differ across datasets).
                    let all_nested: Vec<Value> = arr.iter()
                        .filter_map(|item| {
                            if let Value::Object(obj) = item {
                                obj.get(field_name)
                            } else {
                                None
                            }
                        })
                        .filter_map(|v| if let Value::Array(a) = v { Some(a) } else { None })
                        .flatten()
                        .cloned()
                        .collect();
                    if !all_nested.is_empty() {
                        self.analyze_array(field_name, &all_nested);
                    }
                }
                Some(Value::Object(_)) => {
                    // Skip object fields whose singularized name collides
                    // with this array's schema name — prevents
                    // self-referencing schemas (e.g., @struct root (root: root)).
                    if singularize(field_name) == schema_name {
                        continue;
                    }

                    let nested_objects: Vec<&IndexMap<String, Value>> = arr
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

                    if !nested_objects.is_empty() {
                        self.analyze_nested_objects(field_name, &nested_objects);
                    }
                }
                _ => {}
            }
        }

        // Re-check: recursive nested analysis (both arrays and objects) may have
        // claimed this schema name. This happens when the same field name appears
        // at multiple nesting levels (e.g., "nodes" containing "nodes"). The inner
        // schema was created first (depth-first); preserve it to avoid overwriting
        // with a different structure.
        if self.schemas.contains_key(&schema_name) {
            return;
        }

        // Build schema
        let mut schema = Schema::new(&schema_name);

        // Use union field order (first-seen across all objects)
        for field_name in &field_names {
            if let Some(inferred) = field_types.get(field_name) {
                let mut field_type = inferred.to_field_type(&self.schemas);

                // Mark as nullable if any null values seen or field missing from some objects
                if has_null.get(field_name).copied().unwrap_or(false) {
                    field_type.nullable = true;
                }

                // Check if there's a nested schema for array fields
                // Use representative from any object (field may be optional)
                let representative_arr = arr.iter().find_map(|item| {
                    if let Value::Object(obj) = item {
                        if let Some(Value::Array(nested)) = obj.get(field_name) {
                            Some(nested.as_slice())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });
                if let Some(nested_arr) = representative_arr {
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

                // Check if there's a nested schema for object fields
                // (skip self-references: field singularizing to the schema being built)
                let nested_schema_name = singularize(field_name);
                if nested_schema_name != schema_name && self.schemas.contains_key(&nested_schema_name) {
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
    /// and create a schema if they share common structure (fields not present in all
    /// objects are marked nullable).
    fn analyze_nested_objects(&mut self, field_name: &str, objects: &[&IndexMap<String, Value>]) {
        if objects.is_empty() {
            return;
        }

        // Collect field names as union across all objects.
        // Use the most-complete object's field ordering as canonical schema order.
        let mut field_count: IndexMap<String, usize> = IndexMap::new();
        let total_objects = objects.len();
        let mut most_fields_idx = 0;
        let mut most_fields_len = 0;

        for (i, obj) in objects.iter().enumerate() {
            for key in obj.keys() {
                *field_count.entry(key.clone()).or_insert(0) += 1;
            }
            if obj.len() > most_fields_len {
                most_fields_len = obj.len();
                most_fields_idx = i;
            }
        }

        // Use the most-complete object's field ordering, then append remaining
        let nested_field_names: Vec<String> = {
            let repr = objects[most_fields_idx];
            let mut names: Vec<String> = repr.keys().cloned().collect();
            for key in field_count.keys() {
                if !repr.contains_key(key) {
                    names.push(key.clone());
                }
            }
            names
        };

        // Compute schema name early so we can check if it needs quoting
        let schema_name = singularize(field_name);

        // Skip empty objects, empty field names, or when the schema name itself
        // needs quoting (it appears unquoted in `@struct name(...)` and `@table name [...]`).
        // Field names that need quoting are fine — they get quoted in the definition.
        if nested_field_names.is_empty()
            || nested_field_names.iter().any(|n| n.is_empty())
            || needs_quoting(&schema_name)
        {
            return;
        }

        // Require at least 1 field present in ALL objects (shared structure)
        let common_count = field_count.values().filter(|&&c| c == total_objects).count();
        if common_count == 0 {
            return;
        }

        // Skip if schema already exists
        if self.schemas.contains_key(&schema_name) {
            return;
        }

        // Infer field types across all objects
        let mut field_types: IndexMap<String, InferredType> = IndexMap::new();
        let mut has_null: IndexMap<String, bool> = IndexMap::new();

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

        // Mark fields not present in all objects as nullable
        for (field, count) in &field_count {
            if *count < total_objects {
                *has_null.entry(field.clone()).or_insert(false) = true;
            }
        }

        // Recursively analyze nested fields in field order (depth-first).
        // Single pass mirrors the derive path's field-declaration-order traversal,
        // so CLI and Builder API produce schemas in the same order.
        // Use representative values from any object since optional fields
        // may be absent from some objects.
        for nested_field in &nested_field_names {
            let representative = objects.iter().find_map(|obj| obj.get(nested_field));
            match representative {
                Some(Value::Array(_)) => {
                    // Collect ALL nested array items from ALL parent objects
                    let all_nested: Vec<Value> = objects.iter()
                        .filter_map(|obj| obj.get(nested_field))
                        .filter_map(|v| if let Value::Array(a) = v { Some(a) } else { None })
                        .flatten()
                        .cloned()
                        .collect();
                    if !all_nested.is_empty() {
                        self.analyze_array(nested_field, &all_nested);
                    }
                }
                Some(Value::Object(_)) => {
                    let deeper_objects: Vec<&IndexMap<String, Value>> = objects
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
                _ => {}
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

                // Check if this field has a nested array schema
                // Use representative from any object (field may be optional)
                if matches!(inferred, InferredType::Array(_)) {
                    let representative_arr = objects.iter().find_map(|obj| {
                        if let Some(Value::Array(nested)) = obj.get(nested_field) {
                            Some(nested.as_slice())
                        } else {
                            None
                        }
                    });
                    if let Some(nested_arr) = representative_arr {
                        let nested_schema_name = singularize(nested_field);
                        if let Some(nested_schema) = self.schemas.get(&nested_schema_name) {
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

                // Check if this field has a nested object schema
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

    pub fn into_schemas(self) -> (IndexMap<String, Schema>, Vec<String>) {
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
    data: &IndexMap<String, Value>,
    schemas: &IndexMap<String, Schema>,
    schema_order: &[String],
    unions: &IndexMap<String, Union>,
    union_order: &[String],
) -> String {
    dumps_with_schemas_inner(data, schemas, schema_order, unions, union_order, &FormatOptions::default())
}

/// Serialize data to compact TeaLeaf text format with schemas.
/// Removes insignificant whitespace for token-efficient output.
pub fn dumps_with_schemas_compact(
    data: &IndexMap<String, Value>,
    schemas: &IndexMap<String, Schema>,
    schema_order: &[String],
    unions: &IndexMap<String, Union>,
    union_order: &[String],
) -> String {
    dumps_with_schemas_inner(data, schemas, schema_order, unions, union_order, &FormatOptions::compact())
}

/// Serialize data to TeaLeaf text format with schemas and custom options.
pub fn dumps_with_schemas_with_options(
    data: &IndexMap<String, Value>,
    schemas: &IndexMap<String, Schema>,
    schema_order: &[String],
    unions: &IndexMap<String, Union>,
    union_order: &[String],
    opts: &FormatOptions,
) -> String {
    dumps_with_schemas_inner(data, schemas, schema_order, unions, union_order, opts)
}

fn dumps_with_schemas_inner(
    data: &IndexMap<String, Value>,
    schemas: &IndexMap<String, Schema>,
    schema_order: &[String],
    unions: &IndexMap<String, Union>,
    union_order: &[String],
    opts: &FormatOptions,
) -> String {
    let mut out = String::new();
    let mut has_definitions = false;

    // Write union definitions first (before structs, since structs may reference unions)
    for name in union_order {
        if let Some(union) = unions.get(name) {
            out.push_str("@union ");
            out.push_str(&union.name);
            out.push_str(if opts.compact { "{\n" } else { " {\n" });
            for (vi, variant) in union.variants.iter().enumerate() {
                if !opts.compact { out.push_str("  "); }
                out.push_str(&variant.name);
                out.push_str(if opts.compact { "(" } else { " (" });
                for (fi, field) in variant.fields.iter().enumerate() {
                    if fi > 0 {
                        out.push_str(sep(opts.compact));
                    }
                    out.push_str(&field.name);
                    out.push_str(kv_sep(opts.compact));
                    out.push_str(&field.field_type.to_string());
                }
                out.push(')');
                if vi < union.variants.len() - 1 {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("}\n");
            has_definitions = true;
        }
    }

    // Write struct schemas in order
    for name in schema_order {
        if let Some(schema) = schemas.get(name) {
            out.push_str("@struct ");
            out.push_str(&schema.name);
            out.push_str(if opts.compact { "(" } else { " (" });
            for (i, field) in schema.fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep(opts.compact));
                }
                write_key(&mut out, &field.name);
                out.push_str(kv_sep(opts.compact));
                out.push_str(&field.field_type.to_string());
            }
            out.push_str(")\n");
            has_definitions = true;
        }
    }

    if has_definitions && !opts.compact {
        out.push('\n');
    }

    // Write data (preserves insertion order)
    for (key, value) in data {
        write_key(&mut out, key);
        out.push_str(kv_sep(opts.compact));
        write_value_with_schemas(&mut out, value, schemas, Some(key), 0, None, opts);
        out.push('\n');
    }

    out
}

/// Resolve a schema for a value by trying three strategies in order:
/// 1. Declared type from parent schema's field type (exact match)
/// 2. Singularize the field key name (works for JSON-inference schemas)
/// 3. Case-insensitive singularize (handles derive-macro PascalCase names)
fn resolve_schema<'a>(
    schemas: &'a IndexMap<String, Schema>,
    declared_type: Option<&str>,
    hint_name: Option<&str>,
) -> Option<&'a Schema> {
    // 1. Direct lookup by declared type from parent schema
    if let Some(name) = declared_type {
        if let Some(s) = schemas.get(name) {
            return Some(s);
        }
    }
    // 2. Singularize heuristic (existing behavior for JSON-inference schemas)
    if let Some(hint) = hint_name {
        let singular = singularize(hint);
        if let Some(s) = schemas.get(&singular) {
            return Some(s);
        }
        // 3. Case-insensitive singularize (for derive-macro PascalCase names)
        let singular_lower = singular.to_ascii_lowercase();
        for (name, schema) in schemas {
            if name.to_ascii_lowercase() == singular_lower {
                return Some(schema);
            }
        }
    }
    None
}

fn write_value_with_schemas(
    out: &mut String,
    value: &Value,
    schemas: &IndexMap<String, Schema>,
    hint_name: Option<&str>,
    indent: usize,
    declared_type: Option<&str>,
    opts: &FormatOptions,
) {
    match value {
        Value::Null => out.push('~'),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::UInt(u) => out.push_str(&u.to_string()),
        Value::JsonNumber(s) => out.push_str(s),
        Value::Float(f) => out.push_str(&format_float(*f, opts.compact_floats)),
        Value::String(s) => {
            if needs_quoting(s) {
                out.push('"');
                out.push_str(&escape_string(s));
                out.push('"');
            } else {
                out.push_str(s);
            }
        }
        Value::Bytes(b) => {
            out.push_str("b\"");
            for byte in b {
                out.push_str(&format!("{:02x}", byte));
            }
            out.push('"');
        }
        Value::Array(arr) => {
            // Check if this array can use @table format.
            // Try name-based resolution first, then structural matching as fallback.
            let mut schema = resolve_schema(schemas, declared_type, hint_name);

            // Structural fallback: if name-based resolution failed, find a schema
            // that matches the first element's object keys (allowing nullable fields
            // to be absent).
            // This handles Builder-path documents where the top-level key name
            // (e.g., "orders") doesn't match the schema name (e.g., "SalesOrder").
            // Only apply when we have context (hint_name or declared_type) — inside
            // tuple values (both None), structural matching can pick the wrong schema
            // and produce @table where the parser doesn't expect it.
            if schema.is_none() && (hint_name.is_some() || declared_type.is_some()) {
                if let Some(Value::Object(first_obj)) = arr.first() {
                    let obj_keys: HashSet<&str> = first_obj.keys().map(|k| k.as_str()).collect();
                    for (_, candidate) in schemas {
                        if object_matches_schema(&obj_keys, candidate) {
                            schema = Some(candidate);
                            break;
                        }
                    }
                }
            }

            if let Some(schema) = schema {
                // Verify the first element is an object whose fields match the schema.
                // A name-only lookup isn't enough — if the same field name appears at
                // multiple nesting levels with different shapes, the schema may belong
                // to a different level. Applying the wrong schema drops unmatched keys.
                // Nullable fields are allowed to be absent.
                let schema_matches = if let Some(Value::Object(first_obj)) = arr.first() {
                    let obj_keys: HashSet<&str> = first_obj.keys().map(|k| k.as_str()).collect();
                    object_matches_schema(&obj_keys, schema)
                } else {
                    false
                };

                if schema_matches {
                    out.push_str("@table ");
                    out.push_str(&schema.name);
                    out.push_str(if opts.compact { "[\n" } else { " [\n" });

                    let inner_indent = if opts.compact { 0 } else { indent + 2 };
                    for (i, item) in arr.iter().enumerate() {
                        if !opts.compact {
                            for _ in 0..inner_indent {
                                out.push(' ');
                            }
                        }
                        write_tuple(out, item, schema, schemas, inner_indent, opts);
                        if i < arr.len() - 1 {
                            out.push(',');
                        }
                        out.push('\n');
                    }

                    if !opts.compact {
                        for _ in 0..indent {
                            out.push(' ');
                        }
                    }
                    out.push(']');
                    return;
                }
            }

            // Fall back to regular array format
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep(opts.compact));
                }
                write_value_with_schemas(out, v, schemas, None, indent, None, opts);
            }
            out.push(']');
        }
        Value::Object(obj) => {
            // Find the schema for this object so we can propagate field types to children.
            // Try name-based resolution first, then structural matching as fallback.
            let mut obj_schema = resolve_schema(schemas, declared_type, hint_name);

            if obj_schema.is_none() {
                let obj_keys: HashSet<&str> = obj.keys().map(|k| k.as_str()).collect();
                for (_, candidate) in schemas {
                    if object_matches_schema(&obj_keys, candidate) {
                        obj_schema = Some(candidate);
                        break;
                    }
                }
            }

            out.push('{');
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep(opts.compact));
                }
                write_key(out, k);
                out.push_str(kv_sep(opts.compact));
                // Look up this field's declared type from the parent schema
                let field_type = obj_schema.and_then(|s| {
                    s.fields.iter()
                        .find(|f| f.name == *k)
                        .map(|f| f.field_type.base.as_str())
                });
                write_value_with_schemas(out, v, schemas, Some(k), indent, field_type, opts);
            }
            out.push('}');
        }
        Value::Map(pairs) => {
            out.push_str(if opts.compact { "@map{" } else { "@map {" });
            let mut first = true;
            for (k, v) in pairs {
                if !first {
                    out.push_str(sep(opts.compact));
                }
                first = false;
                write_map_key(out, k);
                out.push_str(kv_sep(opts.compact));
                write_value_with_schemas(out, v, schemas, None, indent, None, opts);
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
            write_value_with_schemas(out, inner, schemas, None, indent, None, opts);
        }
        Value::Timestamp(ts, tz) => {
            out.push_str(&format_timestamp_millis(*ts, *tz));
        }
    }
}

fn write_tuple(
    out: &mut String,
    value: &Value,
    schema: &Schema,
    schemas: &IndexMap<String, Schema>,
    indent: usize,
    opts: &FormatOptions,
) {
    if let Value::Object(obj) = value {
        out.push('(');
        for (i, field) in schema.fields.iter().enumerate() {
            if i > 0 {
                out.push_str(sep(opts.compact));
            }
            if let Some(v) = obj.get(&field.name) {
                let type_base = field.field_type.base.as_str();
                // For array fields with a known schema type, write tuples directly without @table
                if field.field_type.is_array {
                    if let Some(item_schema) = resolve_schema(schemas, Some(type_base), None) {
                        // The schema defines the element type - write array with tuples directly
                        write_schema_array(out, v, item_schema, schemas, indent, opts);
                    } else {
                        // No schema for element type - use regular array format
                        write_value_with_schemas(out, v, schemas, None, indent, None, opts);
                    }
                } else if resolve_schema(schemas, Some(type_base), None).is_some() {
                    // Non-array field with schema type - write as nested tuple
                    let nested_schema = resolve_schema(schemas, Some(type_base), None).unwrap();
                    write_tuple(out, v, nested_schema, schemas, indent, opts);
                } else {
                    write_value_with_schemas(out, v, schemas, None, indent, None, opts);
                }
            } else {
                out.push('~');
            }
        }
        out.push(')');
    } else {
        write_value_with_schemas(out, value, schemas, None, indent, None, opts);
    }
}

/// Write an array of schema-typed values as tuples (without @table annotation)
fn write_schema_array(
    out: &mut String,
    value: &Value,
    schema: &Schema,
    schemas: &IndexMap<String, Schema>,
    indent: usize,
    opts: &FormatOptions,
) {
    if let Value::Array(arr) = value {
        if arr.is_empty() {
            out.push_str("[]");
            return;
        }

        out.push_str("[\n");
        let inner_indent = if opts.compact { 0 } else { indent + 2 };
        for (i, item) in arr.iter().enumerate() {
            if !opts.compact {
                for _ in 0..inner_indent {
                    out.push(' ');
                }
            }
            write_tuple(out, item, schema, schemas, inner_indent, opts);
            if i < arr.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        if !opts.compact {
            for _ in 0..indent {
                out.push(' ');
            }
        }
        out.push(']');
    } else {
        // Not an array - fall back to regular value writing
        write_value_with_schemas(out, value, schemas, None, indent, None, opts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compare two Values treating absent keys as equivalent to null.
    /// With union-based schema inference, optional fields absent in the
    /// original appear as explicit nulls after roundtrip through @table.
    fn values_eq_with_optional_nulls(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Object(oa), Value::Object(ob)) => {
                // All keys in a must be in b with matching values
                for (k, va) in oa {
                    match ob.get(k) {
                        Some(vb) => if !values_eq_with_optional_nulls(va, vb) { return false; }
                        None => if *va != Value::Null { return false; }
                    }
                }
                // Extra keys in b must have null values
                for (k, vb) in ob {
                    if !oa.contains_key(k) && *vb != Value::Null {
                        return false;
                    }
                }
                true
            }
            (Value::Array(aa), Value::Array(ab)) => {
                aa.len() == ab.len()
                    && aa.iter().zip(ab).all(|(a, b)| values_eq_with_optional_nulls(a, b))
            }
            _ => a == b,
        }
    }

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
        let mut entries = IndexMap::new();
        entries.insert("data".to_string(), Value::Bytes(vec![0xca, 0xfe, 0xf0, 0x0d]));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

        let json = doc.to_json().unwrap();
        assert!(json.contains("0xcafef00d"), "Bytes should export as hex string: {}", json);
    }

    #[test]
    fn test_json_export_ref() {
        let mut entries = IndexMap::new();
        entries.insert("config".to_string(), Value::Ref("base_config".to_string()));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

        let json = doc.to_json().unwrap();
        assert!(json.contains("\"$ref\""), "Ref should export with $ref key: {}", json);
        assert!(json.contains("base_config"), "Ref name should be in output: {}", json);
    }

    #[test]
    fn test_json_export_tagged() {
        let mut entries = IndexMap::new();
        entries.insert("status".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

        let json = doc.to_json().unwrap();
        assert!(json.contains("\"$tag\""), "Tagged should export with $tag key: {}", json);
        assert!(json.contains("\"ok\""), "Tag name should be in output: {}", json);
        assert!(json.contains("\"$value\""), "Tagged should have $value key: {}", json);
    }

    #[test]
    fn test_json_export_map() {
        let mut entries = IndexMap::new();
        entries.insert("lookup".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
            (Value::Int(2), Value::String("two".to_string())),
        ]));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

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
        let mut entries = IndexMap::new();
        // 2024-01-15T10:30:00Z = 1705315800000 ms, but let's verify with a known value
        // Use 0 = 1970-01-01T00:00:00Z for simplicity
        entries.insert("created".to_string(), Value::Timestamp(0, 0));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

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
        assert!(created.as_timestamp_millis().is_none(), "ISO timestamp should NOT become Timestamp value");
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
        let mut entries = IndexMap::new();
        entries.insert("data".to_string(), Value::String("a".repeat(1000)));
        entries.insert("count".to_string(), Value::Int(12345));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), true).unwrap(); // compressed

        let reader = Reader::open(temp.path()).unwrap();
        assert_eq!(reader.get("data").unwrap().as_str(), Some("a".repeat(1000).as_str()));
        assert_eq!(reader.get("count").unwrap().as_int(), Some(12345));
    }

    #[test]
    fn test_tl_to_binary_preserves_ref() {
        use tempfile::NamedTempFile;

        let mut entries = IndexMap::new();
        entries.insert("base".to_string(), Value::Object(vec![
            ("host".to_string(), Value::String("localhost".to_string())),
        ].into_iter().collect()));
        entries.insert("config".to_string(), Value::Ref("base".to_string()));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let config = reader.get("config").unwrap();
        assert_eq!(config.as_ref_name(), Some("base"));
    }

    #[test]
    fn test_tl_to_binary_preserves_tagged() {
        use tempfile::NamedTempFile;

        let mut entries = IndexMap::new();
        entries.insert("status".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

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

        let mut entries = IndexMap::new();
        entries.insert("lookup".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
            (Value::Int(2), Value::String("two".to_string())),
        ]));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

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

        let mut entries = IndexMap::new();
        entries.insert("data".to_string(), Value::Bytes(vec![0xca, 0xfe, 0xf0, 0x0d]));
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let data = reader.get("data").unwrap();
        assert_eq!(data.as_bytes(), Some(vec![0xca, 0xfe, 0xf0, 0x0d].as_slice()));
    }

    #[test]
    fn test_tl_to_binary_preserves_timestamp() {
        use tempfile::NamedTempFile;

        let mut entries = IndexMap::new();
        entries.insert("created".to_string(), Value::Timestamp(1705315800000, 0)); // 2024-01-15T10:30:00Z
        let doc = TeaLeaf { data: entries, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).unwrap();

        let reader = Reader::open(temp.path()).unwrap();
        let created = reader.get("created").unwrap();
        assert_eq!(created.as_timestamp_millis(), Some(1705315800000));
    }

    #[test]
    fn test_json_import_limitation_hex_string_remains_string() {
        // Hex strings in JSON should remain strings, NOT become Bytes
        let json = r#"{"data":"0xcafef00d"}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let data = doc.get("data").unwrap();
        // This should be a String, not Bytes
        assert!(data.as_str().is_some(), "Hex string should remain String");
        assert_eq!(data.as_str(), Some("0xcafef00d"));
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
        let mut data = IndexMap::new();
        data.insert("null_val".to_string(), Value::Null);
        data.insert("bool_true".to_string(), Value::Bool(true));
        data.insert("int_val".to_string(), Value::Int(42));
        data.insert("float_val".to_string(), Value::Float(3.14159));
        data.insert("string_val".to_string(), Value::String("hello".to_string()));
        data.insert("bytes_val".to_string(), Value::Bytes(vec![0xca, 0xfe]));
        data.insert("timestamp_val".to_string(), Value::Timestamp(0, 0));
        data.insert("array_val".to_string(), Value::Array(vec![Value::Int(1), Value::Int(2)]));
        data.insert("object_val".to_string(), Value::Object(
            vec![("x".to_string(), Value::Int(1))].into_iter().collect()
        ));
        data.insert("ref_val".to_string(), Value::Ref("object_val".to_string()));
        data.insert("tagged_val".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
        data.insert("map_val".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
        ]));

        let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

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
        assert_eq!(reader.get("timestamp_val").unwrap().as_timestamp_millis(), Some(0));

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
            let mut data = IndexMap::new();
            data.insert("b".to_string(), Value::Bytes(vec![0xca, 0xfe, 0xba, 0xbe]));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Bytes serialize as lowercase hex with 0x prefix
            assert!(json.contains("\"0xcafebabe\""), "Bytes must be 0x-prefixed hex: {}", json);
        }

        #[test]
        fn contract_bytes_empty_to_json() {
            let mut data = IndexMap::new();
            data.insert("b".to_string(), Value::Bytes(vec![]));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Empty bytes serialize as "0x"
            assert!(json.contains("\"0x\""), "Empty bytes must be \"0x\": {}", json);
        }

        #[test]
        fn contract_timestamp_to_json_iso8601() {
            let mut data = IndexMap::new();
            // 2024-01-15T10:50:00.123Z (verified milliseconds since epoch)
            data.insert("ts".to_string(), Value::Timestamp(1705315800123, 0));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Timestamp serializes as ISO 8601 with milliseconds
            assert!(json.contains("2024-01-15T10:50:00.123Z"),
                "Timestamp must be ISO 8601 with ms: {}", json);
        }

        #[test]
        fn contract_timestamp_epoch_to_json() {
            let mut data = IndexMap::new();
            data.insert("ts".to_string(), Value::Timestamp(0, 0));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Unix epoch is 1970-01-01T00:00:00Z (no ms for whole seconds)
            assert!(json.contains("1970-01-01T00:00:00Z"),
                "Epoch must be 1970-01-01T00:00:00Z: {}", json);
        }

        #[test]
        fn contract_ref_to_json() {
            let mut data = IndexMap::new();
            data.insert("r".to_string(), Value::Ref("target_key".to_string()));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Ref serializes as {"$ref": "name"}
            assert!(json.contains("\"$ref\":\"target_key\"") || json.contains("\"$ref\": \"target_key\""),
                "Ref must be {{\"$ref\": \"name\"}}: {}", json);
        }

        #[test]
        fn contract_tagged_to_json() {
            let mut data = IndexMap::new();
            data.insert("t".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Tagged serializes with $tag and $value keys
            assert!(json.contains("\"$tag\""), "Tagged must have $tag: {}", json);
            assert!(json.contains("\"ok\""), "Tag name must be present: {}", json);
            assert!(json.contains("\"$value\""), "Tagged must have $value: {}", json);
            assert!(json.contains("200"), "Inner value must be present: {}", json);
        }

        #[test]
        fn contract_tagged_null_value_to_json() {
            let mut data = IndexMap::new();
            data.insert("t".to_string(), Value::Tagged("none".to_string(), Box::new(Value::Null)));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Tagged with null inner still has $value: null
            assert!(json.contains("\"$value\":null") || json.contains("\"$value\": null"),
                "Tagged with null must have $value:null: {}", json);
        }

        #[test]
        fn contract_map_to_json_pairs() {
            let mut data = IndexMap::new();
            data.insert("m".to_string(), Value::Map(vec![
                (Value::Int(1), Value::String("one".to_string())),
                (Value::Int(2), Value::String("two".to_string())),
            ]));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: Map serializes as array of [key, value] pairs
            assert!(json.contains("[[1,\"one\"],[2,\"two\"]]") ||
                    json.contains("[[1, \"one\"], [2, \"two\"]]"),
                "Map must be [[k,v],...]: {}", json);
        }

        #[test]
        fn contract_map_empty_to_json() {
            let mut data = IndexMap::new();
            data.insert("m".to_string(), Value::Map(vec![]));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

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
            let doc = TeaLeaf::from_json(r#"{"x": "0xcafef00d"}"#).unwrap();
            let x = doc.get("x").unwrap();
            // CONTRACT: Hex strings MUST remain String, NOT become Bytes
            assert_eq!(x.as_str(), Some("0xcafef00d"));
            assert!(x.as_bytes().is_none(), "Hex string must not auto-convert to Bytes");
        }

        #[test]
        fn contract_json_iso_timestamp_stays_string() {
            let doc = TeaLeaf::from_json(r#"{"x": "2024-01-15T10:30:00.000Z"}"#).unwrap();
            let x = doc.get("x").unwrap();
            // CONTRACT: ISO 8601 strings MUST remain String, NOT become Timestamp
            assert_eq!(x.as_str(), Some("2024-01-15T10:30:00.000Z"));
            assert!(x.as_timestamp_millis().is_none(), "ISO string must not auto-convert to Timestamp");
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
            let mut data = IndexMap::new();
            data.insert("f".to_string(), Value::Float(f64::NAN));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

            let json = doc.to_json_compact().unwrap();
            // CONTRACT: NaN serializes as null (JSON has no NaN)
            assert!(json.contains("null"), "NaN must serialize as null: {}", json);
        }

        #[test]
        fn contract_float_infinity_to_null() {
            let mut data = IndexMap::new();
            data.insert("f".to_string(), Value::Float(f64::INFINITY));
            let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

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

        // Fields should preserve insertion order from JSON
        assert_eq!(schema.fields[0].name, "name");
        assert_eq!(schema.fields[1].name, "age");

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
        // Dots are valid in names per spec grammar, so a.b.c should NOT be quoted
        assert!(!tl_text.contains("\"a.b.c\""), "Dots should NOT be quoted per spec grammar: {}", tl_text);

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

    // =========================================================================
    // Coverage: dumps(), write_value(), escape_string(), format_float()
    // =========================================================================

    #[test]
    fn test_dumps_all_value_types() {
        let mut data = IndexMap::new();
        data.insert("null_val".to_string(), Value::Null);
        data.insert("bool_val".to_string(), Value::Bool(true));
        data.insert("int_val".to_string(), Value::Int(42));
        data.insert("uint_val".to_string(), Value::UInt(999));
        data.insert("float_val".to_string(), Value::Float(3.14));
        data.insert("str_val".to_string(), Value::String("hello".to_string()));
        data.insert("bytes_val".to_string(), Value::Bytes(vec![0xca, 0xfe]));
        data.insert("arr_val".to_string(), Value::Array(vec![Value::Int(1), Value::Int(2)]));
        data.insert("obj_val".to_string(), Value::Object(
            vec![("x".to_string(), Value::Int(1))].into_iter().collect()
        ));
        data.insert("map_val".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
        ]));
        data.insert("ref_val".to_string(), Value::Ref("target".to_string()));
        data.insert("tag_val".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
        data.insert("ts_val".to_string(), Value::Timestamp(0, 0));
        data.insert("ts_millis".to_string(), Value::Timestamp(1705315800123, 0));

        let output = dumps(&data);

        assert!(output.contains("~"), "Should contain null");
        assert!(output.contains("true"), "Should contain bool");
        assert!(output.contains("42"), "Should contain int");
        assert!(output.contains("999"), "Should contain uint");
        assert!(output.contains("3.14"), "Should contain float");
        assert!(output.contains("hello"), "Should contain string");
        assert!(output.contains("b\"cafe\""), "Should contain bytes literal");
        assert!(output.contains("[1, 2]"), "Should contain array");
        assert!(output.contains("@map {"), "Should contain map");
        assert!(output.contains("!target"), "Should contain ref");
        assert!(output.contains(":ok 200"), "Should contain tagged");
        assert!(output.contains("1970-01-01T00:00:00Z"), "Should contain epoch timestamp");
        assert!(output.contains(".123Z"), "Should contain millis timestamp");
    }

    #[test]
    fn test_bytes_literal_text_roundtrip() {
        // dumps() emits b"..." → parse() reads it back as Value::Bytes
        let mut data = IndexMap::new();
        data.insert("payload".to_string(), Value::Bytes(vec![0xca, 0xfe, 0xf0, 0x0d]));
        data.insert("empty".to_string(), Value::Bytes(vec![]));

        let text = dumps(&data);
        assert!(text.contains(r#"b"cafef00d""#), "Should emit b\"...\" literal: {}", text);
        assert!(text.contains(r#"b"""#), "Should emit empty bytes literal: {}", text);

        // Parse the text back
        let doc = TeaLeaf::parse(&text).unwrap();
        assert_eq!(doc.data.get("payload").unwrap().as_bytes(), Some(&[0xca, 0xfe, 0xf0, 0x0d][..]));
        assert_eq!(doc.data.get("empty").unwrap().as_bytes(), Some(&[][..]));
    }

    #[test]
    fn test_dumps_string_quoting() {
        let mut data = IndexMap::new();
        data.insert("quoted".to_string(), Value::String("hello world".to_string()));
        data.insert("unquoted".to_string(), Value::String("hello".to_string()));
        data.insert("reserved_true".to_string(), Value::String("true".to_string()));
        data.insert("reserved_null".to_string(), Value::String("null".to_string()));
        data.insert("reserved_tilde".to_string(), Value::String("~".to_string()));
        data.insert("empty".to_string(), Value::String("".to_string()));
        data.insert("at_start".to_string(), Value::String("@directive".to_string()));
        data.insert("hash_start".to_string(), Value::String("#comment".to_string()));
        data.insert("bang_start".to_string(), Value::String("!ref".to_string()));
        data.insert("hex_start".to_string(), Value::String("0xabc".to_string()));
        data.insert("number_like".to_string(), Value::String("42abc".to_string()));
        data.insert("negative_like".to_string(), Value::String("-5".to_string()));
        data.insert("slash".to_string(), Value::String("a/b".to_string()));
        data.insert("dot".to_string(), Value::String("a.b".to_string()));

        let output = dumps(&data);

        // Quoted values should be wrapped in double quotes
        assert!(output.contains("\"hello world\""), "Spaces need quoting");
        assert!(output.contains("\"true\""), "Reserved word true needs quoting");
        assert!(output.contains("\"null\""), "Reserved word null needs quoting");
        assert!(output.contains("\"~\""), "Tilde needs quoting");
        assert!(output.contains("\"\""), "Empty string needs quoting");
        assert!(output.contains("\"@directive\""), "@ prefix needs quoting");
        assert!(output.contains("\"#comment\""), "# prefix needs quoting");
        assert!(output.contains("\"!ref\""), "! prefix needs quoting");
        assert!(output.contains("\"0xabc\""), "0x prefix needs quoting");
        assert!(output.contains("\"42abc\""), "Digit start needs quoting");
        assert!(output.contains("\"-5\""), "Negative number needs quoting");
        assert!(output.contains("\"a/b\""), "Slash needs quoting");
        assert!(!output.contains("\"a.b\""), "Dot should NOT need quoting per spec grammar");
    }

    #[test]
    fn test_escape_string_control_chars() {
        let result = escape_string("tab\there\nnewline\rreturn");
        assert!(result.contains("\\t"), "Tab should be escaped");
        assert!(result.contains("\\n"), "Newline should be escaped");
        assert!(result.contains("\\r"), "CR should be escaped");

        let result = escape_string("\x08backspace\x0cformfeed");
        assert!(result.contains("\\b"), "Backspace should be escaped");
        assert!(result.contains("\\f"), "Formfeed should be escaped");

        let result = escape_string("quote\"and\\backslash");
        assert!(result.contains("\\\""), "Quote should be escaped");
        assert!(result.contains("\\\\"), "Backslash should be escaped");

        // Other control characters use \uXXXX
        let result = escape_string("\x01");
        assert!(result.contains("\\u0001"), "Control char should use \\uXXXX");
    }

    #[test]
    fn test_format_float_both_branches() {
        // Whole number float: Rust's to_string() drops .0, so format_float adds it back
        assert_eq!(format_float(42.0, false), "42.0");

        // Float with decimals should stay as-is
        assert_eq!(format_float(3.14, false), "3.14");

        // Scientific notation stays as-is
        let very_small = format_float(1e-20, false);
        assert!(very_small.contains('e') || very_small.contains('.'));
    }

    #[test]
    fn test_format_float_compact_floats() {
        // With compact_floats=true, whole-number floats strip .0
        assert_eq!(format_float(42.0, true), "42");
        assert_eq!(format_float(0.0, true), "0");
        assert_eq!(format_float(17164000000.0, true), "17164000000");
        assert_eq!(format_float(35934000000.0, true), "35934000000");
        assert_eq!(format_float(-100.0, true), "-100");

        // Non-whole floats are unaffected
        assert_eq!(format_float(3.14, true), "3.14");
        assert_eq!(format_float(0.5, true), "0.5");

        // Special values unaffected
        assert_eq!(format_float(f64::NAN, true), "NaN");
        assert_eq!(format_float(f64::INFINITY, true), "inf");
        assert_eq!(format_float(f64::NEG_INFINITY, true), "-inf");

        // Very large floats use scientific notation (digits > 15), unaffected
        let large = format_float(1e20, true);
        assert!(large.contains('e'), "Very large should use scientific: {}", large);
    }

    #[test]
    fn test_dumps_with_compact_floats() {
        let mut data = IndexMap::new();
        data.insert("revenue".to_string(), Value::Float(35934000000.0));
        data.insert("ratio".to_string(), Value::Float(3.14));
        data.insert("count".to_string(), Value::Int(42));

        // Default: whole floats keep .0
        let pretty = dumps(&data);
        assert!(pretty.contains("35934000000.0"), "Default should have .0: {}", pretty);

        // compact_floats: whole floats stripped
        let opts = FormatOptions::compact().with_compact_floats();
        let compact = dumps_with_options(&data, &opts);
        assert!(compact.contains("35934000000"), "Should have whole number: {}", compact);
        assert!(!compact.contains("35934000000.0"), "Should NOT have .0: {}", compact);
        assert!(compact.contains("3.14"), "Non-whole float preserved: {}", compact);
        assert!(compact.contains("42"), "Int preserved: {}", compact);
    }

    #[test]
    fn test_needs_quoting_various_patterns() {
        // Should need quoting
        assert!(needs_quoting(""), "Empty string");
        assert!(needs_quoting("hello world"), "Whitespace");
        assert!(needs_quoting("a,b"), "Comma");
        assert!(needs_quoting("(x)"), "Parens");
        assert!(needs_quoting("[x]"), "Brackets");
        assert!(needs_quoting("{x}"), "Braces");
        assert!(needs_quoting("a:b"), "Colon");
        assert!(needs_quoting("@x"), "At sign");
        assert!(needs_quoting("a/b"), "Slash");
        assert!(!needs_quoting("a.b"), "Dot is valid in names per spec grammar");
        assert!(needs_quoting("true"), "Reserved true");
        assert!(needs_quoting("false"), "Reserved false");
        assert!(needs_quoting("null"), "Reserved null");
        assert!(needs_quoting("~"), "Reserved tilde");
        assert!(needs_quoting("!bang"), "Bang prefix");
        assert!(needs_quoting("#hash"), "Hash prefix");
        assert!(needs_quoting("0xdead"), "Hex prefix");
        assert!(needs_quoting("0Xdead"), "Hex prefix uppercase");
        assert!(needs_quoting("42abc"), "Starts with digit");
        assert!(needs_quoting("-5"), "Starts with minus+digit");
        assert!(needs_quoting("+5"), "Starts with plus+digit");

        // Should NOT need quoting
        assert!(!needs_quoting("hello"), "Simple word");
        assert!(!needs_quoting("foo_bar"), "Underscore word");
        assert!(!needs_quoting("abc123"), "Alpha then digits");
    }

    // =========================================================================
    // Coverage: singularize()
    // =========================================================================

    #[test]
    fn test_singularize_rules() {
        // -ies → -y
        assert_eq!(singularize("categories"), "category");
        assert_eq!(singularize("entries"), "entry");

        // -sses → -ss (special -es rule)
        assert_eq!(singularize("classes"), "class");
        assert_eq!(singularize("dresses"), "dress");

        // -xes → -x
        assert_eq!(singularize("boxes"), "box");
        assert_eq!(singularize("indexes"), "index");

        // -ches → -ch
        assert_eq!(singularize("watches"), "watch");

        // -shes → -sh
        assert_eq!(singularize("dishes"), "dish");

        // Regular -s
        assert_eq!(singularize("users"), "user");
        assert_eq!(singularize("products"), "product");

        // Words ending in -ss (should NOT remove s)
        assert_eq!(singularize("boss"), "boss");
        assert_eq!(singularize("class"), "class");

        // Already singular (no trailing s)
        assert_eq!(singularize("item"), "item");
        assert_eq!(singularize("child"), "child");
    }

    // =========================================================================
    // Coverage: from_json root primitives, loads()
    // =========================================================================

    #[test]
    fn test_from_json_root_primitive() {
        // Root-level string
        let doc = TeaLeaf::from_json(r#""hello""#).unwrap();
        assert_eq!(doc.get("root").unwrap().as_str(), Some("hello"));
        assert!(!doc.is_root_array);

        // Root-level number
        let doc = TeaLeaf::from_json("42").unwrap();
        assert_eq!(doc.get("root").unwrap().as_int(), Some(42));

        // Root-level bool
        let doc = TeaLeaf::from_json("true").unwrap();
        assert_eq!(doc.get("root").unwrap().as_bool(), Some(true));

        // Root-level null
        let doc = TeaLeaf::from_json("null").unwrap();
        assert!(doc.get("root").unwrap().is_null());
    }

    #[test]
    fn test_from_json_invalid() {
        let result = TeaLeaf::from_json("not valid json {{{");
        assert!(result.is_err());
    }

    #[test]
    fn test_loads_convenience() {
        let data = loads("name: alice\nage: 30").unwrap();
        assert_eq!(data.get("name").unwrap().as_str(), Some("alice"));
        assert_eq!(data.get("age").unwrap().as_int(), Some(30));
    }

    // =========================================================================
    // Coverage: InferredType::merge() branches
    // =========================================================================

    #[test]
    fn test_inferred_type_merge_int_float() {
        let t = infer_type(&Value::Int(42));
        let f = infer_type(&Value::Float(3.14));
        let merged = t.merge(&f);
        assert_eq!(merged, InferredType::Float);

        // Reverse
        let merged = f.merge(&t);
        assert_eq!(merged, InferredType::Float);
    }

    #[test]
    fn test_inferred_type_merge_null_with_type() {
        let n = InferredType::Null;
        let s = InferredType::String;
        let merged = n.merge(&s);
        assert_eq!(merged, InferredType::String);

        // Reverse
        let merged = s.merge(&n);
        assert_eq!(merged, InferredType::String);
    }

    #[test]
    fn test_inferred_type_merge_arrays() {
        let a1 = InferredType::Array(Box::new(InferredType::Int));
        let a2 = InferredType::Array(Box::new(InferredType::Float));
        let merged = a1.merge(&a2);
        assert_eq!(merged, InferredType::Array(Box::new(InferredType::Float)));
    }

    #[test]
    fn test_inferred_type_merge_objects_same_fields() {
        let o1 = InferredType::Object(vec![
            ("a".to_string(), InferredType::Int),
            ("b".to_string(), InferredType::String),
        ]);
        let o2 = InferredType::Object(vec![
            ("a".to_string(), InferredType::Float),
            ("b".to_string(), InferredType::String),
        ]);
        let merged = o1.merge(&o2);
        if let InferredType::Object(fields) = &merged {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].1, InferredType::Float); // Int+Float → Float
            assert_eq!(fields[1].1, InferredType::String);
        } else {
            panic!("Expected Object, got {:?}", merged);
        }
    }

    #[test]
    fn test_inferred_type_merge_objects_different_fields() {
        let o1 = InferredType::Object(vec![
            ("a".to_string(), InferredType::Int),
        ]);
        let o2 = InferredType::Object(vec![
            ("b".to_string(), InferredType::String),
        ]);
        let merged = o1.merge(&o2);
        assert_eq!(merged, InferredType::Mixed);
    }

    #[test]
    fn test_inferred_type_merge_incompatible() {
        let s = InferredType::String;
        let i = InferredType::Int;
        let merged = s.merge(&i);
        assert_eq!(merged, InferredType::Mixed);
    }

    #[test]
    fn test_inferred_type_to_field_type() {
        let schemas = IndexMap::new();

        assert_eq!(InferredType::Null.to_field_type(&schemas).base, "string");
        assert!(InferredType::Null.to_field_type(&schemas).nullable);
        assert_eq!(InferredType::Bool.to_field_type(&schemas).base, "bool");
        assert_eq!(InferredType::Int.to_field_type(&schemas).base, "int");
        assert_eq!(InferredType::Float.to_field_type(&schemas).base, "float");
        assert_eq!(InferredType::String.to_field_type(&schemas).base, "string");
        assert_eq!(InferredType::Mixed.to_field_type(&schemas).base, "any");

        // Array type
        let arr_type = InferredType::Array(Box::new(InferredType::Int));
        let ft = arr_type.to_field_type(&schemas);
        assert_eq!(ft.base, "int");
        assert!(ft.is_array);

        // Object with no matching schema → "any" (not "object", which is a value-only type)
        let obj_type = InferredType::Object(vec![("x".to_string(), InferredType::Int)]);
        assert_eq!(obj_type.to_field_type(&schemas).base, "any");
    }

    #[test]
    fn test_inferred_type_to_field_type_with_matching_schema() {
        let mut schemas = IndexMap::new();
        let mut schema = Schema::new("point");
        schema.add_field("x", FieldType::new("int"));
        schema.add_field("y", FieldType::new("int"));
        schemas.insert("point".to_string(), schema);

        let obj_type = InferredType::Object(vec![
            ("x".to_string(), InferredType::Int),
            ("y".to_string(), InferredType::Int),
        ]);
        let ft = obj_type.to_field_type(&schemas);
        assert_eq!(ft.base, "point");
    }

    #[test]
    fn test_infer_type_special_values() {
        // Bytes, Ref, Tagged, Timestamp, Map all become Mixed
        assert_eq!(infer_type(&Value::Bytes(vec![1, 2])), InferredType::Mixed);
        assert_eq!(infer_type(&Value::Ref("x".to_string())), InferredType::Mixed);
        assert_eq!(infer_type(&Value::Tagged("t".to_string(), Box::new(Value::Null))), InferredType::Mixed);
        assert_eq!(infer_type(&Value::Timestamp(0, 0)), InferredType::Mixed);
        assert_eq!(infer_type(&Value::Map(vec![])), InferredType::Mixed);

        // Empty array
        if let InferredType::Array(inner) = infer_type(&Value::Array(vec![])) {
            assert_eq!(*inner, InferredType::Mixed);
        } else {
            panic!("Expected Array");
        }

        // UInt becomes Int
        assert_eq!(infer_type(&Value::UInt(42)), InferredType::Int);
    }

    #[test]
    fn test_json_with_schemas_empty_nested_object_roundtrip() {
        // Regression: fuzzer found that [{"n":{}}] crashes because the inferrer
        // emits "object" as a field type, which the parser rejects as value-only.
        let doc = TeaLeaf::from_json_with_schemas(r#"[{"n":{}}]"#).unwrap();
        let tl_text = doc.to_tl_with_schemas();
        // Must re-parse without error
        let reparsed = TeaLeaf::parse(&tl_text).unwrap();
        assert_eq!(doc.data.len(), reparsed.data.len());
    }

    // =========================================================================
    // Coverage: to_tl_with_schemas() edge cases
    // =========================================================================

    #[test]
    fn test_to_tl_with_schemas_no_schemas() {
        let mut data = IndexMap::new();
        data.insert("name".to_string(), Value::String("alice".to_string()));
        let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: false };

        let output = doc.to_tl_with_schemas();
        assert!(output.contains("name: alice"), "Should use dumps() format");
        assert!(!output.contains("@struct"), "No schemas");
    }

    #[test]
    fn test_to_tl_with_schemas_root_array() {
        let mut data = IndexMap::new();
        data.insert("root".to_string(), Value::Array(vec![Value::Int(1), Value::Int(2)]));
        let doc = TeaLeaf { data, schemas: IndexMap::new(), unions: IndexMap::new(), is_root_array: true };

        let output = doc.to_tl_with_schemas();
        assert!(output.starts_with("@root-array"), "Should have root-array directive");
    }

    // =========================================================================
    // Coverage: write_value_with_schemas() for special types
    // =========================================================================

    #[test]
    fn test_dumps_with_schemas_all_types() {
        let mut schemas = IndexMap::new();
        let mut schema = Schema::new("item");
        schema.add_field("id", FieldType::new("int"));
        schema.add_field("name", FieldType::new("string"));
        schemas.insert("item".to_string(), schema);

        let mut data = IndexMap::new();
        // Array matching schema → @table
        data.insert("items".to_string(), Value::Array(vec![
            Value::Object(vec![
                ("id".to_string(), Value::Int(1)),
                ("name".to_string(), Value::String("Widget".to_string())),
            ].into_iter().collect()),
        ]));
        // Special types
        data.insert("ref_val".to_string(), Value::Ref("target".to_string()));
        data.insert("tag_val".to_string(), Value::Tagged("ok".to_string(), Box::new(Value::Int(200))));
        data.insert("map_val".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
        ]));
        data.insert("bytes_val".to_string(), Value::Bytes(vec![0xde, 0xad]));
        data.insert("ts_val".to_string(), Value::Timestamp(0, 0));
        data.insert("ts_millis".to_string(), Value::Timestamp(1705315800123, 0));

        let schema_order = vec!["item".to_string()];
        let output = dumps_with_schemas(&data, &schemas, &schema_order, &IndexMap::new(), &[]);

        assert!(output.contains("@struct item"), "Should contain schema def");
        assert!(output.contains("@table item"), "Should use @table format");
        assert!(output.contains("!target"), "Should contain ref");
        assert!(output.contains(":ok 200"), "Should contain tagged");
        assert!(output.contains("@map {"), "Should contain map");
        assert!(output.contains("b\"dead\""), "Should contain bytes literal");
        assert!(output.contains("1970-01-01T00:00:00Z"), "Should contain timestamp");
        assert!(output.contains(".123Z"), "Should contain millis timestamp");
    }

    #[test]
    fn test_dumps_with_schemas_object_value() {
        let schemas = IndexMap::new();
        let mut data = IndexMap::new();
        data.insert("config".to_string(), Value::Object(
            vec![
                ("host".to_string(), Value::String("localhost".to_string())),
                ("port".to_string(), Value::Int(8080)),
            ].into_iter().collect()
        ));

        let output = dumps_with_schemas(&data, &schemas, &[], &IndexMap::new(), &[]);
        assert!(output.contains("config:"), "Should contain key");
        assert!(output.contains("{"), "Should contain object");
    }

    #[test]
    fn test_write_tuple_with_nested_schema() {
        // Test tuple writing with nested struct fields
        let mut schemas = IndexMap::new();

        let mut addr = Schema::new("address");
        addr.add_field("city", FieldType::new("string"));
        addr.add_field("zip", FieldType::new("string"));
        schemas.insert("address".to_string(), addr);

        let mut user = Schema::new("user");
        user.add_field("name", FieldType::new("string"));
        user.add_field("home", FieldType::new("address"));
        schemas.insert("user".to_string(), user);

        let mut data = IndexMap::new();
        data.insert("users".to_string(), Value::Array(vec![
            Value::Object(vec![
                ("name".to_string(), Value::String("Alice".to_string())),
                ("home".to_string(), Value::Object(vec![
                    ("city".to_string(), Value::String("Boston".to_string())),
                    ("zip".to_string(), Value::String("02101".to_string())),
                ].into_iter().collect())),
            ].into_iter().collect()),
        ]));

        let schema_order = vec!["address".to_string(), "user".to_string()];
        let output = dumps_with_schemas(&data, &schemas, &schema_order, &IndexMap::new(), &[]);

        assert!(output.contains("@struct address"), "Should have address schema");
        assert!(output.contains("@struct user"), "Should have user schema");
        assert!(output.contains("@table user"), "Should use @table for users");
        // Nested tuples
        assert!(output.contains("("), "Should have tuple format");
    }

    #[test]
    fn test_write_tuple_with_schema_array_field() {
        // Test tuple writing with array fields that have schemas
        let mut schemas = IndexMap::new();

        let mut tag = Schema::new("tag");
        tag.add_field("name", FieldType::new("string"));
        schemas.insert("tag".to_string(), tag);

        let mut item = Schema::new("item");
        item.add_field("id", FieldType::new("int"));
        item.add_field("tags", FieldType { base: "tag".to_string(), nullable: false, is_array: true });
        schemas.insert("item".to_string(), item);

        let mut data = IndexMap::new();
        data.insert("items".to_string(), Value::Array(vec![
            Value::Object(vec![
                ("id".to_string(), Value::Int(1)),
                ("tags".to_string(), Value::Array(vec![
                    Value::Object(vec![
                        ("name".to_string(), Value::String("rust".to_string())),
                    ].into_iter().collect()),
                ])),
            ].into_iter().collect()),
        ]));

        let schema_order = vec!["tag".to_string(), "item".to_string()];
        let output = dumps_with_schemas(&data, &schemas, &schema_order, &IndexMap::new(), &[]);

        assert!(output.contains("@table item"), "Should use @table for items");
    }

    #[test]
    fn test_write_schema_array_empty() {
        let schemas = IndexMap::new();
        let schema = Schema::new("empty");
        let mut out = String::new();
        write_schema_array(&mut out, &Value::Array(vec![]), &schema, &schemas, 0, &FormatOptions::default());
        assert_eq!(out, "[]");
    }

    #[test]
    fn test_write_schema_array_non_array_fallback() {
        let schemas = IndexMap::new();
        let schema = Schema::new("test");
        let mut out = String::new();
        write_schema_array(&mut out, &Value::Int(42), &schema, &schemas, 0, &FormatOptions::default());
        assert_eq!(out, "42");
    }

    #[test]
    fn test_write_tuple_missing_field() {
        // Test that missing fields in object produce ~
        let schemas = IndexMap::new();
        let mut schema = Schema::new("test");
        schema.add_field("present", FieldType::new("int"));
        schema.add_field("missing", FieldType::new("string"));

        let value = Value::Object(
            vec![("present".to_string(), Value::Int(42))].into_iter().collect()
        );

        let mut out = String::new();
        write_tuple(&mut out, &value, &schema, &schemas, 0, &FormatOptions::default());
        assert!(out.contains("42"), "Present field should be written");
        assert!(out.contains("~"), "Missing field should be ~");
    }

    #[test]
    fn test_write_tuple_non_object() {
        // When tuple receives a non-object value
        let schemas = IndexMap::new();
        let schema = Schema::new("test");

        let mut out = String::new();
        write_tuple(&mut out, &Value::Int(42), &schema, &schemas, 0, &FormatOptions::default());
        assert_eq!(out, "42");
    }

    // =========================================================================
    // Coverage: array_matches_schema()
    // =========================================================================

    #[test]
    fn test_array_matches_schema_empty() {
        let schema = Schema::new("test");
        assert!(!array_matches_schema(&[], &schema));
    }

    #[test]
    fn test_array_matches_schema_non_object() {
        let schema = Schema::new("test");
        assert!(!array_matches_schema(&[Value::Int(1)], &schema));
    }

    #[test]
    fn test_array_matches_schema_matching() {
        let mut schema = Schema::new("user");
        schema.add_field("name", FieldType::new("string"));
        schema.add_field("age", FieldType::new("int"));

        let arr = vec![Value::Object(vec![
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Int(30)),
        ].into_iter().collect())];

        assert!(array_matches_schema(&arr, &schema));
    }

    // =========================================================================
    // Coverage: from_dto, from_dto_array, to_dto, to_dto_vec
    // =========================================================================

    #[test]
    fn test_from_dto_and_back() {
        use crate::convert::{FromTeaLeaf, ConvertError};

        let doc = TeaLeaf::from_dto("greeting", &"hello".to_string());
        assert_eq!(doc.get("greeting").unwrap().as_str(), Some("hello"));

        let result: std::result::Result<String, ConvertError> = String::from_tealeaf_value(doc.get("greeting").unwrap());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_from_dto_array() {
        let items = vec!["apple".to_string(), "banana".to_string()];
        let doc = TeaLeaf::from_dto_array("fruits", &items);
        let arr = doc.get("fruits").unwrap().as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_str(), Some("apple"));
    }

    #[test]
    fn test_to_dto_missing_key() {
        let doc = TeaLeaf::new(IndexMap::new(), IndexMap::new());
        let result: Result<String> = doc.to_dto("missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_dto_vec() {
        let mut data = IndexMap::new();
        data.insert("items".to_string(), Value::Array(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ]));
        let doc = TeaLeaf::new(IndexMap::new(), data);
        let result: Vec<String> = doc.to_dto_vec("items").unwrap();
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_to_dto_vec_not_array() {
        let mut data = IndexMap::new();
        data.insert("item".to_string(), Value::String("not_an_array".to_string()));
        let doc = TeaLeaf::new(IndexMap::new(), data);
        let result: Result<Vec<String>> = doc.to_dto_vec("item");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_dto_vec_missing_key() {
        let doc = TeaLeaf::new(IndexMap::new(), IndexMap::new());
        let result: Result<Vec<String>> = doc.to_dto_vec("missing");
        assert!(result.is_err());
    }

    // =========================================================================
    // Coverage: set_root_array, SchemaInferrer edge cases
    // =========================================================================

    #[test]
    fn test_set_root_array() {
        let mut doc = TeaLeaf::new(IndexMap::new(), IndexMap::new());
        assert!(!doc.is_root_array);
        doc.set_root_array(true);
        assert!(doc.is_root_array);
    }

    #[test]
    fn test_schema_inferrer_non_uniform_array() {
        // Array with different object structures should not create a schema
        let json = r#"{"items": [{"a": 1}, {"b": 2}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        assert!(doc.schema("item").is_none(), "Non-uniform array should not produce schema");
    }

    #[test]
    fn test_schema_inferrer_mixed_types_in_array() {
        // Array with non-objects
        let json = r#"{"items": [1, 2, 3]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        assert!(doc.schema("item").is_none(), "Non-object array should not produce schema");
    }

    #[test]
    fn test_schema_inferrer_empty_array() {
        let json = r#"{"items": []}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        assert!(doc.schema("item").is_none(), "Empty array should not produce schema");
    }

    #[test]
    fn test_schema_inferrer_duplicate_schema_name() {
        // Two arrays that would produce the same schema name
        let json = r#"{
            "items": [{"id": 1, "name": "A"}],
            "nested": {"items": [{"id": 2, "name": "B"}]}
        }"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        // Should have "item" schema (first one wins)
        assert!(doc.schema("item").is_some());
    }

    #[test]
    fn test_schema_inferrer_int_float_merge() {
        // Field that has int in one record and float in another
        let json = r#"{"values": [{"x": 1}, {"x": 2.5}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let schema = doc.schema("value").unwrap();
        let x_field = schema.fields.iter().find(|f| f.name == "x").unwrap();
        assert_eq!(x_field.field_type.base, "float", "Int+Float merge should produce float");
    }

    #[test]
    fn test_schema_inference_with_root_array() {
        let json = r#"[{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        // Root array is stored under "root" key - the schema name should be derived from "root"
        // The singularize of "root" is "root" (no trailing s)
        // Actually, root arrays aren't typically analyzed because the key is "root" and it goes through analyze_value
        let root_val = doc.get("root").unwrap().as_array().unwrap();
        assert_eq!(root_val.len(), 2);
    }

    // =========================================================================
    // Coverage: dumps_with_schemas with quoting in schemas
    // =========================================================================

    #[test]
    fn test_dumps_with_schemas_string_quoting_in_tuples() {
        let mut schemas = IndexMap::new();
        let mut schema = Schema::new("item");
        schema.add_field("name", FieldType::new("string"));
        schemas.insert("item".to_string(), schema);

        let mut data = IndexMap::new();
        data.insert("items".to_string(), Value::Array(vec![
            Value::Object(vec![
                ("name".to_string(), Value::String("hello world".to_string())),
            ].into_iter().collect()),
        ]));

        let schema_order = vec!["item".to_string()];
        let output = dumps_with_schemas(&data, &schemas, &schema_order, &IndexMap::new(), &[]);
        assert!(output.contains("\"hello world\""), "String with space should be quoted in tuple");
    }

    #[test]
    fn test_dumps_with_schemas_array_without_schema() {
        // Array that doesn't match any schema
        let schemas = IndexMap::new();
        let mut data = IndexMap::new();
        data.insert("nums".to_string(), Value::Array(vec![Value::Int(1), Value::Int(2)]));

        let output = dumps_with_schemas(&data, &schemas, &[], &IndexMap::new(), &[]);
        assert!(output.contains("[1, 2]"), "Should use regular array format");
    }

    // =========================================================================
    // Coverage: convenience functions open(), parse(), root array to_json
    // =========================================================================

    #[test]
    fn test_open_convenience_function() {
        // Write a binary file first, then open with the convenience function
        let dir = std::env::temp_dir();
        let path = dir.join("test_open_conv.tlbx");

        let mut data = IndexMap::new();
        data.insert("x".to_string(), Value::Int(42));
        let doc = TeaLeaf::new(IndexMap::new(), data);
        doc.compile(&path, false).unwrap();

        let reader = super::open(&path).unwrap();
        assert_eq!(reader.get("x").unwrap().as_int(), Some(42));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_convenience_function() {
        let doc = super::parse("greeting: hello").unwrap();
        assert_eq!(doc.get("greeting").unwrap().as_str(), Some("hello"));
    }

    #[test]
    fn test_to_json_root_array() {
        let mut data = IndexMap::new();
        data.insert("root".to_string(), Value::Array(vec![Value::Int(1), Value::Int(2)]));
        let mut doc = TeaLeaf::new(IndexMap::new(), data);
        doc.set_root_array(true);

        let json = doc.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array(), "Root array to_json should output array");
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_to_json_compact_root_array() {
        let mut data = IndexMap::new();
        data.insert("root".to_string(), Value::Array(vec![Value::Int(1)]));
        let mut doc = TeaLeaf::new(IndexMap::new(), data);
        doc.set_root_array(true);

        let json = doc.to_json_compact().unwrap();
        assert_eq!(json, "[1]");
    }

    #[test]
    fn test_infer_type_bool_value() {
        let it = infer_type(&Value::Bool(true));
        assert!(matches!(it, InferredType::Bool));
    }

    #[test]
    fn test_schema_inference_nested_object_fields() {
        // JSON with nested objects inside array items
        let json = r#"{"records": [
            {"id": 1, "details": {"city": "NYC", "zip": "10001"}},
            {"id": 2, "details": {"city": "LA", "zip": "90001"}}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        // Should infer both "record" and "detail" schemas
        assert!(doc.schema("record").is_some(), "Should infer record schema");
    }

    #[test]
    fn test_schema_inference_not_all_objects_returns_early() {
        // Array where second element is not an object
        let json = r#"{"items": [{"a": 1}, "not_an_object"]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        assert!(doc.schema("item").is_none(), "Mixed array should not produce schema");
    }

    #[test]
    fn test_to_tl_with_schemas_with_nested_array_field() {
        // Schema with an array-typed field
        let mut schemas = IndexMap::new();
        let mut schema = Schema::new("user");
        schema.add_field("name", FieldType::new("string"));
        schema.add_field("tags", FieldType::new("string").array());
        schemas.insert("user".to_string(), schema);

        let mut data = IndexMap::new();
        let mut obj = IndexMap::new();
        obj.insert("name".to_string(), Value::String("Alice".into()));
        obj.insert("tags".to_string(), Value::Array(vec![
            Value::String("admin".into()),
            Value::String("active".into()),
        ]));
        data.insert("users".to_string(), Value::Array(vec![Value::Object(obj)]));

        let doc = TeaLeaf::new(schemas, data);
        let text = doc.to_tl_with_schemas();
        assert!(text.contains("@struct user"), "Should have schema definition");
        assert!(text.contains("@table user"), "Should use table format");
    }

    // =========================================================================
    // Issue 6: Improved schema matching
    // =========================================================================

    #[test]
    fn test_schema_matching_nullable_fields_allowed_missing() {
        // Schema with nullable field should match objects missing that field
        let mut schemas = IndexMap::new();
        let mut s = Schema::new("Item");
        s.add_field("id", FieldType::new("int"));
        s.add_field("label", FieldType::new("string").nullable());
        schemas.insert("Item".to_string(), s);

        let mut obj1 = IndexMap::new();
        obj1.insert("id".to_string(), Value::Int(1));
        // label is missing — but it's nullable, so it should still match

        let doc = TeaLeaf {
            schemas,
            unions: IndexMap::new(),
            data: {
                let mut d = IndexMap::new();
                d.insert("items".to_string(), Value::Array(vec![Value::Object(obj1)]));
                d
            },
            is_root_array: false,
        };
        let result = doc.find_schema_for_value(doc.data.get("items").unwrap(), "items");
        assert!(result.is_some(), "Should match schema when nullable field is missing");
        assert_eq!(result.unwrap().name, "Item");
    }

    #[test]
    fn test_schema_matching_rejects_extra_keys() {
        // Objects with extra keys not in schema should not match
        let mut schemas = IndexMap::new();
        let mut s = Schema::new("Point");
        s.add_field("x", FieldType::new("int"));
        s.add_field("y", FieldType::new("int"));
        schemas.insert("Point".to_string(), s);

        let mut obj = IndexMap::new();
        obj.insert("x".to_string(), Value::Int(1));
        obj.insert("y".to_string(), Value::Int(2));
        obj.insert("z".to_string(), Value::Int(3)); // extra field

        let doc = TeaLeaf {
            schemas,
            unions: IndexMap::new(),
            data: {
                let mut d = IndexMap::new();
                d.insert("points".to_string(), Value::Array(vec![Value::Object(obj)]));
                d
            },
            is_root_array: false,
        };
        let result = doc.find_schema_for_value(doc.data.get("points").unwrap(), "points");
        assert!(result.is_none(), "Should NOT match schema when extra keys are present");
    }

    #[test]
    fn test_schema_matching_empty_array_no_matching_name() {
        let mut schemas = IndexMap::new();
        let mut s = Schema::new("Anything");
        s.add_field("x", FieldType::new("int"));
        schemas.insert("Anything".to_string(), s);

        let doc = TeaLeaf {
            schemas,
            unions: IndexMap::new(),
            data: {
                let mut d = IndexMap::new();
                d.insert("empty".to_string(), Value::Array(vec![]));
                d
            },
            is_root_array: false,
        };
        let result = doc.find_schema_for_value(doc.data.get("empty").unwrap(), "empty");
        assert!(result.is_none(), "Empty array should return None when no schema name matches");
    }

    #[test]
    fn test_schema_matching_empty_array_matches_by_name() {
        let mut schemas = IndexMap::new();
        let mut s = Schema::new("item");
        s.add_field("id", FieldType::new("int"));
        schemas.insert("item".to_string(), s);

        let doc = TeaLeaf {
            schemas,
            unions: IndexMap::new(),
            data: {
                let mut d = IndexMap::new();
                d.insert("items".to_string(), Value::Array(vec![]));
                d
            },
            is_root_array: false,
        };
        let result = doc.find_schema_for_value(doc.data.get("items").unwrap(), "items");
        assert!(result.is_some(), "Empty array should match schema by singularized key name");
        assert_eq!(result.unwrap().name, "item");
    }

    // =========================================================================
    // Issue 12: Negative timestamp formatting
    // =========================================================================

    #[test]
    fn test_negative_timestamp_formatting() {
        // 1969-12-31T23:59:59Z = -1000 ms (1 second before epoch)
        let formatted = format_timestamp_millis(-1000, 0);
        assert_eq!(formatted, "1969-12-31T23:59:59Z");
    }

    #[test]
    fn test_negative_timestamp_with_millis() {
        // -500 ms = 1969-12-31T23:59:59.500Z
        let formatted = format_timestamp_millis(-500, 0);
        assert_eq!(formatted, "1969-12-31T23:59:59.500Z");
    }

    #[test]
    fn test_negative_timestamp_full_day() {
        // -86400000 ms = exactly one day before epoch = 1969-12-31T00:00:00Z
        let formatted = format_timestamp_millis(-86_400_000, 0);
        assert_eq!(formatted, "1969-12-31T00:00:00Z");
    }

    #[test]
    fn test_epoch_timestamp() {
        let formatted = format_timestamp_millis(0, 0);
        assert_eq!(formatted, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_positive_timestamp_with_millis() {
        // 1123ms = 1 second + 123ms after epoch
        let formatted = format_timestamp_millis(1123, 0);
        assert_eq!(formatted, "1970-01-01T00:00:01.123Z");
    }

    #[test]
    fn test_negative_timestamp_json_export() {
        let mut data = IndexMap::new();
        data.insert("ts".to_string(), Value::Timestamp(-1000, 0));
        let doc = TeaLeaf::new(IndexMap::new(), data);
        let json = doc.to_json().unwrap();
        assert!(json.contains("1969-12-31"), "Negative timestamp should format as pre-epoch date: {}", json);
    }

    // =========================================================================
    // Issue 7: Deterministic serialization (IndexMap preserves insertion order)
    // =========================================================================

    #[test]
    fn test_compile_deterministic_key_order() {
        // Two documents with the same data in the same insertion order
        // should produce identical binary output
        let dir = std::env::temp_dir();
        let path1 = dir.join("test_deterministic_1.tlbx");
        let path2 = dir.join("test_deterministic_2.tlbx");

        let mut data1 = IndexMap::new();
        data1.insert("alpha".to_string(), Value::Int(1));
        data1.insert("beta".to_string(), Value::Int(2));
        data1.insert("gamma".to_string(), Value::Int(3));
        let doc1 = TeaLeaf::new(IndexMap::new(), data1);
        doc1.compile(&path1, false).unwrap();

        let mut data2 = IndexMap::new();
        data2.insert("alpha".to_string(), Value::Int(1));
        data2.insert("beta".to_string(), Value::Int(2));
        data2.insert("gamma".to_string(), Value::Int(3));
        let doc2 = TeaLeaf::new(IndexMap::new(), data2);
        doc2.compile(&path2, false).unwrap();

        let bytes1 = std::fs::read(&path1).unwrap();
        let bytes2 = std::fs::read(&path2).unwrap();
        assert_eq!(bytes1, bytes2, "Binary output should be identical for same insertion order");

        std::fs::remove_file(&path1).ok();
        std::fs::remove_file(&path2).ok();
    }

    #[test]
    fn test_dumps_deterministic_key_order() {
        // dumps() preserves IndexMap insertion order deterministically
        let mut data = IndexMap::new();
        data.insert("zebra".to_string(), Value::Int(3));
        data.insert("alpha".to_string(), Value::Int(1));
        data.insert("middle".to_string(), Value::Int(2));

        let output1 = dumps(&data);
        let output2 = dumps(&data);
        assert_eq!(output1, output2, "dumps() should be deterministic");
        // Keys should appear in insertion order (IndexMap preserves insertion order)
        let lines: Vec<&str> = output1.trim().lines().collect();
        assert!(lines[0].starts_with("zebra:"), "First key should be 'zebra', got: {}", lines[0]);
        assert!(lines[1].starts_with("alpha:"), "Second key should be 'alpha', got: {}", lines[1]);
        assert!(lines[2].starts_with("middle:"), "Third key should be 'middle', got: {}", lines[2]);
    }

    // =========================================================================
    // Order-preservation integration tests
    // =========================================================================

    #[test]
    fn test_json_parse_preserves_key_order() {
        // JSON with intentionally non-alphabetical keys
        let json = r#"{"zebra": 1, "apple": 2, "mango": 3, "banana": 4}"#;
        let doc = TeaLeaf::from_json(json).unwrap();
        let keys: Vec<&String> = doc.data.keys().collect();
        assert_eq!(keys, &["zebra", "apple", "mango", "banana"],
            "JSON parse should preserve key insertion order");
    }

    #[test]
    fn test_json_roundtrip_preserves_key_order() {
        let json = r#"{"zebra": 1, "apple": 2, "mango": 3}"#;
        let doc = TeaLeaf::from_json(json).unwrap();
        let json_out = doc.to_json().unwrap();
        // Parse back and verify order
        let parsed: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        let keys: Vec<&str> = parsed.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, &["zebra", "apple", "mango"],
            "JSON round-trip should preserve key order");
    }

    #[test]
    fn test_tl_text_preserves_section_order() {
        let input = "zebra: 1\napple: 2\nmango: 3\n";
        let doc = TeaLeaf::parse(input).unwrap();
        let keys: Vec<&String> = doc.data.keys().collect();
        assert_eq!(keys, &["zebra", "apple", "mango"],
            "TL text parse should preserve section order");

        // Serialize back and verify order
        let output = doc.to_tl_with_schemas();
        let lines: Vec<&str> = output.trim().lines().collect();
        assert!(lines[0].starts_with("zebra:"), "got: {}", lines[0]);
        assert!(lines[1].starts_with("apple:"), "got: {}", lines[1]);
        assert!(lines[2].starts_with("mango:"), "got: {}", lines[2]);
    }

    #[test]
    fn test_binary_roundtrip_preserves_section_order() {
        let json = r#"{"zebra": 1, "apple": 2, "mango": 3, "banana": 4}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let dir = std::env::temp_dir();
        let path = dir.join("test_order_preserve.tlbx");
        doc.compile(&path, false).unwrap();

        let reader = crate::Reader::open(&path).unwrap();
        let doc2 = TeaLeaf::from_reader(&reader).unwrap();
        let keys: Vec<&String> = doc2.data.keys().collect();
        assert_eq!(keys, &["zebra", "apple", "mango", "banana"],
            "Binary round-trip should preserve section order");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_object_field_order_preserved_through_binary() {
        let json = r#"{"data": {"z_last": 1, "a_first": 2, "m_middle": 3}}"#;
        let doc = TeaLeaf::from_json(json).unwrap();

        let dir = std::env::temp_dir();
        let path = dir.join("test_obj_order.tlbx");
        doc.compile(&path, false).unwrap();

        let reader = crate::Reader::open(&path).unwrap();
        let val = reader.get("data").unwrap();
        let obj = val.as_object().unwrap();
        let keys: Vec<&String> = obj.keys().collect();
        assert_eq!(keys, &["z_last", "a_first", "m_middle"],
            "Object field order should be preserved through binary round-trip");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_nested_object_order_preserved() {
        let json = r#"{"outer": {"z": {"c": 3, "a": 1, "b": 2}, "a": {"x": 10, "w": 20}}}"#;
        let doc = TeaLeaf::from_json(json).unwrap();
        let tl = doc.to_tl_with_schemas();

        // Parse back and check nested order
        let doc2 = TeaLeaf::parse(&tl).unwrap();
        let outer = doc2.get("outer").unwrap().as_object().unwrap();
        let outer_keys: Vec<&String> = outer.keys().collect();
        assert_eq!(outer_keys, &["z", "a"], "Outer keys order preserved");

        let z_obj = outer.get("z").unwrap().as_object().unwrap();
        let z_keys: Vec<&String> = z_obj.keys().collect();
        assert_eq!(z_keys, &["c", "a", "b"], "Nested object keys order preserved");
    }

    #[test]
    fn test_schema_order_preserved_in_text() {
        let input = r#"
            @struct Zebra (z_name: string)
            @struct Apple (a_name: string)
            items: [1, 2, 3]
        "#;
        let doc = TeaLeaf::parse(input).unwrap();
        let schema_keys: Vec<&String> = doc.schemas.keys().collect();
        assert_eq!(schema_keys, &["Zebra", "Apple"],
            "Schema definition order should be preserved");
    }

    // -------------------------------------------------------------------------
    // Fuzz regression tests (full serialize/roundtrip paths)
    // -------------------------------------------------------------------------

    #[test]
    fn test_fuzz_crash_ba05f4f8_serialize_day_zero_no_panic() {
        // Regression: fuzz_serialize crash-ba05f4f81615e2bf2b01137126cd772c6c0cc6d2
        // Timestamp with month=0 or day=0 caused u32 underflow in days_from_epoch.
        // Exercises the full fuzz_serialize path: parse → to_json → to_tl → re-parse.
        let inputs = [
            "ts: 2024-01-00T10:30:00Z",  // day=0
            "ts: 2024-00-15T10:30:00Z",  // month=0
            "ts: 6000-00-00T00:00:00Z",  // both zero
        ];
        for input in &inputs {
            // parse must not panic (should return Err)
            let result = TeaLeaf::parse(input);
            if let Ok(tl) = result {
                let _ = tl.to_json();
                let _ = tl.to_json_compact();
                let text = tl.to_tl_with_schemas();
                let _ = TeaLeaf::parse(&text);
            }
        }
    }

    #[test]
    fn test_fuzz_crash_b085ba0e_roundtrip_day_zero_no_panic() {
        // Regression: fuzz_roundtrip crash-b085ba0e656f074031d8c4cb5173313785fa79d1
        // Same days_from_epoch underflow, hit through the roundtrip path.
        // Exercises the full fuzz_roundtrip path: parse → compile → read → walk.
        let inputs = [
            "ts: 4001-03-00T00:00:00Z",  // day=0 (pattern from artifact)
            "ts: 4401-03-00T00:00:00Z",  // variant
        ];
        for input in &inputs {
            let result = TeaLeaf::parse(input);
            if let Ok(tl) = result {
                let tmp = tempfile::NamedTempFile::new().unwrap();
                if tl.compile(tmp.path(), false).is_ok() {
                    let bytes = std::fs::read(tmp.path()).unwrap();
                    if let Ok(reader) = Reader::from_bytes(bytes) {
                        for key in reader.keys() {
                            let _ = reader.get(key);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_fuzz_crash_48767e10_json_schemas_bare_dash_roundtrip() {
        // Regression: fuzz_json_schemas crash-48767e10b4ec71542bfbee2bc358b1e21831a259
        // JSON string "-" was serialized unquoted, causing re-parse failure.
        for input in [
            r#""-""#, r#""+""#, r#""--""#, r#""-foo""#,
            r#"{"a": "-"}"#, r#"{"a": "+"}"#,
            "\"\\u0660\"",  // Arabic-Indic digit zero
        ] {
            let tl = TeaLeaf::from_json_with_schemas(input);
            if let Ok(tl) = tl {
                let text = tl.to_tl_with_schemas();
                let reparsed = TeaLeaf::parse(&text);
                assert!(
                    reparsed.is_ok(),
                    "re-parse failed for JSON input {}",
                    input,
                );
            }
        }
    }

    #[test]
    fn test_fuzz_crash_820dac71_empty_key_roundtrip() {
        // Regression: fuzz_json_schemas crash-820dac71c95d324067cd88de5f24897c65ace57a
        // JSON object with empty key was serialized without quoting, losing the key.
        for input in [
            r#"{"":{}}"#,                // empty key with empty object
            r#"[{"":{}}}]"#,             // root array variant (crash-66a8d85176f76ed68ada9f9526abe4efd8352f27)
            r#"{"":"value"}"#,            // empty key with string value
        ] {
            if let Ok(tl) = TeaLeaf::from_json_with_schemas(input) {
                let text = tl.to_tl_with_schemas();
                let reparsed = TeaLeaf::parse(&text);
                assert!(
                    reparsed.is_ok(),
                    "re-parse failed for JSON input {}",
                    input,
                );
            }
        }
    }

    #[test]
    fn test_fuzz_crash_66a8d851_root_array_empty_key() {
        // Regression: fuzz_json_schemas crash-66a8d85176f76ed68ada9f9526abe4efd8352f27
        // Root array with empty-key object: schema inference + to_tl_with_schemas roundtrip
        let input = r#"[{"":{}}]"#;
        if let Ok(tl) = TeaLeaf::from_json_with_schemas(input) {
            let text = tl.to_tl_with_schemas();
            let reparsed = TeaLeaf::parse(&text);
            assert!(reparsed.is_ok(), "re-parse failed for root array with empty key");
        }
    }

    #[test]
    fn test_fuzz_crash_847a9194_uint_roundtrip() {
        // Regression: fuzz_json_schemas crash-847a919462bb567fab268023a5a29d04e92db779
        // Large u64 values (> i64::MAX) were demoted to f64 on re-parse, losing precision.
        let input = "9999999999999999999";  // > i64::MAX, fits in u64
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        let text = tl.to_tl_with_schemas();
        let reparsed = TeaLeaf::parse(&text).unwrap();
        let orig = tl.data.get("root").unwrap();
        let re = reparsed.data.get("root").unwrap();
        assert_eq!(orig, re, "UInt roundtrip mismatch");
    }

    #[test]
    fn test_fuzz_crash_3902c5cc_float_infinity_roundtrip() {
        // Regression: fuzz_serialize crash-3902c5cc99e5e4150d08d40372c86207fbc6db7f
        // 5e550 and -5e550 overflow f64 and are now stored as JsonNumber.
        // NaN remains Float(NaN).
        let tl = TeaLeaf::parse("b: NaN").unwrap();
        let text = tl.to_tl_with_schemas();
        let reparsed = TeaLeaf::parse(&text).unwrap();
        let orig = tl.data.get("b").unwrap();
        let re = reparsed.data.get("b").unwrap();
        match (orig, re) {
            (Value::Float(a), Value::Float(b)) => {
                assert_eq!(a.to_bits(), b.to_bits(), "NaN roundtrip failed");
            }
            _ => panic!("expected Float, got {:?} / {:?}", orig, re),
        }

        // 5e550 and -5e550 are now JsonNumber (overflow f64)
        for input in &["b: 5e550", "b: -5e550"] {
            let tl = TeaLeaf::parse(input).unwrap();
            let text = tl.to_tl_with_schemas();
            let reparsed = TeaLeaf::parse(&text).unwrap();
            let orig = tl.data.get("b").unwrap();
            let re = reparsed.data.get("b").unwrap();
            match (orig, re) {
                (Value::JsonNumber(a), Value::JsonNumber(b)) => {
                    assert_eq!(a, b, "JsonNumber roundtrip failed for {}", input);
                }
                _ => panic!("expected JsonNumber, got {:?} / {:?}", orig, re),
            }
        }
    }

    #[test]
    fn test_needs_quoting_bare_sign() {
        assert!(needs_quoting("-"));
        assert!(needs_quoting("+"));
        assert!(needs_quoting("--"));
        assert!(needs_quoting("-foo"));
        assert!(needs_quoting("+bar"));
        assert!(needs_quoting("-1")); // negative number
        assert!(needs_quoting("+1")); // positive number
        assert!(needs_quoting("\u{0660}")); // Arabic-Indic digit zero
        assert!(!needs_quoting("hello"));
        assert!(!needs_quoting("foo-bar"));
    }

    #[test]
    fn test_fuzz_crash_nan_string_needs_quoting() {
        // Regression: fuzz_parse/fuzz_serialize crash — string "NaN" must be quoted
        // to avoid re-parsing as Float(NaN).
        assert!(needs_quoting("NaN"));
        assert!(needs_quoting("inf"));
        assert!(needs_quoting("Infinity"));

        // Roundtrip: String("NaN") must survive parse → dumps → re-parse
        for word in &["NaN", "inf", "Infinity"] {
            let input = format!("a: \"{}\"", word);
            let tl = TeaLeaf::parse(&input).unwrap();
            assert!(matches!(tl.get("a"), Some(Value::String(_))));
            let text = dumps(&tl.data);
            let reparsed = TeaLeaf::parse(&text).unwrap();
            assert_eq!(
                reparsed.get("a").unwrap().as_str(),
                Some(*word),
                "roundtrip failed for string {:?}",
                word,
            );
        }
    }

    #[test]
    fn test_json_any_type_compile_roundtrip() {
        // Regression: from_json_with_schemas infers "any" for fields whose nested objects
        // don't match a schema. encode_typed_value must fall back to generic encoding
        // instead of erroring with "requires a schema for encoding".
        use tempfile::NamedTempFile;

        let json = r#"[
            {"name": "alice", "meta": {"x": 1}},
            {"name": "bob",   "meta": {"y": "two", "z": true}}
        ]"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        // "meta" has varying shapes → inferred as "any"
        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).expect("compile with 'any' field must not error");

        // Read back and verify data survived
        let reader = Reader::open(temp.path()).unwrap();
        assert_eq!(reader.keys().len(), doc.data.len());
    }

    #[test]
    fn json_any_array_binary_roundtrip() {
        // Regression: []any fields (from JSON inference of heterogeneous arrays inside
        // schema-typed objects) caused binary corruption. encode_typed_value wrote
        // TLType::Struct as the element type for "any" (the to_tl_type default),
        // but the actual data was heterogeneous. The reader then read garbage bytes
        // as schema indices, crashing with "schema index N out of bounds".
        use tempfile::NamedTempFile;

        let json = r#"{
            "events": [
                {
                    "id": "E1",
                    "type": "sale",
                    "data": ["SKU-100", 3, 29.99, true],
                    "tags": ["flash", "online"]
                },
                {
                    "id": "E2",
                    "type": "return",
                    "data": ["SKU-200", 1, 15.0, false],
                    "tags": ["in-store"]
                }
            ]
        }"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();

        // Verify inference: "data" should be []any (heterogeneous), "tags" should be []string
        let event_schema = doc.schemas.get("event").expect("missing 'event' schema");
        let data_field = event_schema.fields.iter().find(|f| f.name == "data").unwrap();
        assert!(data_field.field_type.is_array, "data should be array");
        assert_eq!(data_field.field_type.base, "any", "data should be []any, got []{}", data_field.field_type.base);

        // Compile to binary
        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), false).expect("compile must not error");

        // Read back and verify full data integrity
        let reader = Reader::open(temp.path()).unwrap();
        let events_val = reader.get("events").expect("missing 'events' key");
        let events = events_val.as_array().expect("events should be array");
        assert_eq!(events.len(), 2, "should have 2 events");

        // Verify first event's heterogeneous data array
        let e1 = events[0].as_object().expect("event should be object");
        assert_eq!(e1.get("id").unwrap().as_str(), Some("E1"));
        let data1 = e1.get("data").unwrap().as_array().expect("data should be array");
        assert_eq!(data1.len(), 4);
        assert_eq!(data1[0].as_str(), Some("SKU-100"));
        assert_eq!(data1[2].as_float(), Some(29.99));
    }

    #[test]
    fn retail_orders_json_binary_roundtrip() {
        // End-to-end: retail_orders.json → infer schemas → compile → read → JSON
        // Exercises the full path that was missing from the test suite: complex
        // real-world JSON with heterogeneous arrays ([]any) inside schema-typed objects.
        use tempfile::NamedTempFile;

        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/retail_orders.json");
        let json = std::fs::read_to_string(&fixture)
            .unwrap_or_else(|e| panic!("read fixture {}: {e}", fixture.display()));

        let doc = TeaLeaf::from_json_with_schemas(&json).unwrap();
        let temp = NamedTempFile::new().unwrap();
        doc.compile(temp.path(), true).expect("compile retail_orders must not error");

        // Read binary back to JSON and compare
        let reader = Reader::open(temp.path()).unwrap();
        let keys = reader.keys();
        assert_eq!(keys.len(), 5, "expected 5 top-level keys, got {keys:?}");

        // Verify all sections are readable and have correct element counts
        let orders_val = reader.get("orders").unwrap();
        let orders = orders_val.as_array().expect("orders");
        assert_eq!(orders.len(), 10, "expected 10 orders");

        let products_val = reader.get("products").unwrap();
        let products = products_val.as_array().expect("products");
        assert_eq!(products.len(), 4, "expected 4 products");

        let customers_val = reader.get("customers").unwrap();
        let customers = customers_val.as_array().expect("customers");
        assert_eq!(customers.len(), 3, "expected 3 customers");

        // Spot-check: first order preserves heterogeneous fields
        let order1 = orders[0].as_object().expect("order should be object");
        assert_eq!(order1.get("order_id").unwrap().as_str(), Some("ORD-2024-00001"));
        let items = order1.get("items").unwrap().as_array().expect("items");
        assert_eq!(items.len(), 3, "first order should have 3 items");
    }

    #[test]
    fn fuzz_repro_json_schema_bool_field_name() {
        // Fuzz crash: field named "bool" conflicts with type keyword
        let input = r#"[{"bool":{"b":2}}]"#;
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        let tl_text = tl.to_tl_with_schemas();
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        assert_eq!(tl.data.len(), reparsed.data.len(), "key count mismatch");
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    /// Helper: verify that a JSON field named after a built-in type correctly
    /// round-trips through TL text when schema inference is used.
    fn assert_builtin_name_text_roundtrip(type_name: &str, inner_json: &str) {
        let input = format!(r#"[{{"{type_name}":{inner_json}}}]"#);
        let tl = TeaLeaf::from_json_with_schemas(&input)
            .unwrap_or_else(|e| panic!("[{type_name}] from_json_with_schemas failed: {e}"));
        let tl_text = tl.to_tl_with_schemas();

        // The schema should appear in the text output
        assert!(
            tl_text.contains(&format!("@struct {type_name}")),
            "[{type_name}] expected @struct {type_name} in TL text:\n{tl_text}"
        );

        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("[{type_name}] re-parse failed: {e}\nTL text:\n{tl_text}"));

        assert_eq!(
            tl.data.len(), reparsed.data.len(),
            "[{type_name}] key count mismatch"
        );
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key)
                .unwrap_or_else(|| panic!("[{type_name}] lost key '{key}'"));
            assert_eq!(orig_val, re_val, "[{type_name}] value mismatch for key '{key}'");
        }
    }

    #[test]
    fn schema_name_shadows_builtin_bool() {
        assert_builtin_name_text_roundtrip("bool", r#"{"x":1}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_int() {
        // Inner value is a string so field type "string" doesn't collide with schema "int"
        assert_builtin_name_text_roundtrip("int", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_int8() {
        assert_builtin_name_text_roundtrip("int8", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_int16() {
        assert_builtin_name_text_roundtrip("int16", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_int32() {
        assert_builtin_name_text_roundtrip("int32", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_int64() {
        assert_builtin_name_text_roundtrip("int64", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_uint() {
        assert_builtin_name_text_roundtrip("uint", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_uint8() {
        assert_builtin_name_text_roundtrip("uint8", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_uint16() {
        assert_builtin_name_text_roundtrip("uint16", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_uint32() {
        assert_builtin_name_text_roundtrip("uint32", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_uint64() {
        assert_builtin_name_text_roundtrip("uint64", r#"{"x":"hello"}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_float() {
        assert_builtin_name_text_roundtrip("float", r#"{"x":1}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_float32() {
        assert_builtin_name_text_roundtrip("float32", r#"{"x":1}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_float64() {
        assert_builtin_name_text_roundtrip("float64", r#"{"x":1}"#);
    }

    #[test]
    fn schema_name_shadows_builtin_string() {
        assert_builtin_name_text_roundtrip("string", r#"{"x":1}"#);
    }

    // Note: "bytes" is not tested via JSON inference because singularize("bytes") = "byte"
    // which is NOT a built-in type. The direct TL-parsing test below covers "bytes" as a
    // schema name.

    #[test]
    fn schema_name_shadows_builtin_timestamp() {
        assert_builtin_name_text_roundtrip("timestamp", r#"{"x":1}"#);
    }

    /// Test built-in type names as schemas via direct TL text parsing (not JSON inference).
    /// This covers names that can't arise through singularization (like "bytes").
    #[test]
    fn schema_name_shadows_builtin_direct_tl_parse() {
        let test_cases = &[
            // (TL text, expected field name, expected inner value)
            (
                "@struct bytes (x: int)\n@struct root (data: bytes)\nroot: @table root [\n  ((42))\n]",
                "data",
                Value::Object(IndexMap::from([
                    ("x".to_string(), Value::Int(42)),
                ])),
            ),
            (
                "@struct bool (a: int, b: string)\n@struct root (flag: bool)\nroot: @table root [\n  ((1, hello))\n]",
                "flag",
                Value::Object(IndexMap::from([
                    ("a".to_string(), Value::Int(1)),
                    ("b".to_string(), Value::String("hello".into())),
                ])),
            ),
        ];

        for (tl_text, field_name, expected_val) in test_cases {
            let doc = TeaLeaf::parse(tl_text)
                .unwrap_or_else(|e| panic!("parse failed for field '{field_name}': {e}\n{tl_text}"));

            let root_arr = doc.data.get("root").expect("missing 'root' key");
            if let Value::Array(arr) = root_arr {
                if let Value::Object(obj) = &arr[0] {
                    let actual = obj.get(*field_name)
                        .unwrap_or_else(|| panic!("missing field '{field_name}'"));
                    assert_eq!(actual, expected_val, "mismatch for field '{field_name}'");
                } else {
                    panic!("expected Object, got {:?}", arr[0]);
                }
            } else {
                panic!("expected Array, got {:?}", root_arr);
            }
        }
    }

    /// Self-referencing case: @struct int (x: int) where the inner field type
    /// matches the schema name. The LParen guard ensures `x: int` resolves to
    /// primitive int (next token is a literal, not `(`).
    #[test]
    fn schema_name_shadows_builtin_self_referencing() {
        // JSON: [{"int": {"x": 1}}] — creates @struct int (x: int)
        // The inner field "x: int" must resolve to primitive int, not struct "int"
        let input = r#"[{"int":{"x":1}}]"#;
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        let tl_text = tl.to_tl_with_schemas();

        assert!(tl_text.contains("@struct int"), "expected @struct int in:\n{tl_text}");

        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("re-parse failed: {e}\nTL text:\n{tl_text}"));

        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key)
                .unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    /// Self-referencing: @struct int (int: int) — field name AND type both "int"
    #[test]
    fn schema_name_shadows_builtin_self_ref_same_field_name() {
        let tl_text = "\
@struct int (int: int)
@struct root (val: int)

root: @table root [
  ((42))
]
";
        let doc = TeaLeaf::parse(tl_text)
            .unwrap_or_else(|e| panic!("parse failed: {e}\nTL text:\n{tl_text}"));

        let json = doc.to_json().unwrap();
        eprintln!("=== JSON ===\n{json}");

        // The root array should have one element with field "val" as an Object
        let root_arr = doc.data.get("root").expect("missing 'root'");
        if let Value::Array(arr) = root_arr {
            if let Value::Object(obj) = &arr[0] {
                let val = obj.get("val").expect("missing field 'val'");
                // val should be Object({"int": Int(42)}) — struct "int" with field "int" = 42
                assert_eq!(
                    val,
                    &Value::Object(IndexMap::from([
                        ("int".to_string(), Value::Int(42)),
                    ])),
                    "expected struct instance, got {val:?}"
                );
            } else {
                panic!("expected Object, got {:?}", arr[0]);
            }
        } else {
            panic!("expected Array, got {root_arr:?}");
        }
    }

    /// Duplicate @struct declarations: second overwrites first
    #[test]
    fn schema_name_shadows_builtin_duplicate_struct_decl() {
        let tl_text = "\
@struct int (x: int)
@struct int (int: int)
@struct root (val: int)

root: @table root [
  ((42))
]
";
        let result = TeaLeaf::parse(tl_text);
        match &result {
            Ok(doc) => {
                let json = doc.to_json().unwrap();
                eprintln!("=== JSON ===\n{json}");
                eprintln!("=== schemas ===");
                for (name, schema) in &doc.schemas {
                    let fields: Vec<String> = schema.fields.iter()
                        .map(|f| format!("{}: {}", f.name, f.field_type.base))
                        .collect();
                    eprintln!("  @struct {name} ({})", fields.join(", "));
                }
            }
            Err(e) => {
                eprintln!("=== parse error ===\n{e}");
            }
        }
        // Assert that parsing succeeds
        result.unwrap();
    }

    /// Multiple built-in-named schemas in the same document
    #[test]
    fn schema_name_shadows_multiple_builtins() {
        let input = r#"[{"bool":{"a":1},"int":{"b":"hello"},"float":{"c":true}}]"#;
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        let tl_text = tl.to_tl_with_schemas();

        assert!(tl_text.contains("@struct bool"), "missing @struct bool");
        assert!(tl_text.contains("@struct int"), "missing @struct int");
        assert!(tl_text.contains("@struct float"), "missing @struct float");

        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("re-parse failed: {e}\nTL text:\n{tl_text}"));

        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key)
                .unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }


    /// Fuzz crash: singularize("s") → "" (empty string), producing invalid
    /// @struct definitions with missing names.
    #[test]
    fn fuzz_repro_singularize_single_char_s() {
        let input = r#"[{"s":{"b":1}}]"#;
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        let tl_text = tl.to_tl_with_schemas();

        // Schema name must not be empty — singularize("s") should return "s"
        assert!(
            tl_text.contains("@struct s"),
            "expected @struct s in TL text:\n{tl_text}"
        );

        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        assert_eq!(tl.data.len(), reparsed.data.len(), "key count mismatch");
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    #[test]
    fn singularize_does_not_produce_empty_string() {
        // All single-character inputs must pass through unchanged
        for c in 'a'..='z' {
            let s = String::from(c);
            let result = super::singularize(&s);
            assert!(!result.is_empty(), "singularize({s:?}) produced empty string");
            assert_eq!(result, s, "singularize({s:?}) should return {s:?}, got {result:?}");
        }
    }

    /// Fuzz crash: field name with dots causes value mismatch on roundtrip
    #[test]
    fn fuzz_repro_dots_in_field_name() {
        // Fuzz regression: field "root" inside root-array wrapper both singularize to "root",
        // causing analyze_nested_objects to create a correct inner schema that analyze_array
        // then overwrites with a self-referencing @struct root (root: root).
        let input = r#"[{"root":{"Z.lll.i0...A":44444440.0}}]"#;
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        let tl_text = tl.to_tl_with_schemas();
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        assert_eq!(tl.data.len(), reparsed.data.len(), "key count mismatch");
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    #[test]
    fn schema_name_collision_field_matches_parent() {
        // When an array field name singularizes to the same name as its parent schema,
        // the inner schema should be preserved (not overwritten with a self-reference).
        // This tests the general case, not just the root-array wrapper collision.
        let input = r#"{"items": [{"items": {"a": 1, "b": 2}}]}"#;
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        let tl_text = tl.to_tl_with_schemas();
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    #[test]
    fn analyze_node_nesting_stress_test() {
        // Stress test: "node" appears at many nesting levels with different shapes.
        // Schema inference should NOT create conflicting schemas or lose data.
        let input = r#"{
          "node": {
            "id": 1,
            "name": "root",
            "active": true,
            "node": {
              "id": "child-1",
              "metrics": {
                "node": {
                  "value": 42.7,
                  "unit": "ms",
                  "thresholds": [10, 20, 30]
                }
              },
              "node": [
                {
                  "id": 2,
                  "enabled": false
                },
                {
                  "id": 3,
                  "enabled": "sometimes",
                  "node": {
                    "status": null,
                    "confidence": 0.93
                  }
                }
              ]
            }
          },
          "nodeMetadata": {
            "node": {
              "version": 5,
              "checksum": "a94a8fe5ccb19ba61c4c0873d391e987",
              "flags": {
                "node": true
              }
            }
          }
        }"#;

        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        eprintln!("=== schemas ({}) ===", tl.schemas.len());
        for (name, schema) in &tl.schemas {
            let fields: Vec<String> = schema.fields.iter()
                .map(|f| format!("{}: {}{}{}", f.name, f.field_type.base,
                    if f.field_type.is_array { "[]" } else { "" },
                    if f.field_type.nullable { "?" } else { "" }))
                .collect();
            eprintln!("  @struct {name} ({})", fields.join(", "));
        }
        let tl_text = tl.to_tl_with_schemas();
        eprintln!("=== TL text ===\n{tl_text}");

        // Core correctness check: round-trip must preserve all data.
        // With union-based schema inference, optional fields absent in the
        // original may appear as null after roundtrip (schema adds ~).
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert!(values_eq_with_optional_nulls(orig_val, re_val),
                "value mismatch for key '{key}':\n  orig: {:?}\n  rt:   {:?}", orig_val, re_val);
        }
    }

    #[test]
    fn schema_collision_recursive_arrays() {
        // "nodes" appears as arrays at two levels with different shapes.
        // Inner: [{name, value}], Outer: [{name, nodes}]
        // Both singularize to "node" — only one schema can exist.
        let input = r#"{
          "nodes": [
            {
              "name": "parent",
              "nodes": [
                {"name": "child", "value": 42}
              ]
            }
          ]
        }"#;
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        eprintln!("=== schemas ({}) ===", tl.schemas.len());
        for (name, schema) in &tl.schemas {
            let fields: Vec<String> = schema.fields.iter()
                .map(|f| format!("{}: {}{}{}", f.name, f.field_type.base,
                    if f.field_type.is_array { "[]" } else { "" },
                    if f.field_type.nullable { "?" } else { "" }))
                .collect();
            eprintln!("  @struct {name} ({})", fields.join(", "));
        }
        let tl_text = tl.to_tl_with_schemas();
        eprintln!("=== TL text ===\n{tl_text}");
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    #[test]
    fn schema_collision_recursive_same_shape() {
        // "nodes" appears at two levels but SAME shape [{id, name}].
        // Schema "node" created for inner array should also work for outer.
        let input = r#"{
          "nodes": [
            {
              "id": 1,
              "name": "parent",
              "children": [
                {"id": 10, "name": "child-a"},
                {"id": 11, "name": "child-b"}
              ]
            },
            {
              "id": 2,
              "name": "sibling",
              "children": [
                {"id": 20, "name": "child-c"}
              ]
            }
          ]
        }"#;
        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        eprintln!("=== schemas ({}) ===", tl.schemas.len());
        for (name, schema) in &tl.schemas {
            let fields: Vec<String> = schema.fields.iter()
                .map(|f| format!("{}: {}{}{}", f.name, f.field_type.base,
                    if f.field_type.is_array { "[]" } else { "" },
                    if f.field_type.nullable { "?" } else { "" }))
                .collect();
            eprintln!("  @struct {name} ({})", fields.join(", "));
        }
        let tl_text = tl.to_tl_with_schemas();
        eprintln!("=== TL text ===\n{tl_text}");
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    #[test]
    fn schema_collision_three_level_nesting() {
        // "nodes" at 3 levels: L1 and L2 have same shape {name, nodes},
        // L3 has different shape {name, score}. All singularize to "node".
        // The deepest schema wins (depth-first); outer levels fall back to
        // generic format. No data loss at any level.
        let input = r#"{
          "nodes": [
            {
              "name": "grandparent",
              "nodes": [
                {
                  "name": "parent",
                  "nodes": [
                    {"name": "leaf-a", "score": 99.5},
                    {"name": "leaf-b", "score": 42.0}
                  ]
                }
              ]
            },
            {
              "name": "uncle",
              "nodes": [
                {
                  "name": "cousin",
                  "nodes": [
                    {"name": "leaf-c", "score": 77.3}
                  ]
                }
              ]
            }
          ]
        }"#;

        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        eprintln!("=== schemas ({}) ===", tl.schemas.len());
        for (name, schema) in &tl.schemas {
            let fields: Vec<String> = schema.fields.iter()
                .map(|f| format!("{}: {}{}{}", f.name, f.field_type.base,
                    if f.field_type.is_array { "[]" } else { "" },
                    if f.field_type.nullable { "?" } else { "" }))
                .collect();
            eprintln!("  @struct {name} ({})", fields.join(", "));
        }
        let tl_text = tl.to_tl_with_schemas();
        eprintln!("=== TL text ===\n{tl_text}");

        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    #[test]
    fn schema_collision_three_level_divergent_leaves() {
        // L1: [{name, nodes}], L2: [{name, nodes}] (same shape),
        // L3: [{id, value}] in one branch, [{identifier, points}] in another.
        // The depth-first analysis only sees the first branch's L3 shape.
        // The second branch's L3 must fall back to generic format.
        let input = r#"{
          "nodes": [
            {
              "name": "grandparent",
              "nodes": [
                {
                  "name": "parent",
                  "nodes": [
                    {"id": "leaf-a", "value": 99.5},
                    {"id": "leaf-b", "value": 42.0}
                  ]
                }
              ]
            },
            {
              "name": "uncle",
              "nodes": [
                {
                  "name": "cousin",
                  "nodes": [
                    {"identifier": "leaf-c", "points": 77.3}
                  ]
                }
              ]
            }
          ]
        }"#;

        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        eprintln!("=== schemas ({}) ===", tl.schemas.len());
        for (name, schema) in &tl.schemas {
            let fields: Vec<String> = schema.fields.iter()
                .map(|f| format!("{}: {}{}{}", f.name, f.field_type.base,
                    if f.field_type.is_array { "[]" } else { "" },
                    if f.field_type.nullable { "?" } else { "" }))
                .collect();
            eprintln!("  @struct {name} ({})", fields.join(", "));
        }
        let tl_text = tl.to_tl_with_schemas();
        eprintln!("=== TL text ===\n{tl_text}");

        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL text:\n{tl_text}"));
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    #[test]
    fn json_inference_nested_array_inside_object() {
        // JSON inference must discover array schemas inside nested objects.
        // e.g., items[].product.stock[] should get its own @struct stock schema,
        // not fall back to []any.
        let input = r#"{
          "items": [
            {
              "name": "Widget",
              "product": {
                "id": "P-1",
                "stock": [
                  {"warehouse": "W1", "qty": 100, "backordered": false},
                  {"warehouse": "W2", "qty": 50, "backordered": true}
                ]
              }
            },
            {
              "name": "Gadget",
              "product": {
                "id": "P-2",
                "stock": [
                  {"warehouse": "W1", "qty": 200, "backordered": false}
                ]
              }
            }
          ]
        }"#;

        let tl = TeaLeaf::from_json_with_schemas(input).unwrap();
        let tl_text = tl.to_tl_with_schemas();

        // Must have a "stock" schema (from singularize("stock") = "stock")
        assert!(tl.schemas.contains_key("stock"),
            "Missing 'stock' schema. Schemas: {:?}\nTL:\n{tl_text}",
            tl.schemas.keys().collect::<Vec<_>>());

        // The product schema must reference stock[] not []any
        let product_schema = tl.schemas.get("product").expect("missing product schema");
        let stock_field = product_schema.fields.iter().find(|f| f.name == "stock")
            .expect("product schema missing stock field");
        assert!(stock_field.field_type.is_array, "stock should be array");
        assert_eq!(stock_field.field_type.base, "stock",
            "stock field type should be 'stock', got '{}'", stock_field.field_type.base);

        // Must produce @table for items and tuples for stock inside product
        assert!(tl_text.contains("@table item"), "Missing @table item:\n{tl_text}");

        // Round-trip: parse back and verify data integrity
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL:\n{tl_text}"));
        for (key, orig_val) in &tl.data {
            let re_val = reparsed.data.get(key).unwrap_or_else(|| panic!("lost key '{key}'"));
            assert_eq!(orig_val, re_val, "value mismatch for key '{key}'");
        }
    }

    // ── Compact formatting tests ──────────────────────────────────────

    #[test]
    fn test_dumps_compact_basic() {
        let mut data = IndexMap::new();
        data.insert("name".to_string(), Value::String("alice".to_string()));
        data.insert("age".to_string(), Value::Int(30));
        let output = dumps_compact(&data);
        assert!(output.contains("name:alice\n"), "got: {output}");
        assert!(output.contains("age:30\n"), "got: {output}");
    }

    #[test]
    fn test_dumps_compact_array() {
        let mut data = IndexMap::new();
        data.insert("items".to_string(), Value::Array(vec![
            Value::Int(1), Value::Int(2), Value::Int(3),
        ]));
        let output = dumps_compact(&data);
        assert!(output.contains("[1,2,3]"), "got: {output}");
    }

    #[test]
    fn test_dumps_compact_object() {
        let mut data = IndexMap::new();
        let obj: IndexMap<String, Value> = vec![
            ("host".to_string(), Value::String("localhost".to_string())),
            ("port".to_string(), Value::Int(8080)),
        ].into_iter().collect();
        data.insert("config".to_string(), Value::Object(obj));
        let output = dumps_compact(&data);
        assert!(output.contains("{host:localhost,port:8080}"), "got: {output}");
    }

    #[test]
    fn test_dumps_compact_map() {
        let mut data = IndexMap::new();
        data.insert("m".to_string(), Value::Map(vec![
            (Value::Int(1), Value::String("one".to_string())),
            (Value::Int(2), Value::String("two".to_string())),
        ]));
        let output = dumps_compact(&data);
        assert!(output.contains("@map{1:one,2:two}"), "got: {output}");
    }

    #[test]
    fn test_dumps_compact_tagged_keeps_space() {
        let mut data = IndexMap::new();
        data.insert("val".to_string(), Value::Tagged(
            "ok".to_string(), Box::new(Value::Int(200)),
        ));
        let output = dumps_compact(&data);
        assert!(output.contains(":ok 200"), "Space after :tag must be kept (tag/value would merge), got: {output}");
    }

    #[test]
    fn test_compact_struct_definition() {
        let json = r#"{"users": [{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let compact = doc.to_tl_with_schemas_compact();
        // Struct def should have no space before ( and no spaces after ,
        assert!(compact.contains("@struct user("), "got: {compact}");
        assert!(compact.contains("id:int"), "got: {compact}");
        // Table should have no space before [
        assert!(compact.contains("@table user["), "got: {compact}");
        // No indentation on table rows
        assert!(compact.contains("\n("), "rows should start at column 0, got: {compact}");
        assert!(!compact.contains("  ("), "no indentation in compact, got: {compact}");
        // No blank line between definitions and data
        assert!(!compact.contains(")\n\n"), "no blank line after struct def, got: {compact}");
    }

    #[test]
    fn test_compact_is_smaller_than_pretty() {
        let json = r#"{"users": [{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let pretty = doc.to_tl_with_schemas();
        let compact = doc.to_tl_with_schemas_compact();
        assert!(
            compact.len() < pretty.len(),
            "Compact ({}) should be smaller than pretty ({})\nCompact:\n{compact}\nPretty:\n{pretty}",
            compact.len(), pretty.len()
        );
    }

    #[test]
    fn test_compact_roundtrip() {
        // Compact output must re-parse to the same data
        let json = r#"{
            "company": "FastTrack Logistics",
            "shipments": [
                {"id": "S1", "origin": "Los Angeles, CA", "weight": 250, "cost": 450.0, "delivered": true},
                {"id": "S2", "origin": "Chicago, IL", "weight": 180, "cost": 320.0, "delivered": false}
            ]
        }"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let compact = doc.to_tl_with_schemas_compact();
        let reparsed = TeaLeaf::parse(&compact)
            .unwrap_or_else(|e| panic!("Failed to re-parse compact: {e}\nCompact:\n{compact}"));

        let json1 = doc.to_json().unwrap();
        let json2 = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(&json1).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json2).unwrap();
        assert_eq!(v1, v2, "Compact round-trip data mismatch");
    }

    #[test]
    fn test_compact_preserves_quoted_strings() {
        // Strings with spaces must keep their quotes and content intact
        let json = r#"{"items": [{"city": "New York, NY", "name": "Alice Smith"}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let compact = doc.to_tl_with_schemas_compact();
        assert!(compact.contains("\"New York, NY\""), "Quoted string must be preserved, got: {compact}");
        assert!(compact.contains("\"Alice Smith\""), "Quoted string must be preserved, got: {compact}");
    }

    #[test]
    fn test_compact_root_array_single_newline() {
        let json = r#"[1, 2, 3]"#;
        let doc = TeaLeaf::from_json(json).unwrap();
        let compact = doc.to_tl_with_schemas_compact();
        assert!(compact.starts_with("@root-array\n"), "got: {compact}");
        assert!(!compact.starts_with("@root-array\n\n"), "Should not have double newline in compact, got: {compact}");
    }

    #[test]
    fn test_compact_no_schemas_path() {
        // Documents without schemas should also compact correctly
        let mut data = IndexMap::new();
        let obj: IndexMap<String, Value> = vec![
            ("x".to_string(), Value::Int(1)),
            ("y".to_string(), Value::Int(2)),
        ].into_iter().collect();
        data.insert("point".to_string(), Value::Object(obj));
        data.insert("label".to_string(), Value::String("origin".to_string()));
        let doc = TeaLeaf {
            schemas: IndexMap::new(),
            unions: IndexMap::new(),
            data,
            is_root_array: false,
        };
        let compact = doc.to_tl_with_schemas_compact();
        assert!(compact.contains("point:{x:1,y:2}"), "got: {compact}");
        assert!(compact.contains("label:origin"), "got: {compact}");
    }

    #[test]
    fn test_compact_canonical_roundtrip() {
        // Verify compact output round-trips for all canonical samples
        let canonical_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../canonical/samples");
        let samples = [
            "primitives", "arrays", "objects", "schemas", "timestamps",
            "unicode_escaping", "numbers_extended", "refs_tags_maps",
            "special_types", "unions", "mixed_schemas", "large_data", "quoted_keys",
        ];
        for name in &samples {
            let path = canonical_dir.join(format!("{}.tl", name));
            if !path.exists() { continue; }
            let doc = TeaLeaf::load(&path).unwrap();
            let compact = doc.to_tl_with_schemas_compact();
            let reparsed = TeaLeaf::parse(&compact)
                .unwrap_or_else(|e| panic!("Failed to re-parse compact {name}: {e}\nCompact:\n{compact}"));
            let json1 = doc.to_json().unwrap();
            let json2 = reparsed.to_json().unwrap();
            let v1: serde_json::Value = serde_json::from_str(&json1).unwrap();
            let v2: serde_json::Value = serde_json::from_str(&json2).unwrap();
            assert_eq!(v1, v2, "Compact round-trip failed for {name}");
        }
    }

    #[test]
    fn test_schema_inference_with_at_prefixed_keys() {
        // JSON-LD style @type keys should trigger schema inference with quoted field names
        let json = r#"{"records": [
            {"@type": "MCAP", "name": "alpha"},
            {"@type": "DCAT", "name": "beta"}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // Should have inferred a schema with "@type" quoted
        assert!(tl_text.contains("@struct"), "Should infer a schema: {}", tl_text);
        assert!(tl_text.contains("\"@type\""), "Field @type should be quoted in schema: {}", tl_text);
        assert!(tl_text.contains("@table"), "Should use @table encoding: {}", tl_text);
    }

    #[test]
    fn test_schema_inference_quoted_field_roundtrip() {
        // Full JSON -> TL -> JSON roundtrip with @type keys
        let json = r#"{"records": [
            {"@type": "MCAP", "accessLevel": "public"},
            {"@type": "DCAT", "accessLevel": "restricted"}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // Parse TL back and convert to JSON
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse TL with quoted fields: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();

        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Roundtrip failed.\nTL:\n{tl_text}\nJSON out:\n{json_out}");
    }

    #[test]
    fn test_schema_inference_skips_when_schema_name_needs_quoting() {
        // When the inferred schema name itself would need quoting, skip inference
        let json = r#"{"@items": [{"name": "x"}, {"name": "y"}]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // Should NOT have inferred a schema because "@items" -> "@item" needs quoting
        assert!(!tl_text.contains("@struct"), "Should NOT infer schema when name needs quoting: {}", tl_text);
        assert!(!tl_text.contains("@table"), "Should NOT use @table when name needs quoting: {}", tl_text);
    }

    #[test]
    fn test_schema_inference_root_array_with_at_keys() {
        // Root-level array with @type keys should also get schema inference
        let json = r#"[
            {"@type": "MCAP", "issued": "2026-01-27"},
            {"@type": "DCAT", "issued": "2026-02-01"}
        ]"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Root array should infer schema: {}", tl_text);
        assert!(tl_text.contains("\"@type\""), "Field @type should be quoted: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Root array roundtrip failed");
    }

    #[test]
    fn test_schema_inference_dollar_prefixed_keys() {
        // JSON Schema / OpenAPI style $ref, $id, $schema keys
        let json = r##"{"definitions": [
            {"$ref": "#/components/User", "$id": "def1", "name": "UserRef"},
            {"$ref": "#/components/Order", "$id": "def2", "name": "OrderRef"}
        ]}"##;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema with $-prefixed keys: {}", tl_text);
        assert!(tl_text.contains("\"$ref\""), "$ref should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"$id\""), "$id should be quoted: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Roundtrip failed for $-prefixed keys");
    }

    #[test]
    fn test_schema_inference_hash_prefixed_keys() {
        // XML-to-JSON style #text, #cdata keys
        let json = r##"{"nodes": [
            {"#text": "Hello world", "tag": "p", "#comment": "intro"},
            {"#text": "Goodbye", "tag": "span", "#comment": "outro"}
        ]}"##;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema with #-prefixed keys: {}", tl_text);
        assert!(tl_text.contains("\"#text\""), "#text should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"#comment\""), "#comment should be quoted: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Roundtrip failed for #-prefixed keys");
    }

    #[test]
    fn test_schema_inference_colon_in_keys() {
        // XML namespace style keys like xsi:type, dc:title
        let json = r#"{"elements": [
            {"xsi:type": "string", "dc:title": "Document A", "id": 1},
            {"xsi:type": "int", "dc:title": "Document B", "id": 2}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema with colon keys: {}", tl_text);
        assert!(tl_text.contains("\"xsi:type\""), "xsi:type should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"dc:title\""), "dc:title should be quoted: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Roundtrip failed for colon keys");
    }

    #[test]
    fn test_schema_inference_odata_keys() {
        // OData style @odata.type, @odata.id keys
        let json = r##"{"results": [
            {"@odata.type": "#Microsoft.Graph.User", "@odata.id": "users/1", "displayName": "Alice"},
            {"@odata.type": "#Microsoft.Graph.User", "@odata.id": "users/2", "displayName": "Bob"}
        ]}"##;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema with OData keys: {}", tl_text);
        assert!(tl_text.contains("\"@odata.type\""), "@odata.type should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"@odata.id\""), "@odata.id should be quoted: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Roundtrip failed for OData keys");
    }

    #[test]
    fn test_schema_inference_uri_keys() {
        // RDF/JSON style with full URI keys
        let json = r#"{"triples": [
            {"http://schema.org/name": "Alice", "http://schema.org/age": "30", "id": "s1"},
            {"http://schema.org/name": "Bob", "http://schema.org/age": "25", "id": "s2"}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema with URI keys: {}", tl_text);
        assert!(tl_text.contains("\"http://schema.org/name\""), "URI key should be quoted: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Roundtrip failed for URI keys");
    }

    #[test]
    fn test_schema_inference_space_in_keys() {
        // Keys with spaces (common in human-friendly exports, spreadsheet-to-JSON)
        let json = r#"{"rows": [
            {"First Name": "Alice", "Last Name": "Smith", "age": 30},
            {"First Name": "Bob", "Last Name": "Jones", "age": 25}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema with space keys: {}", tl_text);
        assert!(tl_text.contains("\"First Name\""), "Space key should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"Last Name\""), "Space key should be quoted: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Roundtrip failed for space keys");
    }

    #[test]
    fn test_schema_inference_mixed_special_keys() {
        // Mix of regular and special-character keys in one schema
        let json = r#"{"catalog": [
            {"@type": "Product", "$id": "p1", "name": "Widget", "sku:code": "W-100"},
            {"@type": "Product", "$id": "p2", "name": "Gadget", "sku:code": "G-200"}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema with mixed keys: {}", tl_text);
        assert!(tl_text.contains("\"@type\""), "@type should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"$id\""), "$id should be quoted: {}", tl_text);
        assert!(tl_text.contains("\"sku:code\""), "sku:code should be quoted: {}", tl_text);
        // Regular key should NOT be quoted
        assert!(!tl_text.contains("\"name\""), "Regular key should not be quoted: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v1: serde_json::Value = serde_json::from_str(json).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        assert_eq!(v1, v2, "Roundtrip failed for mixed special keys");
    }

    // ---- Optional/nullable field inference tests ----

    #[test]
    fn test_schema_inference_optional_fields() {
        // Objects with different field sets — 'c' only in first object
        let json = r#"{"items": [
            {"a": 1, "b": 2, "c": 3},
            {"a": 4, "b": 5}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema: {}", tl_text);
        // 'c' should be nullable since it's missing from second object
        assert!(tl_text.contains("c: int?"), "Field 'c' should be nullable: {}", tl_text);
        // 'a' and 'b' should NOT be nullable (check they are NOT followed by ?)
        assert!(tl_text.contains("a: int,") || tl_text.contains("a: int)"), "'a' should not be nullable: {}", tl_text);
        assert!(tl_text.contains("b: int,") || tl_text.contains("b: int)"), "'b' should not be nullable: {}", tl_text);
        // Should use @table format
        assert!(tl_text.contains("@table"), "Should use @table: {}", tl_text);
        // Second row should have ~ for missing 'c'
        assert!(tl_text.contains("~"), "Missing field should produce ~: {}", tl_text);
    }

    #[test]
    fn test_schema_inference_optional_fields_roundtrip() {
        let json = r#"{"items": [
            {"a": 1, "b": "hello", "c": true},
            {"a": 2, "b": "world"},
            {"a": 3, "b": "foo", "c": false}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // Roundtrip: TL -> JSON should reconstruct missing fields as null
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v_out: serde_json::Value = serde_json::from_str(&json_out).unwrap();

        // Original items
        let v_in: serde_json::Value = serde_json::from_str(json).unwrap();
        let items_in = v_in["items"].as_array().unwrap();
        let items_out = v_out["items"].as_array().unwrap();

        assert_eq!(items_in.len(), items_out.len(), "Array length mismatch");

        // First and third items should match exactly
        assert_eq!(items_in[0]["a"], items_out[0]["a"]);
        assert_eq!(items_in[0]["b"], items_out[0]["b"]);
        assert_eq!(items_in[0]["c"], items_out[0]["c"]);
        assert_eq!(items_in[2]["c"], items_out[2]["c"]);

        // Second item: 'a' and 'b' match, 'c' should be absent (was missing)
        assert_eq!(items_in[1]["a"], items_out[1]["a"]);
        assert_eq!(items_in[1]["b"], items_out[1]["b"]);
        assert!(items_out[1].get("c").is_none(), "Missing field should remain absent after roundtrip");
    }

    #[test]
    fn test_schema_inference_no_common_fields_skipped() {
        // No fields in common — should NOT infer a schema
        let json = r#"{"items": [
            {"x": 1},
            {"y": 2}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(!tl_text.contains("@struct"), "Should NOT infer schema when no common fields: {}", tl_text);
        assert!(!tl_text.contains("@table"), "Should NOT use @table: {}", tl_text);
    }

    #[test]
    fn test_schema_inference_single_common_field() {
        // Only 'id' is shared — should infer schema with optional fields
        let json = r#"{"items": [
            {"id": 1, "a": "x"},
            {"id": 2, "b": "y"}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema with 1 common field: {}", tl_text);
        assert!(tl_text.contains("@table"), "Should use @table: {}", tl_text);
        // 'a' and 'b' should be nullable
        assert!(tl_text.contains("a: string?"), "'a' should be nullable: {}", tl_text);
        assert!(tl_text.contains("b: string?"), "'b' should be nullable: {}", tl_text);
    }

    #[test]
    fn test_schema_inference_optional_nested_array() {
        // Nested array field present in some objects but not others
        let json = r#"{"records": [
            {"name": "Alice", "tags": ["a", "b"]},
            {"name": "Bob"},
            {"name": "Carol", "tags": ["c"]}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema: {}", tl_text);
        assert!(tl_text.contains("tags: []string?"), "Optional array field should be nullable: {}", tl_text);
    }

    #[test]
    fn test_schema_inference_optional_nested_object() {
        // Nested object field present in some objects but not others
        let json = r#"{"people": [
            {"name": "Alice", "address": {"city": "Seattle", "state": "WA"}},
            {"name": "Bob"},
            {"name": "Carol", "address": {"city": "Portland", "state": "OR"}}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema: {}", tl_text);
        // The address field should be nullable
        assert!(tl_text.contains("?"), "Optional nested object should be nullable: {}", tl_text);

        // Roundtrip
        let reparsed = TeaLeaf::parse(&tl_text)
            .unwrap_or_else(|e| panic!("Failed to re-parse: {e}\nTL:\n{tl_text}"));
        let json_out = reparsed.to_json().unwrap();
        let v_out: serde_json::Value = serde_json::from_str(&json_out).unwrap();
        let people = v_out["people"].as_array().unwrap();
        assert_eq!(people[0]["address"]["city"], "Seattle");
        assert!(people[1].get("address").is_none(),
            "Missing address should be absent: {:?}", people[1]);
        assert_eq!(people[2]["address"]["city"], "Portland");
    }

    #[test]
    fn test_schema_inference_wa_health_data_pattern() {
        // Pattern matching the WA health dataset: many shared fields, some optional
        let json = r#"{"dataset": [
            {"@type": "dcat:Dataset", "accessLevel": "public", "identifier": "d1", "modified": "2024-01-01", "title": "Dataset A", "temporal": "2020/2024"},
            {"@type": "dcat:Dataset", "accessLevel": "public", "identifier": "d2", "modified": "2024-02-01", "title": "Dataset B", "theme": ["health"]},
            {"@type": "dcat:Dataset", "accessLevel": "public", "identifier": "d3", "modified": "2024-03-01", "title": "Dataset C", "temporal": "2021/2024", "theme": ["education"]}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        assert!(tl_text.contains("@struct"), "Should infer schema for DCAT-like data: {}", tl_text);
        assert!(tl_text.contains("@table"), "Should use @table: {}", tl_text);
        // Common fields should not be nullable
        assert!(tl_text.contains("\"@type\": string,"), "@type should be non-nullable: {}", tl_text);
        assert!(tl_text.contains("accessLevel: string,"), "accessLevel should be non-nullable: {}", tl_text);
        // Optional fields should be nullable
        assert!(tl_text.contains("temporal: string?"), "temporal should be nullable: {}", tl_text);
    }

    #[test]
    fn test_write_schemas_nullable_field_matching() {
        // Verify that write_value_with_schemas correctly applies @table
        // when objects are missing nullable fields
        let json = r#"{"users": [
            {"id": 1, "name": "Alice", "email": "alice@test.com"},
            {"id": 2, "name": "Bob"},
            {"id": 3, "name": "Carol", "email": "carol@test.com"}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // Should produce @table, not inline objects
        assert!(tl_text.contains("@table"), "Should use @table with nullable fields: {}", tl_text);
        assert!(!tl_text.contains("{id:"), "Should NOT fall back to inline objects: {}", tl_text);

        // Verify compact mode too
        let compact = doc.to_tl_with_schemas_compact();
        assert!(compact.contains("@table"), "Compact should also use @table: {}", compact);
    }

    #[test]
    fn test_schema_field_ordering_uses_most_complete_object() {
        // The most-complete object (most fields) should determine schema field order
        let json = r#"{"items": [
            {"a": 1, "b": 2},
            {"c": 3, "b": 4, "a": 5}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // The second object has 3 fields (most), so its order should be used: c, b, a
        assert!(tl_text.contains("@struct item (c: int?, b: int, a: int)"),
            "Schema should use most-complete object's field order: {}", tl_text);
    }

    #[test]
    fn test_schema_field_ordering_appends_extra_fields() {
        // Fields not in the most-complete object are appended at the end
        let json = r#"{"items": [
            {"x": 1, "y": 2, "z": 3},
            {"y": 4, "x": 5},
            {"x": 6, "y": 7, "w": 8}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // First and third objects tie at 3 fields; first encountered wins: x, y, z
        // Then 'w' is appended from the third object
        assert!(tl_text.contains("@struct item (x: int, y: int, z: int?, w: int?)"),
            "Schema should use first most-complete object's order with extras appended: {}", tl_text);
    }

    #[test]
    fn test_schema_field_ordering_roundtrip_preserves_order() {
        // Roundtrip should preserve the most-complete object's field ordering
        let json = r#"{"records": [
            {"name": "Alice", "age": 30, "email": "a@test.com"},
            {"age": 25, "name": "Bob"}
        ]}"#;
        let doc = TeaLeaf::from_json_with_schemas(json).unwrap();
        let tl_text = doc.to_tl_with_schemas();

        // Most-complete object (first, 3 fields): name, age, email
        let reparsed = TeaLeaf::parse(&tl_text).unwrap();
        let json_str = reparsed.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let records = parsed["records"].as_array().unwrap();
        let first_keys: Vec<&str> = records[0].as_object().unwrap().keys().map(|k| k.as_str()).collect();
        assert_eq!(first_keys, vec!["name", "age", "email"],
            "Roundtripped first record should preserve field order");
        let second_keys: Vec<&str> = records[1].as_object().unwrap().keys().map(|k| k.as_str()).collect();
        assert_eq!(second_keys, vec!["name", "age"],
            "Roundtripped second record should preserve field order (minus absent nullable)");
    }
}
