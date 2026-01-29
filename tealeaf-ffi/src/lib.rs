//! C FFI bindings for the TeaLeaf format library
//!
//! This crate provides a C-compatible API for use from other languages
//! like C#, Python, and Node.js.

use tealeaf::{TeaLeaf, Value, Reader, Writer, dumps};
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// =============================================================================
// Thread-Local Error Storage
// =============================================================================

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = RefCell::new(None);
}

/// Store an error message in thread-local storage
fn set_last_error(msg: String) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = Some(msg);
    });
}

/// Clear the last error
fn clear_last_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}

/// Get the last error message.
/// Returns NULL if no error. Caller must free with tl_string_free.
#[no_mangle]
pub extern "C" fn tl_get_last_error() -> *mut c_char {
    LAST_ERROR.with(|e| {
        match e.borrow().as_ref() {
            Some(msg) => CString::new(msg.as_str())
                .map(|c| c.into_raw())
                .unwrap_or(ptr::null_mut()),
            None => ptr::null_mut(),
        }
    })
}

/// Clear the last error message.
#[no_mangle]
pub extern "C" fn tl_clear_error() {
    clear_last_error();
}

// =============================================================================
// Opaque Types
// =============================================================================

/// Opaque handle to a TeaLeaf document
pub struct TLDocument {
    inner: TeaLeaf,
}

/// Opaque handle to a TeaLeaf value
pub struct TLValue {
    inner: Value,
}

/// Opaque handle to a binary reader
pub struct TLReader {
    inner: Reader,
}

/// Opaque handle to a binary writer
#[allow(dead_code)]
pub struct TLWriter {
    inner: Writer,
}

/// Result type for FFI operations
#[repr(C)]
pub struct TLResult {
    pub success: bool,
    pub error_message: *mut c_char,
}

impl TLResult {
    fn ok() -> Self {
        Self {
            success: true,
            error_message: ptr::null_mut(),
        }
    }

    fn err(msg: &str) -> Self {
        let c_str = CString::new(msg).unwrap_or_default();
        Self {
            success: false,
            error_message: c_str.into_raw(),
        }
    }
}

// =============================================================================
// Document API
// =============================================================================

/// Parse a TeaLeaf text document from a string.
/// Returns NULL on error. Use tl_get_last_error() for error details.
#[no_mangle]
pub unsafe extern "C" fn tl_parse(text: *const c_char) -> *mut TLDocument {
    clear_last_error();

    if text.is_null() {
        set_last_error("Null pointer passed to tl_parse".to_string());
        return ptr::null_mut();
    }

    let c_str = match CStr::from_ptr(text).to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in input: {}", e));
            return ptr::null_mut();
        }
    };

    match TeaLeaf::parse(c_str) {
        Ok(doc) => Box::into_raw(Box::new(TLDocument { inner: doc })),
        Err(e) => {
            set_last_error(format!("Parse error: {}", e));
            ptr::null_mut()
        }
    }
}

/// Parse a TeaLeaf text document from a file path.
/// Returns NULL on error. Use tl_get_last_error() for error details.
#[no_mangle]
pub unsafe extern "C" fn tl_parse_file(path: *const c_char) -> *mut TLDocument {
    clear_last_error();

    if path.is_null() {
        set_last_error("Null pointer passed to tl_parse_file".to_string());
        return ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in path: {}", e));
            return ptr::null_mut();
        }
    };

    match TeaLeaf::load(path_str) {
        Ok(doc) => Box::into_raw(Box::new(TLDocument { inner: doc })),
        Err(e) => {
            set_last_error(format!("Failed to load '{}': {}", path_str, e));
            ptr::null_mut()
        }
    }
}

/// Free a TeaLeaf document.
#[no_mangle]
pub unsafe extern "C" fn tl_document_free(doc: *mut TLDocument) {
    if !doc.is_null() {
        drop(Box::from_raw(doc));
    }
}

