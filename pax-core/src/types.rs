//! Core types for Pax

use std::collections::HashMap;
use std::fmt;
use std::io;

// =============================================================================
// Constants
// =============================================================================

pub const MAGIC: [u8; 4] = *b"PAX\0";
/// Binary format version (major) - for compatibility checks
pub const VERSION_MAJOR: u16 = 2;
/// Binary format version (minor) - for compatibility checks
pub const VERSION_MINOR: u16 = 0;
/// Library version string (beta/RFC stage)
pub const VERSION: &str = "2.0.0-beta.1";
pub const HEADER_SIZE: usize = 64;

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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::InvalidMagic => write!(f, "Invalid Pax magic bytes"),
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
pub enum PaxType {
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
    Array = 0x20,
    Object = 0x21,
    Struct = 0x22,
    Map = 0x23,
    Tuple = 0x24,
    Ref = 0x30,
    Tagged = 0x31,
    Timestamp = 0x32,
}

impl TryFrom<u8> for PaxType {
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

    pub fn to_pax_type(&self) -> PaxType {
        if self.is_array {
            return PaxType::Array;
        }
        match self.base.as_str() {
            "bool" => PaxType::Bool,
            "int8" => PaxType::Int8,
            "int16" => PaxType::Int16,
            "int" | "int32" => PaxType::Int32,
            "int64" => PaxType::Int64,
            "uint8" => PaxType::UInt8,
            "uint16" => PaxType::UInt16,
            "uint" | "uint32" => PaxType::UInt32,
            "uint64" => PaxType::UInt64,
            "float32" => PaxType::Float32,
            "float" | "float64" => PaxType::Float64,
            "string" => PaxType::String,
            "bytes" => PaxType::Bytes,
            "timestamp" => PaxType::Timestamp,
            // Note: map, object, ref, tagged are value types, not schema types
            // They fall through to Struct and will fail on lookup
            _ => PaxType::Struct, // Assume struct reference
        }
    }

    pub fn is_struct(&self) -> bool {
        !self.is_array && self.to_pax_type() == PaxType::Struct
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
    Object(HashMap<String, Value>),
    Map(Vec<(Value, Value)>),  // Key-value pairs preserving order
    Ref(String),
    Tagged(String, Box<Value>),
    Timestamp(i64),  // Unix timestamp in milliseconds
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
            Value::UInt(u) => Some(*u as i64),
            _ => None,
        }
    }

    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Value::UInt(u) => Some(*u),
            Value::Int(i) if *i >= 0 => Some(*i as u64),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::UInt(u) => Some(*u as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
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

    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
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

    pub fn pax_type(&self) -> PaxType {
        match self {
            Value::Null => PaxType::Null,
            Value::Bool(_) => PaxType::Bool,
            Value::Int(i) => {
                if *i >= i8::MIN as i64 && *i <= i8::MAX as i64 {
                    PaxType::Int8
                } else if *i >= i16::MIN as i64 && *i <= i16::MAX as i64 {
                    PaxType::Int16
                } else if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                    PaxType::Int32
                } else {
                    PaxType::Int64
                }
            }
            Value::UInt(u) => {
                if *u <= u8::MAX as u64 {
                    PaxType::UInt8
                } else if *u <= u16::MAX as u64 {
                    PaxType::UInt16
                } else if *u <= u32::MAX as u64 {
                    PaxType::UInt32
                } else {
                    PaxType::UInt64
                }
            }
            Value::Float(_) => PaxType::Float64,
            Value::String(_) => PaxType::String,
            Value::Bytes(_) => PaxType::Bytes,
            Value::Array(_) => PaxType::Array,
            Value::Object(_) => PaxType::Object,
            Value::Map(_) => PaxType::Map,
            Value::Ref(_) => PaxType::Ref,
            Value::Tagged(_, _) => PaxType::Tagged,
            Value::Timestamp(_) => PaxType::Timestamp,
        }
    }

    pub fn as_timestamp(&self) -> Option<i64> {
        match self {
            Value::Timestamp(ts) => Some(*ts),
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

impl From<HashMap<String, Value>> for Value {
    fn from(m: HashMap<String, Value>) -> Self {
        Value::Object(m)
    }
}
