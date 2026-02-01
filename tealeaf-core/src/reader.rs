//! Binary format reader for TeaLeaf
//!
//! Supports two modes:
//! - `open()` - Reads file into memory (Vec<u8>)
//! - `open_mmap()` - Memory-maps file for zero-copy access

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use memmap2::Mmap;

use crate::{Error, Result, Value, Schema, Field, FieldType, TLType, MAGIC, HEADER_SIZE};

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
    string_lengths: Vec<u16>,
    string_data_offset: usize,
    pub schemas: Vec<Schema>,
    schema_map: HashMap<String, usize>,
    sections: HashMap<String, SectionInfo>,
    /// Indicates the source JSON was a root-level array (for round-trip fidelity)
    is_root_array: bool,
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
        let major = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        let minor = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        if major != 2 {
            return Err(Error::InvalidVersion { major, minor });
        }

        // Read flags: bit 0 = compressed (handled per-section), bit 1 = root_array
        let flags = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let is_root_array = (flags & 0x02) != 0;

        let str_off = u64::from_le_bytes(bytes[16..24].try_into().unwrap()) as usize;
        let sch_off = u64::from_le_bytes(bytes[24..32].try_into().unwrap()) as usize;
        let idx_off = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;
        let str_cnt = u32::from_le_bytes(bytes[48..52].try_into().unwrap()) as usize;
        let sch_cnt = u32::from_le_bytes(bytes[52..56].try_into().unwrap()) as usize;
        let sec_cnt = u32::from_le_bytes(bytes[56..60].try_into().unwrap()) as usize;

        // Parse string table
        let mut off = str_off + 8;
        let string_offsets: Vec<u32> = (0..str_cnt)
            .map(|i| u32::from_le_bytes(bytes[off + i * 4..off + i * 4 + 4].try_into().unwrap()))
            .collect();
        off += str_cnt * 4;
        let string_lengths: Vec<u16> = (0..str_cnt)
            .map(|i| u16::from_le_bytes(bytes[off + i * 2..off + i * 2 + 2].try_into().unwrap()))
            .collect();
        let string_data_offset = off + str_cnt * 2;

        let mut reader = Self {
            data,
            string_offsets,
            string_lengths,
            string_data_offset,
            schemas: Vec::new(),
            schema_map: HashMap::new(),
            sections: HashMap::new(),
            is_root_array,
        };

        reader.parse_schemas(sch_off, sch_cnt)?;
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
        let start = self.string_data_offset + self.string_offsets[idx] as usize;
        let len = self.string_lengths[idx] as usize;
        String::from_utf8(self.data()[start..start + len].to_vec())
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
        let section = self.sections.get(key)
            .ok_or_else(|| Error::MissingField(key.to_string()))?;

        let start = section.offset as usize;
        let end = start + section.size as usize;

        let data = if section.compressed {
            decompress_data(&self.data()[start..end])?
        } else {
            self.data()[start..end].to_vec()
        };

        let mut cursor = Cursor::new(&data);

        if section.is_array && section.schema_idx >= 0 {
            return self.decode_struct_array(&mut cursor, section.schema_idx as usize);
        }

        match section.tl_type {
            TLType::Array => self.decode_array(&mut cursor),
            TLType::Object => self.decode_object(&mut cursor),
            TLType::Struct => self.decode_struct(&mut cursor),
            TLType::Map => self.decode_map(&mut cursor),
            _ => self.decode_value(&mut cursor, section.tl_type),
        }
    }

    fn parse_schemas(&mut self, off: usize, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }

        let data = self.data.as_ref();
        let o = off + 8;
        let offsets: Vec<u32> = (0..count)
            .map(|i| u32::from_le_bytes(data[o + i * 4..o + i * 4 + 4].try_into().unwrap()))
            .collect();
        let start = o + count * 4;

        for i in 0..count {
            let so = start + offsets[i] as usize;
            let name_idx = u32::from_le_bytes(data[so..so + 4].try_into().unwrap());
            let field_count = u16::from_le_bytes(data[so + 4..so + 6].try_into().unwrap());

            let name = self.get_string(name_idx as usize)?;
            let mut schema = Schema::new(&name);

            let mut fo = so + 8;
            for _ in 0..field_count {
                let fname_idx = u32::from_le_bytes(data[fo..fo + 4].try_into().unwrap());
                let ftype = data[fo + 4];
                let fflags = data[fo + 5];
                let fextra = u16::from_le_bytes(data[fo + 6..fo + 8].try_into().unwrap());

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

    fn parse_index(&mut self, off: usize, count: usize) -> Result<()> {
        let data = self.data.as_ref();
        let mut o = off + 8;

        for _ in 0..count {
            let key_idx = u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
            let offset = u64::from_le_bytes(data[o + 4..o + 12].try_into().unwrap());
            let size = u32::from_le_bytes(data[o + 12..o + 16].try_into().unwrap());
            let uncompressed = u32::from_le_bytes(data[o + 16..o + 20].try_into().unwrap());
            let schema_idx = u16::from_le_bytes(data[o + 20..o + 22].try_into().unwrap());
            let ptype = data[o + 22];
            let flags = data[o + 23];
            let item_count = u32::from_le_bytes(data[o + 24..o + 28].try_into().unwrap());

            let key = self.get_string(key_idx as usize)?;
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

    fn decode_struct_array(&self, cursor: &mut Cursor, schema_idx: usize) -> Result<Value> {
        let count = cursor.read_u32();
        let _si = cursor.read_u16();
        let bitmap_size = cursor.read_u16() as usize;

        if schema_idx >= self.schemas.len() {
            return Err(Error::ParseError(format!(
                "struct array schema index {} out of bounds ({} schemas available)",
                schema_idx, self.schemas.len()
            )));
        }
        let schema = &self.schemas[schema_idx];
        let mut result = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let mut bitmap: u64 = 0;
            for byte_idx in 0..bitmap_size {
                bitmap |= (cursor.read_u8() as u64) << (byte_idx * 8);
            }

            let mut obj = HashMap::new();
            for (i, field) in schema.fields.iter().enumerate() {
                if bitmap & (1 << i) != 0 {
                    obj.insert(field.name.clone(), Value::Null);
                } else {
                    let tl_type = field.field_type.to_tl_type();
                    obj.insert(field.name.clone(), self.decode_value(cursor, tl_type)?);
                }
            }
            result.push(Value::Object(obj));
        }

        Ok(Value::Array(result))
    }

    fn decode_array(&self, cursor: &mut Cursor) -> Result<Value> {
        let count = cursor.read_u32();
        if count == 0 {
            return Ok(Value::Array(Vec::new()));
        }

        let elem_type = cursor.read_u8();
        let mut result = Vec::with_capacity(count as usize);

        if elem_type == 0xFF {
            for _ in 0..count {
                let t = TLType::try_from(cursor.read_u8())?;
                result.push(self.decode_value(cursor, t)?);
            }
        } else {
            let t = TLType::try_from(elem_type)?;
            for _ in 0..count {
                result.push(self.decode_value(cursor, t)?);
            }
        }

        Ok(Value::Array(result))
    }

    fn decode_object(&self, cursor: &mut Cursor) -> Result<Value> {
        let count = cursor.read_u16();
        let mut obj = HashMap::new();

        for _ in 0..count {
            let key_idx = cursor.read_u32();
            let t = TLType::try_from(cursor.read_u8())?;
            let key = self.get_string(key_idx as usize)?;
            obj.insert(key, self.decode_value(cursor, t)?);
        }

        Ok(Value::Object(obj))
    }

    fn decode_struct(&self, cursor: &mut Cursor) -> Result<Value> {
        let schema_idx = cursor.read_u16() as usize;
        if schema_idx >= self.schemas.len() {
            return Err(Error::ParseError(format!(
                "struct schema index {} out of bounds ({} schemas available)",
                schema_idx, self.schemas.len()
            )));
        }
        let schema = &self.schemas[schema_idx];
        let bitmap_size = (schema.fields.len() + 7) / 8;

        let mut bitmap: u64 = 0;
        for byte_idx in 0..bitmap_size {
            bitmap |= (cursor.read_u8() as u64) << (byte_idx * 8);
        }

        let mut obj = HashMap::new();
        for (i, field) in schema.fields.iter().enumerate() {
            if bitmap & (1 << i) != 0 {
                obj.insert(field.name.clone(), Value::Null);
            } else {
                let tl_type = field.field_type.to_tl_type();
                obj.insert(field.name.clone(), self.decode_value(cursor, tl_type)?);
            }
        }

        Ok(Value::Object(obj))
    }

    fn decode_map(&self, cursor: &mut Cursor) -> Result<Value> {
        let count = cursor.read_u32();
        let mut pairs = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let key_type = TLType::try_from(cursor.read_u8())?;
            let key = self.decode_value(cursor, key_type)?;
            let val_type = TLType::try_from(cursor.read_u8())?;
            let val = self.decode_value(cursor, val_type)?;
            pairs.push((key, val));
        }

        Ok(Value::Map(pairs))
    }

    fn decode_value(&self, cursor: &mut Cursor, tl_type: TLType) -> Result<Value> {
        Ok(match tl_type {
            TLType::Null => Value::Null,
            TLType::Bool => Value::Bool(cursor.read_u8() != 0),
            TLType::Int8 => Value::Int(cursor.read_i8() as i64),
            TLType::Int16 => Value::Int(cursor.read_i16() as i64),
            TLType::Int32 => Value::Int(cursor.read_i32() as i64),
            TLType::Int64 => Value::Int(cursor.read_i64()),
            TLType::UInt8 => Value::UInt(cursor.read_u8() as u64),
            TLType::UInt16 => Value::UInt(cursor.read_u16() as u64),
            TLType::UInt32 => Value::UInt(cursor.read_u32() as u64),
            TLType::UInt64 => Value::UInt(cursor.read_u64()),
            TLType::Float32 => Value::Float(cursor.read_f32() as f64),
            TLType::Float64 => Value::Float(cursor.read_f64()),
            TLType::String => {
                let idx = cursor.read_u32();
                Value::String(self.get_string(idx as usize)?)
            }
            TLType::Bytes => {
                let len = cursor.read_varint() as usize;
                Value::Bytes(cursor.read_bytes(len))
            }
            TLType::Array => self.decode_array(cursor)?,
            TLType::Object => self.decode_object(cursor)?,
            TLType::Struct => self.decode_struct(cursor)?,
            TLType::Ref => {
                let idx = cursor.read_u32();
                Value::Ref(self.get_string(idx as usize)?)
            }
            TLType::Tagged => {
                let tag_idx = cursor.read_u32();
                let inner_type = TLType::try_from(cursor.read_u8())?;
                let tag = self.get_string(tag_idx as usize)?;
                let inner = self.decode_value(cursor, inner_type)?;
                Value::Tagged(tag, Box::new(inner))
            }
            TLType::Map => self.decode_map(cursor)?,
            TLType::Timestamp => Value::Timestamp(cursor.read_i64()),
            TLType::Tuple => {
                // Tuple is decoded as an array
                self.decode_array(cursor)?
            }
        })
    }
}

// Simple cursor for reading binary data
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn read_u8(&mut self) -> u8 {
        let v = self.data[self.pos];
        self.pos += 1;
        v
    }

    fn read_i8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    fn read_u16(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.data[self.pos..self.pos + 2].try_into().unwrap());
        self.pos += 2;
        v
    }

    fn read_i16(&mut self) -> i16 {
        self.read_u16() as i16
    }

    fn read_u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        v
    }

    fn read_i32(&mut self) -> i32 {
        self.read_u32() as i32
    }

    fn read_u64(&mut self) -> u64 {
        let v = u64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        v
    }

    fn read_i64(&mut self) -> i64 {
        self.read_u64() as i64
    }

    fn read_f32(&mut self) -> f32 {
        let v = f32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        v
    }

    fn read_f64(&mut self) -> f64 {
        let v = f64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        v
    }

    fn read_varint(&mut self) -> u64 {
        let mut result: u64 = 0;
        let mut shift = 0;
        loop {
            let b = self.read_u8();
            result |= ((b & 0x7F) as u64) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        result
    }

    fn read_bytes(&mut self, len: usize) -> Vec<u8> {
        let v = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        v
    }
}

fn decompress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)
        .map_err(|_| Error::ParseError("Decompression failed".to_string()))?;
    Ok(result)
}
