//! DTO conversion traits for TeaLeaf documents
//!
//! This module provides the `ToTeaLeaf` and `FromTeaLeaf` traits for converting
//! between Rust types and TeaLeaf `Value`s, along with automatic schema collection.

use std::collections::HashMap;
use std::fmt;

use crate::{Error, FieldType, Schema, Value};

// =============================================================================
// ConvertError
// =============================================================================

/// Errors that occur during DTO-to-TeaLeaf conversion
#[derive(Debug)]
pub enum ConvertError {
    /// A required field was missing from the TeaLeaf Value
    MissingField {
        struct_name: String,
        field: String,
    },
    /// A Value variant did not match the expected Rust type
    TypeMismatch {
        expected: String,
        got: String,
        path: String,
    },
    /// A nested conversion failed
    Nested {
        path: String,
        source: Box<ConvertError>,
    },
    /// A custom error message
    Custom(String),
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::MissingField { struct_name, field } => {
                write!(f, "Missing field '{}' in struct '{}'", field, struct_name)
            }
            ConvertError::TypeMismatch {
                expected,
                got,
                path,
            } => {
                write!(
                    f,
                    "Type mismatch at '{}': expected {}, got {}",
                    path, expected, got
                )
            }
            ConvertError::Nested { path, source } => {
                write!(f, "At '{}': {}", path, source)
            }
            ConvertError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ConvertError {}

impl From<ConvertError> for Error {
    fn from(e: ConvertError) -> Self {
        Error::ParseError(e.to_string())
    }
}

// =============================================================================
// Core Traits
// =============================================================================

/// Convert a Rust type into a TeaLeaf `Value` and collect associated schemas.
pub trait ToTeaLeaf {
    /// Convert this value to a TeaLeaf `Value`.
    fn to_tealeaf_value(&self) -> Value;

    /// Collect all schemas required by this type and its nested types.
    ///
    /// Returns a map from schema name to Schema definition.
    /// Default implementation returns an empty map (for primitives).
    fn collect_schemas() -> HashMap<String, Schema> {
        HashMap::new()
    }

    /// The TeaLeaf field type that represents this type in a schema.
    fn tealeaf_field_type() -> FieldType;
}

/// Convert a TeaLeaf `Value` back into a Rust type.
pub trait FromTeaLeaf: Sized {
    /// Attempt to reconstruct this type from a TeaLeaf `Value`.
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError>;
}

/// Extension trait providing convenience methods on types implementing `ToTeaLeaf`.
pub trait ToTeaLeafExt: ToTeaLeaf + Sized {
    /// Convert to a TeaLeaf document with a single root entry.
    fn to_tealeaf_doc(&self, key: &str) -> crate::TeaLeaf {
        crate::TeaLeaf::from_dto(key, self)
    }

    /// Convert to TeaLeaf text format string.
    fn to_tl_string(&self, key: &str) -> String {
        self.to_tealeaf_doc(key).to_tl_with_schemas()
    }

    /// Compile to binary .tlbx format.
    fn to_tlbx(
        &self,
        key: &str,
        path: impl AsRef<std::path::Path>,
        compress: bool,
    ) -> crate::Result<()> {
        self.to_tealeaf_doc(key).compile(path, compress)
    }

