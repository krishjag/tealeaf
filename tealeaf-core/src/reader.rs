//! Binary format reader for TeaLeaf
//!
//! Supports two modes:
//! - `open()` - Reads file into memory (Vec<u8>)
//! - `open_mmap()` - Memory-maps file for zero-copy access

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use indexmap::IndexMap;
use crate::types::ObjectMap;

use memmap2::Mmap;

use crate::{Error, Result, Value, Schema, Union, Variant, Field, FieldType, TLType, MAGIC, HEADER_SIZE};

/// Maximum allowed decompressed data size (256 MB)
const MAX_DECOMPRESSED_SIZE: usize = 256 * 1024 * 1024;

/// Maximum varint encoding length in bytes (ceil(64/7) = 10)
const MAX_VARINT_BYTES: usize = 10;

/// Maximum recursion depth for nested decode calls (arrays, objects, maps, tagged values).
/// Set above the text parser's tested 200-level nesting to ensure binary round-trip parity.
const MAX_DECODE_DEPTH: usize = 256;

/// Maximum number of elements allowed in a single decoded collection (array, map, struct array).
/// Also used to cap Vec::with_capacity during decode. Prevents OOM from crafted count values
/// in small files (e.g. a 335-byte file claiming 973M Null elements).
const MAX_COLLECTION_SIZE: usize = 1024 * 1024;

/// Read a u16 from data at the given offset, with bounds checking
fn read_u16_at(data: &[u8], offset: usize) -> Result<u16> {
    let end = offset.checked_add(2)
        .ok_or_else(|| Error::ParseError("offset overflow".into()))?;
    if end > data.len() {
        return Err(Error::ParseError(format!(
            "read u16 out of bounds at offset {} (data len {})", offset, data.len()
        )));
    }
    Ok(u16::from_le_bytes(data[offset..end].try_into().map_err(|_|
        Error::ParseError(format!("u16 slice conversion failed at offset {}", offset))
    )?))
}

/// Read a u32 from data at the given offset, with bounds checking
fn read_u32_at(data: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4)
        .ok_or_else(|| Error::ParseError("offset overflow".into()))?;
    if end > data.len() {
        return Err(Error::ParseError(format!(
            "read u32 out of bounds at offset {} (data len {})", offset, data.len()
        )));
    }
    Ok(u32::from_le_bytes(data[offset..end].try_into().map_err(|_|
        Error::ParseError(format!("u32 slice conversion failed at offset {}", offset))
    )?))
}

/// Read a u64 from data at the given offset, with bounds checking
fn read_u64_at(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8)
        .ok_or_else(|| Error::ParseError("offset overflow".into()))?;
    if end > data.len() {
        return Err(Error::ParseError(format!(
            "read u64 out of bounds at offset {} (data len {})", offset, data.len()
        )));
    }
    Ok(u64::from_le_bytes(data[offset..end].try_into().map_err(|_|
        Error::ParseError(format!("u64 slice conversion failed at offset {}", offset))
    )?))
}

/// Storage backend for reader data
enum DataSource {
    /// Owned bytes (from file read)
    Owned(Vec<u8>),
    /// Memory-mapped file (zero-copy)
    Mapped(Arc<Mmap>),
}

impl AsRef<[u8]> for DataSource {
    fn as_ref(&self) -> &[u8] {
        match self {
            DataSource::Owned(v) => v.as_slice(),
            DataSource::Mapped(m) => m.as_ref(),
        }
    }
}

/// Binary format reader with mmap support for zero-copy access
pub struct Reader {
    data: DataSource,
    string_offsets: Vec<u32>,
    string_lengths: Vec<u32>,
    string_data_offset: usize,
    pub schemas: Vec<Schema>,
    schema_map: HashMap<String, usize>,
    pub unions: Vec<Union>,
    union_map: HashMap<String, usize>,
    sections: IndexMap<String, SectionInfo>,
    /// Indicates the source JSON was a root-level array (for round-trip fidelity)
    is_root_array: bool,
    /// Cache for decompressed and decoded values
    cache: RefCell<HashMap<String, Value>>,
}

#[allow(dead_code)]
struct SectionInfo {
    offset: u64,
    size: u32,
    uncompressed_size: u32,
    schema_idx: i32,
    tl_type: TLType,
    compressed: bool,
    is_array: bool,
    item_count: u32,
}

