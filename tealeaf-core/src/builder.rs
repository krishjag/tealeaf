//! Builder for constructing TeaLeaf documents from multiple DTOs.

use std::collections::HashMap;

use crate::convert::ToTeaLeaf;
use crate::{Schema, TeaLeaf, Value};

/// Builder for constructing TeaLeaf documents from multiple DTOs.
///
/// # Example
///
/// ```ignore
/// use tealeaf::TeaLeafBuilder;
///
/// let doc = TeaLeafBuilder::new()
///     .add("config", &config)
///     .add_vec("users", &users)
///     .build();
/// ```
pub struct TeaLeafBuilder {
    schemas: HashMap<String, Schema>,
    data: HashMap<String, Value>,
    is_root_array: bool,
}

impl TeaLeafBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            data: HashMap::new(),
            is_root_array: false,
        }
    }

    /// Add a single DTO under the given key.
    pub fn add<T: ToTeaLeaf>(mut self, key: &str, dto: &T) -> Self {
        self.schemas.extend(T::collect_schemas());
        self.data
            .insert(key.to_string(), dto.to_tealeaf_value());
        self
    }

    /// Add a raw Value under the given key (no schema collection).
    pub fn add_value(mut self, key: &str, value: Value) -> Self {
        self.data.insert(key.to_string(), value);
        self
    }

    /// Add a schema definition.
    pub fn add_schema(mut self, schema: Schema) -> Self {
        self.schemas.insert(schema.name.clone(), schema);
        self
    }

    /// Add a Vec of DTOs as an array under the given key.
    pub fn add_vec<T: ToTeaLeaf>(mut self, key: &str, items: &[T]) -> Self {
        self.schemas.extend(T::collect_schemas());
        let arr = Value::Array(items.iter().map(|i| i.to_tealeaf_value()).collect());
        self.data.insert(key.to_string(), arr);
        self
    }

    /// Mark the document as a root array (for JSON round-trip fidelity).
    pub fn root_array(mut self) -> Self {
        self.is_root_array = true;
        self
    }

    /// Build the TeaLeaf document.
    pub fn build(self) -> TeaLeaf {
        let mut doc = TeaLeaf::new(self.schemas, self.data);
        doc.set_root_array(self.is_root_array);
        doc
    }
}

impl Default for TeaLeafBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FieldType;

    #[test]
    fn test_builder_basic() {
        let doc = TeaLeafBuilder::new()
            .add_value("name", Value::String("test".into()))
            .add_value("count", Value::Int(42))
            .build();

        assert_eq!(
            doc.get("name"),
            Some(&Value::String("test".into()))
        );
        assert_eq!(doc.get("count"), Some(&Value::Int(42)));
    }

    #[test]
    fn test_builder_with_schema() {
        let schema = crate::Schema::new("point")
            .field("x", FieldType::new("float"))
            .field("y", FieldType::new("float"));

        let doc = TeaLeafBuilder::new()
            .add_schema(schema)
            .add_value("origin", Value::Object({
                let mut m = HashMap::new();
                m.insert("x".to_string(), Value::Float(0.0));
                m.insert("y".to_string(), Value::Float(0.0));
                m
            }))
            .build();

        assert!(doc.schema("point").is_some());
        assert!(doc.get("origin").is_some());
    }

    #[test]
    fn test_builder_default() {
        let doc = TeaLeafBuilder::default().build();
        assert!(doc.get("anything").is_none());
    }
}