    /// Convert to JSON string (pretty-printed).
    fn to_tealeaf_json(&self, key: &str) -> crate::Result<String> {
        self.to_tealeaf_doc(key).to_json()
    }
}

// Blanket implementation: any ToTeaLeaf type gets convenience methods
impl<T: ToTeaLeaf> ToTeaLeafExt for T {}

// =============================================================================
// Primitive ToTeaLeaf Implementations
// =============================================================================

impl ToTeaLeaf for bool {
    fn to_tealeaf_value(&self) -> Value {
        Value::Bool(*self)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("bool")
    }
}

impl ToTeaLeaf for i8 {
    fn to_tealeaf_value(&self) -> Value {
        Value::Int(*self as i64)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("int8")
    }
}

impl ToTeaLeaf for i16 {
    fn to_tealeaf_value(&self) -> Value {
        Value::Int(*self as i64)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("int16")
    }
}

impl ToTeaLeaf for i32 {
    fn to_tealeaf_value(&self) -> Value {
        Value::Int(*self as i64)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("int")
    }
}

impl ToTeaLeaf for i64 {
    fn to_tealeaf_value(&self) -> Value {
        Value::Int(*self)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("int64")
    }
}

impl ToTeaLeaf for u8 {
    fn to_tealeaf_value(&self) -> Value {
        Value::UInt(*self as u64)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("uint8")
    }
}

impl ToTeaLeaf for u16 {
    fn to_tealeaf_value(&self) -> Value {
        Value::UInt(*self as u64)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("uint16")
    }
}

impl ToTeaLeaf for u32 {
    fn to_tealeaf_value(&self) -> Value {
        Value::UInt(*self as u64)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("uint")
    }
}

impl ToTeaLeaf for u64 {
    fn to_tealeaf_value(&self) -> Value {
        Value::UInt(*self)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("uint64")
    }
}

impl ToTeaLeaf for f32 {
    fn to_tealeaf_value(&self) -> Value {
        Value::Float(*self as f64)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("float32")
    }
}

impl ToTeaLeaf for f64 {
    fn to_tealeaf_value(&self) -> Value {
        Value::Float(*self)
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("float")
    }
}

impl ToTeaLeaf for String {
    fn to_tealeaf_value(&self) -> Value {
        Value::String(self.clone())
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("string")
    }
}

impl ToTeaLeaf for &str {
    fn to_tealeaf_value(&self) -> Value {
        Value::String(self.to_string())
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("string")
    }
}

// Vec<u8> is special: maps to Bytes, not Array
impl ToTeaLeaf for Vec<u8> {
    fn to_tealeaf_value(&self) -> Value {
        Value::Bytes(self.clone())
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("bytes")
    }
}

// =============================================================================
// Generic ToTeaLeaf Implementations
// =============================================================================

impl<T: ToTeaLeaf> ToTeaLeaf for Option<T> {
    fn to_tealeaf_value(&self) -> Value {
        match self {
            Some(v) => v.to_tealeaf_value(),
            None => Value::Null,
        }
    }
    fn collect_schemas() -> HashMap<String, Schema> {
        T::collect_schemas()
    }
    fn tealeaf_field_type() -> FieldType {
        T::tealeaf_field_type().nullable()
    }
}

/// Marker trait to exclude `u8` from generic `Vec<T>` impl.
/// `Vec<u8>` has its own specialization mapping to `Value::Bytes`.
pub trait NotU8 {}
impl NotU8 for bool {}
impl NotU8 for i8 {}
impl NotU8 for i16 {}
impl NotU8 for i32 {}
impl NotU8 for i64 {}
impl NotU8 for u16 {}
impl NotU8 for u32 {}
impl NotU8 for u64 {}
impl NotU8 for f32 {}
impl NotU8 for f64 {}
impl NotU8 for String {}
impl<T> NotU8 for Vec<T> {}
impl<T> NotU8 for Option<T> {}
impl<K, V> NotU8 for HashMap<K, V> {}
impl<T> NotU8 for Box<T> {}
impl<T> NotU8 for std::sync::Arc<T> {}
impl<T> NotU8 for std::rc::Rc<T> {}

impl<T: ToTeaLeaf + NotU8> ToTeaLeaf for Vec<T> {
    fn to_tealeaf_value(&self) -> Value {
        Value::Array(self.iter().map(|v| v.to_tealeaf_value()).collect())
    }
    fn collect_schemas() -> HashMap<String, Schema> {
        T::collect_schemas()
    }
    fn tealeaf_field_type() -> FieldType {
        T::tealeaf_field_type().array()
    }
}

impl<V: ToTeaLeaf> ToTeaLeaf for HashMap<String, V> {
    fn to_tealeaf_value(&self) -> Value {
        Value::Object(
            self.iter()
                .map(|(k, v)| (k.clone(), v.to_tealeaf_value()))
                .collect(),
        )
    }
    fn collect_schemas() -> HashMap<String, Schema> {
        V::collect_schemas()
    }
    fn tealeaf_field_type() -> FieldType {
        FieldType::new("object")
    }
}

// Transparent wrappers
impl<T: ToTeaLeaf> ToTeaLeaf for Box<T> {
    fn to_tealeaf_value(&self) -> Value {
        (**self).to_tealeaf_value()
    }
    fn collect_schemas() -> HashMap<String, Schema> {
        T::collect_schemas()
    }
    fn tealeaf_field_type() -> FieldType {
        T::tealeaf_field_type()
    }
}

impl<T: ToTeaLeaf> ToTeaLeaf for std::sync::Arc<T> {
    fn to_tealeaf_value(&self) -> Value {
        (**self).to_tealeaf_value()
    }
    fn collect_schemas() -> HashMap<String, Schema> {
        T::collect_schemas()
    }
    fn tealeaf_field_type() -> FieldType {
        T::tealeaf_field_type()
    }
}

impl<T: ToTeaLeaf> ToTeaLeaf for std::rc::Rc<T> {
    fn to_tealeaf_value(&self) -> Value {
        (**self).to_tealeaf_value()
    }
    fn collect_schemas() -> HashMap<String, Schema> {
        T::collect_schemas()
    }
    fn tealeaf_field_type() -> FieldType {
        T::tealeaf_field_type()
    }
}

// Tuple implementations (2 through 6 elements)
macro_rules! impl_to_tealeaf_tuple {
    ($($idx:tt: $T:ident),+) => {
        impl<$($T: ToTeaLeaf),+> ToTeaLeaf for ($($T,)+) {
            fn to_tealeaf_value(&self) -> Value {
                Value::Array(vec![$(self.$idx.to_tealeaf_value()),+])
            }
            fn collect_schemas() -> HashMap<String, Schema> {
                let mut schemas = HashMap::new();
                $(schemas.extend($T::collect_schemas());)+
                schemas
            }
            fn tealeaf_field_type() -> FieldType {
                FieldType::new("tuple")
            }
        }
    };
}

impl_to_tealeaf_tuple!(0: A, 1: B);
impl_to_tealeaf_tuple!(0: A, 1: B, 2: C);
impl_to_tealeaf_tuple!(0: A, 1: B, 2: C, 3: D);
impl_to_tealeaf_tuple!(0: A, 1: B, 2: C, 3: D, 4: E);
impl_to_tealeaf_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F);