/// Get a value from the document by key.
/// Returns NULL if the key doesn't exist.
#[no_mangle]
pub unsafe extern "C" fn tl_document_get(
    doc: *const TLDocument,
    key: *const c_char,
) -> *mut TLValue {
    if doc.is_null() || key.is_null() {
        return ptr::null_mut();
    }

    let key_str = match CStr::from_ptr(key).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match (*doc).inner.get(key_str) {
        Some(value) => Box::into_raw(Box::new(TLValue { inner: value.clone() })),
        None => ptr::null_mut(),
    }
}

/// Get all keys in the document.
/// Returns a NULL-terminated array of strings. Caller must free with tl_string_array_free.
#[no_mangle]
pub unsafe extern "C" fn tl_document_keys(doc: *const TLDocument) -> *mut *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }

    let keys: Vec<_> = (*doc).inner.data.keys().collect();
    let mut result: Vec<*mut c_char> = keys
        .iter()
        .filter_map(|k| CString::new(k.as_str()).ok())
        .map(|s| s.into_raw())
        .collect();
    result.push(ptr::null_mut()); // NULL terminator

    let ptr = result.as_mut_ptr();
    std::mem::forget(result);
    ptr
}

/// Convert document to TeaLeaf text format with schema definitions.
/// This is the default output format that includes @struct definitions.
/// Caller must free the returned string with tl_string_free.
#[no_mangle]
pub unsafe extern "C" fn tl_document_to_text(doc: *const TLDocument) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }

    let text = (*doc).inner.to_tl_with_schemas();
    match CString::new(text) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Convert document to TeaLeaf text format without schema definitions (data only).
/// Use this when you only want the data portion without @struct definitions.
/// Caller must free the returned string with tl_string_free.
#[no_mangle]
pub unsafe extern "C" fn tl_document_to_text_data_only(doc: *const TLDocument) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }

    let text = dumps(&(*doc).inner.data);
    match CString::new(text) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Compile document to binary format and write to file.
#[no_mangle]
pub unsafe extern "C" fn tl_document_compile(
    doc: *const TLDocument,
    path: *const c_char,
    compress: bool,
) -> TLResult {
    if doc.is_null() || path.is_null() {
        return TLResult::err("Null pointer");
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return TLResult::err("Invalid path encoding"),
    };

    match (*doc).inner.compile(path_str, compress) {
        Ok(_) => TLResult::ok(),
        Err(e) => TLResult::err(&e.to_string()),
    }
}

// =============================================================================
// JSON Conversion API
// =============================================================================

/// Parse a JSON string to create a TeaLeaf document with automatic schema inference.
/// Detects uniform object arrays and creates @struct definitions.
/// Returns NULL on error. Use tl_get_last_error() for error details.
#[no_mangle]
pub unsafe extern "C" fn tl_document_from_json(json: *const c_char) -> *mut TLDocument {
    clear_last_error();

    if json.is_null() {
        set_last_error("Null pointer passed to tl_document_from_json".to_string());
        return ptr::null_mut();
    }

    let json_str = match CStr::from_ptr(json).to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in JSON: {}", e));
            return ptr::null_mut();
        }
    };

    match TeaLeaf::from_json_with_schemas(json_str) {
        Ok(doc) => Box::into_raw(Box::new(TLDocument { inner: doc })),
        Err(e) => {
            set_last_error(format!("JSON parse error: {}", e));
            ptr::null_mut()
        }
    }
}

/// Convert a TeaLeaf document to pretty-printed JSON.
/// Caller must free the returned string with tl_string_free.
/// Returns NULL on error. Use tl_get_last_error() for error details.
#[no_mangle]
pub unsafe extern "C" fn tl_document_to_json(doc: *const TLDocument) -> *mut c_char {
    clear_last_error();

    if doc.is_null() {
        set_last_error("Null pointer passed to tl_document_to_json".to_string());
        return ptr::null_mut();
    }

    match (*doc).inner.to_json() {
        Ok(json) => CString::new(json).map(|c| c.into_raw()).unwrap_or_else(|e| {
            set_last_error(format!("Failed to create C string: {}", e));
            ptr::null_mut()
        }),
        Err(e) => {
            set_last_error(format!("JSON serialization error: {}", e));
            ptr::null_mut()
        }
    }
}

