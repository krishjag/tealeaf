//! Binary format writer for TeaLeaf

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write, Seek, SeekFrom};
use std::path::Path;

use crate::{Result, Value, Schema, FieldType, TLType, MAGIC, VERSION_MAJOR, VERSION_MINOR, HEADER_SIZE};

pub struct Writer {
    strings: Vec<String>,
    string_map: HashMap<String, u32>,
    schemas: Vec<Schema>,
    schema_map: HashMap<String, u16>,
    sections: Vec<Section>,
    /// Indicates the source JSON was a root-level array (for round-trip fidelity)
    is_root_array: bool,
}

struct Section {
    key: String,
    data: Vec<u8>,
    schema_idx: i16,
    tl_type: TLType,
    is_array: bool,
    item_count: u32,
}

impl Writer {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            string_map: HashMap::new(),
            schemas: Vec::new(),
            schema_map: HashMap::new(),
            sections: Vec::new(),
            is_root_array: false,
        }
    }

    /// Set whether the source JSON was a root-level array
    pub fn set_root_array(&mut self, is_root_array: bool) {
        self.is_root_array = is_root_array;
    }

    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.string_map.get(s) { return idx; }
        let idx = self.strings.len() as u32;
        self.strings.push(s.to_string());
        self.string_map.insert(s.to_string(), idx);
        idx
    }

    pub fn add_schema(&mut self, schema: Schema) -> u16 {
        if let Some(&idx) = self.schema_map.get(&schema.name) { return idx; }
        for field in &schema.fields { self.intern(&field.name); }
        self.intern(&schema.name);
        let idx = self.schemas.len() as u16;
        self.schema_map.insert(schema.name.clone(), idx);
        self.schemas.push(schema);
        idx
    }

    pub fn add_section(&mut self, key: &str, value: &Value, schema: Option<&Schema>) {
        self.intern(key);
        let schema_idx = schema.map(|s| self.schema_map.get(&s.name).copied().unwrap_or(0xFFFF) as i16).unwrap_or(-1);
        let (data, tl_type, is_array, item_count) = self.encode_value(value, schema);
        self.sections.push(Section { key: key.to_string(), data, schema_idx, tl_type, is_array, item_count });
    }

    pub fn write<P: AsRef<Path>>(&self, path: P, compress: bool) -> Result<()> {
        let file = File::create(path)?;
        let mut w = BufWriter::new(file);
        w.write_all(&[0u8; HEADER_SIZE])?;

        let str_off = HEADER_SIZE as u64;
        self.write_string_table(&mut w)?;
        let sch_off = str_off + self.string_table_size() as u64;
        self.write_schema_table(&mut w)?;
        let idx_off = sch_off + self.schema_table_size() as u64;
        let index_size = 8 + self.sections.len() * 32;
        w.write_all(&vec![0u8; index_size])?;
        let data_off = idx_off + index_size as u64;

        let mut entries = Vec::new();
        let mut cur_off = data_off;
        for sec in &self.sections {
            let (written, compressed) = if compress && sec.data.len() > 64 {
                let c = compress_data(&sec.data);
                if c.len() < (sec.data.len() as f64 * 0.9) as usize { (c, true) } else { (sec.data.clone(), false) }
            } else { (sec.data.clone(), false) };
            w.write_all(&written)?;
            entries.push((self.string_map[&sec.key], cur_off, written.len() as u32, sec.data.len() as u32, sec.schema_idx, sec.tl_type, compressed, sec.is_array, sec.item_count));
            cur_off += written.len() as u64;
        }

        w.seek(SeekFrom::Start(0))?;
        w.write_all(&MAGIC)?;
        w.write_all(&VERSION_MAJOR.to_le_bytes())?;
        w.write_all(&VERSION_MINOR.to_le_bytes())?;
        // Flags: bit 0 = compressed, bit 1 = root_array
        let mut flags: u32 = 0;
        if compress { flags |= 0x01; }
        if self.is_root_array { flags |= 0x02; }
        w.write_all(&flags.to_le_bytes())?;
        w.write_all(&0u32.to_le_bytes())?;
        w.write_all(&str_off.to_le_bytes())?;
        w.write_all(&sch_off.to_le_bytes())?;
        w.write_all(&idx_off.to_le_bytes())?;
        w.write_all(&data_off.to_le_bytes())?;
        w.write_all(&(self.strings.len() as u32).to_le_bytes())?;
        w.write_all(&(self.schemas.len() as u32).to_le_bytes())?;
        w.write_all(&(self.sections.len() as u32).to_le_bytes())?;
        w.write_all(&0u32.to_le_bytes())?;

        w.seek(SeekFrom::Start(idx_off))?;
        w.write_all(&(index_size as u32).to_le_bytes())?;
        w.write_all(&(entries.len() as u32).to_le_bytes())?;
        for (ki, off, sz, usz, si, pt, comp, arr, cnt) in entries {
            w.write_all(&ki.to_le_bytes())?;
            w.write_all(&off.to_le_bytes())?;
            w.write_all(&sz.to_le_bytes())?;
            w.write_all(&usz.to_le_bytes())?;
            w.write_all(&(if si < 0 { 0xFFFFu16 } else { si as u16 }).to_le_bytes())?;
            w.write_all(&[pt as u8])?;
            w.write_all(&[(if comp { 1 } else { 0 }) | (if arr { 2 } else { 0 })])?;
            w.write_all(&cnt.to_le_bytes())?;
            w.write_all(&[0u8; 4])?;
        }
        w.flush()?;
        Ok(())
    }

    fn string_table_size(&self) -> usize {
        8 + self.strings.len() * 6 + self.strings.iter().map(|s| s.len()).sum::<usize>()
    }

    fn schema_table_size(&self) -> usize {
        if self.schemas.is_empty() { return 8; }
        8 + self.schemas.len() * 4 + self.schemas.iter().map(|s| 8 + s.fields.len() * 8).sum::<usize>()
    }

    fn write_string_table<W: Write>(&self, w: &mut W) -> Result<()> {
        let mut off = 0u32;
        let offsets: Vec<u32> = self.strings.iter().map(|s| { let o = off; off += s.len() as u32; o }).collect();
        w.write_all(&(self.string_table_size() as u32).to_le_bytes())?;
        w.write_all(&(self.strings.len() as u32).to_le_bytes())?;
        for o in &offsets { w.write_all(&o.to_le_bytes())?; }
        for s in &self.strings { w.write_all(&(s.len() as u16).to_le_bytes())?; }
        for s in &self.strings { w.write_all(s.as_bytes())?; }
        Ok(())
    }

    fn write_schema_table<W: Write>(&self, w: &mut W) -> Result<()> {
        if self.schemas.is_empty() {
            w.write_all(&8u32.to_le_bytes())?;
            w.write_all(&0u32.to_le_bytes())?;
            return Ok(());
        }
        let mut data = Vec::new();
        let mut off = 0u32;
        let offsets: Vec<u32> = self.schemas.iter().map(|s| {
            let o = off;
            off += (8 + s.fields.len() * 8) as u32;
            o
        }).collect();
        for schema in &self.schemas {
            data.extend_from_slice(&self.string_map[&schema.name].to_le_bytes());
            data.extend_from_slice(&(schema.fields.len() as u16).to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            for f in &schema.fields {
                data.extend_from_slice(&self.string_map[&f.name].to_le_bytes());
                data.push(f.field_type.to_tl_type() as u8);
                let mut flags: u8 = 0;
                if f.field_type.nullable { flags |= 0x01; }
                if f.field_type.is_array { flags |= 0x02; }
                data.push(flags);
                // Store struct type name string index (0xFFFF = no type)
                if f.field_type.to_tl_type() == TLType::Struct {
                    let type_name_idx = self.string_map.get(&f.field_type.base)
                        .copied()
                        .map(|i| i as u16)
                        .unwrap_or(0xFFFF);
                    data.extend_from_slice(&type_name_idx.to_le_bytes());
                } else {
                    data.extend_from_slice(&0xFFFFu16.to_le_bytes());
                }
            }
        }
        w.write_all(&(self.schema_table_size() as u32).to_le_bytes())?;
        w.write_all(&(self.schemas.len() as u16).to_le_bytes())?;
        w.write_all(&0u16.to_le_bytes())?;
        for o in &offsets { w.write_all(&o.to_le_bytes())?; }
        w.write_all(&data)?;
        Ok(())
    }

    fn encode_value(&mut self, value: &Value, schema: Option<&Schema>) -> (Vec<u8>, TLType, bool, u32) {
        match value {
            Value::Null => (vec![], TLType::Null, false, 0),
            Value::Bool(b) => (vec![if *b { 1 } else { 0 }], TLType::Bool, false, 0),
            Value::Int(i) => encode_int(*i),
            Value::UInt(u) => encode_uint(*u),
            Value::Float(f) => (f.to_le_bytes().to_vec(), TLType::Float64, false, 0),
            Value::String(s) => { let idx = self.intern(s); (idx.to_le_bytes().to_vec(), TLType::String, false, 0) }
            Value::Bytes(b) => { let mut buf = Vec::new(); write_varint(&mut buf, b.len() as u64); buf.extend(b); (buf, TLType::Bytes, false, 0) }
            Value::Array(arr) => self.encode_array(arr, schema),
            Value::Object(obj) => self.encode_object(obj),
            Value::Map(pairs) => self.encode_map(pairs),
            Value::Ref(r) => { let idx = self.intern(r); (idx.to_le_bytes().to_vec(), TLType::Ref, false, 0) }
            Value::Tagged(tag, inner) => {
                let ti = self.intern(tag);
                let (d, t, _, _) = self.encode_value(inner, None);
                let mut buf = ti.to_le_bytes().to_vec();
                buf.push(t as u8);
                buf.extend(d);
                (buf, TLType::Tagged, false, 0)
            }
            Value::Timestamp(ts) => (ts.to_le_bytes().to_vec(), TLType::Timestamp, false, 0),
        }
    }

    fn encode_map(&mut self, pairs: &[(Value, Value)]) -> (Vec<u8>, TLType, bool, u32) {
        let mut buf = (pairs.len() as u32).to_le_bytes().to_vec();
        for (k, v) in pairs {
            let (kd, kt, _, _) = self.encode_value(k, None);
            let (vd, vt, _, _) = self.encode_value(v, None);
            buf.push(kt as u8);
            buf.extend(kd);
            buf.push(vt as u8);
            buf.extend(vd);
        }
        (buf, TLType::Map, false, pairs.len() as u32)
    }

    fn encode_array(&mut self, arr: &[Value], schema: Option<&Schema>) -> (Vec<u8>, TLType, bool, u32) {
        let mut buf = (arr.len() as u32).to_le_bytes().to_vec();
        if arr.is_empty() { return (buf, TLType::Array, true, 0); }
        if schema.is_some() && arr.iter().all(|v| matches!(v, Value::Object(_))) {
            return self.encode_struct_array(arr, schema.unwrap());
        }
        if arr.iter().all(|v| matches!(v, Value::Int(_))) {
            buf.push(TLType::Int32 as u8);
            for v in arr { if let Value::Int(i) = v { buf.extend((*i as i32).to_le_bytes()); } }
            return (buf, TLType::Array, true, arr.len() as u32);
        }
        if arr.iter().all(|v| matches!(v, Value::String(_))) {
            buf.push(TLType::String as u8);
            for v in arr { if let Value::String(s) = v { buf.extend(self.intern(s).to_le_bytes()); } }
            return (buf, TLType::Array, true, arr.len() as u32);
        }
        buf.push(0xFF);
        for v in arr { let (d, t, _, _) = self.encode_value(v, None); buf.push(t as u8); buf.extend(d); }
        (buf, TLType::Array, true, arr.len() as u32)
    }

    fn encode_struct_array(&mut self, arr: &[Value], schema: &Schema) -> (Vec<u8>, TLType, bool, u32) {
        let mut buf = (arr.len() as u32).to_le_bytes().to_vec();
        let si = *self.schema_map.get(&schema.name).unwrap();
        buf.extend(si.to_le_bytes());
        let bms = (schema.fields.len() + 7) / 8;
        buf.extend((bms as u16).to_le_bytes());
        for v in arr {
            if let Value::Object(obj) = v {
                let mut bm: u64 = 0;
                for (i, f) in schema.fields.iter().enumerate() {
                    if obj.get(&f.name).map(|v| v.is_null()).unwrap_or(true) { bm |= 1 << i; }
                }
                for bi in 0..bms { buf.push(((bm >> (bi * 8)) & 0xFF) as u8); }
                for (i, f) in schema.fields.iter().enumerate() {
                    if bm & (1 << i) == 0 {
                        if let Some(v) = obj.get(&f.name) {
                            let nested_schema = self.schemas.iter().find(|s| s.name == f.field_type.base).cloned();
                            let data = self.encode_typed_value(v, &f.field_type, nested_schema.as_ref());
                            buf.extend(data);
                        }
                    }
                }
            }
        }
        (buf, TLType::Struct, true, arr.len() as u32)
    }

    /// Encode a value according to a specific field type (schema-aware encoding)
    fn encode_typed_value(&mut self, value: &Value, field_type: &FieldType, nested_schema: Option<&Schema>) -> Vec<u8> {
        use crate::TLType;

        // Handle arrays
        if field_type.is_array {
            if let Value::Array(arr) = value {
                let mut buf = (arr.len() as u32).to_le_bytes().to_vec();
                if arr.is_empty() { return buf; }

                // Determine element type
                let elem_type = FieldType::new(&field_type.base);
                let elem_tl_type = elem_type.to_tl_type();

                // For struct arrays, look up the correct element schema
                let elem_schema = self.schemas.iter()
                    .find(|s| s.name == field_type.base)
                    .cloned();

                // Write element type byte (standard array format)
                buf.push(elem_tl_type as u8);

                // Encode each element with proper type
                for v in arr {
                    buf.extend(self.encode_typed_value(v, &elem_type, elem_schema.as_ref()));
                }
                return buf;
            }
            return vec![];
        }

        let tl_type = field_type.to_tl_type();
        match tl_type {
            TLType::Null => vec![],
            TLType::Bool => {
                if let Value::Bool(b) = value { vec![if *b { 1 } else { 0 }] }
                else { vec![0] }
            }
            TLType::Int8 => {
                let i = match value { Value::Int(i) => *i, Value::UInt(u) => *u as i64, _ => 0 };
                (i as i8).to_le_bytes().to_vec()
            }
            TLType::Int16 => {
                let i = match value { Value::Int(i) => *i, Value::UInt(u) => *u as i64, _ => 0 };
                (i as i16).to_le_bytes().to_vec()
            }
            TLType::Int32 => {
                let i = match value { Value::Int(i) => *i, Value::UInt(u) => *u as i64, _ => 0 };
                (i as i32).to_le_bytes().to_vec()
            }
            TLType::Int64 => {
                let i = match value { Value::Int(i) => *i, Value::UInt(u) => *u as i64, _ => 0 };
                i.to_le_bytes().to_vec()
            }
            TLType::UInt8 => {
                let u = match value { Value::UInt(u) => *u, Value::Int(i) => *i as u64, _ => 0 };
                (u as u8).to_le_bytes().to_vec()
            }
            TLType::UInt16 => {
                let u = match value { Value::UInt(u) => *u, Value::Int(i) => *i as u64, _ => 0 };
                (u as u16).to_le_bytes().to_vec()
            }
            TLType::UInt32 => {
                let u = match value { Value::UInt(u) => *u, Value::Int(i) => *i as u64, _ => 0 };
                (u as u32).to_le_bytes().to_vec()
            }
            TLType::UInt64 => {
                let u = match value { Value::UInt(u) => *u, Value::Int(i) => *i as u64, _ => 0 };
                u.to_le_bytes().to_vec()
            }
            TLType::Float32 => {
                let f = match value { Value::Float(f) => *f, Value::Int(i) => *i as f64, _ => 0.0 };
                (f as f32).to_le_bytes().to_vec()
            }
            TLType::Float64 => {
                let f = match value { Value::Float(f) => *f, Value::Int(i) => *i as f64, _ => 0.0 };
                f.to_le_bytes().to_vec()
            }
            TLType::String => {
                if let Value::String(s) = value { self.intern(s).to_le_bytes().to_vec() }
                else { self.intern("").to_le_bytes().to_vec() }
            }
            TLType::Bytes => {
                if let Value::Bytes(b) = value {
                    let mut buf = Vec::new();
                    write_varint(&mut buf, b.len() as u64);
                    buf.extend(b);
                    buf
                } else { vec![0] }
            }
            TLType::Timestamp => {
                if let Value::Timestamp(ts) = value { ts.to_le_bytes().to_vec() }
                else { 0i64.to_le_bytes().to_vec() }
            }
            TLType::Struct => {
                // Nested struct - encode recursively
                if let (Value::Object(obj), Some(schema)) = (value, nested_schema) {
                    let mut buf = Vec::new();

                    // Write schema index
                    let schema_idx = *self.schema_map.get(&schema.name).unwrap_or(&0);
                    buf.extend(schema_idx.to_le_bytes());

                    let bms = (schema.fields.len() + 7) / 8;

                    // Bitmap
                    let mut bm: u64 = 0;
                    for (i, f) in schema.fields.iter().enumerate() {
                        if obj.get(&f.name).map(|v| v.is_null()).unwrap_or(true) { bm |= 1 << i; }
                    }
                    for bi in 0..bms { buf.push(((bm >> (bi * 8)) & 0xFF) as u8); }

                    // Fields
                    for (i, f) in schema.fields.iter().enumerate() {
                        if bm & (1 << i) == 0 {
                            if let Some(v) = obj.get(&f.name) {
                                let nested = self.schemas.iter().find(|s| s.name == f.field_type.base).cloned();
                                buf.extend(self.encode_typed_value(v, &f.field_type, nested.as_ref()));
                            }
                        }
                    }
                    buf
                } else {
                    #[cfg(debug_assertions)]
                    if nested_schema.is_none() {
                        eprintln!("tealeaf: warning: missing schema for struct-typed field, data will be dropped");
                    }
                    vec![]
                }
            }
            _ => {
                // Fallback to generic encoding
                let (d, _, _, _) = self.encode_value(value, None);
                d
            }
        }
    }

    fn encode_object(&mut self, obj: &HashMap<String, Value>) -> (Vec<u8>, TLType, bool, u32) {
        let mut buf = (obj.len() as u16).to_le_bytes().to_vec();
        for (k, v) in obj {
            buf.extend(self.intern(k).to_le_bytes());
            let (d, t, _, _) = self.encode_value(v, None);
            buf.push(t as u8);
            buf.extend(d);
        }
        (buf, TLType::Object, false, 0)
    }
}

