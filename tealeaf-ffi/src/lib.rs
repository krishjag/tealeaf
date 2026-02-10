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
///
/// # Safety
///
/// `text` must be a valid, null-terminated C string or null.
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
///
/// # Safety
///
/// `path` must be a valid, null-terminated C string or null.
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
///
/// # Safety
///
/// `doc` must be a pointer returned by `tl_parse`, `tl_parse_file`, or
/// `tl_document_from_json`, and must not have been freed already. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn tl_document_free(doc: *mut TLDocument) {
    if !doc.is_null() {
        drop(Box::from_raw(doc));
    }
}

/// Get a value from the document by key.
/// Returns NULL if the key doesn't exist.
///
/// # Safety
///
/// `doc` must be a valid `TLDocument` pointer or null. `key` must be a valid,
/// null-terminated C string or null. The returned `TLValue` must be freed with
/// `tl_value_free`.
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
///
/// # Safety
///
/// `doc` must be a valid `TLDocument` pointer or null.
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
///
/// # Safety
///
/// `doc` must be a valid `TLDocument` pointer or null.
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
///
/// # Safety
///
/// `doc` must be a valid `TLDocument` pointer or null.
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
///
/// # Safety
///
/// `doc` must be a valid `TLDocument` pointer or null. `path` must be a valid,
/// null-terminated C string or null.
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
///
/// # Safety
///
/// `json` must be a valid, null-terminated C string or null.
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
///
/// # Safety
///
/// `doc` must be a valid `TLDocument` pointer or null.
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
///
/// # Safety
///
/// `doc` must be a valid `TLDocument` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
        Value::Timestamp(_, _) => TLValueType::Timestamp,
        Value::JsonNumber(_) => TLValueType::String,
    }
}

/// Free a TeaLeaf value.
///
/// # Safety
///
/// `value` must be a pointer returned by a `tl_document_get`, `tl_reader_get`,
/// `tl_value_array_get`, `tl_value_object_get`, `tl_value_map_get_key`,
/// `tl_value_map_get_value`, or `tl_value_tag_value` call, and must not have been
/// freed already. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn tl_value_free(value: *mut TLValue) {
    if !value.is_null() {
        drop(Box::from_raw(value));
    }
}

/// Get boolean value. Returns false if not a bool.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_bool(value: *const TLValue) -> bool {
    if value.is_null() {
        return false;
    }
    (*value).inner.as_bool().unwrap_or(false)
}

/// Get integer value. Returns 0 if not an int.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_int(value: *const TLValue) -> i64 {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_int().unwrap_or(0)
}

/// Get unsigned integer value. Returns 0 if not a uint.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_uint(value: *const TLValue) -> u64 {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_uint().unwrap_or(0)
}

/// Get float value. Returns 0.0 if not a float.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_float(value: *const TLValue) -> f64 {
    if value.is_null() {
        return 0.0;
    }
    (*value).inner.as_float().unwrap_or(0.0)
}

/// Get string value. Returns NULL if not a string.
/// Caller must free with tl_string_free.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_timestamp(value: *const TLValue) -> i64 {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_timestamp_millis().unwrap_or(0)
}

/// Get timestamp timezone offset in minutes. Returns 0 if not a timestamp.
/// Positive values are east of UTC, negative values are west.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_value_as_timestamp_offset(value: *const TLValue) -> i16 {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_timestamp().map(|(_, tz)| tz).unwrap_or(0)
}

/// Get bytes length. Returns 0 if not bytes.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null. The returned pointer borrows
/// from `value` and must not be used after `value` is freed.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_value_map_len(value: *const TLValue) -> usize {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_map().map(|m| m.len()).unwrap_or(0)
}

/// Get map entry key by index. Returns NULL if out of bounds or not a map.
/// Caller must free with tl_value_free.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_value_array_len(value: *const TLValue) -> usize {
    if value.is_null() {
        return 0;
    }
    (*value).inner.as_array().map(|a| a.len()).unwrap_or(0)
}

/// Get array element by index. Returns NULL if out of bounds or not an array.
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null. `key` must be a valid,
/// null-terminated C string or null.
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
///
/// # Safety
///
/// `value` must be a valid `TLValue` pointer or null.
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
///
/// # Safety
///
/// `path` must be a valid, null-terminated C string or null.
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
///
/// # Safety
///
/// `reader` must be a pointer returned by `tl_reader_open` or `tl_reader_open_mmap`,
/// and must not have been freed already. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_free(reader: *mut TLReader) {
    if !reader.is_null() {
        drop(Box::from_raw(reader));
    }
}

/// Get a value from a binary file by key.
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null. `key` must be a valid,
/// null-terminated C string or null.
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
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null.
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
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null.
#[no_mangle]
pub unsafe extern "C" fn tl_reader_schema_count(reader: *const TLReader) -> usize {
    if reader.is_null() {
        return 0;
    }
    (*reader).inner.schemas.len()
}