// =============================================================================
// Primitive FromTeaLeaf Implementations
// =============================================================================

impl FromTeaLeaf for bool {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value.as_bool().ok_or_else(|| ConvertError::TypeMismatch {
            expected: "bool".into(),
            got: format!("{:?}", value.tl_type()),
            path: String::new(),
        })
    }
}

impl FromTeaLeaf for i8 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_int()
            .map(|i| i as i8)
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "int8".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

impl FromTeaLeaf for i16 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_int()
            .map(|i| i as i16)
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "int16".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

impl FromTeaLeaf for i32 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_int()
            .map(|i| i as i32)
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "int".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

impl FromTeaLeaf for i64 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value.as_int().ok_or_else(|| ConvertError::TypeMismatch {
            expected: "int64".into(),
            got: format!("{:?}", value.tl_type()),
            path: String::new(),
        })
    }
}

impl FromTeaLeaf for u8 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_uint()
            .map(|u| u as u8)
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "uint8".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

impl FromTeaLeaf for u16 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_uint()
            .map(|u| u as u16)
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "uint16".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

impl FromTeaLeaf for u32 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_uint()
            .map(|u| u as u32)
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "uint".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

impl FromTeaLeaf for u64 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value.as_uint().ok_or_else(|| ConvertError::TypeMismatch {
            expected: "uint64".into(),
            got: format!("{:?}", value.tl_type()),
            path: String::new(),
        })
    }
}

impl FromTeaLeaf for f32 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_float()
            .map(|f| f as f32)
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "float32".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

impl FromTeaLeaf for f64 {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value.as_float().ok_or_else(|| ConvertError::TypeMismatch {
            expected: "float".into(),
            got: format!("{:?}", value.tl_type()),
            path: String::new(),
        })
    }
}