impl Reader {
    /// Open a binary TeaLeaf file (reads into memory)
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Self::from_bytes(data)
    }

    /// Open a binary TeaLeaf file with memory mapping (zero-copy)
    ///
    /// This is faster for large files as the OS handles paging.
    /// The file must not be modified while the reader is open.
    ///
    /// # Safety
    /// The underlying file must not be modified while the reader exists.
    pub fn open_mmap<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Self::from_data_source(DataSource::Mapped(Arc::new(mmap)))
    }

    /// Create reader from owned bytes
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        Self::from_data_source(DataSource::Owned(data))
    }

    /// Create reader from data source (internal)
    fn from_data_source(data: DataSource) -> Result<Self> {
        let bytes = data.as_ref();

        if bytes.len() < HEADER_SIZE {
            return Err(Error::InvalidMagic);
        }
        if &bytes[0..4] != MAGIC {
            return Err(Error::InvalidMagic);
        }

        // Check version - we support major version 2
        let major = read_u16_at(bytes, 4)?;
        let minor = read_u16_at(bytes, 6)?;
        if major != 2 {
            return Err(Error::InvalidVersion { major, minor });
        }

        // Read flags: bit 0 = compressed (handled per-section), bit 1 = root_array
        let flags = read_u32_at(bytes, 8)?;
        let is_root_array = (flags & 0x02) != 0;

        let str_off = read_u64_at(bytes, 16)? as usize;
        let sch_off = read_u64_at(bytes, 24)? as usize;
        let idx_off = read_u64_at(bytes, 32)? as usize;
        let dat_off = read_u64_at(bytes, 40)? as usize;
        let str_cnt = read_u32_at(bytes, 48)? as usize;
        let sch_cnt = read_u32_at(bytes, 52)? as usize;
        let sec_cnt = read_u32_at(bytes, 56)? as usize;

        // Validate region offsets are within file bounds
        if str_off > bytes.len() || sch_off > bytes.len() || idx_off > bytes.len() || dat_off > bytes.len() {
            return Err(Error::ParseError("header region offsets exceed file size".into()));
        }

        // Parse string table
        let str_header_end = str_off.checked_add(8)
            .ok_or_else(|| Error::ParseError("string table offset overflow".into()))?;
        if str_header_end > bytes.len() {
            return Err(Error::ParseError("string table header out of bounds".into()));
        }

        let offsets_size = str_cnt.checked_mul(4)
            .ok_or_else(|| Error::ParseError("string count overflow".into()))?;
        let lengths_size = str_cnt.checked_mul(4)
            .ok_or_else(|| Error::ParseError("string count overflow".into()))?;
        let str_table_end = str_header_end
            .checked_add(offsets_size)
            .and_then(|v| v.checked_add(lengths_size))
            .ok_or_else(|| Error::ParseError("string table size overflow".into()))?;
        if str_table_end > bytes.len() {
            return Err(Error::ParseError("string table out of bounds".into()));
        }

        let mut off = str_off + 8;
        let string_offsets: Vec<u32> = (0..str_cnt)
            .map(|i| read_u32_at(bytes, off + i * 4))
            .collect::<Result<Vec<u32>>>()?;
        off += offsets_size;
        let string_lengths: Vec<u32> = (0..str_cnt)
            .map(|i| read_u32_at(bytes, off + i * 4))
            .collect::<Result<Vec<u32>>>()?;
        let string_data_offset = off + lengths_size;

        // Read union_count from schema region header (sch_off+6..sch_off+8)
        let union_cnt = if sch_off + 8 <= bytes.len() {
            read_u16_at(bytes, sch_off + 6)? as usize
        } else {
            0
        };

        let mut reader = Self {
            data,
            string_offsets,
            string_lengths,
            string_data_offset,
            schemas: Vec::new(),
            schema_map: HashMap::new(),
            unions: Vec::new(),
            union_map: HashMap::new(),
            sections: IndexMap::new(),
            is_root_array,
            cache: RefCell::new(HashMap::new()),
        };

        reader.parse_schemas(sch_off, sch_cnt)?;
        if union_cnt > 0 {
            reader.parse_unions(sch_off, sch_cnt, union_cnt)?;
        }
        reader.parse_index(idx_off, sec_cnt)?;

        Ok(reader)
    }

    /// Get the underlying data as a byte slice
    fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    /// Get a string by index
    pub fn get_string(&self, idx: usize) -> Result<String> {
        if idx >= self.string_offsets.len() {
            return Err(Error::ParseError(format!("String index {} out of bounds", idx)));
        }
        let start = self.string_data_offset
            .checked_add(self.string_offsets[idx] as usize)
            .ok_or_else(|| Error::ParseError("string data offset overflow".into()))?;
        let len = self.string_lengths[idx] as usize;
        let end = start.checked_add(len)
            .ok_or_else(|| Error::ParseError("string data range overflow".into()))?;
        if end > self.data().len() {
            return Err(Error::ParseError(format!(
                "string data out of bounds: {}..{} exceeds file size {}", start, end, self.data().len()
            )));
        }
        String::from_utf8(self.data()[start..end].to_vec())
            .map_err(|_| Error::InvalidUtf8)
    }

    /// Get section keys
    pub fn keys(&self) -> Vec<&str> {
        self.sections.keys().map(|s| s.as_str()).collect()
    }

    /// Check if the source JSON was a root-level array
    ///
    /// When true, the "root" key contains the array and `to_json` should
    /// output it directly without wrapping in an object.
    pub fn is_root_array(&self) -> bool {
        self.is_root_array
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Result<Value> {
        // Check cache first
        if let Some(cached) = self.cache.borrow().get(key) {
            return Ok(cached.clone());
        }

        let section = self.sections.get(key)
            .ok_or_else(|| Error::MissingField(key.to_string()))?;

        let start = section.offset as usize;
        let end = start.checked_add(section.size as usize)
            .ok_or_else(|| Error::ParseError("section offset overflow".into()))?;
        if end > self.data().len() {
            return Err(Error::ParseError(format!(
                "section '{}' data range {}..{} exceeds file size {}",
                key, start, end, self.data().len()
            )));
        }

        let data: Cow<'_, [u8]> = if section.compressed {
            Cow::Owned(decompress_data(&self.data()[start..end])?)
        } else {
            Cow::Borrowed(&self.data()[start..end])
        };

        let mut cursor = Cursor::new(data.as_ref());

        let result = if section.is_array && section.schema_idx >= 0 {
            self.decode_struct_array(&mut cursor, section.schema_idx as usize, 0)?
        } else {
            match section.tl_type {
                TLType::Array => self.decode_array(&mut cursor, 0)?,
                TLType::Object => self.decode_object(&mut cursor, 0)?,
                TLType::Struct => self.decode_struct(&mut cursor, 0)?,
                TLType::Map => self.decode_map(&mut cursor, 0)?,
                _ => self.decode_value(&mut cursor, section.tl_type, 0)?,
            }
        };

        self.cache.borrow_mut().insert(key.to_string(), result.clone());
        Ok(result)
    }

    /// Clear the decompression cache to free memory
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    fn parse_schemas(&mut self, off: usize, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }

        let data = self.data.as_ref();
        let o = off.checked_add(8)
            .ok_or_else(|| Error::ParseError("schema offset overflow".into()))?;

        // Validate offset table bounds
        let offsets_size = count.checked_mul(4)
            .ok_or_else(|| Error::ParseError("schema count overflow".into()))?;
        let offsets_end = o.checked_add(offsets_size)
            .ok_or_else(|| Error::ParseError("schema offsets overflow".into()))?;
        if offsets_end > data.len() {
            return Err(Error::ParseError("schema offset table out of bounds".into()));
        }

        let offsets: Vec<u32> = (0..count)
            .map(|i| read_u32_at(data, o + i * 4))
            .collect::<Result<Vec<u32>>>()?;
        let start = offsets_end;

        for i in 0..count {
            let so = start.checked_add(offsets[i] as usize)
                .ok_or_else(|| Error::ParseError("schema entry offset overflow".into()))?;

            // Need at least 8 bytes for schema entry header
            if so.checked_add(8).map_or(true, |end| end > data.len()) {
                return Err(Error::ParseError(format!("schema entry {} out of bounds", i)));
            }

            let name_idx = read_u32_at(data, so)?;
            let field_count = read_u16_at(data, so + 4)? as usize;

            let name = self.get_string(name_idx as usize)?;
            let mut schema = Schema::new(&name);

            let mut fo = so + 8;
            for fi in 0..field_count {
                // Each field entry is 8 bytes
                if fo.checked_add(8).map_or(true, |end| end > data.len()) {
                    return Err(Error::ParseError(format!(
                        "schema '{}' field {} out of bounds", name, fi
                    )));
                }

                let fname_idx = read_u32_at(data, fo)?;
                let ftype = data[fo + 4];
                let fflags = data[fo + 5];
                let fextra = read_u16_at(data, fo + 6)?;

                let fname = self.get_string(fname_idx as usize)?;
                let tl_type = TLType::try_from(ftype)?;

                let base = match tl_type {
                    TLType::Bool => "bool".to_string(),
                    TLType::Int8 => "int8".to_string(),
                    TLType::Int16 => "int16".to_string(),
                    TLType::Int32 => "int".to_string(),
                    TLType::Int64 => "int64".to_string(),
                    TLType::UInt8 => "uint8".to_string(),
                    TLType::UInt16 => "uint16".to_string(),
                    TLType::UInt32 => "uint".to_string(),
                    TLType::UInt64 => "uint64".to_string(),
                    TLType::Float32 => "float32".to_string(),
                    TLType::Float64 => "float".to_string(),
                    TLType::String => "string".to_string(),
                    TLType::Bytes => "bytes".to_string(),
                    TLType::Timestamp => "timestamp".to_string(),
                    TLType::Struct => {
                        // Read struct type name from string table (0xFFFF = no type)
                        if fextra != 0xFFFF {
                            self.get_string(fextra as usize)?
                        } else {
                            "object".to_string()
                        }
                    }
                    TLType::Tagged => {
                        // Union-typed field: read union name from string table
                        if fextra != 0xFFFF {
                            self.get_string(fextra as usize)?
                        } else {
                            "tagged".to_string()
                        }
                    }
                    TLType::Object => "object".to_string(),
                    TLType::Tuple => "tuple".to_string(),
                    TLType::Map => "map".to_string(),
                    _ => "string".to_string(),
                };

                let mut field_type = FieldType::new(&base);
                if fflags & 0x01 != 0 {
                    field_type.nullable = true;
                }
                if fflags & 0x02 != 0 {
                    field_type.is_array = true;
                }

                schema.fields.push(Field::new(fname, field_type));
                fo += 8;
            }

            self.schema_map.insert(name, self.schemas.len());
            self.schemas.push(schema);
        }

        Ok(())
    }

    fn parse_unions(&mut self, sch_off: usize, struct_count: usize, union_count: usize) -> Result<()> {
        let data = self.data.as_ref();

        // Calculate where struct offsets + struct data end
        // Schema region layout:
        //   [region_size: u32][struct_count: u16][union_count: u16]
        //   [struct_offsets: u32 * struct_count]
        //   [struct_data...]
        //   [union_offsets: u32 * union_count]
        //   [union_data...]
        let struct_offsets_start = sch_off.checked_add(8)
            .ok_or_else(|| Error::ParseError("union region offset overflow".into()))?;
        let struct_offsets_size = struct_count.checked_mul(4)
            .ok_or_else(|| Error::ParseError("struct count overflow".into()))?;
        let struct_data_start = struct_offsets_start.checked_add(struct_offsets_size)
            .ok_or_else(|| Error::ParseError("struct data start overflow".into()))?;
        let struct_data_size: usize = self.schemas.iter()
            .map(|s| 8 + s.fields.len() * 8)
            .sum();
        let union_offsets_start = struct_data_start.checked_add(struct_data_size)
            .ok_or_else(|| Error::ParseError("union offsets start overflow".into()))?;

        // Validate union offset table bounds
        let union_offsets_size = union_count.checked_mul(4)
            .ok_or_else(|| Error::ParseError("union count overflow".into()))?;
        let union_offsets_end = union_offsets_start.checked_add(union_offsets_size)
            .ok_or_else(|| Error::ParseError("union offsets end overflow".into()))?;
        if union_offsets_end > data.len() {
            return Err(Error::ParseError("union offset table out of bounds".into()));
        }

        // Read union offsets
        let union_offsets: Vec<u32> = (0..union_count)
            .map(|i| read_u32_at(data, union_offsets_start + i * 4))
            .collect::<Result<Vec<u32>>>()?;
        let union_data_start = union_offsets_end;

        for i in 0..union_count {
            let uo = union_data_start.checked_add(union_offsets[i] as usize)
                .ok_or_else(|| Error::ParseError("union entry offset overflow".into()))?;

            // Need at least 8 bytes for union entry header
            if uo.checked_add(8).map_or(true, |end| end > data.len()) {
                return Err(Error::ParseError(format!("union entry {} out of bounds", i)));
            }

            let name_idx = read_u32_at(data, uo)?;
            let variant_count = read_u16_at(data, uo + 4)? as usize;
            // uo + 6..uo + 8 is flags (reserved)

            let name = self.get_string(name_idx as usize)?;
            let mut union = Union::new(&name);

            let mut vo = uo + 8;
            for vi in 0..variant_count {
                // Need at least 8 bytes for variant header
                if vo.checked_add(8).map_or(true, |end| end > data.len()) {
                    return Err(Error::ParseError(format!(
                        "union '{}' variant {} out of bounds", name, vi
                    )));
                }

                let vname_idx = read_u32_at(data, vo)?;
                let field_count = read_u16_at(data, vo + 4)? as usize;
                // vo + 6..vo + 8 is flags (reserved)

                let vname = self.get_string(vname_idx as usize)?;
                let mut variant = Variant::new(&vname);

                let mut fo = vo + 8;
                for fi in 0..field_count {
                    // Each field entry is 8 bytes
                    if fo.checked_add(8).map_or(true, |end| end > data.len()) {
                        return Err(Error::ParseError(format!(
                            "union '{}' variant '{}' field {} out of bounds", name, vname, fi
                        )));
                    }

                    let fname_idx = read_u32_at(data, fo)?;
                    let ftype = data[fo + 4];
                    let fflags = data[fo + 5];
                    let fextra = read_u16_at(data, fo + 6)?;

                    let fname = self.get_string(fname_idx as usize)?;
                    let tl_type = TLType::try_from(ftype)?;

                    let base = match tl_type {
                        TLType::Bool => "bool".to_string(),
                        TLType::Int8 => "int8".to_string(),
                        TLType::Int16 => "int16".to_string(),
                        TLType::Int32 => "int".to_string(),
                        TLType::Int64 => "int64".to_string(),
                        TLType::UInt8 => "uint8".to_string(),
                        TLType::UInt16 => "uint16".to_string(),
                        TLType::UInt32 => "uint".to_string(),
                        TLType::UInt64 => "uint64".to_string(),
                        TLType::Float32 => "float32".to_string(),
                        TLType::Float64 => "float".to_string(),
                        TLType::String => "string".to_string(),
                        TLType::Bytes => "bytes".to_string(),
                        TLType::Timestamp => "timestamp".to_string(),
                        TLType::Struct => {
                            if fextra != 0xFFFF {
                                self.get_string(fextra as usize)?
                            } else {
                                "object".to_string()
                            }
                        }
                        TLType::Tagged => {
                            if fextra != 0xFFFF {
                                self.get_string(fextra as usize)?
                            } else {
                                "tagged".to_string()
                            }
                        }
                        TLType::Object => "object".to_string(),
                        TLType::Tuple => "tuple".to_string(),
                        TLType::Map => "map".to_string(),
                        _ => "string".to_string(),
                    };

                    let mut field_type = FieldType::new(&base);
                    if fflags & 0x01 != 0 { field_type.nullable = true; }
                    if fflags & 0x02 != 0 { field_type.is_array = true; }

                    variant.fields.push(Field::new(fname, field_type));
                    fo += 8;
                }

                union.variants.push(variant);
                vo = fo;
            }

            self.union_map.insert(name, self.unions.len());
            self.unions.push(union);
        }

        Ok(())
    }

    fn parse_index(&mut self, off: usize, count: usize) -> Result<()> {
        let data = self.data.as_ref();
        let mut o = off.checked_add(8)
            .ok_or_else(|| Error::ParseError("index offset overflow".into()))?;

        // Validate index table bounds
        let index_size = count.checked_mul(32)
            .ok_or_else(|| Error::ParseError("index count overflow".into()))?;
        let index_end = o.checked_add(index_size)
            .ok_or_else(|| Error::ParseError("index region overflow".into()))?;
        if index_end > data.len() {
            return Err(Error::ParseError("index table out of bounds".into()));
        }

        for _ in 0..count {
            let key_idx = read_u32_at(data, o)?;
            let offset = read_u64_at(data, o + 4)?;
            let size = read_u32_at(data, o + 12)?;
            let uncompressed = read_u32_at(data, o + 16)?;
            let schema_idx = read_u16_at(data, o + 20)?;
            let ptype = data[o + 22];
            let flags = data[o + 23];
            let item_count = read_u32_at(data, o + 24)?;

            let key = self.get_string(key_idx as usize)?;

            // Validate section data range against file bounds
            let sec_start = offset as usize;
            let sec_end = sec_start.checked_add(size as usize)
                .ok_or_else(|| Error::ParseError(format!(
                    "section '{}' offset overflow", key
                )))?;
            if sec_end > data.len() {
                return Err(Error::ParseError(format!(
                    "section '{}' data range {}..{} exceeds file size {}",
                    key, sec_start, sec_end, data.len()
                )));
            }

            self.sections.insert(key, SectionInfo {
                offset,
                size,
                uncompressed_size: uncompressed,
                schema_idx: if schema_idx == 0xFFFF { -1 } else { schema_idx as i32 },
                tl_type: TLType::try_from(ptype)?,
                compressed: flags & 0x01 != 0,
                is_array: flags & 0x02 != 0,
                item_count,
            });
            o += 32;
        }

        Ok(())
    }

    fn decode_struct_array(&self, cursor: &mut Cursor, schema_idx: usize, depth: usize) -> Result<Value> {
        if depth > MAX_DECODE_DEPTH {
            return Err(Error::ParseError("maximum decode nesting depth exceeded".into()));
        }
        let count = cursor.read_u32()?;
        if count as usize > MAX_COLLECTION_SIZE {
            return Err(Error::ParseError(format!(
                "struct array element count {} exceeds limit of {}", count, MAX_COLLECTION_SIZE
            )));
        }
        let _si = cursor.read_u16()?;
        let bitmap_size = cursor.read_u16()? as usize;

        if schema_idx >= self.schemas.len() {
            return Err(Error::ParseError(format!(
                "struct array schema index {} out of bounds ({} schemas available)",
                schema_idx, self.schemas.len()
            )));
        }
        let schema = &self.schemas[schema_idx];
        let capacity = (count as usize).min(cursor.remaining()).min(MAX_COLLECTION_SIZE);
        let mut result = Vec::with_capacity(capacity);

        // Two-bit field state encoding: bitmap_size = 2 * bms
        let bms = bitmap_size / 2;
        for _ in 0..count {
            let mut bitmap = Vec::with_capacity(bitmap_size.min(cursor.remaining()));
            for _ in 0..bitmap_size {
                bitmap.push(cursor.read_u8()?);
            }
            let lo_bitmap = &bitmap[..bms.min(bitmap.len())];
            let hi_bitmap = if bitmap.len() > bms { &bitmap[bms..] } else { &[] as &[u8] };

            // Null array element: all fields code=2 (lo=0, hi=1)
            let all_absent = (0..schema.fields.len()).all(|i| {
                let lo = i / 8 < lo_bitmap.len() && (lo_bitmap[i / 8] & (1 << (i % 8))) != 0;
                let hi = i / 8 < hi_bitmap.len() && (hi_bitmap[i / 8] & (1 << (i % 8))) != 0;
                !lo && hi
            });

            if all_absent {
                result.push(Value::Null);
            } else {
                let mut obj = ObjectMap::new();
                for (i, field) in schema.fields.iter().enumerate() {
                    let lo = i / 8 < lo_bitmap.len() && (lo_bitmap[i / 8] & (1 << (i % 8))) != 0;
                    let hi = i / 8 < hi_bitmap.len() && (hi_bitmap[i / 8] & (1 << (i % 8))) != 0;
                    let code = (lo as u8) | ((hi as u8) << 1);
                    match code {
                        0 => {
                            // Has value — decode inline data
                            let tl_type = if self.union_map.contains_key(&field.field_type.base) {
                                TLType::Tagged
                            } else {
                                field.field_type.to_tl_type()
                            };
                            obj.insert(field.name.clone(), self.decode_value(cursor, tl_type, depth + 1)?);
                        }
                        1 => {
                            // Explicit null — always preserve
                            obj.insert(field.name.clone(), Value::Null);
                        }
                        2 => {
                            // Absent — drop for nullable fields
                            if !field.field_type.nullable {
                                obj.insert(field.name.clone(), Value::Null);
                            }
                        }
                        _ => {} // reserved
                    }
                }
                result.push(Value::Object(obj));
            }
        }

        Ok(Value::Array(result))
    }

    fn decode_array(&self, cursor: &mut Cursor, depth: usize) -> Result<Value> {
        if depth > MAX_DECODE_DEPTH {
            return Err(Error::ParseError("maximum decode nesting depth exceeded".into()));
        }
        let count = cursor.read_u32()?;
        if count == 0 {
            return Ok(Value::Array(Vec::new()));
        }
        if count as usize > MAX_COLLECTION_SIZE {
            return Err(Error::ParseError(format!(
                "array element count {} exceeds limit of {}", count, MAX_COLLECTION_SIZE
            )));
        }

        let elem_type = cursor.read_u8()?;
        let capacity = (count as usize).min(cursor.remaining()).min(MAX_COLLECTION_SIZE);
        let mut result = Vec::with_capacity(capacity);

        if elem_type == 0xFF {
            for _ in 0..count {
                let t = TLType::try_from(cursor.read_u8()?)?;
                result.push(self.decode_value(cursor, t, depth + 1)?);
            }
        } else {
            let t = TLType::try_from(elem_type)?;
            for _ in 0..count {
                result.push(self.decode_value(cursor, t, depth + 1)?);
            }
        }

        Ok(Value::Array(result))
    }

    fn decode_object(&self, cursor: &mut Cursor, depth: usize) -> Result<Value> {
        if depth > MAX_DECODE_DEPTH {
            return Err(Error::ParseError("maximum decode nesting depth exceeded".into()));
        }
        let count = cursor.read_u16()?;
        let mut obj = ObjectMap::new();

        for _ in 0..count {
            let key_idx = cursor.read_u32()?;
            let t = TLType::try_from(cursor.read_u8()?)?;
            let key = self.get_string(key_idx as usize)?;
            obj.insert(key, self.decode_value(cursor, t, depth + 1)?);
        }

        Ok(Value::Object(obj))
    }

    fn decode_struct(&self, cursor: &mut Cursor, depth: usize) -> Result<Value> {
        if depth > MAX_DECODE_DEPTH {
            return Err(Error::ParseError("maximum decode nesting depth exceeded".into()));
        }
        let schema_idx = cursor.read_u16()? as usize;
        if schema_idx >= self.schemas.len() {
            return Err(Error::ParseError(format!(
                "struct schema index {} out of bounds ({} schemas available)",
                schema_idx, self.schemas.len()
            )));
        }
        let schema = &self.schemas[schema_idx];
        let bms = (schema.fields.len() + 7) / 8;
        let bitmap_size = 2 * bms;

        let mut bitmap = Vec::with_capacity(bitmap_size.min(cursor.remaining()));
        for _ in 0..bitmap_size {
            bitmap.push(cursor.read_u8()?);
        }
        let lo_bitmap = &bitmap[..bms.min(bitmap.len())];
        let hi_bitmap = if bitmap.len() > bms { &bitmap[bms..] } else { &[] as &[u8] };

        let mut obj = ObjectMap::new();
        for (i, field) in schema.fields.iter().enumerate() {
            let lo = i / 8 < lo_bitmap.len() && (lo_bitmap[i / 8] & (1 << (i % 8))) != 0;
            let hi = i / 8 < hi_bitmap.len() && (hi_bitmap[i / 8] & (1 << (i % 8))) != 0;
            let code = (lo as u8) | ((hi as u8) << 1);
            match code {
                0 => {
                    // Has value — decode inline data
                    let tl_type = if self.union_map.contains_key(&field.field_type.base) {
                        TLType::Tagged
                    } else {
                        field.field_type.to_tl_type()
                    };
                    obj.insert(field.name.clone(), self.decode_value(cursor, tl_type, depth + 1)?);
                }
                1 => {
                    // Explicit null — always preserve
                    obj.insert(field.name.clone(), Value::Null);
                }
                2 => {
                    // Absent — drop for nullable fields
                    if !field.field_type.nullable {
                        obj.insert(field.name.clone(), Value::Null);
                    }
                }
                _ => {} // reserved
            }
        }

        Ok(Value::Object(obj))
    }

    fn decode_map(&self, cursor: &mut Cursor, depth: usize) -> Result<Value> {
        if depth > MAX_DECODE_DEPTH {
            return Err(Error::ParseError("maximum decode nesting depth exceeded".into()));
        }
        let count = cursor.read_u32()?;
        if count as usize > MAX_COLLECTION_SIZE {
            return Err(Error::ParseError(format!(
                "map element count {} exceeds limit of {}", count, MAX_COLLECTION_SIZE
            )));
        }
        let capacity = (count as usize).min(cursor.remaining()).min(MAX_COLLECTION_SIZE);
        let mut pairs = Vec::with_capacity(capacity);

        for _ in 0..count {
            let key_type = TLType::try_from(cursor.read_u8()?)?;
            let key = self.decode_value(cursor, key_type, depth + 1)?;
            // Validate map key type per spec: map keys must be string, int, or uint
            match &key {
                Value::String(_) | Value::Int(_) | Value::UInt(_) => {}
                _ => return Err(Error::ParseError(
                    format!("invalid map key type {:?}: map keys must be string, int, or uint", key.tl_type())
                )),
            }
            let val_type = TLType::try_from(cursor.read_u8()?)?;
            let val = self.decode_value(cursor, val_type, depth + 1)?;
            pairs.push((key, val));
        }

        Ok(Value::Map(pairs))
    }

    fn decode_value(&self, cursor: &mut Cursor, tl_type: TLType, depth: usize) -> Result<Value> {
        if depth > MAX_DECODE_DEPTH {
            return Err(Error::ParseError("maximum decode nesting depth exceeded".into()));
        }
        Ok(match tl_type {
            TLType::Null => Value::Null,
            TLType::Bool => Value::Bool(cursor.read_u8()? != 0),
            TLType::Int8 => Value::Int(cursor.read_i8()? as i64),
            TLType::Int16 => Value::Int(cursor.read_i16()? as i64),
            TLType::Int32 => Value::Int(cursor.read_i32()? as i64),
            TLType::Int64 => Value::Int(cursor.read_i64()?),
            TLType::UInt8 => Value::UInt(cursor.read_u8()? as u64),
            TLType::UInt16 => Value::UInt(cursor.read_u16()? as u64),
            TLType::UInt32 => Value::UInt(cursor.read_u32()? as u64),
            TLType::UInt64 => Value::UInt(cursor.read_u64()?),
            TLType::Float32 => Value::Float(cursor.read_f32()? as f64),
            TLType::Float64 => Value::Float(cursor.read_f64()?),
            TLType::String => {
                let idx = cursor.read_u32()?;
                Value::String(self.get_string(idx as usize)?)
            }
            TLType::Bytes => {
                let len = cursor.read_varint()? as usize;
                Value::Bytes(cursor.read_bytes(len)?)
            }
            TLType::Array => self.decode_array(cursor, depth)?,
            TLType::Object => self.decode_object(cursor, depth)?,
            TLType::Struct => self.decode_struct(cursor, depth)?,
            TLType::Ref => {
                let idx = cursor.read_u32()?;
                Value::Ref(self.get_string(idx as usize)?)
            }
            TLType::Tagged => {
                let tag_idx = cursor.read_u32()?;
                let inner_type = TLType::try_from(cursor.read_u8()?)?;
                let tag = self.get_string(tag_idx as usize)?;
                let inner = self.decode_value(cursor, inner_type, depth + 1)?;
                Value::Tagged(tag, Box::new(inner))
            }
            TLType::Map => self.decode_map(cursor, depth)?,
            TLType::Timestamp => {
                let ts = cursor.read_i64()?;
                let tz = cursor.read_i16()?;
                Value::Timestamp(ts, tz)
            }
            TLType::JsonNumber => {
                let idx = cursor.read_u32()?;
                Value::JsonNumber(self.get_string(idx as usize)?)
            }
            TLType::Tuple => {
                // Tuple is decoded as an array
                self.decode_array(cursor, depth)?
            }
        })
    }
}