/// Get a schema name by index.
/// Caller must free the returned string with tl_string_free.
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null.
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
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null.
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
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null.
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
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null.
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
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null.
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
///
/// # Safety
///
/// `reader` must be a valid `TLReader` pointer or null.
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
///
/// # Safety
///
/// `s` must be a pointer returned by a `tl_*` function that documents "caller must
/// free with tl_string_free", and must not have been freed already. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn tl_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Free a string array returned by the library.
///
/// # Safety
///
/// `arr` must be a NULL-terminated array pointer returned by `tl_document_keys`,
/// `tl_reader_keys`, or `tl_value_object_keys`, and must not have been freed already.
/// Null is accepted.
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
///
/// # Safety
///
/// `result` must be a valid pointer to a `TLResult` or null. The `error_message`
/// field must not have been freed already.
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
    static VERSION: &[u8] = b"2.0.0-beta.6\0";
    VERSION.as_ptr() as *const c_char
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::ptr;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    const SIMPLE_FIXTURE: &str = "name: alice\nage: 30\n";

    const ALL_TYPES_FIXTURE: &str = r#"
null_val: ~
bool_true: true
bool_false: false
int_val: 42
int_neg: -123
float_val: 3.14
string_val: "hello world"
string_empty: ""
timestamp_val: 2024-01-15T10:30:00Z
timestamp_offset: 2024-01-15T16:00:00+05:30
array_val: [1, 2, 3]
empty_array: []
object_val: {name: alice, age: 30}
empty_object: {}
tagged_val: :ok 200
"#;

    const SCHEMA_FIXTURE: &str = r#"
@struct Person (name: string, age: int)
people: @table Person [
  ("Alice", 30),
  ("Bob", 25),
]
"#;

    const MAP_FIXTURE: &str = r#"