impl FromTeaLeaf for String {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "string".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

// Vec<u8> from Bytes
impl FromTeaLeaf for Vec<u8> {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        value
            .as_bytes()
            .map(|b| b.to_vec())
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "bytes".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })
    }
}

// =============================================================================
// Generic FromTeaLeaf Implementations
// =============================================================================

impl<T: FromTeaLeaf> FromTeaLeaf for Option<T> {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        if value.is_null() {
            Ok(None)
        } else {
            T::from_tealeaf_value(value).map(Some)
        }
    }
}

impl<T: FromTeaLeaf + NotU8> FromTeaLeaf for Vec<T> {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        let arr = value.as_array().ok_or_else(|| ConvertError::TypeMismatch {
            expected: "array".into(),
            got: format!("{:?}", value.tl_type()),
            path: String::new(),
        })?;
        arr.iter()
            .enumerate()
            .map(|(i, v)| {
                T::from_tealeaf_value(v).map_err(|e| ConvertError::Nested {
                    path: format!("[{}]", i),
                    source: Box::new(e),
                })
            })
            .collect()
    }
}

impl<V: FromTeaLeaf> FromTeaLeaf for HashMap<String, V> {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        let obj = value
            .as_object()
            .ok_or_else(|| ConvertError::TypeMismatch {
                expected: "object".into(),
                got: format!("{:?}", value.tl_type()),
                path: String::new(),
            })?;
        let mut map = HashMap::new();
        for (k, v) in obj {
            let val = V::from_tealeaf_value(v).map_err(|e| ConvertError::Nested {
                path: k.clone(),
                source: Box::new(e),
            })?;
            map.insert(k.clone(), val);
        }
        Ok(map)
    }
}

impl<T: FromTeaLeaf> FromTeaLeaf for Box<T> {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        T::from_tealeaf_value(value).map(Box::new)
    }
}

impl<T: FromTeaLeaf> FromTeaLeaf for std::sync::Arc<T> {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        T::from_tealeaf_value(value).map(std::sync::Arc::new)
    }
}

impl<T: FromTeaLeaf> FromTeaLeaf for std::rc::Rc<T> {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
        T::from_tealeaf_value(value).map(std::rc::Rc::new)
    }
}

// Tuple FromTeaLeaf implementations
macro_rules! impl_from_tealeaf_tuple {
    ($($idx:tt: $T:ident),+) => {
        impl<$($T: FromTeaLeaf),+> FromTeaLeaf for ($($T,)+) {
            fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError> {
                let arr = value.as_array().ok_or_else(|| ConvertError::TypeMismatch {
                    expected: "tuple (array)".into(),
                    got: format!("{:?}", value.tl_type()),
                    path: String::new(),
                })?;
                Ok(($(
                    $T::from_tealeaf_value(
                        arr.get($idx).ok_or_else(|| ConvertError::MissingField {
                            struct_name: "tuple".into(),
                            field: format!("index {}", $idx),
                        })?
                    ).map_err(|e| ConvertError::Nested {
                        path: format!("[{}]", $idx),
                        source: Box::new(e),
                    })?,
                )+))
            }
        }
    };
}