// Simple cursor for reading binary data with bounds checking
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn check_bounds(&self, len: usize) -> Result<()> {
        let end = self.pos.checked_add(len)
            .ok_or_else(|| Error::ParseError("cursor position overflow".into()))?;
        if end > self.data.len() {
            return Err(Error::ParseError(format!(
                "read out of bounds: pos={} len={} data_len={}", self.pos, len, self.data.len()
            )));
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8> {
        self.check_bounds(1)?;
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    fn read_u16(&mut self) -> Result<u16> {
        self.check_bounds(2)?;
        let end = self.pos + 2; // safe: check_bounds verified this won't exceed data.len()
        let v = u16::from_le_bytes(self.data[self.pos..end].try_into().map_err(|_|
            Error::ParseError("u16 cursor conversion failed".into())
        )?);
        self.pos = end;
        Ok(v)
    }

    fn read_i16(&mut self) -> Result<i16> {
        Ok(self.read_u16()? as i16)
    }

    fn read_u32(&mut self) -> Result<u32> {
        self.check_bounds(4)?;
        let end = self.pos + 4;
        let v = u32::from_le_bytes(self.data[self.pos..end].try_into().map_err(|_|
            Error::ParseError("u32 cursor conversion failed".into())
        )?);
        self.pos = end;
        Ok(v)
    }

    fn read_i32(&mut self) -> Result<i32> {
        Ok(self.read_u32()? as i32)
    }

    fn read_u64(&mut self) -> Result<u64> {
        self.check_bounds(8)?;
        let end = self.pos + 8;
        let v = u64::from_le_bytes(self.data[self.pos..end].try_into().map_err(|_|
            Error::ParseError("u64 cursor conversion failed".into())
        )?);
        self.pos = end;
        Ok(v)
    }

    fn read_i64(&mut self) -> Result<i64> {
        Ok(self.read_u64()? as i64)
    }

    fn read_f32(&mut self) -> Result<f32> {
        self.check_bounds(4)?;
        let end = self.pos + 4;
        let v = f32::from_le_bytes(self.data[self.pos..end].try_into().map_err(|_|
            Error::ParseError("f32 cursor conversion failed".into())
        )?);
        self.pos = end;
        Ok(v)
    }

    fn read_f64(&mut self) -> Result<f64> {
        self.check_bounds(8)?;
        let end = self.pos + 8;
        let v = f64::from_le_bytes(self.data[self.pos..end].try_into().map_err(|_|
            Error::ParseError("f64 cursor conversion failed".into())
        )?);
        self.pos = end;
        Ok(v)
    }

    fn read_varint(&mut self) -> Result<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;
        for _ in 0..MAX_VARINT_BYTES {
            let b = self.read_u8()?;
            result |= ((b & 0x7F) as u64) << shift;
            if b & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
        }
        Err(Error::ParseError("varint exceeds maximum length".into()))
    }

    fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        self.check_bounds(len)?;
        let end = self.pos.checked_add(len)
            .ok_or_else(|| Error::ParseError("read_bytes offset overflow".into()))?;
        let v = self.data[self.pos..end].to_vec();
        self.pos = end;
        Ok(v)
    }
}