/// Convert a TeaLeaf document to compact JSON (no extra whitespace).
/// Caller must free the returned string with tl_string_free.
/// Returns NULL on error. Use tl_get_last_error() for error details.
#[no_mangle]
pub unsafe extern "C" fn tl_document_to_json_compact(doc: *const TLDocument) -> *mut c_char {
    clear_last_error();

    if doc.is_null() {
        set_last_error("Null pointer passed to tl_document_to_json_compact".to_string());
        return ptr::null_mut();
    }

    match (*doc).inner.to_json_compact() {
        Ok(json) => CString::new(json).map(|c| c.into_raw()).unwrap_or_else(|e| {
            set_last_error(format!("Failed to create C string: {}", e));
            ptr::null_mut()
        }),
        Err(e) => {
            set_last_error(format!("JSON serialization error: {}", e));
            ptr::null_mut()
        }
    }
}

// =============================================================================
// Value API
// =============================================================================

/// Value type enumeration
#[repr(C)]
pub enum TLValueType {
    Null = 0,
    Bool = 1,
    Int = 2,
    UInt = 3,
    Float = 4,
    String = 5,
    Bytes = 6,
    Array = 7,
    Object = 8,
    Map = 9,
    Ref = 10,
    Tagged = 11,
    Timestamp = 12,
}

/// Get the type of a value.
#[no_mangle]
pub unsafe extern "C" fn tl_value_type(value: *const TLValue) -> TLValueType {
    if value.is_null() {
        return TLValueType::Null;
    }

    match &(*value).inner {
        Value::Null => TLValueType::Null,
        Value::Bool(_) => TLValueType::Bool,
        Value::Int(_) => TLValueType::Int,
        Value::UInt(_) => TLValueType::UInt,
        Value::Float(_) => TLValueType::Float,
        Value::String(_) => TLValueType::String,
        Value::Bytes(_) => TLValueType::Bytes,
        Value::Array(_) => TLValueType::Array,
        Value::Object(_) => TLValueType::Object,
        Value::Map(_) => TLValueType::Map,
        Value::Ref(_) => TLValueType::Ref,
        Value::Tagged(_, _) => TLValueType::Tagged,
        Value::Timestamp(_) => TLValueType::Timestamp,
    }
}

/// Free a TeaLeaf value.
#[no_mangle]
pub unsafe extern "C" fn tl_value_free(value: *mut TLValue) {
    if !value.is_null() {
        drop(Box::from_raw(value));
    }
}

/// Get boolean value. Returns false if not a bool.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_bool(value: *const TLValue) -> bool {
    if value.is_null() {
        return false;
    }
    (*value).inner.as_bool().unwrap_or(false)
}

/// Get integer value. Returns 0 if not an int.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_int(value: *const TLValue) -> i64 {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_int().unwrap_or(0)
}

/// Get unsigned integer value. Returns 0 if not a uint.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_uint(value: *const TLValue) -> u64 {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_uint().unwrap_or(0)
}

/// Get float value. Returns 0.0 if not a float.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_float(value: *const TLValue) -> f64 {
    if value.is_null() {
        return 0.0;
    }
    (*value).inner.as_float().unwrap_or(0.0)
}

/// Get string value. Returns NULL if not a string.
/// Caller must free with tl_string_free.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_string(value: *const TLValue) -> *mut c_char {
    if value.is_null() {
        return ptr::null_mut();
    }
    match (*value).inner.as_str() {
        Some(s) => CString::new(s).map(|c| c.into_raw()).unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    }
}

/// Get timestamp value (Unix milliseconds). Returns 0 if not a timestamp.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_timestamp(value: *const TLValue) -> i64 {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_timestamp().unwrap_or(0)
}

/// Get bytes length. Returns 0 if not bytes.
#[no_mangle]
pub unsafe extern "C" fn tl_value_bytes_len(value: *const TLValue) -> usize {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_bytes().map(|b| b.len()).unwrap_or(0)
}

