//! Binary format writer for Pax

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write, Seek, SeekFrom};
use std::path::Path;

use crate::{Result, Value, Schema, FieldType, PaxType, MAGIC, VERSION_MAJOR, VERSION_MINOR, HEADER_SIZE};

pub struct Writer {
    strings: Vec<String>,
    string_map: HashMap<String, u32>,
    schemas: Vec<Schema>,
    schema_map: HashMap<String, u16>,
    sections: Vec<Section>,
}

struct Section {
    key: String,
    data: Vec<u8>,
    schema_idx: i16,
    pax_type: PaxType,
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
        }
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
        let (data, pax_type, is_array, item_count) = self.encode_value(value, schema);
        self.sections.push(Section { key: key.to_string(), data, schema_idx, pax_type, is_array, item_count });
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
            entries.push((self.string_map[&sec.key], cur_off, written.len() as u32, sec.data.len() as u32, sec.schema_idx, sec.pax_type, compressed, sec.is_array, sec.item_count));
            cur_off += written.len() as u64;
        }

        w.seek(SeekFrom::Start(0))?;
        w.write_all(&MAGIC)?;
        w.write_all(&VERSION_MAJOR.to_le_bytes())?;
        w.write_all(&VERSION_MINOR.to_le_bytes())?;
        w.write_all(&(if compress { 0x01u32 } else { 0u32 }).to_le_bytes())?;
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
                data.push(f.field_type.to_pax_type() as u8);
                let mut flags: u8 = 0;
                if f.field_type.nullable { flags |= 0x01; }
                if f.field_type.is_array { flags |= 0x02; }
                data.push(flags);
                // Store struct type name string index (0xFFFF = no type)
                if f.field_type.to_pax_type() == PaxType::Struct {
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

    fn encode_value(&mut self, value: &Value, schema: Option<&Schema>) -> (Vec<u8>, PaxType, bool, u32) {
        match value {
            Value::Null => (vec![], PaxType::Null, false, 0),
            Value::Bool(b) => (vec![if *b { 1 } else { 0 }], PaxType::Bool, false, 0),
            Value::Int(i) => encode_int(*i),
            Value::UInt(u) => encode_uint(*u),
            Value::Float(f) => (f.to_le_bytes().to_vec(), PaxType::Float64, false, 0),
            Value::String(s) => { let idx = self.intern(s); (idx.to_le_bytes().to_vec(), PaxType::String, false, 0) }
            Value::Bytes(b) => { let mut buf = Vec::new(); write_varint(&mut buf, b.len() as u64); buf.extend(b); (buf, PaxType::Bytes, false, 0) }
            Value::Array(arr) => self.encode_array(arr, schema),
            Value::Object(obj) => self.encode_object(obj),
            Value::Map(pairs) => self.encode_map(pairs),
            Value::Ref(r) => { let idx = self.intern(r); (idx.to_le_bytes().to_vec(), PaxType::Ref, false, 0) }
            Value::Tagged(tag, inner) => {
                let ti = self.intern(tag);
                let (d, t, _, _) = self.encode_value(inner, None);
                let mut buf = ti.to_le_bytes().to_vec();
                buf.push(t as u8);
                buf.extend(d);
                (buf, PaxType::Tagged, false, 0)
            }
            Value::Timestamp(ts) => (ts.to_le_bytes().to_vec(), PaxType::Timestamp, false, 0),
        }
    }

    fn encode_map(&mut self, pairs: &[(Value, Value)]) -> (Vec<u8>, PaxType, bool, u32) {
        let mut buf = (pairs.len() as u32).to_le_bytes().to_vec();
        for (k, v) in pairs {
            let (kd, kt, _, _) = self.encode_value(k, None);
            let (vd, vt, _, _) = self.encode_value(v, None);
            buf.push(kt as u8);
            buf.extend(kd);
            buf.push(vt as u8);
            buf.extend(vd);
        }
        (buf, PaxType::Map, false, pairs.len() as u32)
    }

    fn encode_array(&mut self, arr: &[Value], schema: Option<&Schema>) -> (Vec<u8>, PaxType, bool, u32) {
        let mut buf = (arr.len() as u32).to_le_bytes().to_vec();
        if arr.is_empty() { return (buf, PaxType::Array, true, 0); }
        if schema.is_some() && arr.iter().all(|v| matches!(v, Value::Object(_))) {
            return self.encode_struct_array(arr, schema.unwrap());
        }
        if arr.iter().all(|v| matches!(v, Value::Int(_))) {
            buf.push(PaxType::Int32 as u8);
            for v in arr { if let Value::Int(i) = v { buf.extend((*i as i32).to_le_bytes()); } }
            return (buf, PaxType::Array, true, arr.len() as u32);
        }
        if arr.iter().all(|v| matches!(v, Value::String(_))) {
            buf.push(PaxType::String as u8);
            for v in arr { if let Value::String(s) = v { buf.extend(self.intern(s).to_le_bytes()); } }
            return (buf, PaxType::Array, true, arr.len() as u32);
        }
        buf.push(0xFF);
        for v in arr { let (d, t, _, _) = self.encode_value(v, None); buf.push(t as u8); buf.extend(d); }
        (buf, PaxType::Array, true, arr.len() as u32)
    }

    fn encode_struct_array(&mut self, arr: &[Value], schema: &Schema) -> (Vec<u8>, PaxType, bool, u32) {
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
        (buf, PaxType::Struct, true, arr.len() as u32)
    }

    /// Encode a value according to a specific field type (schema-aware encoding)
    fn encode_typed_value(&mut self, value: &Value, field_type: &FieldType, nested_schema: Option<&Schema>) -> Vec<u8> {
        use crate::PaxType;

        // Handle arrays
        if field_type.is_array {
            if let Value::Array(arr) = value {
                let mut buf = (arr.len() as u32).to_le_bytes().to_vec();
                if arr.is_empty() { return buf; }

                // Determine element type
                let elem_type = FieldType::new(&field_type.base);
                let elem_pax_type = elem_type.to_pax_type();

                // For struct arrays, look up the correct element schema
                let elem_schema = self.schemas.iter()
                    .find(|s| s.name == field_type.base)
                    .cloned();

                // Write element type byte (standard array format)
                buf.push(elem_pax_type as u8);

                // Encode each element with proper type
                for v in arr {
                    buf.extend(self.encode_typed_value(v, &elem_type, elem_schema.as_ref()));
                }
                return buf;
            }
            return vec![];
        }

        let pax_type = field_type.to_pax_type();
        match pax_type {
            PaxType::Null => vec![],
            PaxType::Bool => {
                if let Value::Bool(b) = value { vec![if *b { 1 } else { 0 }] }
                else { vec![0] }
            }
            PaxType::Int8 => {
                let i = match value { Value::Int(i) => *i, Value::UInt(u) => *u as i64, _ => 0 };
                (i as i8).to_le_bytes().to_vec()
            }
            PaxType::Int16 => {
                let i = match value { Value::Int(i) => *i, Value::UInt(u) => *u as i64, _ => 0 };
                (i as i16).to_le_bytes().to_vec()
            }
            PaxType::Int32 => {
                let i = match value { Value::Int(i) => *i, Value::UInt(u) => *u as i64, _ => 0 };
                (i as i32).to_le_bytes().to_vec()
            }
            PaxType::Int64 => {
                let i = match value { Value::Int(i) => *i, Value::UInt(u) => *u as i64, _ => 0 };
                i.to_le_bytes().to_vec()
            }
            PaxType::UInt8 => {
                let u = match value { Value::UInt(u) => *u, Value::Int(i) => *i as u64, _ => 0 };
                (u as u8).to_le_bytes().to_vec()
            }
            PaxType::UInt16 => {
                let u = match value { Value::UInt(u) => *u, Value::Int(i) => *i as u64, _ => 0 };
                (u as u16).to_le_bytes().to_vec()
            }
            PaxType::UInt32 => {
                let u = match value { Value::UInt(u) => *u, Value::Int(i) => *i as u64, _ => 0 };
                (u as u32).to_le_bytes().to_vec()
            }
            PaxType::UInt64 => {
                let u = match value { Value::UInt(u) => *u, Value::Int(i) => *i as u64, _ => 0 };
                u.to_le_bytes().to_vec()
            }
            PaxType::Float32 => {
                let f = match value { Value::Float(f) => *f, Value::Int(i) => *i as f64, _ => 0.0 };
                (f as f32).to_le_bytes().to_vec()
            }
            PaxType::Float64 => {
                let f = match value { Value::Float(f) => *f, Value::Int(i) => *i as f64, _ => 0.0 };
                f.to_le_bytes().to_vec()
            }
            PaxType::String => {
                if let Value::String(s) = value { self.intern(s).to_le_bytes().to_vec() }
                else { self.intern("").to_le_bytes().to_vec() }
            }
            PaxType::Bytes => {
                if let Value::Bytes(b) = value {
                    let mut buf = Vec::new();
                    write_varint(&mut buf, b.len() as u64);
                    buf.extend(b);
                    buf
                } else { vec![0] }
            }
            PaxType::Timestamp => {
                if let Value::Timestamp(ts) = value { ts.to_le_bytes().to_vec() }
                else { 0i64.to_le_bytes().to_vec() }
            }
            PaxType::Struct => {
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

    fn encode_object(&mut self, obj: &HashMap<String, Value>) -> (Vec<u8>, PaxType, bool, u32) {
        let mut buf = (obj.len() as u16).to_le_bytes().to_vec();
        for (k, v) in obj {
            buf.extend(self.intern(k).to_le_bytes());
            let (d, t, _, _) = self.encode_value(v, None);
            buf.push(t as u8);
            buf.extend(d);
        }
        (buf, PaxType::Object, false, 0)
    }
}

impl Default for Writer { fn default() -> Self { Self::new() } }

fn encode_int(i: i64) -> (Vec<u8>, PaxType, bool, u32) {
    if i >= i8::MIN as i64 && i <= i8::MAX as i64 { ((i as i8).to_le_bytes().to_vec(), PaxType::Int8, false, 0) }
    else if i >= i16::MIN as i64 && i <= i16::MAX as i64 { ((i as i16).to_le_bytes().to_vec(), PaxType::Int16, false, 0) }
    else if i >= i32::MIN as i64 && i <= i32::MAX as i64 { ((i as i32).to_le_bytes().to_vec(), PaxType::Int32, false, 0) }
    else { (i.to_le_bytes().to_vec(), PaxType::Int64, false, 0) }
}

fn encode_uint(u: u64) -> (Vec<u8>, PaxType, bool, u32) {
    if u <= u8::MAX as u64 { ((u as u8).to_le_bytes().to_vec(), PaxType::UInt8, false, 0) }
    else if u <= u16::MAX as u64 { ((u as u16).to_le_bytes().to_vec(), PaxType::UInt16, false, 0) }
    else if u <= u32::MAX as u64 { ((u as u32).to_le_bytes().to_vec(), PaxType::UInt32, false, 0) }
    else { (u.to_le_bytes().to_vec(), PaxType::UInt64, false, 0) }
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
