//! Core types for TeaLeaf

use std::collections::HashMap;
use std::fmt;
use std::io;
use indexmap::IndexMap;

/// Ordered map type for object fields — preserves insertion order.
pub type ObjectMap<K, V> = IndexMap<K, V>;

// =============================================================================
// Constants
// =============================================================================

pub const MAGIC: [u8; 4] = *b"TLBX";
/// Binary format version (major) - for compatibility checks
pub const VERSION_MAJOR: u16 = 2;
/// Binary format version (minor) - for compatibility checks
pub const VERSION_MINOR: u16 = 0;
/// Library version string (beta/RFC stage)
pub const VERSION: &str = "2.0.0-beta.10";
pub const HEADER_SIZE: usize = 64;
/// Maximum length of a string in the string table (u32 encoding limit)
pub const MAX_STRING_LENGTH: usize = u32::MAX as usize;
/// Maximum number of fields in an object/struct (u16 encoding limit)
pub const MAX_OBJECT_FIELDS: usize = u16::MAX as usize;
/// Maximum number of elements in an array (u32 encoding limit)
pub const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

// =============================================================================
// Error Type
// =============================================================================

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    InvalidMagic,
    InvalidVersion { major: u16, minor: u16 },
    InvalidType(u8),
    InvalidUtf8,
    UnexpectedToken { expected: String, got: String },
    UnexpectedEof,
    UnknownStruct(String),
    MissingField(String),
    ParseError(String),
    ValueOutOfRange(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::InvalidMagic => write!(f, "Invalid TeaLeaf magic bytes"),
            Error::InvalidVersion { major, minor } => {
                write!(f, "Unsupported version: {}.{}", major, minor)
            }
            Error::InvalidType(t) => write!(f, "Invalid type code: 0x{:02X}", t),
            Error::InvalidUtf8 => write!(f, "Invalid UTF-8"),
            Error::UnexpectedToken { expected, got } => {
                write!(f, "Expected {}, got {}", expected, got)
            }
            Error::UnexpectedEof => write!(f, "Unexpected end of input"),
            Error::UnknownStruct(s) => write!(f, "Unknown struct: {}", s),
            Error::MissingField(s) => write!(f, "Missing field: {}", s),
            Error::ParseError(s) => write!(f, "Parse error: {}", s),
            Error::ValueOutOfRange(s) => write!(f, "Value out of range: {}", s),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// =============================================================================
// Type Codes
// =============================================================================

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TLType {
    Null = 0x00,
    Bool = 0x01,
    Int8 = 0x02,
    Int16 = 0x03,
    Int32 = 0x04,
    Int64 = 0x05,
    UInt8 = 0x06,
    UInt16 = 0x07,
    UInt32 = 0x08,
    UInt64 = 0x09,
    Float32 = 0x0A,
    Float64 = 0x0B,
    String = 0x10,
    Bytes = 0x11,
    JsonNumber = 0x12,
    Array = 0x20,
    Object = 0x21,
    Struct = 0x22,
    Map = 0x23,
    Tuple = 0x24,
    Ref = 0x30,
    Tagged = 0x31,
    Timestamp = 0x32,
}

impl TryFrom<u8> for TLType {
    type Error = Error;

    fn try_from(v: u8) -> Result<Self> {
        match v {
            0x00 => Ok(Self::Null),
            0x01 => Ok(Self::Bool),
            0x02 => Ok(Self::Int8),
            0x03 => Ok(Self::Int16),
            0x04 => Ok(Self::Int32),
            0x05 => Ok(Self::Int64),
            0x06 => Ok(Self::UInt8),
            0x07 => Ok(Self::UInt16),
            0x08 => Ok(Self::UInt32),
            0x09 => Ok(Self::UInt64),
            0x0A => Ok(Self::Float32),
            0x0B => Ok(Self::Float64),
            0x10 => Ok(Self::String),
            0x11 => Ok(Self::Bytes),
            0x12 => Ok(Self::JsonNumber),
            0x20 => Ok(Self::Array),
            0x21 => Ok(Self::Object),
            0x22 => Ok(Self::Struct),
            0x23 => Ok(Self::Map),
            0x24 => Ok(Self::Tuple),
            0x30 => Ok(Self::Ref),
            0x31 => Ok(Self::Tagged),
            0x32 => Ok(Self::Timestamp),
            _ => Err(Error::InvalidType(v)),
        }
    }
}

// =============================================================================
// Field Type
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct FieldType {
    pub base: String,
    pub nullable: bool,
    pub is_array: bool,
}

impl FieldType {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            nullable: false,
            is_array: false,
        }
    }

    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    pub fn array(mut self) -> Self {
        self.is_array = true;
        self
    }

    pub fn parse(s: &str) -> Self {
        let mut s = s.trim();
        let mut nullable = false;
        let mut is_array = false;

        // Check nullable
        if s.ends_with('?') {
            nullable = true;
            s = &s[..s.len() - 1];
        }

        // Check array
        if s.starts_with("[]") {
            is_array = true;
            s = &s[2..];
        }

        Self {
            base: s.to_string(),
            nullable,
            is_array,
        }
    }

    pub fn to_tl_type(&self) -> TLType {
        if self.is_array {
            return TLType::Array;
        }
        match self.base.as_str() {
            "bool" => TLType::Bool,
            "int8" => TLType::Int8,
            "int16" => TLType::Int16,
            "int" | "int32" => TLType::Int32,
            "int64" => TLType::Int64,
            "uint8" => TLType::UInt8,
            "uint16" => TLType::UInt16,
            "uint" | "uint32" => TLType::UInt32,
            "uint64" => TLType::UInt64,
            "float32" => TLType::Float32,
            "float" | "float64" => TLType::Float64,
            "string" => TLType::String,
            "bytes" => TLType::Bytes,
            "timestamp" => TLType::Timestamp,
            "object" => TLType::Object,
            "tuple" => TLType::Tuple,
            "map" => TLType::Map,
            _ => TLType::Struct, // Assume struct reference
        }
    }

    pub fn is_struct(&self) -> bool {
        !self.is_array && self.to_tl_type() == TLType::Struct
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_array {
            write!(f, "[]")?;
        }
        write!(f, "{}", self.base)?;
        if self.nullable {
            write!(f, "?")?;
        }
        Ok(())
    }
}