/// Get bytes data pointer. Returns NULL if not bytes.
/// The returned pointer is valid only while the TLValue is alive.
/// Do not free this pointer directly.
#[no_mangle]
pub unsafe extern "C" fn tl_value_bytes_data(value: *const TLValue) -> *const u8 {
    if value.is_null() {
        return ptr::null();
    }
    match (*value).inner.as_bytes() {
        Some(b) if !b.is_empty() => b.as_ptr(),
        _ => ptr::null(),
    }
}

/// Get reference name. Returns NULL if not a ref.
/// Caller must free with tl_string_free.
#[no_mangle]
pub unsafe extern "C" fn tl_value_ref_name(value: *const TLValue) -> *mut c_char {
    if value.is_null() {
        return ptr::null_mut();
    }
    match (*value).inner.as_ref_name() {
        Some(name) => CString::new(name).map(|c| c.into_raw()).unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    }
}

/// Get tag name from a tagged value. Returns NULL if not tagged.
/// Caller must free with tl_string_free.
#[no_mangle]
pub unsafe extern "C" fn tl_value_tag_name(value: *const TLValue) -> *mut c_char {
    if value.is_null() {
        return ptr::null_mut();
    }
    match (*value).inner.as_tagged() {
        Some((tag, _)) => CString::new(tag).map(|c| c.into_raw()).unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    }
}

/// Get inner value from a tagged value. Returns NULL if not tagged.
/// Caller must free with tl_value_free.
#[no_mangle]
pub unsafe extern "C" fn tl_value_tag_value(value: *const TLValue) -> *mut TLValue {
    if value.is_null() {
        return ptr::null_mut();
    }
    match (*value).inner.as_tagged() {
        Some((_, inner)) => Box::into_raw(Box::new(TLValue { inner: inner.clone() })),
        None => ptr::null_mut(),
    }
}

/// Get map length. Returns 0 if not a map.
#[no_mangle]
pub unsafe extern "C" fn tl_value_map_len(value: *const TLValue) -> usize {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_map().map(|m| m.len()).unwrap_or(0)
}

/// Get map entry key by index. Returns NULL if out of bounds or not a map.
/// Caller must free with tl_value_free.
#[no_mangle]
pub unsafe extern "C" fn tl_value_map_get_key(value: *const TLValue, index: usize) -> *mut TLValue {
    if value.is_null() {
        return ptr::null_mut();
    }
    match (*value).inner.as_map() {
        Some(m) if index < m.len() => {
            Box::into_raw(Box::new(TLValue { inner: m[index].0.clone() }))
        }
        _ => ptr::null_mut(),
    }
}

/// Get map entry value by index. Returns NULL if out of bounds or not a map.
/// Caller must free with tl_value_free.
#[no_mangle]
pub unsafe extern "C" fn tl_value_map_get_value(value: *const TLValue, index: usize) -> *mut TLValue {
    if value.is_null() {
        return ptr::null_mut();
    }
    match (*value).inner.as_map() {
        Some(m) if index < m.len() => {
            Box::into_raw(Box::new(TLValue { inner: m[index].1.clone() }))
        }
        _ => ptr::null_mut(),
    }
}

/// Get array length. Returns 0 if not an array.
#[no_mangle]
pub unsafe extern "C" fn tl_value_array_len(value: *const TLValue) -> usize {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_array().map(|a| a.len()).unwrap_or(0)
}

/// Get array element by index. Returns NULL if out of bounds or not an array.
#[no_mangle]
pub unsafe extern "C" fn tl_value_array_get(
    value: *const TLValue,
    index: usize,
) -> *mut TLValue {
    if value.is_null() {
        return ptr::null_mut();
    }
    match (*value).inner.as_array() {
        Some(arr) if index < arr.len() => {
            Box::into_raw(Box::new(TLValue { inner: arr[index].clone() }))
        }
        _ => ptr::null_mut(),
    }
}