impl Default for Writer { fn default() -> Self { Self::new() } }

fn encode_int(i: i64) -> (Vec<u8>, TLType, bool, u32) {
    if i >= i8::MIN as i64 && i <= i8::MAX as i64 { ((i as i8).to_le_bytes().to_vec(), TLType::Int8, false, 0) }
    else if i >= i16::MIN as i64 && i <= i16::MAX as i64 { ((i as i16).to_le_bytes().to_vec(), TLType::Int16, false, 0) }
    else if i >= i32::MIN as i64 && i <= i32::MAX as i64 { ((i as i32).to_le_bytes().to_vec(), TLType::Int32, false, 0) }
    else { (i.to_le_bytes().to_vec(), TLType::Int64, false, 0) }
}

fn encode_uint(u: u64) -> (Vec<u8>, TLType, bool, u32) {
    if u <= u8::MAX as u64 { ((u as u8).to_le_bytes().to_vec(), TLType::UInt8, false, 0) }
    else if u <= u16::MAX as u64 { ((u as u16).to_le_bytes().to_vec(), TLType::UInt16, false, 0) }
    else if u <= u32::MAX as u64 { ((u as u32).to_le_bytes().to_vec(), TLType::UInt32, false, 0) }
    else { (u.to_le_bytes().to_vec(), TLType::UInt64, false, 0) }
}