// =============================================================================
// Schema
// =============================================================================

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
}

impl Field {
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub fields: Vec<Field>,
}

impl Schema {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, name: impl Into<String>, field_type: FieldType) -> Self {
        self.fields.push(Field::new(name, field_type));
        self
    }

    pub fn add_field(&mut self, name: impl Into<String>, field_type: FieldType) {
        self.fields.push(Field::new(name, field_type));
    }

    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }
}

// =============================================================================
// Union (Discriminated Union)
// =============================================================================

/// A variant in a union type
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}

impl Variant {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, name: impl Into<String>, field_type: FieldType) -> Self {
        self.fields.push(Field::new(name, field_type));
        self
    }
}

/// A discriminated union type
#[derive(Debug, Clone)]
pub struct Union {
    pub name: String,
    pub variants: Vec<Variant>,
}

impl Union {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variants: Vec::new(),
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variants.push(variant);
        self
    }

    pub fn add_variant(&mut self, variant: Variant) {
        self.variants.push(variant);
    }

    pub fn get_variant(&self, name: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.name == name)
    }
}

// =============================================================================
// Value
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(ObjectMap<String, Value>),
    Map(Vec<(Value, Value)>),  // Key-value pairs preserving order
    Ref(String),
    Tagged(String, Box<Value>),
    Timestamp(i64, i16),  // Unix milliseconds, timezone offset in minutes
    JsonNumber(String),  // Arbitrary-precision number (raw decimal string)
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::UInt(u) if *u <= i64::MAX as u64 => Some(*u as i64),
            Value::JsonNumber(s) => s.parse::<i64>().ok(),
            _ => None,
        }
    }

    pub fn as_int_checked(&self) -> Result<i64> {
        match self {
            Value::Int(i) => Ok(*i),
            Value::UInt(u) if *u <= i64::MAX as u64 => Ok(*u as i64),
            Value::UInt(u) => Err(Error::ValueOutOfRange(
                format!("uint {} exceeds i64::MAX", u),
            )),
            Value::JsonNumber(s) => s.parse::<i64>().map_err(|_| Error::ValueOutOfRange(
                format!("json number '{}' does not fit in i64", s),
            )),
            _ => Err(Error::ValueOutOfRange(
                format!("cannot convert {:?} to i64", self),
            )),
        }
    }

    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Value::UInt(u) => Some(*u),
            Value::Int(i) if *i >= 0 => Some(*i as u64),
            Value::JsonNumber(s) => s.parse::<u64>().ok(),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::UInt(u) => Some(*u as f64),
            Value::JsonNumber(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            Value::JsonNumber(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&ObjectMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object()?.get(key)
    }

    pub fn index(&self, idx: usize) -> Option<&Value> {
        self.as_array()?.get(idx)
    }

    pub fn tl_type(&self) -> TLType {
        match self {
            Value::Null => TLType::Null,
            Value::Bool(_) => TLType::Bool,
            Value::Int(i) => {
                if *i >= i8::MIN as i64 && *i <= i8::MAX as i64 {
                    TLType::Int8
                } else if *i >= i16::MIN as i64 && *i <= i16::MAX as i64 {
                    TLType::Int16
                } else if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                    TLType::Int32
                } else {
                    TLType::Int64
                }
            }
            Value::UInt(u) => {
                if *u <= u8::MAX as u64 {
                    TLType::UInt8
                } else if *u <= u16::MAX as u64 {
                    TLType::UInt16
                } else if *u <= u32::MAX as u64 {
                    TLType::UInt32
                } else {
                    TLType::UInt64
                }
            }
            Value::Float(_) => TLType::Float64,
            Value::String(_) => TLType::String,
            Value::Bytes(_) => TLType::Bytes,
            Value::Array(_) => TLType::Array,
            Value::Object(_) => TLType::Object,
            Value::Map(_) => TLType::Map,
            Value::Ref(_) => TLType::Ref,
            Value::Tagged(_, _) => TLType::Tagged,
            Value::Timestamp(_, _) => TLType::Timestamp,
            Value::JsonNumber(_) => TLType::JsonNumber,
        }
    }

    pub fn as_timestamp(&self) -> Option<(i64, i16)> {
        match self {
            Value::Timestamp(ts, tz) => Some((*ts, *tz)),
            _ => None,
        }
    }

    pub fn as_timestamp_millis(&self) -> Option<i64> {
        match self {
            Value::Timestamp(ts, _) => Some(*ts),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&[(Value, Value)]> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_ref_name(&self) -> Option<&str> {
        match self {
            Value::Ref(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_tagged(&self) -> Option<(&str, &Value)> {
        match self {
            Value::Tagged(tag, value) => Some((tag, value)),
            _ => None,
        }
    }

    pub fn as_json_number(&self) -> Option<&str> {
        match self {
            Value::JsonNumber(s) => Some(s),
            _ => None,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

// Conversions
impl From<bool> for Value {
    fn from(b: bool) -> Self { Value::Bool(b) }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self { Value::Int(i as i64) }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self { Value::Int(i) }
}

impl From<u32> for Value {
    fn from(u: u32) -> Self { Value::UInt(u as u64) }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self { Value::UInt(u) }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self { Value::Float(f) }
}

impl From<String> for Value {
    fn from(s: String) -> Self { Value::String(s) }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self { Value::String(s.to_string()) }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl From<ObjectMap<String, Value>> for Value {
    fn from(m: ObjectMap<String, Value>) -> Self {
        Value::Object(m)
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(m: HashMap<String, Value>) -> Self {
        Value::Object(m.into_iter().collect())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // TLType::try_from
    // -------------------------------------------------------------------------

    #[test]
    fn test_tltype_try_from_all_valid() {
        let cases: Vec<(u8, TLType)> = vec![
            (0x00, TLType::Null),
            (0x01, TLType::Bool),
            (0x02, TLType::Int8),
            (0x03, TLType::Int16),
            (0x04, TLType::Int32),
            (0x05, TLType::Int64),
            (0x06, TLType::UInt8),
            (0x07, TLType::UInt16),
            (0x08, TLType::UInt32),
            (0x09, TLType::UInt64),
            (0x0A, TLType::Float32),
            (0x0B, TLType::Float64),
            (0x10, TLType::String),
            (0x11, TLType::Bytes),
            (0x20, TLType::Array),
            (0x21, TLType::Object),
            (0x22, TLType::Struct),
            (0x23, TLType::Map),
            (0x24, TLType::Tuple),
            (0x30, TLType::Ref),
            (0x31, TLType::Tagged),
            (0x32, TLType::Timestamp),
            (0x12, TLType::JsonNumber),
        ];
        for (byte, expected) in cases {
            assert_eq!(TLType::try_from(byte).unwrap(), expected, "byte=0x{:02X}", byte);
        }
    }

    #[test]
    fn test_tltype_try_from_invalid() {
        let err = TLType::try_from(0xFF).unwrap_err();
        assert!(matches!(err, Error::InvalidType(0xFF)));

        let err2 = TLType::try_from(0x0C).unwrap_err();
        assert!(matches!(err2, Error::InvalidType(0x0C)));
    }

    // -------------------------------------------------------------------------
    // FieldType
    // -------------------------------------------------------------------------

    #[test]
    fn test_fieldtype_new() {
        let ft = FieldType::new("int");
        assert_eq!(ft.base, "int");
        assert!(!ft.nullable);
        assert!(!ft.is_array);
    }

    #[test]
    fn test_fieldtype_nullable() {
        let ft = FieldType::new("string").nullable();
        assert!(ft.nullable);
        assert!(!ft.is_array);
    }

    #[test]
    fn test_fieldtype_array() {
        let ft = FieldType::new("int").array();
        assert!(ft.is_array);
        assert!(!ft.nullable);
    }

    #[test]
    fn test_fieldtype_parse_simple() {
        let ft = FieldType::parse("int");
        assert_eq!(ft.base, "int");
        assert!(!ft.nullable);
        assert!(!ft.is_array);
    }

    #[test]
    fn test_fieldtype_parse_nullable() {
        let ft = FieldType::parse("string?");
        assert_eq!(ft.base, "string");
        assert!(ft.nullable);
        assert!(!ft.is_array);
    }

    #[test]
    fn test_fieldtype_parse_array() {
        let ft = FieldType::parse("[]int");
        assert_eq!(ft.base, "int");
        assert!(!ft.nullable);
        assert!(ft.is_array);
    }

    #[test]
    fn test_fieldtype_parse_array_nullable() {
        let ft = FieldType::parse("[]string?");
        assert_eq!(ft.base, "string");
        assert!(ft.nullable);
        assert!(ft.is_array);
    }

    #[test]
    fn test_fieldtype_parse_with_whitespace() {
        let ft = FieldType::parse("  int64  ");
        assert_eq!(ft.base, "int64");
    }

    #[test]
    fn test_fieldtype_display() {
        assert_eq!(FieldType::new("int").to_string(), "int");
        assert_eq!(FieldType::new("string").nullable().to_string(), "string?");
        assert_eq!(FieldType::new("int").array().to_string(), "[]int");
        assert_eq!(
            FieldType::new("string").array().nullable().to_string(),
            "[]string?"
        );
    }

    #[test]
    fn test_fieldtype_to_tl_type_all_bases() {
        let cases = vec![
            ("bool", TLType::Bool),
            ("int8", TLType::Int8),
            ("int16", TLType::Int16),
            ("int", TLType::Int32),
            ("int32", TLType::Int32),
            ("int64", TLType::Int64),
            ("uint8", TLType::UInt8),
            ("uint16", TLType::UInt16),
            ("uint", TLType::UInt32),
            ("uint32", TLType::UInt32),
            ("uint64", TLType::UInt64),
            ("float32", TLType::Float32),
            ("float", TLType::Float64),
            ("float64", TLType::Float64),
            ("string", TLType::String),
            ("bytes", TLType::Bytes),
            ("timestamp", TLType::Timestamp),
            ("object", TLType::Object),
            ("tuple", TLType::Tuple),
            ("map", TLType::Map),
            ("MyStruct", TLType::Struct),
            ("SomeUnknown", TLType::Struct),
        ];
        for (base, expected) in cases {
            let ft = FieldType::new(base);
            assert_eq!(ft.to_tl_type(), expected, "base={}", base);
        }
    }

    #[test]
    fn test_fieldtype_array_overrides_base() {
        let ft = FieldType::new("int").array();
        assert_eq!(ft.to_tl_type(), TLType::Array);
    }

    #[test]
    fn test_fieldtype_is_struct() {
        assert!(FieldType::new("MyStruct").is_struct());
        assert!(!FieldType::new("int").is_struct());
        assert!(!FieldType::new("string").is_struct());
        assert!(!FieldType::new("int").array().is_struct()); // array is not struct
    }

    // -------------------------------------------------------------------------
    // Schema
    // -------------------------------------------------------------------------

    #[test]
    fn test_schema_builder() {
        let schema = Schema::new("User")
            .field("id", FieldType::new("int64"))
            .field("name", FieldType::new("string"));
        assert_eq!(schema.name, "User");
        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.fields[0].name, "id");
        assert_eq!(schema.fields[1].name, "name");
    }

    #[test]
    fn test_schema_add_field() {
        let mut schema = Schema::new("Event");
        schema.add_field("ts", FieldType::new("timestamp"));
        assert_eq!(schema.fields.len(), 1);
        assert_eq!(schema.fields[0].name, "ts");
    }

    #[test]
    fn test_schema_get_field_found() {
        let schema = Schema::new("User")
            .field("id", FieldType::new("int64"))
            .field("name", FieldType::new("string"));
        let f = schema.get_field("name").unwrap();
        assert_eq!(f.name, "name");
        assert_eq!(f.field_type.base, "string");
    }

    #[test]
    fn test_schema_get_field_missing() {
        let schema = Schema::new("User")
            .field("id", FieldType::new("int64"));
        assert!(schema.get_field("nonexistent").is_none());
    }

    #[test]
    fn test_schema_field_index_found() {
        let schema = Schema::new("User")
            .field("id", FieldType::new("int64"))
            .field("name", FieldType::new("string"));
        assert_eq!(schema.field_index("id"), Some(0));
        assert_eq!(schema.field_index("name"), Some(1));
    }

    #[test]
    fn test_schema_field_index_missing() {
        let schema = Schema::new("User")
            .field("id", FieldType::new("int64"));
        assert_eq!(schema.field_index("missing"), None);
    }

    // -------------------------------------------------------------------------
    // Union / Variant
    // -------------------------------------------------------------------------

    #[test]
    fn test_variant_builder() {
        let v = Variant::new("Circle")
            .field("radius", FieldType::new("float"));
        assert_eq!(v.name, "Circle");
        assert_eq!(v.fields.len(), 1);
        assert_eq!(v.fields[0].name, "radius");
    }

    #[test]
    fn test_union_builder() {
        let u = Union::new("Shape")
            .variant(Variant::new("Circle").field("radius", FieldType::new("float")))
            .variant(Variant::new("Point"));
        assert_eq!(u.name, "Shape");
        assert_eq!(u.variants.len(), 2);
    }

    #[test]
    fn test_union_add_variant() {
        let mut u = Union::new("Shape");
        u.add_variant(Variant::new("Circle"));
        assert_eq!(u.variants.len(), 1);
    }

    #[test]
    fn test_union_get_variant() {
        let u = Union::new("Shape")
            .variant(Variant::new("Circle").field("radius", FieldType::new("float")))
            .variant(Variant::new("Point"));
        assert!(u.get_variant("Circle").is_some());
        assert!(u.get_variant("Point").is_some());
        assert!(u.get_variant("Unknown").is_none());
    }

    // -------------------------------------------------------------------------
    // Value accessors
    // -------------------------------------------------------------------------

    #[test]
    fn test_value_is_null() {
        assert!(Value::Null.is_null());
        assert!(!Value::Bool(false).is_null());
    }

    #[test]
    fn test_value_as_bool() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Int(1).as_bool(), None);
    }

    #[test]
    fn test_value_as_int_from_uint() {
        // UInt coercion to i64
        assert_eq!(Value::UInt(42).as_int(), Some(42));
    }

    #[test]
    fn test_value_as_int_overflow_returns_none() {
        // UInt > i64::MAX should return None, not silently wrap
        assert_eq!(Value::UInt(u64::MAX).as_int(), None);
        assert_eq!(Value::UInt(i64::MAX as u64 + 1).as_int(), None);
        // Boundary: exactly i64::MAX should succeed
        assert_eq!(Value::UInt(i64::MAX as u64).as_int(), Some(i64::MAX));
    }

    #[test]
    fn test_value_as_int_checked_success() {
        assert_eq!(Value::Int(42).as_int_checked().unwrap(), 42);
        assert_eq!(Value::UInt(42).as_int_checked().unwrap(), 42);
        assert_eq!(Value::UInt(i64::MAX as u64).as_int_checked().unwrap(), i64::MAX);
    }

    #[test]
    fn test_value_as_int_checked_overflow_error() {
        let result = Value::UInt(u64::MAX).as_int_checked();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ValueOutOfRange(_)));
    }

    #[test]
    fn test_value_as_int_checked_wrong_type_error() {
        let result = Value::String("nope".into()).as_int_checked();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ValueOutOfRange(_)));
    }

    #[test]
    fn test_value_as_int_from_non_numeric() {
        assert_eq!(Value::String("nope".into()).as_int(), None);
    }

    #[test]
    fn test_value_as_uint_from_positive_int() {
        // Positive Int coercion to u64
        assert_eq!(Value::Int(42).as_uint(), Some(42));
    }

    #[test]
    fn test_value_as_uint_from_negative_int() {
        // Negative Int should not coerce to u64
        assert_eq!(Value::Int(-1).as_uint(), None);
    }

    #[test]
    fn test_value_as_uint_from_non_numeric() {
        assert_eq!(Value::Bool(true).as_uint(), None);
    }

    #[test]
    fn test_value_as_float_from_int() {
        assert_eq!(Value::Int(42).as_float(), Some(42.0));
    }

    #[test]
    fn test_value_as_float_from_uint() {
        assert_eq!(Value::UInt(100).as_float(), Some(100.0));
    }

    #[test]
    fn test_value_as_float_from_non_numeric() {
        assert_eq!(Value::String("no".into()).as_float(), None);
    }

    #[test]
    fn test_value_as_str() {
        assert_eq!(Value::String("hello".into()).as_str(), Some("hello"));
        assert_eq!(Value::Int(1).as_str(), None);
    }

    #[test]
    fn test_value_as_bytes() {
        let data = vec![1u8, 2, 3];
        assert_eq!(Value::Bytes(data.clone()).as_bytes(), Some(data.as_slice()));
        assert_eq!(Value::Null.as_bytes(), None);
    }

    #[test]
    fn test_value_as_array() {
        let arr = vec![Value::Int(1), Value::Int(2)];
        assert_eq!(
            Value::Array(arr.clone()).as_array(),
            Some(arr.as_slice())
        );
        assert_eq!(Value::Null.as_array(), None);
    }

    #[test]
    fn test_value_as_object() {
        let mut obj = ObjectMap::new();
        obj.insert("k".to_string(), Value::Int(1));
        assert!(Value::Object(obj).as_object().is_some());
        assert!(Value::Null.as_object().is_none());
    }

    #[test]
    fn test_value_get() {
        let mut obj = ObjectMap::new();
        obj.insert("key".to_string(), Value::Int(42));
        let val = Value::Object(obj);
        assert_eq!(val.get("key"), Some(&Value::Int(42)));
        assert_eq!(val.get("missing"), None);
        assert_eq!(Value::Int(1).get("x"), None);
    }

    #[test]
    fn test_value_index() {
        let val = Value::Array(vec![Value::Int(10), Value::Int(20)]);
        assert_eq!(val.index(0), Some(&Value::Int(10)));
        assert_eq!(val.index(1), Some(&Value::Int(20)));
        assert_eq!(val.index(5), None);
        assert_eq!(Value::Int(1).index(0), None);
    }

    #[test]
    fn test_value_as_timestamp() {
        assert_eq!(Value::Timestamp(1700000000, 0).as_timestamp(), Some((1700000000, 0)));
        assert_eq!(Value::Int(42).as_timestamp(), None);
    }

    #[test]
    fn test_value_as_map() {
        let pairs = vec![(Value::String("k".into()), Value::Int(1))];
        assert_eq!(
            Value::Map(pairs.clone()).as_map(),
            Some(pairs.as_slice())
        );
        assert_eq!(Value::Null.as_map(), None);
    }

    #[test]
    fn test_value_as_ref_name() {
        assert_eq!(Value::Ref("MyRef".into()).as_ref_name(), Some("MyRef"));
        assert_eq!(Value::Null.as_ref_name(), None);
    }

    #[test]
    fn test_value_as_tagged() {
        let val = Value::Tagged("tag".into(), Box::new(Value::Int(1)));
        let (tag, inner) = val.as_tagged().unwrap();
        assert_eq!(tag, "tag");
        assert_eq!(inner, &Value::Int(1));
        assert_eq!(Value::Null.as_tagged(), None);
    }

    #[test]
    fn test_value_default() {
        assert_eq!(Value::default(), Value::Null);
    }

    // -------------------------------------------------------------------------
    // Value::tl_type() boundary values
    // -------------------------------------------------------------------------

    #[test]
    fn test_value_tl_type_int_boundaries() {
        // i8 range
        assert_eq!(Value::Int(0).tl_type(), TLType::Int8);
        assert_eq!(Value::Int(127).tl_type(), TLType::Int8);
        assert_eq!(Value::Int(-128).tl_type(), TLType::Int8);

        // i16 range
        assert_eq!(Value::Int(128).tl_type(), TLType::Int16);
        assert_eq!(Value::Int(-129).tl_type(), TLType::Int16);
        assert_eq!(Value::Int(32767).tl_type(), TLType::Int16);
        assert_eq!(Value::Int(-32768).tl_type(), TLType::Int16);

        // i32 range
        assert_eq!(Value::Int(32768).tl_type(), TLType::Int32);
        assert_eq!(Value::Int(-32769).tl_type(), TLType::Int32);
        assert_eq!(Value::Int(i32::MAX as i64).tl_type(), TLType::Int32);
        assert_eq!(Value::Int(i32::MIN as i64).tl_type(), TLType::Int32);

        // i64 range
        assert_eq!(Value::Int(i32::MAX as i64 + 1).tl_type(), TLType::Int64);
        assert_eq!(Value::Int(i32::MIN as i64 - 1).tl_type(), TLType::Int64);
        assert_eq!(Value::Int(i64::MAX).tl_type(), TLType::Int64);
        assert_eq!(Value::Int(i64::MIN).tl_type(), TLType::Int64);
    }

    #[test]
    fn test_value_tl_type_uint_boundaries() {
        // u8 range
        assert_eq!(Value::UInt(0).tl_type(), TLType::UInt8);
        assert_eq!(Value::UInt(255).tl_type(), TLType::UInt8);

        // u16 range
        assert_eq!(Value::UInt(256).tl_type(), TLType::UInt16);
        assert_eq!(Value::UInt(65535).tl_type(), TLType::UInt16);

        // u32 range
        assert_eq!(Value::UInt(65536).tl_type(), TLType::UInt32);
        assert_eq!(Value::UInt(u32::MAX as u64).tl_type(), TLType::UInt32);

        // u64 range
        assert_eq!(Value::UInt(u32::MAX as u64 + 1).tl_type(), TLType::UInt64);
        assert_eq!(Value::UInt(u64::MAX).tl_type(), TLType::UInt64);
    }

    #[test]
    fn test_value_tl_type_other_variants() {
        assert_eq!(Value::Null.tl_type(), TLType::Null);
        assert_eq!(Value::Bool(true).tl_type(), TLType::Bool);
        assert_eq!(Value::Float(1.0).tl_type(), TLType::Float64);
        assert_eq!(Value::String("s".into()).tl_type(), TLType::String);
        assert_eq!(Value::Bytes(vec![]).tl_type(), TLType::Bytes);
        assert_eq!(Value::Array(vec![]).tl_type(), TLType::Array);
        assert_eq!(Value::Object(ObjectMap::new()).tl_type(), TLType::Object);
        assert_eq!(Value::Map(vec![]).tl_type(), TLType::Map);
        assert_eq!(Value::Ref("r".into()).tl_type(), TLType::Ref);
        assert_eq!(
            Value::Tagged("t".into(), Box::new(Value::Null)).tl_type(),
            TLType::Tagged
        );
        assert_eq!(Value::Timestamp(0, 0).tl_type(), TLType::Timestamp);
        assert_eq!(
            Value::JsonNumber("123.456".into()).tl_type(),
            TLType::JsonNumber
        );
    }

    // -------------------------------------------------------------------------
    // Value::From impls
    // -------------------------------------------------------------------------

    #[test]
    fn test_value_from_bool() {
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(false), Value::Bool(false));
    }

    #[test]
    fn test_value_from_i32() {
        assert_eq!(Value::from(42i32), Value::Int(42));
        assert_eq!(Value::from(-1i32), Value::Int(-1));
    }

    #[test]
    fn test_value_from_i64() {
        assert_eq!(Value::from(42i64), Value::Int(42));
    }

    #[test]
    fn test_value_from_u32() {
        assert_eq!(Value::from(42u32), Value::UInt(42));
    }

    #[test]
    fn test_value_from_u64() {
        assert_eq!(Value::from(42u64), Value::UInt(42));
    }

    #[test]
    fn test_value_from_f64() {
        assert_eq!(Value::from(3.14f64), Value::Float(3.14));
    }

    #[test]
    fn test_value_from_string() {
        assert_eq!(Value::from("hello".to_string()), Value::String("hello".into()));
    }

    #[test]
    fn test_value_from_str() {
        assert_eq!(Value::from("hello"), Value::String("hello".into()));
    }

    #[test]
    fn test_value_from_vec() {
        let v: Vec<i32> = vec![1, 2, 3];
        assert_eq!(
            Value::from(v),
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn test_value_from_objectmap() {
        let mut m = ObjectMap::new();
        m.insert("key".to_string(), Value::Int(42));
        let val = Value::from(m.clone());
        assert_eq!(val, Value::Object(m));
    }

    #[test]
    fn test_value_from_hashmap() {
        let mut m = HashMap::new();
        m.insert("key".to_string(), Value::Int(42));
        let val = Value::from(m);
        assert!(matches!(val, Value::Object(_)));
        assert_eq!(val.as_object().unwrap().get("key").unwrap().as_int(), Some(42));
    }

    // -------------------------------------------------------------------------
    // Error Display
    // -------------------------------------------------------------------------

    #[test]
    fn test_error_display_all_variants() {
        assert_eq!(
            Error::Io(io::Error::new(io::ErrorKind::NotFound, "gone")).to_string(),
            "IO error: gone"
        );
        assert_eq!(Error::InvalidMagic.to_string(), "Invalid TeaLeaf magic bytes");
        assert_eq!(
            Error::InvalidVersion { major: 99, minor: 1 }.to_string(),
            "Unsupported version: 99.1"
        );
        assert_eq!(
            Error::InvalidType(0xFF).to_string(),
            "Invalid type code: 0xFF"
        );
        assert_eq!(Error::InvalidUtf8.to_string(), "Invalid UTF-8");
        assert_eq!(
            Error::UnexpectedToken {
                expected: "number".into(),
                got: "string".into()
            }
            .to_string(),
            "Expected number, got string"
        );
        assert_eq!(Error::UnexpectedEof.to_string(), "Unexpected end of input");
        assert_eq!(
            Error::UnknownStruct("Foo".into()).to_string(),
            "Unknown struct: Foo"
        );
        assert_eq!(
            Error::MissingField("bar".into()).to_string(),
            "Missing field: bar"
        );
        assert_eq!(
            Error::ParseError("bad input".into()).to_string(),
            "Parse error: bad input"
        );
        assert_eq!(
            Error::ValueOutOfRange("too big".into()).to_string(),
            "Value out of range: too big"
        );
    }

    #[test]
    fn test_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let tl_err = Error::from(io_err);
        assert!(matches!(tl_err, Error::Io(_)));
        assert!(tl_err.to_string().contains("denied"));
    }

    // -------------------------------------------------------------------------
    // Field
    // -------------------------------------------------------------------------

    #[test]
    fn test_field_new() {
        let f = Field::new("age", FieldType::new("int"));
        assert_eq!(f.name, "age");
        assert_eq!(f.field_type.base, "int");
    }
}