/// Get object field by key. Returns NULL if not found or not an object.
#[no_mangle]
pub unsafe extern "C" fn tl_value_object_get(
    value: *const TLValue,
    key: *const c_char,
) -> *mut TLValue {
    if value.is_null() || key.is_null() {
        return ptr::null_mut();
    }

    let key_str = match CStr::from_ptr(key).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match (*value).inner.as_object() {
        Some(obj) => match obj.get(key_str) {
            Some(v) => Box::into_raw(Box::new(TLValue { inner: v.clone() })),
            None => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

/// Get object keys. Returns NULL-terminated array.
/// Caller must free with tl_string_array_free.
#[no_mangle]
pub unsafe extern "C" fn tl_value_object_keys(value: *const TLValue) -> *mut *mut c_char {
    if value.is_null() {
        return ptr::null_mut();
    }

    match (*value).inner.as_object() {
        Some(obj) => {
            let mut result: Vec<*mut c_char> = obj
                .keys()
                .filter_map(|k| CString::new(k.as_str()).ok())
                .map(|s| s.into_raw())
                .collect();
            result.push(ptr::null_mut());
            let ptr = result.as_mut_ptr();
            std::mem::forget(result);
            ptr
        }
        None => ptr::null_mut(),
    }
}

// =============================================================================
// Binary Reader API
// =============================================================================

/// Open a binary TeaLeaf file for reading (reads file into memory).
/// Returns NULL on error. Use tl_get_last_error() for error details.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_open(path: *const c_char) -> *mut TLReader {
    clear_last_error();

    if path.is_null() {
        set_last_error("Null pointer passed to tl_reader_open".to_string());
        return ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in path: {}", e));
            return ptr::null_mut();
        }
    };

    match Reader::open(path_str) {
        Ok(reader) => Box::into_raw(Box::new(TLReader { inner: reader })),
        Err(e) => {
            set_last_error(format!("Failed to open '{}': {}", path_str, e));
            ptr::null_mut()
        }
    }
}

/// Open a binary TeaLeaf file with memory mapping (zero-copy access).
/// This is more efficient for large files as the OS handles paging.
/// Returns NULL on error. Use tl_get_last_error() for error details.
///
/// # Safety
/// The file must not be modified while the reader is open.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_open_mmap(path: *const c_char) -> *mut TLReader {
    clear_last_error();

    if path.is_null() {
        set_last_error("Null pointer passed to tl_reader_open_mmap".to_string());
        return ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in path: {}", e));
            return ptr::null_mut();
        }
    };

    match Reader::open_mmap(path_str) {
        Ok(reader) => Box::into_raw(Box::new(TLReader { inner: reader })),
        Err(e) => {
            set_last_error(format!("Failed to memory-map '{}': {}", path_str, e));
            ptr::null_mut()
        }
    }
}

/// Free a binary reader.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_free(reader: *mut TLReader) {
    if !reader.is_null() {
        drop(Box::from_raw(reader));
    }
}

/// Get a value from a binary file by key.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_get(
    reader: *const TLReader,
    key: *const c_char,
) -> *mut TLValue {
    if reader.is_null() || key.is_null() {
        return ptr::null_mut();
    }

    let key_str = match CStr::from_ptr(key).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match (*reader).inner.get(key_str) {
        Ok(value) => Box::into_raw(Box::new(TLValue { inner: value })),
        Err(_) => ptr::null_mut(),
    }
}

/// Get all keys from a binary file.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_keys(reader: *const TLReader) -> *mut *mut c_char {
    if reader.is_null() {
        return ptr::null_mut();
    }

    let keys = (*reader).inner.keys();
    let mut result: Vec<*mut c_char> = keys
        .iter()
        .filter_map(|k| CString::new(*k).ok())
        .map(|s| s.into_raw())
        .collect();
    result.push(ptr::null_mut());
    let ptr = result.as_mut_ptr();
    std::mem::forget(result);
    ptr
}

// =============================================================================
// Schema API (for dynamic typing support)
// =============================================================================