fn write_varint(buf: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 { buf.push(((v & 0x7F) | 0x80) as u8); v >>= 7; }
    buf.push(v as u8);
}

fn compress_data(data: &[u8]) -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::Reader;

    #[test]
    fn test_writer_default() {
        let w = Writer::default();
        assert_eq!(w.strings.len(), 0);
        assert_eq!(w.schemas.len(), 0);
    }

    #[test]
    fn test_encode_uint_ranges() {
        // encode_uint for small values (u8 range)
        let (data, tl_type, _, _) = encode_uint(42);
        assert_eq!(tl_type, TLType::UInt8);
        assert_eq!(data, vec![42u8]);

        // encode_uint for u16 range
        let (data, tl_type, _, _) = encode_uint(300);
        assert_eq!(tl_type, TLType::UInt16);
        assert_eq!(data, 300u16.to_le_bytes().to_vec());

        // encode_uint for u32 range
        let (data, tl_type, _, _) = encode_uint(100_000);
        assert_eq!(tl_type, TLType::UInt32);
        assert_eq!(data, 100_000u32.to_le_bytes().to_vec());

        // encode_uint for u64 range
        let (data, tl_type, _, _) = encode_uint(5_000_000_000);
        assert_eq!(tl_type, TLType::UInt64);
        assert_eq!(data, 5_000_000_000u64.to_le_bytes().to_vec());
    }

    #[test]
    fn test_uint_value_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_uint_roundtrip.tlbx");

        let mut w = Writer::new();
        w.add_section("small", &Value::UInt(42), None);
        w.add_section("medium", &Value::UInt(300), None);
        w.add_section("large", &Value::UInt(100_000), None);
        w.add_section("huge", &Value::UInt(5_000_000_000), None);
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(r.get("small").unwrap().as_uint(), Some(42));
        assert_eq!(r.get("medium").unwrap().as_uint(), Some(300));
        assert_eq!(r.get("large").unwrap().as_uint(), Some(100_000));
        assert_eq!(r.get("huge").unwrap().as_uint(), Some(5_000_000_000));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_typed_schema_roundtrip() {
        // Build a schema with various typed fields to exercise encode_typed_value
        let dir = std::env::temp_dir();
        let path = dir.join("test_typed_schema.tlbx");

        let mut schema = Schema::new("TypedRecord");
        schema.add_field("flag", FieldType::new("bool"));
        schema.add_field("small_int", FieldType::new("int8"));
        schema.add_field("med_int", FieldType::new("int16"));
        schema.add_field("int32_val", FieldType::new("int"));
        schema.add_field("int64_val", FieldType::new("int64"));
        schema.add_field("small_uint", FieldType::new("uint8"));
        schema.add_field("med_uint", FieldType::new("uint16"));
        schema.add_field("uint32_val", FieldType::new("uint"));
        schema.add_field("uint64_val", FieldType::new("uint64"));
        schema.add_field("f32_val", FieldType::new("float32"));
        schema.add_field("f64_val", FieldType::new("float"));
        schema.add_field("name", FieldType::new("string"));
        schema.add_field("data", FieldType::new("bytes"));

        let mut w = Writer::new();
        w.add_schema(schema.clone());

        // Create a record with all typed fields
        let mut obj = HashMap::new();
        obj.insert("flag".to_string(), Value::Bool(true));
        obj.insert("small_int".to_string(), Value::Int(42));
        obj.insert("med_int".to_string(), Value::Int(1000));
        obj.insert("int32_val".to_string(), Value::Int(50000));
        obj.insert("int64_val".to_string(), Value::Int(1_000_000_000_000));
        obj.insert("small_uint".to_string(), Value::UInt(200));
        obj.insert("med_uint".to_string(), Value::UInt(40000));
        obj.insert("uint32_val".to_string(), Value::UInt(3_000_000));
        obj.insert("uint64_val".to_string(), Value::UInt(9_000_000_000));
        obj.insert("f32_val".to_string(), Value::Float(3.14));
        obj.insert("f64_val".to_string(), Value::Float(2.718281828));
        obj.insert("name".to_string(), Value::String("test".into()));
        obj.insert("data".to_string(), Value::Bytes(vec![0xDE, 0xAD]));

        let arr = Value::Array(vec![Value::Object(obj)]);
        w.add_section("records", &arr, Some(&schema));
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let records = r.get("records").unwrap();
        let items = records.as_array().unwrap();
        assert_eq!(items.len(), 1);

        let rec = items[0].as_object().unwrap();
        assert_eq!(rec.get("flag").unwrap().as_bool(), Some(true));
        assert_eq!(rec.get("name").unwrap().as_str(), Some("test"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_typed_schema_array_field() {
        // Schema with an array field to exercise typed array encoding
        let dir = std::env::temp_dir();
        let path = dir.join("test_typed_array_field.tlbx");

        let mut schema = Schema::new("WithArray");
        schema.add_field("name", FieldType::new("string"));
        schema.add_field("scores", FieldType::new("int").array());

        let mut w = Writer::new();
        w.add_schema(schema.clone());

        let mut obj = HashMap::new();
        obj.insert("name".to_string(), Value::String("Alice".into()));
        obj.insert("scores".to_string(), Value::Array(vec![Value::Int(90), Value::Int(85)]));

        let arr = Value::Array(vec![Value::Object(obj)]);
        w.add_section("users", &arr, Some(&schema));
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let users = r.get("users").unwrap();
        let items = users.as_array().unwrap();
        assert_eq!(items.len(), 1);
        let rec = items[0].as_object().unwrap();
        assert_eq!(rec.get("name").unwrap().as_str(), Some("Alice"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_object_encoding_roundtrip() {
        // Direct object (non-struct-array) encoding
        let dir = std::env::temp_dir();
        let path = dir.join("test_object_enc.tlbx");

        let mut obj = HashMap::new();
        obj.insert("x".to_string(), Value::Int(10));
        obj.insert("y".to_string(), Value::String("hello".into()));

        let mut w = Writer::new();
        w.add_section("data", &Value::Object(obj), None);
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let val = r.get("data").unwrap();
        let o = val.as_object().unwrap();
        assert_eq!(o.get("x").unwrap().as_int(), Some(10));
        assert_eq!(o.get("y").unwrap().as_str(), Some("hello"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_map_roundtrip_binary() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_map_binary.tlbx");

        let pairs = vec![
            (Value::String("key1".into()), Value::Int(100)),
            (Value::String("key2".into()), Value::Bool(true)),
        ];

        let mut w = Writer::new();
        w.add_section("mapping", &Value::Map(pairs), None);
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let val = r.get("mapping").unwrap();
        if let Value::Map(pairs) = val {
            assert_eq!(pairs.len(), 2);
        } else {
            panic!("Expected Map value");
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_ref_and_tagged_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_ref_tagged.tlbx");

        let mut w = Writer::new();
        w.add_section("myref", &Value::Ref("some_ref".into()), None);
        w.add_section("mytag", &Value::Tagged("label".into(), Box::new(Value::Int(42))), None);
        w.add_section("myts", &Value::Timestamp(1700000000000), None);
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();

        if let Value::Ref(s) = r.get("myref").unwrap() {
            assert_eq!(s, "some_ref");
        } else { panic!("Expected Ref"); }

        if let Value::Tagged(tag, inner) = r.get("mytag").unwrap() {
            assert_eq!(tag, "label");
            assert_eq!(inner.as_int(), Some(42));
        } else { panic!("Expected Tagged"); }

        if let Value::Timestamp(ts) = r.get("myts").unwrap() {
            assert_eq!(ts, 1700000000000);
        } else { panic!("Expected Timestamp"); }

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_compressed_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_compressed.tlbx");

        // Create large enough data to trigger compression
        let mut arr = Vec::new();
        for i in 0..100 {
            arr.push(Value::Int(i));
        }

        let mut w = Writer::new();
        w.add_section("numbers", &Value::Array(arr), None);
        w.write(&path, true).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let val = r.get("numbers").unwrap();
        let items = val.as_array().unwrap();
        assert_eq!(items.len(), 100);
        assert_eq!(items[0].as_int(), Some(0));
        assert_eq!(items[99].as_int(), Some(99));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_root_array_flag() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_root_array_flag.tlbx");

        let mut w = Writer::new();
        w.set_root_array(true);
        w.add_section("root", &Value::Array(vec![Value::Int(1)]), None);
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        assert!(r.is_root_array());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_bytes_value_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_bytes_val.tlbx");

        let mut w = Writer::new();
        w.add_section("blob", &Value::Bytes(vec![1, 2, 3, 4, 5]), None);
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let val = r.get("blob").unwrap();
        assert_eq!(val.as_bytes(), Some(&[1u8, 2, 3, 4, 5][..]));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_nested_struct_schema_roundtrip() {
        // Test encode_typed_value with TLType::Struct (nested struct field)
        let dir = std::env::temp_dir();
        let path = dir.join("test_nested_struct.tlbx");

        let mut inner_schema = Schema::new("Address");
        inner_schema.add_field("city", FieldType::new("string"));
        inner_schema.add_field("zip", FieldType::new("string"));

        let mut outer_schema = Schema::new("Person");
        outer_schema.add_field("name", FieldType::new("string"));
        outer_schema.add_field("home", FieldType::new("Address"));

        let mut w = Writer::new();
        w.add_schema(inner_schema.clone());
        w.add_schema(outer_schema.clone());

        let mut addr = HashMap::new();
        addr.insert("city".to_string(), Value::String("Seattle".into()));
        addr.insert("zip".to_string(), Value::String("98101".into()));

        let mut person = HashMap::new();
        person.insert("name".to_string(), Value::String("Alice".into()));
        person.insert("home".to_string(), Value::Object(addr));

        let arr = Value::Array(vec![Value::Object(person)]);
        w.add_section("people", &arr, Some(&outer_schema));
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let people = r.get("people").unwrap();
        let items = people.as_array().unwrap();
        assert_eq!(items.len(), 1);
        let p = items[0].as_object().unwrap();
        assert_eq!(p.get("name").unwrap().as_str(), Some("Alice"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_timestamp_typed_field() {
        // Struct array with a timestamp field
        let dir = std::env::temp_dir();
        let path = dir.join("test_ts_typed.tlbx");

        let mut schema = Schema::new("Event");
        schema.add_field("name", FieldType::new("string"));
        schema.add_field("ts", FieldType::new("timestamp"));

        let mut w = Writer::new();
        w.add_schema(schema.clone());

        let mut obj = HashMap::new();
        obj.insert("name".to_string(), Value::String("deploy".into()));
        obj.insert("ts".to_string(), Value::Timestamp(1700000000000));

        let arr = Value::Array(vec![Value::Object(obj)]);
        w.add_section("events", &arr, Some(&schema));
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let events = r.get("events").unwrap();
        let items = events.as_array().unwrap();
        assert_eq!(items.len(), 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_bytes_typed_field() {
        // Struct array with a bytes field
        let dir = std::env::temp_dir();
        let path = dir.join("test_bytes_typed.tlbx");

        let mut schema = Schema::new("Blob");
        schema.add_field("name", FieldType::new("string"));
        schema.add_field("data", FieldType::new("bytes"));

        let mut w = Writer::new();
        w.add_schema(schema.clone());

        let mut obj = HashMap::new();
        obj.insert("name".to_string(), Value::String("img".into()));
        obj.insert("data".to_string(), Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));

        let arr = Value::Array(vec![Value::Object(obj)]);
        w.add_section("blobs", &arr, Some(&schema));
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let blobs = r.get("blobs").unwrap();
        let items = blobs.as_array().unwrap();
        assert_eq!(items.len(), 1);
        std::fs::remove_file(&path).ok();
    }
}