fn decompress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;

    let decoder = ZlibDecoder::new(data);
    let mut limited = decoder.take((MAX_DECOMPRESSED_SIZE as u64) + 1);
    let mut result = Vec::new();
    limited.read_to_end(&mut result)
        .map_err(|_| Error::ParseError("Decompression failed".to_string()))?;
    if result.len() > MAX_DECOMPRESSED_SIZE {
        return Err(Error::ParseError(format!(
            "Decompressed data exceeds maximum size of {} bytes", MAX_DECOMPRESSED_SIZE
        )));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::Writer;

    #[test]
    fn test_open_mmap() {
        // Write a binary file first, then open with mmap
        let dir = std::env::temp_dir();
        let path = dir.join("test_reader_mmap.tlbx");

        let mut w = Writer::new();
        w.add_section("val", &Value::Int(42), None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::open_mmap(&path).unwrap();
        assert_eq!(r.get("val").unwrap().as_int(), Some(42));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_open_regular() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_reader_open.tlbx");

        let mut w = Writer::new();
        w.add_section("greeting", &Value::String("hi".into()), None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::open(&path).unwrap();
        assert_eq!(r.get("greeting").unwrap().as_str(), Some("hi"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_invalid_magic() {
        let result = Reader::from_bytes(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_too_short_data() {
        let result = Reader::from_bytes(vec![0; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_version() {
        let mut data = vec![0u8; 64];
        // Set correct magic bytes "TLFX"
        data[0] = b'T'; data[1] = b'L'; data[2] = b'F'; data[3] = b'X';
        // Set wrong major version (3)
        data[4] = 3; data[5] = 0;
        let result = Reader::from_bytes(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_index_out_of_bounds() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_str_oob.tlbx");

        let mut w = Writer::new();
        w.add_section("x", &Value::Int(1), None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let result = r.get_string(9999);
        assert!(result.is_err());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_keys() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_reader_keys.tlbx");

        let mut w = Writer::new();
        w.add_section("alpha", &Value::Int(1), None).unwrap();
        w.add_section("beta", &Value::Int(2), None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let keys = r.keys();
        assert!(keys.contains(&"alpha"));
        assert!(keys.contains(&"beta"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_missing_key() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_reader_missing.tlbx");

        let mut w = Writer::new();
        w.add_section("exists", &Value::Int(1), None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        assert!(r.get("nonexistent").is_err());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_struct_section_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_struct_section.tlbx");

        let mut schema = Schema::new("Point");
        schema.add_field("x", FieldType::new("int"));
        schema.add_field("y", FieldType::new("int"));

        let mut w = Writer::new();
        w.add_schema(schema.clone());

        let mut obj1 = ObjectMap::new();
        obj1.insert("x".to_string(), Value::Int(10));
        obj1.insert("y".to_string(), Value::Int(20));

        let mut obj2 = ObjectMap::new();
        obj2.insert("x".to_string(), Value::Int(30));
        obj2.insert("y".to_string(), Value::Null);

        let arr = Value::Array(vec![Value::Object(obj1), Value::Object(obj2)]);
        w.add_section("points", &arr, Some(&schema)).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        assert!(!r.schemas.is_empty());

        let points = r.get("points").unwrap();
        let items = points.as_array().unwrap();
        assert_eq!(items.len(), 2);
        let p1 = items[0].as_object().unwrap();
        assert_eq!(p1.get("x").unwrap().as_int(), Some(10));
        let p2 = items[1].as_object().unwrap();
        assert!(p2.get("y").unwrap().is_null());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_heterogeneous_array() {
        // Mixed-type array uses 0xFF element type marker
        let dir = std::env::temp_dir();
        let path = dir.join("test_hetero_arr.tlbx");

        let arr = Value::Array(vec![
            Value::Int(1),
            Value::String("hello".into()),
            Value::Bool(true),
        ]);

        let mut w = Writer::new();
        w.add_section("mixed", &arr, None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let val = r.get("mixed").unwrap();
        let items = val.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_int(), Some(1));
        assert_eq!(items[1].as_str(), Some("hello"));
        assert_eq!(items[2].as_bool(), Some(true));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_empty_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_empty_arr.tlbx");

        let mut w = Writer::new();
        w.add_section("empty", &Value::Array(vec![]), None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let val = r.get("empty").unwrap();
        let items = val.as_array().unwrap();
        assert_eq!(items.len(), 0);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_truncated_section_data() {
        // Craft a file where section offset points past end of file
        let dir = std::env::temp_dir();
        let path = dir.join("test_truncated.tlbx");

        let mut w = Writer::new();
        w.add_section("val", &Value::Int(42), None).unwrap();
        w.write(&path, false).unwrap();

        // Read and truncate the data
        let mut data = std::fs::read(&path).unwrap();
        data.truncate(data.len() - 1); // Remove last byte
        // The reader should either error during parsing or during get()
        // (depends on what the last byte was), but should not panic
        let result = Reader::from_bytes(data);
        if let Ok(r) = result {
            // If it parsed headers OK, get should fail gracefully
            let _ = r.get("val"); // Should not panic
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_cursor_bounds_checking() {
        // Ensure cursor read methods return errors on out-of-bounds
        let data = vec![1u8, 2];
        let mut cursor = Cursor::new(&data);
        assert!(cursor.read_u8().is_ok());
        assert!(cursor.read_u8().is_ok());
        assert!(cursor.read_u8().is_err()); // Past end

        let mut cursor2 = Cursor::new(&data);
        assert!(cursor2.read_u32().is_err()); // Only 2 bytes available

        let empty: Vec<u8> = vec![];
        let mut cursor3 = Cursor::new(&empty);
        assert!(cursor3.read_u8().is_err());
        assert!(cursor3.read_varint().is_err());
    }

    #[test]
    fn test_varint_too_long() {
        // All continuation bytes (0x80) - should error after MAX_VARINT_BYTES
        let data = vec![0x80u8; 20];
        let mut cursor = Cursor::new(&data);
        assert!(cursor.read_varint().is_err());
    }

    // =========================================================================
    // Issue 10: Decompression cache
    // =========================================================================

    #[test]
    fn test_cache_returns_same_value() {
        use crate::Writer;
        let dir = std::env::temp_dir();
        let path = dir.join("test_cache_hit.tlbx");

        let mut w = Writer::new();
        w.add_section("greeting", &crate::Value::String("hello".into()), None).unwrap();
        w.add_section("number", &crate::Value::Int(42), None).unwrap();
        w.write(&path, true).unwrap(); // compressed

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();

        // First call populates cache
        let v1 = r.get("greeting").unwrap();
        // Second call should return cached value
        let v2 = r.get("greeting").unwrap();
        assert_eq!(v1, v2);
        assert_eq!(v1.as_str(), Some("hello"));

        // Verify cache has entry
        assert!(r.cache.borrow().contains_key("greeting"));
        assert!(!r.cache.borrow().contains_key("number")); // not yet accessed

        // Access number
        let num = r.get("number").unwrap();
        assert_eq!(num.as_int(), Some(42));
        assert!(r.cache.borrow().contains_key("number"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_fuzz_crash_crafted_tlbx_2_no_panic() {
        // Regression: fuzz_reader crash-5a6d5f6582c97f5bdc177383430d35c687c640fb
        let data: Vec<u8> = vec![
            0x54, 0x4C, 0x42, 0x58, 0x02, 0x00, 0x0E, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x3A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x58, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        ];
        let result = Reader::from_bytes(data);
        if let Ok(r) = result {
            for key in r.keys() {
                let _ = r.get(key); // Must not panic
            }
        }
    }

    #[test]
    fn test_fuzz_oom_crafted_tlbx_no_panic() {
        // Regression: fuzz_reader oom-1aecaf7b31c65092524b8d5ea7e9b979f06da315
        // Crafted TLBX that triggers large allocations via inflated count fields.
        // Must return Err (or Ok with bounded memory), not OOM.
        let data: Vec<u8> = vec![
            0x54, 0x4C, 0x42, 0x58, 0x02, 0x00, 0x0E, 0x03, 0x00, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x3A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x02, 0x24, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2D, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x42, 0x4C,
            0x54, 0x26, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x23,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x67, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFE, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xFE, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
            0x00, 0x30, 0x30, 0x30, 0x30, 0x30, 0x20, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
            0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00,
            0x00,
        ];
        let result = Reader::from_bytes(data);
        if let Ok(r) = result {
            for key in r.keys() {
                let _ = r.get(key); // Must not panic or OOM
            }
        }
    }

    #[test]
    fn test_fuzz_oom_null_array_no_oom() {
        // Regression: fuzz_reader oom-691da8e2c0990bf219571aef468e5fce6e23cabc
        // 335-byte file with count=973,078,528 and elem_type=Null (0x00).
        // Null elements consume 0 cursor bytes, so the loop never fails on its own.
        // Without MAX_COLLECTION_SIZE check, this allocates ~31 GB → OOM.
        let data: Vec<u8> = vec![
            0x54, 0x4C, 0x42, 0x58, 0x02, 0x00, 0x0E, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x3A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // ... padding (all zeros for entries 1-6) ...
        ];
        // Pad to 335 bytes (same structure as the crash artifact)
        let mut padded = data.clone();
        padded.resize(290, 0x00);
        // Entry 7 at offset 290: key_idx=0, offset=29, size=32, ptype=0x20 (Array)
        padded.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00,             // key_idx = 0
            0x1D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // offset = 29
            0x20, 0x00, 0x00, 0x00,             // size = 32
            0x00, 0x00, 0x00, 0x00,             // uncompressed = 0
            0x00, 0x00,                         // schema_idx = 0
            0x20,                               // ptype = Array
            0x00,                               // flags = 0
            0x00, 0x58, 0x00, 0x00,             // item_count
            0x00, 0x00, 0x00, 0x00,             // padding
        ]);
        padded.resize(335, 0x00);

        let result = Reader::from_bytes(padded);
        if let Ok(r) = result {
            for key in r.keys() {
                let _ = r.get(key); // Must not OOM
            }
        }
    }

    #[test]
    fn test_fuzz_crash_crafted_tlbx_no_panic() {
        // Regression: fuzz_reader crash-b239a207c06b3584ad33adddeac470e0fa792cb9
        // Crafted TLBX with valid magic but manipulated section pointers.
        // Must return Err, not panic.
        let data: Vec<u8> = vec![
            0x54, 0x4C, 0x42, 0x58, 0x02, 0x00, 0x0E, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x3A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x54,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x75, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x00, 0x00,
            0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let result = Reader::from_bytes(data);
        if let Ok(r) = result {
            for key in r.keys() {
                let _ = r.get(key); // Must not panic
            }
        }
    }

    #[test]
    fn test_clear_cache() {
        use crate::Writer;
        let dir = std::env::temp_dir();
        let path = dir.join("test_cache_clear.tlbx");

        let mut w = Writer::new();
        w.add_section("val", &crate::Value::Int(99), None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let _ = r.get("val").unwrap();
        assert_eq!(r.cache.borrow().len(), 1);

        r.clear_cache();
        assert_eq!(r.cache.borrow().len(), 0);

        // Re-access still works
        let v = r.get("val").unwrap();
        assert_eq!(v.as_int(), Some(99));

        std::fs::remove_file(&path).ok();
    }
}