headers: @map {"Content-Type": "application/json", "Accept": "*/*"}
"#;

    // =========================================================================
    // Helpers
    // =========================================================================

    unsafe fn read_and_free_string(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null(), "Expected non-null string pointer");
        let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        tl_string_free(ptr);
        s
    }


    unsafe fn collect_and_free_string_array(arr: *mut *mut c_char) -> Vec<String> {
        assert!(!arr.is_null(), "Expected non-null string array");
        let mut result = Vec::new();
        let mut i = 0;
        loop {
            let ptr = *arr.add(i);
            if ptr.is_null() {
                break;
            }
            result.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
            i += 1;
        }
        tl_string_array_free(arr);
        result
    }

    unsafe fn parse_doc(text: &str) -> *mut TLDocument {
        let text_c = CString::new(text).unwrap();
        let doc = tl_parse(text_c.as_ptr());
        assert!(!doc.is_null(), "Failed to parse test document");
        doc
    }

    unsafe fn doc_get(doc: *const TLDocument, key: &str) -> *mut TLValue {
        let key_c = CString::new(key).unwrap();
        let val = tl_document_get(doc, key_c.as_ptr());
        assert!(!val.is_null(), "Expected non-null value for key '{}'", key);
        val
    }

    unsafe fn assert_last_error_contains(substring: &str) {
        let err = tl_get_last_error();
        assert!(!err.is_null(), "Expected error to be set, but was null");
        let msg = CStr::from_ptr(err).to_string_lossy().into_owned();
        tl_string_free(err);
        assert!(
            msg.contains(substring),
            "Expected error containing '{}', got: '{}'",
            substring, msg
        );
    }

    unsafe fn assert_no_error() {
        let err = tl_get_last_error();
        if !err.is_null() {
            let msg = CStr::from_ptr(err).to_string_lossy().into_owned();
            tl_string_free(err);
            panic!("Expected no error, but found: '{}'", msg);
        }
    }

    // =========================================================================
    // Group 1: Null Pointer Safety — Document API
    // =========================================================================

    #[test]
    fn null_tl_parse() {
        unsafe {
            tl_clear_error();
            let result = tl_parse(ptr::null());
            assert!(result.is_null());
            assert_last_error_contains("Null pointer");
        }
    }

    #[test]
    fn null_tl_parse_file() {
        unsafe {
            tl_clear_error();
            let result = tl_parse_file(ptr::null());
            assert!(result.is_null());
            assert_last_error_contains("Null pointer");
        }
    }

    #[test]
    fn null_tl_document_free() {
        unsafe {
            tl_document_free(ptr::null_mut()); // Should not crash
        }
    }

    #[test]
    fn null_tl_document_get_null_doc() {
        unsafe {
            let key = CString::new("key").unwrap();
            let result = tl_document_get(ptr::null(), key.as_ptr());
            assert!(result.is_null());
        }
    }

    #[test]
    fn null_tl_document_get_null_key() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let result = tl_document_get(doc, ptr::null());
            assert!(result.is_null());
            tl_document_free(doc);
        }
    }

    #[test]
    fn null_tl_document_keys() {
        unsafe {
            let result = tl_document_keys(ptr::null());
            assert!(result.is_null());
        }
    }

    #[test]
    fn null_tl_document_to_text() {
        unsafe {
            let result = tl_document_to_text(ptr::null());
            assert!(result.is_null());
        }
    }

    #[test]
    fn null_tl_document_to_text_data_only() {
        unsafe {
            let result = tl_document_to_text_data_only(ptr::null());
            assert!(result.is_null());
        }
    }

    #[test]
    fn null_tl_document_compile() {
        unsafe {
            let path = CString::new("test.tlbx").unwrap();
            let result = tl_document_compile(ptr::null(), path.as_ptr(), false);
            assert!(!result.success);
            if !result.error_message.is_null() {
                tl_string_free(result.error_message);
            }
        }
    }

    #[test]
    fn null_tl_document_from_json() {
        unsafe {
            tl_clear_error();
            let result = tl_document_from_json(ptr::null());
            assert!(result.is_null());
            assert_last_error_contains("Null pointer");
        }
    }

    #[test]
    fn null_tl_document_to_json() {
        unsafe {
            tl_clear_error();
            let result = tl_document_to_json(ptr::null());
            assert!(result.is_null());
            assert_last_error_contains("Null pointer");
        }
    }

    #[test]
    fn null_tl_document_to_json_compact() {
        unsafe {
            tl_clear_error();
            let result = tl_document_to_json_compact(ptr::null());
            assert!(result.is_null());
            assert_last_error_contains("Null pointer");
        }
    }

    // =========================================================================
    // Group 1: Null Pointer Safety — Value API
    // =========================================================================

    #[test]
    fn null_tl_value_free() {
        unsafe {
            tl_value_free(ptr::null_mut()); // Should not crash
        }
    }

    #[test]
    fn null_tl_value_type() {
        unsafe {
            let t = tl_value_type(ptr::null());
            assert!(matches!(t, TLValueType::Null));
        }
    }

    #[test]
    fn null_value_scalar_accessors() {
        unsafe {
            assert_eq!(tl_value_as_bool(ptr::null()), false);
            assert_eq!(tl_value_as_int(ptr::null()), 0);
            assert_eq!(tl_value_as_uint(ptr::null()), 0);
            assert_eq!(tl_value_as_float(ptr::null()), 0.0);
            assert!(tl_value_as_string(ptr::null()).is_null());
            assert_eq!(tl_value_as_timestamp(ptr::null()), 0);
            assert_eq!(tl_value_bytes_len(ptr::null()), 0);
            assert!(tl_value_bytes_data(ptr::null()).is_null());
            assert!(tl_value_ref_name(ptr::null()).is_null());
            assert!(tl_value_tag_name(ptr::null()).is_null());
            assert!(tl_value_tag_value(ptr::null()).is_null());
        }
    }

    #[test]
    fn null_value_collection_accessors() {
        unsafe {
            assert_eq!(tl_value_array_len(ptr::null()), 0);
            assert!(tl_value_array_get(ptr::null(), 0).is_null());
            assert_eq!(tl_value_map_len(ptr::null()), 0);
            assert!(tl_value_map_get_key(ptr::null(), 0).is_null());
            assert!(tl_value_map_get_value(ptr::null(), 0).is_null());
            assert!(tl_value_object_get(ptr::null(), CString::new("k").unwrap().as_ptr()).is_null());
            assert!(tl_value_object_keys(ptr::null()).is_null());
        }
    }

    #[test]
    fn null_value_object_get_null_key() {
        unsafe {
            let doc = parse_doc("obj: {a: 1}");
            let val = doc_get(doc, "obj");
            let result = tl_value_object_get(val, ptr::null());
            assert!(result.is_null());
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 1: Null Pointer Safety — Reader API
    // =========================================================================

    #[test]
    fn null_reader_open() {
        unsafe {
            tl_clear_error();
            let result = tl_reader_open(ptr::null());
            assert!(result.is_null());
            assert_last_error_contains("Null pointer");
        }
    }

    #[test]
    fn null_reader_open_mmap() {
        unsafe {
            tl_clear_error();
            let result = tl_reader_open_mmap(ptr::null());
            assert!(result.is_null());
            assert_last_error_contains("Null pointer");
        }
    }

    #[test]
    fn null_reader_free() {
        unsafe {
            tl_reader_free(ptr::null_mut()); // Should not crash
        }
    }

    #[test]
    fn null_reader_get() {
        unsafe {
            let key = CString::new("key").unwrap();
            let result = tl_reader_get(ptr::null(), key.as_ptr());
            assert!(result.is_null());
        }
    }

    #[test]
    fn null_reader_keys() {
        unsafe {
            let result = tl_reader_keys(ptr::null());
            assert!(result.is_null());
        }
    }

    #[test]
    fn null_reader_schema_functions() {
        unsafe {
            assert_eq!(tl_reader_schema_count(ptr::null()), 0);
            assert!(tl_reader_schema_name(ptr::null(), 0).is_null());
            assert_eq!(tl_reader_schema_field_count(ptr::null(), 0), 0);
            assert!(tl_reader_schema_field_name(ptr::null(), 0, 0).is_null());
            assert!(tl_reader_schema_field_type(ptr::null(), 0, 0).is_null());
            assert_eq!(tl_reader_schema_field_nullable(ptr::null(), 0, 0), false);
            assert_eq!(tl_reader_schema_field_is_array(ptr::null(), 0, 0), false);
        }
    }

    // =========================================================================
    // Group 1: Null Pointer Safety — Memory Management
    // =========================================================================

    #[test]
    fn null_string_free() {
        unsafe {
            tl_string_free(ptr::null_mut()); // Should not crash
        }
    }

    #[test]
    fn null_string_array_free() {
        unsafe {
            tl_string_array_free(ptr::null_mut()); // Should not crash
        }
    }

    #[test]
    fn null_result_free() {
        unsafe {
            tl_result_free(ptr::null_mut()); // Should not crash
        }
    }

    // =========================================================================
    // Group 2: Error Handling Lifecycle
    // =========================================================================

    #[test]
    fn error_initially_null() {
        tl_clear_error();
        let err = tl_get_last_error();
        assert!(err.is_null(), "Error should be null initially");
    }

    #[test]
    fn error_set_on_parse_failure() {
        unsafe {
            tl_clear_error();
            let bad = CString::new("key: \"unterminated").unwrap();
            let doc = tl_parse(bad.as_ptr());
            assert!(doc.is_null(), "Unterminated string should fail to parse");
            let err = tl_get_last_error();
            assert!(!err.is_null(), "Error should be set after parse failure");
            tl_string_free(err);
        }
    }

    #[test]
    fn error_clear_works() {
        unsafe {
            // Set an error
            let bad = CString::new("key: \"unterminated").unwrap();
            let _ = tl_parse(bad.as_ptr());
            // Clear it
            tl_clear_error();
            assert_no_error();
        }
    }

    #[test]
    fn error_overwritten_by_successful_call() {
        unsafe {
            // Trigger an error
            let bad = CString::new("key: \"unterminated").unwrap();
            let _ = tl_parse(bad.as_ptr());
            // Successful call should clear the error
            let good = CString::new(SIMPLE_FIXTURE).unwrap();
            let doc = tl_parse(good.as_ptr());
            assert!(!doc.is_null());
            assert_no_error();
            tl_document_free(doc);
        }
    }

    #[test]
    fn error_sticky_across_non_error_calls() {
        unsafe {
            // Trigger an error
            let bad = CString::new("key: \"unterminated").unwrap();
            let _ = tl_parse(bad.as_ptr());
            // Non-error-setting calls should NOT clear the error
            let _ = tl_value_as_int(ptr::null());
            let _ = tl_value_type(ptr::null());
            // Error should still be there
            let err = tl_get_last_error();
            assert!(!err.is_null(), "Error should persist across non-error-setting calls");
            tl_string_free(err);
            tl_clear_error();
        }
    }

    // =========================================================================
    // Group 3: Memory Lifecycle
    // =========================================================================

    #[test]
    fn lifecycle_document() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            // Use it
            let keys = tl_document_keys(doc);
            assert!(!keys.is_null());
            let key_list = collect_and_free_string_array(keys);
            assert!(key_list.len() >= 2);
            // Free it
            tl_document_free(doc);
        }
    }

    #[test]
    fn lifecycle_value() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let val = doc_get(doc, "name");
            // Read it
            let s = tl_value_as_string(val);
            let name = read_and_free_string(s);
            assert_eq!(name, "alice");
            // Free value first, then document
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn lifecycle_string() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let val = doc_get(doc, "name");
            let s = tl_value_as_string(val);
            assert!(!s.is_null());
            let name = read_and_free_string(s);
            assert_eq!(name, "alice");
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn lifecycle_string_array() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let keys = tl_document_keys(doc);
            assert!(!keys.is_null());
            let key_list = collect_and_free_string_array(keys);
            assert!(key_list.contains(&"name".to_string()));
            assert!(key_list.contains(&"age".to_string()));
            tl_document_free(doc);
        }
    }

    #[test]
    fn lifecycle_result() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let bad_path = CString::new("nonexistent_dir/sub/out.tlbx").unwrap();
            let mut result = tl_document_compile(doc, bad_path.as_ptr(), false);
            assert!(!result.success, "Compile to bad path should fail");
            tl_result_free(&mut result as *mut TLResult);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 4: Value Type Access
    // =========================================================================

    #[test]
    fn value_type_null() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "null_val");
            assert!(matches!(tl_value_type(val), TLValueType::Null));
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_bool_true() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "bool_true");
            assert!(matches!(tl_value_type(val), TLValueType::Bool));
            assert_eq!(tl_value_as_bool(val), true);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_bool_false() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "bool_false");
            assert!(matches!(tl_value_type(val), TLValueType::Bool));
            assert_eq!(tl_value_as_bool(val), false);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_int() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "int_val");
            assert!(matches!(tl_value_type(val), TLValueType::Int));
            assert_eq!(tl_value_as_int(val), 42);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_int_negative() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "int_neg");
            assert!(matches!(tl_value_type(val), TLValueType::Int));
            assert_eq!(tl_value_as_int(val), -123);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_float() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "float_val");
            assert!(matches!(tl_value_type(val), TLValueType::Float));
            let f = tl_value_as_float(val);
            assert!((f - 3.14).abs() < 0.001, "Expected ~3.14, got {}", f);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_string() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "string_val");
            assert!(matches!(tl_value_type(val), TLValueType::String));
            let s = tl_value_as_string(val);
            let name = read_and_free_string(s);
            assert_eq!(name, "hello world");
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_string_empty() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "string_empty");
            assert!(matches!(tl_value_type(val), TLValueType::String));
            let s = tl_value_as_string(val);
            let name = read_and_free_string(s);
            assert_eq!(name, "");
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_timestamp() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "timestamp_val");
            assert!(matches!(tl_value_type(val), TLValueType::Timestamp));
            let ts = tl_value_as_timestamp(val);
            assert!(ts > 0, "Timestamp should be positive, got {}", ts);
            // UTC timestamp should have offset 0
            let tz = tl_value_as_timestamp_offset(val);
            assert_eq!(tz, 0, "UTC timestamp should have offset 0");
            tl_value_free(val);

            // Timestamp with +05:30 offset
            let val2 = doc_get(doc, "timestamp_offset");
            assert!(matches!(tl_value_type(val2), TLValueType::Timestamp));
            let ts2 = tl_value_as_timestamp(val2);
            assert!(ts2 > 0, "Offset timestamp should be positive, got {}", ts2);
            let tz2 = tl_value_as_timestamp_offset(val2);
            assert_eq!(tz2, 330, "+05:30 should be 330 minutes, got {}", tz2);
            tl_value_free(val2);

            // Non-timestamp should return 0
            let int_val = doc_get(doc, "int_val");
            assert_eq!(tl_value_as_timestamp_offset(int_val), 0);
            tl_value_free(int_val);

            // Null should return 0
            assert_eq!(tl_value_as_timestamp_offset(ptr::null()), 0);

            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_array() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "array_val");
            assert!(matches!(tl_value_type(val), TLValueType::Array));
            assert_eq!(tl_value_array_len(val), 3);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_object() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "object_val");
            assert!(matches!(tl_value_type(val), TLValueType::Object));
            let keys = tl_value_object_keys(val);
            let key_list = collect_and_free_string_array(keys);
            assert!(key_list.contains(&"name".to_string()));
            assert!(key_list.contains(&"age".to_string()));
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_tagged() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "tagged_val");
            assert!(matches!(tl_value_type(val), TLValueType::Tagged));
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn value_type_map() {
        unsafe {
            let doc = parse_doc(MAP_FIXTURE);
            let val = doc_get(doc, "headers");
            assert!(matches!(tl_value_type(val), TLValueType::Map));
            assert_eq!(tl_value_map_len(val), 2);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 5: Wrong-Type Access
    // =========================================================================

    #[test]
    fn wrong_type_int_on_string() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "string_val");
            assert_eq!(tl_value_as_int(val), 0);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn wrong_type_string_on_int() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "int_val");
            let s = tl_value_as_string(val);
            assert!(s.is_null());
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn wrong_type_bool_on_int() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "int_val");
            assert_eq!(tl_value_as_bool(val), false);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn wrong_type_float_on_string() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "string_val");
            assert_eq!(tl_value_as_float(val), 0.0);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn wrong_type_array_len_on_object() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "object_val");
            assert_eq!(tl_value_array_len(val), 0);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn wrong_type_map_len_on_array() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "array_val");
            assert_eq!(tl_value_map_len(val), 0);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn wrong_type_ref_name_on_string() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "string_val");
            let r = tl_value_ref_name(val);
            assert!(r.is_null());
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 6: Collection Bounds Access
    // =========================================================================

    #[test]
    fn array_in_bounds() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "array_val");
            for i in 0..3 {
                let elem = tl_value_array_get(val, i);
                assert!(!elem.is_null(), "Element {} should not be null", i);
                tl_value_free(elem);
            }
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn array_out_of_bounds() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "array_val");
            assert!(tl_value_array_get(val, 3).is_null());
            assert!(tl_value_array_get(val, usize::MAX).is_null());
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn array_empty() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "empty_array");
            assert_eq!(tl_value_array_len(val), 0);
            assert!(tl_value_array_get(val, 0).is_null());
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn map_in_bounds() {
        unsafe {
            let doc = parse_doc(MAP_FIXTURE);
            let val = doc_get(doc, "headers");
            let len = tl_value_map_len(val);
            assert_eq!(len, 2);
            for i in 0..len {
                let k = tl_value_map_get_key(val, i);
                assert!(!k.is_null(), "Map key {} should not be null", i);
                let v = tl_value_map_get_value(val, i);
                assert!(!v.is_null(), "Map value {} should not be null", i);
                tl_value_free(k);
                tl_value_free(v);
            }
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn map_out_of_bounds() {
        unsafe {
            let doc = parse_doc(MAP_FIXTURE);
            let val = doc_get(doc, "headers");
            assert!(tl_value_map_get_key(val, 99).is_null());
            assert!(tl_value_map_get_value(val, 99).is_null());
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn object_get_existing_key() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "object_val");
            let key = CString::new("name").unwrap();
            let inner = tl_value_object_get(val, key.as_ptr());
            assert!(!inner.is_null());
            let s = tl_value_as_string(inner);
            let name = read_and_free_string(s);
            assert_eq!(name, "alice");
            tl_value_free(inner);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn object_get_missing_key() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "object_val");
            let key = CString::new("nonexistent").unwrap();
            let inner = tl_value_object_get(val, key.as_ptr());
            assert!(inner.is_null());
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn object_keys_correct() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "object_val");
            let keys = tl_value_object_keys(val);
            let key_list = collect_and_free_string_array(keys);
            assert_eq!(key_list.len(), 2);
            assert!(key_list.contains(&"name".to_string()));
            assert!(key_list.contains(&"age".to_string()));
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 7: Tagged Values
    // =========================================================================

    #[test]
    fn tagged_value_name_and_inner() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "tagged_val");
            assert!(matches!(tl_value_type(val), TLValueType::Tagged));

            let tag = tl_value_tag_name(val);
            let tag_str = read_and_free_string(tag);
            assert_eq!(tag_str, "ok");

            let inner = tl_value_tag_value(val);
            assert!(!inner.is_null());
            assert_eq!(tl_value_as_int(inner), 200);
            tl_value_free(inner);

            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn tag_accessors_on_non_tagged_value() {
        unsafe {
            let doc = parse_doc(ALL_TYPES_FIXTURE);
            let val = doc_get(doc, "int_val");
            assert!(tl_value_tag_name(val).is_null());
            assert!(tl_value_tag_value(val).is_null());
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 8: Schema Introspection via Reader
    // =========================================================================

    #[test]
    fn schema_introspection() {
        unsafe {
            let doc = parse_doc(SCHEMA_FIXTURE);
            let temp = tempfile::NamedTempFile::new().expect("temp file");
            let path_str = temp.path().to_str().expect("path");
            let path_c = CString::new(path_str).unwrap();

            let result = tl_document_compile(doc, path_c.as_ptr(), false);
            assert!(result.success, "Compile should succeed");

            let reader = tl_reader_open(path_c.as_ptr());
            assert!(!reader.is_null(), "Reader should open compiled file");

            // Schema count
            let count = tl_reader_schema_count(reader);
            assert!(count >= 1, "Should have at least 1 schema, got {}", count);

            // Schema name
            let name = tl_reader_schema_name(reader, 0);
            let name_str = read_and_free_string(name);
            assert_eq!(name_str, "Person");

            // Field count
            let field_count = tl_reader_schema_field_count(reader, 0);
            assert_eq!(field_count, 2);

            // Field names
            let f0_name = read_and_free_string(tl_reader_schema_field_name(reader, 0, 0));
            let f1_name = read_and_free_string(tl_reader_schema_field_name(reader, 0, 1));
            assert_eq!(f0_name, "name");
            assert_eq!(f1_name, "age");

            // Field types
            let f0_type = read_and_free_string(tl_reader_schema_field_type(reader, 0, 0));
            let f1_type = read_and_free_string(tl_reader_schema_field_type(reader, 0, 1));
            assert_eq!(f0_type, "string");
            assert_eq!(f1_type, "int");

            tl_reader_free(reader);
            tl_document_free(doc);
        }
    }

    #[test]
    fn schema_out_of_bounds() {
        unsafe {
            let doc = parse_doc(SCHEMA_FIXTURE);
            let temp = tempfile::NamedTempFile::new().expect("temp file");
            let path_str = temp.path().to_str().expect("path");
            let path_c = CString::new(path_str).unwrap();

            let result = tl_document_compile(doc, path_c.as_ptr(), false);
            assert!(result.success);

            let reader = tl_reader_open(path_c.as_ptr());
            assert!(!reader.is_null());

            let count = tl_reader_schema_count(reader);
            // Out of bounds schema
            assert!(tl_reader_schema_name(reader, count).is_null());
            assert_eq!(tl_reader_schema_field_count(reader, count), 0);

            // Out of bounds field
            let fc = tl_reader_schema_field_count(reader, 0);
            assert!(tl_reader_schema_field_name(reader, 0, fc).is_null());
            assert!(tl_reader_schema_field_type(reader, 0, fc).is_null());
            assert_eq!(tl_reader_schema_field_nullable(reader, 0, fc), false);
            assert_eq!(tl_reader_schema_field_is_array(reader, 0, fc), false);

            tl_reader_free(reader);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 9: Reader Operations
    // =========================================================================

    #[test]
    fn reader_open_valid_file() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let temp = tempfile::NamedTempFile::new().expect("temp file");
            let path_str = temp.path().to_str().expect("path");
            let path_c = CString::new(path_str).unwrap();

            let result = tl_document_compile(doc, path_c.as_ptr(), false);
            assert!(result.success);

            let reader = tl_reader_open(path_c.as_ptr());
            assert!(!reader.is_null());

            tl_reader_free(reader);
            tl_document_free(doc);
        }
    }

    #[test]
    fn reader_open_nonexistent_file() {
        unsafe {
            tl_clear_error();
            let path = CString::new("totally_nonexistent_file.tlbx").unwrap();
            let reader = tl_reader_open(path.as_ptr());
            assert!(reader.is_null());
            let err = tl_get_last_error();
            assert!(!err.is_null(), "Error should be set for nonexistent file");
            tl_string_free(err);
        }
    }

    #[test]
    fn reader_keys_and_get() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let temp = tempfile::NamedTempFile::new().expect("temp file");
            let path_str = temp.path().to_str().expect("path");
            let path_c = CString::new(path_str).unwrap();

            let result = tl_document_compile(doc, path_c.as_ptr(), false);
            assert!(result.success);

            let reader = tl_reader_open(path_c.as_ptr());
            assert!(!reader.is_null());

            // Keys
            let keys = tl_reader_keys(reader);
            let key_list = collect_and_free_string_array(keys);
            assert!(key_list.contains(&"name".to_string()));
            assert!(key_list.contains(&"age".to_string()));

            // Get existing key
            let key_c = CString::new("name").unwrap();
            let val = tl_reader_get(reader, key_c.as_ptr());
            assert!(!val.is_null());
            let s = tl_value_as_string(val);
            let name = read_and_free_string(s);
            assert_eq!(name, "alice");
            tl_value_free(val);

            // Get missing key
            let bad_key = CString::new("nonexistent").unwrap();
            let val = tl_reader_get(reader, bad_key.as_ptr());
            assert!(val.is_null());

            tl_reader_free(reader);
            tl_document_free(doc);
        }
    }

    #[test]
    fn reader_get_null_key() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let temp = tempfile::NamedTempFile::new().expect("temp file");
            let path_str = temp.path().to_str().expect("path");
            let path_c = CString::new(path_str).unwrap();

            let result = tl_document_compile(doc, path_c.as_ptr(), false);
            assert!(result.success);

            let reader = tl_reader_open(path_c.as_ptr());
            assert!(!reader.is_null());

            let val = tl_reader_get(reader, ptr::null());
            assert!(val.is_null());

            tl_reader_free(reader);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 10: JSON Roundtrip Through FFI
    // =========================================================================

    #[test]
    fn json_roundtrip() {
        unsafe {
            tl_clear_error();
            // Parse text -> to_json
            let doc = parse_doc(SIMPLE_FIXTURE);
            let json_ptr = tl_document_to_json(doc);
            let json_str = read_and_free_string(json_ptr);
            tl_document_free(doc);

            // from_json -> to_json again
            let json_c = CString::new(json_str.as_str()).unwrap();
            let doc2 = tl_document_from_json(json_c.as_ptr());
            assert!(!doc2.is_null(), "from_json should succeed");
            let json_ptr2 = tl_document_to_json(doc2);
            let json_str2 = read_and_free_string(json_ptr2);
            tl_document_free(doc2);

            // Parse both as serde_json values and compare
            let val1: serde_json::Value = serde_json::from_str(&json_str).unwrap();
            let val2: serde_json::Value = serde_json::from_str(&json_str2).unwrap();
            assert_eq!(val1, val2, "JSON roundtrip should be stable");
        }
    }

    #[test]
    fn json_compact_vs_pretty() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);

            let pretty_ptr = tl_document_to_json(doc);
            let pretty = read_and_free_string(pretty_ptr);

            let compact_ptr = tl_document_to_json_compact(doc);
            let compact = read_and_free_string(compact_ptr);

            tl_document_free(doc);

            let val_pretty: serde_json::Value = serde_json::from_str(&pretty).unwrap();
            let val_compact: serde_json::Value = serde_json::from_str(&compact).unwrap();
            assert_eq!(val_pretty, val_compact, "Pretty and compact JSON should parse to same value");
        }
    }

    #[test]
    fn json_from_invalid() {
        unsafe {
            tl_clear_error();
            let bad = CString::new("{invalid json}").unwrap();
            let doc = tl_document_from_json(bad.as_ptr());
            assert!(doc.is_null());
            let err = tl_get_last_error();
            assert!(!err.is_null(), "Error should be set for invalid JSON");
            tl_string_free(err);
        }
    }

    // =========================================================================
    // Group 11: Compile Through FFI
    // =========================================================================

    #[test]
    fn compile_and_read_back() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let temp = tempfile::NamedTempFile::new().expect("temp file");
            let path_str = temp.path().to_str().expect("path");
            let path_c = CString::new(path_str).unwrap();

            let result = tl_document_compile(doc, path_c.as_ptr(), false);
            assert!(result.success, "Compile should succeed");

            let reader = tl_reader_open(path_c.as_ptr());
            assert!(!reader.is_null());

            let key_c = CString::new("name").unwrap();
            let val = tl_reader_get(reader, key_c.as_ptr());
            assert!(!val.is_null());
            let s = tl_value_as_string(val);
            let name = read_and_free_string(s);
            assert_eq!(name, "alice");

            let key_c2 = CString::new("age").unwrap();
            let val2 = tl_reader_get(reader, key_c2.as_ptr());
            assert!(!val2.is_null());
            assert_eq!(tl_value_as_int(val2), 30);

            tl_value_free(val);
            tl_value_free(val2);
            tl_reader_free(reader);
            tl_document_free(doc);
        }
    }

    #[test]
    fn compile_null_path() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let result = tl_document_compile(doc, ptr::null(), false);
            assert!(!result.success);
            if !result.error_message.is_null() {
                tl_string_free(result.error_message);
            }
            tl_document_free(doc);
        }
    }

    #[test]
    fn compile_invalid_path() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);
            let bad_path = CString::new("nonexistent_dir/sub/deep/out.tlbx").unwrap();
            let mut result = tl_document_compile(doc, bad_path.as_ptr(), false);
            assert!(!result.success);
            tl_result_free(&mut result as *mut TLResult);
            tl_document_free(doc);
        }
    }

    // =========================================================================
    // Group 12: Version API
    // =========================================================================

    #[test]
    fn version_returns_expected() {
        let v = tl_version();
        assert!(!v.is_null());
        let version = unsafe { CStr::from_ptr(v) }.to_str().unwrap();
        assert_eq!(version, "2.0.0-beta.6");
        // Note: do NOT free this - it's a static string
    }

    // =========================================================================
    // Group 13: Edge Cases
    // =========================================================================

    #[test]
    fn parse_empty_string() {
        unsafe {
            let empty = CString::new("").unwrap();
            let doc = tl_parse(empty.as_ptr());
            assert!(!doc.is_null(), "Empty string should parse successfully");
            let keys = tl_document_keys(doc);
            let key_list = collect_and_free_string_array(keys);
            assert_eq!(key_list.len(), 0, "Empty doc should have 0 keys");
            tl_document_free(doc);
        }
    }

    #[test]
    fn parse_whitespace_only() {
        unsafe {
            let ws = CString::new("  \n\n  ").unwrap();
            let doc = tl_parse(ws.as_ptr());
            assert!(!doc.is_null(), "Whitespace-only should parse successfully");
            let keys = tl_document_keys(doc);
            let key_list = collect_and_free_string_array(keys);
            assert_eq!(key_list.len(), 0);
            tl_document_free(doc);
        }
    }

    #[test]
    fn document_to_text_roundtrip() {
        unsafe {
            let doc = parse_doc(SIMPLE_FIXTURE);

            // to_json for baseline comparison
            let json_ptr = tl_document_to_json(doc);
            let json1 = read_and_free_string(json_ptr);

            // to_text
            let text_ptr = tl_document_to_text(doc);
            let text = read_and_free_string(text_ptr);
            tl_document_free(doc);

            // parse the text back
            let text_c = CString::new(text.as_str()).unwrap();
            let doc2 = tl_parse(text_c.as_ptr());
            assert!(!doc2.is_null(), "Re-parsing text output should succeed");

            // to_json from re-parsed document
            let json_ptr2 = tl_document_to_json(doc2);
            let json2 = read_and_free_string(json_ptr2);
            tl_document_free(doc2);

            // Compare as parsed JSON (order-independent)
            let val1: serde_json::Value = serde_json::from_str(&json1).unwrap();
            let val2: serde_json::Value = serde_json::from_str(&json2).unwrap();
            assert_eq!(val1, val2, "text -> parse -> text roundtrip should preserve content");
        }
    }

    #[test]
    fn parse_invalid_syntax() {
        unsafe {
            tl_clear_error();
            let bad = CString::new("key: \"unterminated string").unwrap();
            let doc = tl_parse(bad.as_ptr());
            assert!(doc.is_null(), "Unterminated string should fail to parse");
            let err = tl_get_last_error();
            assert!(!err.is_null(), "Error should be set for invalid syntax");
            tl_string_free(err);
        }
    }

    #[test]
    fn long_string_value() {
        unsafe {
            let long_str = "x".repeat(10_000);
            let input = format!("long: \"{}\"", long_str);
            let doc = parse_doc(&input);
            let val = doc_get(doc, "long");
            let s = tl_value_as_string(val);
            let extracted = read_and_free_string(s);
            assert_eq!(extracted.len(), 10_000, "Long string should preserve length");
            assert_eq!(extracted, long_str);
            tl_value_free(val);
            tl_document_free(doc);
        }
    }

    #[test]
    fn bytes_literal_parse_and_roundtrip() {
        unsafe {
            // Parse .tl text containing b"..." bytes literal
            let doc = parse_doc(r#"payload: b"cafef00d"
empty: b""
"#);

            // Verify bytes values via FFI accessors
            let payload = doc_get(doc, "payload");
            assert!(matches!(tl_value_type(payload), TLValueType::Bytes));
            assert_eq!(tl_value_bytes_len(payload), 4);
            let data_ptr = tl_value_bytes_data(payload);
            assert!(!data_ptr.is_null());
            let bytes = std::slice::from_raw_parts(data_ptr, 4);
            assert_eq!(bytes, &[0xca, 0xfe, 0xf0, 0x0d]);
            tl_value_free(payload);

            let empty = doc_get(doc, "empty");
            assert!(matches!(tl_value_type(empty), TLValueType::Bytes));
            assert_eq!(tl_value_bytes_len(empty), 0);
            tl_value_free(empty);

            // Round-trip: to_text should emit b"..." syntax
            let text_ptr = tl_document_to_text(doc);
            let text = read_and_free_string(text_ptr);
            assert!(text.contains(r#"b"cafef00d""#), "to_text should emit b\"...\" for bytes: {}", text);
            assert!(text.contains(r#"b"""#), "to_text should emit b\"\" for empty bytes: {}", text);

            // Re-parse the emitted text
            let text_c = CString::new(text.as_str()).unwrap();
            let doc2 = tl_parse(text_c.as_ptr());
            assert!(!doc2.is_null(), "Re-parsing bytes text should succeed");

            let payload2 = doc_get(doc2, "payload");
            assert!(matches!(tl_value_type(payload2), TLValueType::Bytes));
            assert_eq!(tl_value_bytes_len(payload2), 4);
            let data_ptr2 = tl_value_bytes_data(payload2);
            let bytes2 = std::slice::from_raw_parts(data_ptr2, 4);
            assert_eq!(bytes2, &[0xca, 0xfe, 0xf0, 0x0d]);
            tl_value_free(payload2);

            tl_document_free(doc2);
            tl_document_free(doc);
        }
    }

    #[test]
    fn document_to_text_data_only_vs_with_schemas() {
        unsafe {
            let doc = parse_doc(SCHEMA_FIXTURE);

            let with_schemas = read_and_free_string(tl_document_to_text(doc));
            let data_only = read_and_free_string(tl_document_to_text_data_only(doc));

            // with_schemas should contain @struct, data_only should not
            assert!(with_schemas.contains("@struct"), "to_text should include schemas");
            assert!(!data_only.contains("@struct"), "to_text_data_only should not include schemas");

            tl_document_free(doc);
        }
    }
}