/// Get the number of schemas in a binary file.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_schema_count(reader: *const TLReader) -> usize {
    if reader.is_null() {
        return 0;
    }
    (*reader).inner.schemas.len()
}

/// Get a schema name by index.
/// Caller must free the returned string with tl_string_free.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_schema_name(
    reader: *const TLReader,
    index: usize,
) -> *mut c_char {
    if reader.is_null() {
        return ptr::null_mut();
    }
    match (&(*reader).inner.schemas).get(index) {
        Some(schema) => CString::new(schema.name.as_str())
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    }
}

/// Get the number of fields in a schema.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_schema_field_count(
    reader: *const TLReader,
    schema_index: usize,
) -> usize {
    if reader.is_null() {
        return 0;
    }
    match (&(*reader).inner.schemas).get(schema_index) {
        Some(schema) => schema.fields.len(),
        None => 0,
    }
}

/// Get a field name from a schema.
/// Caller must free the returned string with tl_string_free.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_schema_field_name(
    reader: *const TLReader,
    schema_index: usize,
    field_index: usize,
) -> *mut c_char {
    if reader.is_null() {
        return ptr::null_mut();
    }
    let schema = match (&(*reader).inner.schemas).get(schema_index) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };
    match schema.fields.get(field_index) {
        Some(field) => CString::new(field.name.as_str())
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    }
}

/// Get a field's base type from a schema.
/// Caller must free the returned string with tl_string_free.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_schema_field_type(
    reader: *const TLReader,
    schema_index: usize,
    field_index: usize,
) -> *mut c_char {
    if reader.is_null() {
        return ptr::null_mut();
    }
    let schema = match (&(*reader).inner.schemas).get(schema_index) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };
    match schema.fields.get(field_index) {
        Some(field) => CString::new(field.field_type.base.as_str())
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    }
}

/// Check if a field is nullable.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_schema_field_nullable(
    reader: *const TLReader,
    schema_index: usize,
    field_index: usize,
) -> bool {
    if reader.is_null() {
        return false;
    }
    let schema = match (&(*reader).inner.schemas).get(schema_index) {
        Some(s) => s,
        None => return false,
    };
    match schema.fields.get(field_index) {
        Some(field) => field.field_type.nullable,
        None => false,
    }
}

/// Check if a field is an array type.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_schema_field_is_array(
    reader: *const TLReader,
    schema_index: usize,
    field_index: usize,
) -> bool {
    if reader.is_null() {
        return false;
    }
    let schema = match (&(*reader).inner.schemas).get(schema_index) {
        Some(s) => s,
        None => return false,
    };
    match schema.fields.get(field_index) {
        Some(field) => field.field_type.is_array,
        None => false,
    }
}

// =============================================================================
// Memory Management
// =============================================================================

/// Free a string returned by the library.
#[no_mangle]
pub unsafe extern "C" fn tl_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Free a string array returned by the library.
#[no_mangle]
pub unsafe extern "C" fn tl_string_array_free(arr: *mut *mut c_char) {
    if arr.is_null() {
        return;
    }

    let mut i = 0;
    loop {
        let ptr = *arr.add(i);
        if ptr.is_null() {
            break;
        }
        drop(CString::from_raw(ptr));
        i += 1;
    }

    // Free the array itself
    // Note: We need to know the capacity, so we reconstruct the Vec
    // This is a simplification - in production, track the length
    drop(Vec::from_raw_parts(arr, i + 1, i + 1));
}

/// Free a TLResult's error message if present.
#[no_mangle]
pub unsafe extern "C" fn tl_result_free(result: *mut TLResult) {
    if !result.is_null() && !(*result).error_message.is_null() {
        drop(CString::from_raw((*result).error_message));
        (*result).error_message = ptr::null_mut();
    }
}

// =============================================================================
// Version Info
// =============================================================================

/// Get the library version string.
#[no_mangle]
pub extern "C" fn tl_version() -> *const c_char {
    static VERSION: &[u8] = b"2.0.0-beta.1\0";
    VERSION.as_ptr() as *const c_char
}