impl_from_tealeaf_tuple!(0: A, 1: B);
impl_from_tealeaf_tuple!(0: A, 1: B, 2: C);
impl_from_tealeaf_tuple!(0: A, 1: B, 2: C, 3: D);
impl_from_tealeaf_tuple!(0: A, 1: B, 2: C, 3: D, 4: E);
impl_from_tealeaf_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F);

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_to_tealeaf() {
        assert_eq!(42i64.to_tealeaf_value(), Value::Int(42));
        assert_eq!(true.to_tealeaf_value(), Value::Bool(true));
        assert_eq!("hello".to_tealeaf_value(), Value::String("hello".into()));
        assert_eq!(3.14f64.to_tealeaf_value(), Value::Float(3.14));
        assert_eq!(42u32.to_tealeaf_value(), Value::UInt(42));
    }

    #[test]
    fn test_primitive_from_tealeaf() {
        assert_eq!(i64::from_tealeaf_value(&Value::Int(42)).unwrap(), 42);
        assert_eq!(
            bool::from_tealeaf_value(&Value::Bool(true)).unwrap(),
            true
        );
        assert_eq!(
            String::from_tealeaf_value(&Value::String("hi".into())).unwrap(),
            "hi"
        );
        assert_eq!(
            f64::from_tealeaf_value(&Value::Float(3.14)).unwrap(),
            3.14
        );
    }

    #[test]
    fn test_option_roundtrip() {
        let some: Option<i64> = Some(42);
        let none: Option<i64> = None;
        assert_eq!(some.to_tealeaf_value(), Value::Int(42));
        assert_eq!(none.to_tealeaf_value(), Value::Null);
        assert_eq!(
            Option::<i64>::from_tealeaf_value(&Value::Int(42)).unwrap(),
            Some(42)
        );
        assert_eq!(
            Option::<i64>::from_tealeaf_value(&Value::Null).unwrap(),
            None
        );
    }

    #[test]
    fn test_vec_roundtrip() {
        let v = vec![1i64, 2, 3];
        let val = v.to_tealeaf_value();
        assert_eq!(
            val,
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
        assert_eq!(Vec::<i64>::from_tealeaf_value(&val).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_vec_u8_as_bytes() {
        let v: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let val = v.to_tealeaf_value();
        assert_eq!(val, Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
        assert_eq!(Vec::<u8>::from_tealeaf_value(&val).unwrap(), v);
    }

    #[test]
    fn test_hashmap_roundtrip() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), 42i64);
        let val = map.to_tealeaf_value();
        let restored = HashMap::<String, i64>::from_tealeaf_value(&val).unwrap();
        assert_eq!(restored.get("key"), Some(&42));
    }

    #[test]
    fn test_field_type_primitives() {
        assert_eq!(i32::tealeaf_field_type(), FieldType::new("int"));
        assert_eq!(String::tealeaf_field_type(), FieldType::new("string"));
        assert_eq!(
            Option::<i32>::tealeaf_field_type(),
            FieldType::new("int").nullable()
        );
        assert_eq!(
            Vec::<String>::tealeaf_field_type(),
            FieldType::new("string").array()
        );
        assert_eq!(Vec::<u8>::tealeaf_field_type(), FieldType::new("bytes"));
    }

    #[test]
    fn test_type_mismatch_error() {
        let err = i64::from_tealeaf_value(&Value::String("oops".into())).unwrap_err();
        assert!(matches!(err, ConvertError::TypeMismatch { .. }));
    }

    #[test]
    fn test_box_transparent() {
        let boxed = Box::new(42i64);
        assert_eq!(boxed.to_tealeaf_value(), Value::Int(42));
        assert_eq!(
            Box::<i64>::from_tealeaf_value(&Value::Int(42)).unwrap(),
            Box::new(42)
        );
    }

    #[test]
    fn test_tuple_roundtrip() {
        let t = (1i64, "hello".to_string());
        let val = t.to_tealeaf_value();
        assert_eq!(
            val,
            Value::Array(vec![Value::Int(1), Value::String("hello".into())])
        );
        let restored = <(i64, String)>::from_tealeaf_value(&val).unwrap();
        assert_eq!(restored, (1, "hello".to_string()));
    }

    #[test]
    fn test_nested_option_vec() {
        let v: Option<Vec<i32>> = Some(vec![1, 2, 3]);
        let val = v.to_tealeaf_value();
        assert_eq!(
            val,
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
        let restored = Option::<Vec<i32>>::from_tealeaf_value(&val).unwrap();
        assert_eq!(restored, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_convert_error_display() {
        let err = ConvertError::MissingField {
            struct_name: "User".into(),
            field: "name".into(),
        };
        assert_eq!(err.to_string(), "Missing field 'name' in struct 'User'");

        let err = ConvertError::TypeMismatch {
            expected: "int".into(),
            got: "String".into(),
            path: "User.age".into(),
        };
        assert_eq!(
            err.to_string(),
            "Type mismatch at 'User.age': expected int, got String"
        );
    }
}
