//! Binary format writer for TeaLeaf

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write, Seek, SeekFrom};
use std::path::Path;
use crate::types::ObjectMap;

use crate::{Result, Value, Schema, Union, FieldType, TLType, MAGIC, VERSION_MAJOR, VERSION_MINOR, HEADER_SIZE,
    MAX_STRING_LENGTH, MAX_OBJECT_FIELDS, MAX_ARRAY_LENGTH};

pub struct Writer {
    strings: Vec<String>,
    string_map: HashMap<String, u32>,
    schemas: Vec<Schema>,
    schema_map: HashMap<String, u16>,
    unions: Vec<Union>,
    union_map: HashMap<String, u16>,
    sections: Vec<Section>,
    /// Indicates the source JSON was a root-level array (for round-trip fidelity)
    is_root_array: bool,
}

struct Section {
    key: String,
    data: Vec<u8>,
    schema_idx: i32,
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
            unions: Vec::new(),
            union_map: HashMap::new(),
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

    pub fn add_union(&mut self, union: Union) -> u16 {
        if let Some(&idx) = self.union_map.get(&union.name) { return idx; }
        self.intern(&union.name);
        for variant in &union.variants {
            self.intern(&variant.name);
            for field in &variant.fields {
                self.intern(&field.name);
            }
        }
        let idx = self.unions.len() as u16;
        self.union_map.insert(union.name.clone(), idx);
        self.unions.push(union);
        idx
    }

    pub fn add_section(&mut self, key: &str, value: &Value, schema: Option<&Schema>) -> Result<()> {
        self.intern(key);
        let (data, tl_type, is_array, item_count) = self.encode_value(value, schema)?;
        // Compute schema_idx AFTER encoding, since encode_value may register the schema
        let schema_idx = schema.map(|s| self.schema_map.get(&s.name).copied().unwrap_or(0xFFFF) as i32).unwrap_or(-1);
        self.sections.push(Section { key: key.to_string(), data, schema_idx, tl_type, is_array, item_count });
        Ok(())
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
            let (written, compressed): (Cow<'_, [u8]>, bool) = if compress && sec.data.len() > 64 {
                let c = compress_data(&sec.data)?;
                if c.len() < (sec.data.len() as f64 * 0.9) as usize {
                    (Cow::Owned(c), true)
                } else {
                    (Cow::Borrowed(sec.data.as_slice()), false)
                }
            } else {
                (Cow::Borrowed(sec.data.as_slice()), false)
            };
            w.write_all(written.as_ref())?;
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
        8 + self.strings.len() * 8 + self.strings.iter().map(|s| s.len()).sum::<usize>()
    }

    fn schema_table_size(&self) -> usize {
        if self.schemas.is_empty() && self.unions.is_empty() { return 8; }
        let struct_size = self.schemas.len() * 4
            + self.schemas.iter().map(|s| 8 + s.fields.len() * 8).sum::<usize>();
        let union_size = self.unions.len() * 4
            + self.unions.iter().map(|u| {
                8 + u.variants.iter().map(|v| 8 + v.fields.len() * 8).sum::<usize>()
            }).sum::<usize>();
        8 + struct_size + union_size
    }

    fn write_string_table<W: Write>(&self, w: &mut W) -> Result<()> {
        let table_size = self.string_table_size();
        if table_size > u32::MAX as usize {
            return Err(crate::Error::ValueOutOfRange(
                format!("String table size {} exceeds u32::MAX", table_size)));
        }
        let total_string_bytes: usize = self.strings.iter().map(|s| s.len()).sum();
        if total_string_bytes > u32::MAX as usize {
            return Err(crate::Error::ValueOutOfRange(
                format!("Total string data {} bytes exceeds u32::MAX", total_string_bytes)));
        }
        let mut off = 0u32;
        let offsets: Vec<u32> = self.strings.iter().map(|s| { let o = off; off += s.len() as u32; o }).collect();
        w.write_all(&(table_size as u32).to_le_bytes())?;
        w.write_all(&(self.strings.len() as u32).to_le_bytes())?;
        for o in &offsets { w.write_all(&o.to_le_bytes())?; }
        for s in &self.strings {
            if s.len() > MAX_STRING_LENGTH {
                return Err(crate::Error::ValueOutOfRange(
                    format!("String length {} exceeds maximum {}", s.len(), MAX_STRING_LENGTH)));
            }
            w.write_all(&(s.len() as u32).to_le_bytes())?;
        }
        for s in &self.strings { w.write_all(s.as_bytes())?; }
        Ok(())
    }

    fn write_schema_table<W: Write>(&self, w: &mut W) -> Result<()> {
        if self.schemas.is_empty() && self.unions.is_empty() {
            w.write_all(&8u32.to_le_bytes())?;
            w.write_all(&0u32.to_le_bytes())?;
            return Ok(());
        }

        // --- Struct data ---
        let mut struct_data = Vec::new();
        let mut off = 0u32;
        let struct_offsets: Vec<u32> = self.schemas.iter().map(|s| {
            let o = off;
            off += (8 + s.fields.len() * 8) as u32;
            o
        }).collect();
        for schema in &self.schemas {
            struct_data.extend_from_slice(&self.string_map[&schema.name].to_le_bytes());
            struct_data.extend_from_slice(&(schema.fields.len() as u16).to_le_bytes());
            struct_data.extend_from_slice(&0u16.to_le_bytes());
            for f in &schema.fields {
                struct_data.extend_from_slice(&self.string_map[&f.name].to_le_bytes());
                // Resolve union types: if the base name is in union_map, emit Tagged instead of Struct
                let resolved_tl_type = if self.union_map.contains_key(&f.field_type.base) {
                    TLType::Tagged
                } else {
                    f.field_type.to_tl_type()
                };
                struct_data.push(resolved_tl_type as u8);
                let mut flags: u8 = 0;
                if f.field_type.nullable { flags |= 0x01; }
                if f.field_type.is_array { flags |= 0x02; }
                struct_data.push(flags);
                // Store struct/union type name string index (0xFFFF = no type)
                if resolved_tl_type == TLType::Struct {
                    let type_name_idx = self.string_map.get(&f.field_type.base)
                        .copied()
                        .map(|i| i as u16)
                        .unwrap_or(0xFFFF);
                    struct_data.extend_from_slice(&type_name_idx.to_le_bytes());
                } else if resolved_tl_type == TLType::Tagged {
                    // Union-typed field: store union name string index
                    let type_name_idx = self.string_map.get(&f.field_type.base)
                        .copied()
                        .map(|i| i as u16)
                        .unwrap_or(0xFFFF);
                    struct_data.extend_from_slice(&type_name_idx.to_le_bytes());
                } else {
                    struct_data.extend_from_slice(&0xFFFFu16.to_le_bytes());
                }
            }
        }

        // --- Union data ---
        let mut union_data = Vec::new();
        let mut uoff = 0u32;
        let union_offsets: Vec<u32> = self.unions.iter().map(|u| {
            let o = uoff;
            uoff += (8 + u.variants.iter().map(|v| 8 + v.fields.len() * 8).sum::<usize>()) as u32;
            o
        }).collect();
        for union in &self.unions {
            union_data.extend_from_slice(&self.string_map[&union.name].to_le_bytes());
            union_data.extend_from_slice(&(union.variants.len() as u16).to_le_bytes());
            union_data.extend_from_slice(&0u16.to_le_bytes()); // flags (reserved)
            for variant in &union.variants {
                union_data.extend_from_slice(&self.string_map[&variant.name].to_le_bytes());
                union_data.extend_from_slice(&(variant.fields.len() as u16).to_le_bytes());
                union_data.extend_from_slice(&0u16.to_le_bytes()); // flags (reserved)
                for f in &variant.fields {
                    union_data.extend_from_slice(&self.string_map[&f.name].to_le_bytes());
                    let resolved_tl_type = if self.union_map.contains_key(&f.field_type.base) {
                        TLType::Tagged
                    } else {
                        f.field_type.to_tl_type()
                    };
                    union_data.push(resolved_tl_type as u8);
                    let mut flags: u8 = 0;
                    if f.field_type.nullable { flags |= 0x01; }
                    if f.field_type.is_array { flags |= 0x02; }
                    union_data.push(flags);
                    if resolved_tl_type == TLType::Struct || resolved_tl_type == TLType::Tagged {
                        let type_name_idx = self.string_map.get(&f.field_type.base)
                            .copied()
                            .map(|i| i as u16)
                            .unwrap_or(0xFFFF);
                        union_data.extend_from_slice(&type_name_idx.to_le_bytes());
                    } else {
                        union_data.extend_from_slice(&0xFFFFu16.to_le_bytes());
                    }
                }
            }
        }

        // --- Write header ---
        w.write_all(&(self.schema_table_size() as u32).to_le_bytes())?;
        w.write_all(&(self.schemas.len() as u16).to_le_bytes())?;
        w.write_all(&(self.unions.len() as u16).to_le_bytes())?; // was padding=0
        // Struct offsets, then struct data
        for o in &struct_offsets { w.write_all(&o.to_le_bytes())?; }
        w.write_all(&struct_data)?;
        // Union offsets, then union data
        for o in &union_offsets { w.write_all(&o.to_le_bytes())?; }
        w.write_all(&union_data)?;
        Ok(())
    }

    fn encode_value(&mut self, value: &Value, schema: Option<&Schema>) -> Result<(Vec<u8>, TLType, bool, u32)> {
        match value {
            Value::Null => Ok((vec![], TLType::Null, false, 0)),
            Value::Bool(b) => Ok((vec![if *b { 1 } else { 0 }], TLType::Bool, false, 0)),
            Value::Int(i) => Ok(encode_int(*i)),
            Value::UInt(u) => Ok(encode_uint(*u)),
            Value::Float(f) => Ok((f.to_le_bytes().to_vec(), TLType::Float64, false, 0)),
            Value::String(s) => { let idx = self.intern(s); Ok((idx.to_le_bytes().to_vec(), TLType::String, false, 0)) }
            Value::Bytes(b) => { let mut buf = Vec::new(); write_varint(&mut buf, b.len() as u64); buf.extend(b); Ok((buf, TLType::Bytes, false, 0)) }
            Value::Array(arr) => self.encode_array(arr, schema),
            Value::Object(obj) => self.encode_object(obj),
            Value::Map(pairs) => self.encode_map(pairs),
            Value::Ref(r) => { let idx = self.intern(r); Ok((idx.to_le_bytes().to_vec(), TLType::Ref, false, 0)) }
            Value::Tagged(tag, inner) => {
                let ti = self.intern(tag);
                let (d, t, _, _) = self.encode_value(inner, None)?;
                let mut buf = ti.to_le_bytes().to_vec();
                buf.push(t as u8);
                buf.extend(d);
                Ok((buf, TLType::Tagged, false, 0))
            }
            Value::Timestamp(ts, tz) => {
                let mut buf = ts.to_le_bytes().to_vec();
                buf.extend(tz.to_le_bytes());
                Ok((buf, TLType::Timestamp, false, 0))
            }
            Value::JsonNumber(s) => { let idx = self.intern(s); Ok((idx.to_le_bytes().to_vec(), TLType::JsonNumber, false, 0)) }
        }
    }

    fn encode_map(&mut self, pairs: &[(Value, Value)]) -> Result<(Vec<u8>, TLType, bool, u32)> {
        let mut buf = (pairs.len() as u32).to_le_bytes().to_vec();
        for (k, v) in pairs {
            // Validate map keys per spec: map_key = string | name | integer
            match k {
                Value::String(_) | Value::Int(_) | Value::UInt(_) => {}
                _ => return Err(crate::Error::ParseError(
                    format!("Invalid map key type {:?}: map keys must be string, int, or uint per spec", k.tl_type())
                )),
            }
            let (kd, kt, _, _) = self.encode_value(k, None)?;
            let (vd, vt, _, _) = self.encode_value(v, None)?;
            buf.push(kt as u8);
            buf.extend(kd);
            buf.push(vt as u8);
            buf.extend(vd);
        }
        Ok((buf, TLType::Map, false, pairs.len() as u32))
    }

    fn encode_array(&mut self, arr: &[Value], schema: Option<&Schema>) -> Result<(Vec<u8>, TLType, bool, u32)> {
        if arr.len() > MAX_ARRAY_LENGTH {
            return Err(crate::Error::ValueOutOfRange(
                format!("Array has {} elements, exceeds maximum {}", arr.len(), MAX_ARRAY_LENGTH)));
        }
        let mut buf = (arr.len() as u32).to_le_bytes().to_vec();
        if arr.is_empty() { return Ok((buf, TLType::Array, true, 0)); }
        if schema.is_some() && arr.iter().all(|v| matches!(v, Value::Object(_) | Value::Null)) {
            return self.encode_struct_array(arr, schema.unwrap());
        }
        // Spec-conformant homogeneous encoding: only Int32 and String for top-level arrays.
        // All other types (UInt, Bool, Float, Timestamp, Int64) use heterogeneous 0xFF encoding.
        // Schema-typed arrays (within @struct) use homogeneous encoding for any type via encode_typed_value.
        if arr.iter().all(|v| matches!(v, Value::Int(_))) {
            let all_fit_i32 = arr.iter().all(|v| {
                if let Value::Int(i) = v { *i >= i32::MIN as i64 && *i <= i32::MAX as i64 } else { false }
            });
            if all_fit_i32 {
                buf.push(TLType::Int32 as u8);
                for v in arr { if let Value::Int(i) = v { buf.extend((*i as i32).to_le_bytes()); } }
                return Ok((buf, TLType::Array, true, arr.len() as u32));
            }
            // Int values exceeding i32 range fall through to heterogeneous encoding
        }
        if arr.iter().all(|v| matches!(v, Value::String(_))) {
            buf.push(TLType::String as u8);
            for v in arr { if let Value::String(s) = v { buf.extend(self.intern(s).to_le_bytes()); } }
            return Ok((buf, TLType::Array, true, arr.len() as u32));
        }
        buf.push(0xFF);
        for v in arr { let (d, t, _, _) = self.encode_value(v, None)?; buf.push(t as u8); buf.extend(d); }
        Ok((buf, TLType::Array, true, arr.len() as u32))
    }

    fn encode_struct_array(&mut self, arr: &[Value], schema: &Schema) -> Result<(Vec<u8>, TLType, bool, u32)> {
        let mut buf = (arr.len() as u32).to_le_bytes().to_vec();
        let si = match self.schema_map.get(&schema.name) {
            Some(&idx) => idx,
            None => self.add_schema(schema.clone()),
        };
        buf.extend(si.to_le_bytes());
        let bms = (schema.fields.len() + 7) / 8;
        buf.extend((bms as u16).to_le_bytes());
        // Pre-build schema index lookup to avoid O(n×m) linear scans per field per row.
        let nested_schema_indices: Vec<Option<usize>> = schema.fields.iter()
            .map(|f| self.schema_map.get(&f.field_type.base).copied().map(|i| i as usize))
            .collect();
        for v in arr {
            if let Value::Object(obj) = v {
                let mut bitmap = vec![0u8; bms];
                for (i, f) in schema.fields.iter().enumerate() {
                    if obj.get(&f.name).map(|v| v.is_null()).unwrap_or(true) {
                        bitmap[i / 8] |= 1 << (i % 8);
                    }
                }
                buf.extend_from_slice(&bitmap);
                for (i, f) in schema.fields.iter().enumerate() {
                    let is_null = bitmap[i / 8] & (1 << (i % 8)) != 0;
                    if !is_null {
                        if let Some(v) = obj.get(&f.name) {
                            let nested_schema = nested_schema_indices[i]
                                .and_then(|idx| self.schemas.get(idx));
                            let data = self.encode_typed_value(v, &f.field_type, nested_schema)?;
                            buf.extend(data);
                        }
                    }
                }
            } else {
                // Null element: write bitmap with all field bits set, no field data
                let mut bitmap = vec![0u8; bms];
                for i in 0..schema.fields.len() {
                    bitmap[i / 8] |= 1 << (i % 8);
                }
                buf.extend_from_slice(&bitmap);
            }
        }
        Ok((buf, TLType::Struct, true, arr.len() as u32))
    }

    /// Encode a value according to a specific field type (schema-aware encoding)
    fn encode_typed_value(&mut self, value: &Value, field_type: &FieldType, nested_schema: Option<&Schema>) -> Result<Vec<u8>> {
        use crate::TLType;

        // Handle arrays
        if field_type.is_array {
            if let Value::Array(arr) = value {
                let mut buf = (arr.len() as u32).to_le_bytes().to_vec();
                if arr.is_empty() { return Ok(buf); }

                // Determine element type, resolving unions via union_map
                let elem_type = FieldType::new(&field_type.base);
                let elem_tl_type = if self.union_map.contains_key(&field_type.base) {
                    TLType::Tagged
                } else {
                    elem_type.to_tl_type()
                };

                // For struct arrays, look up the correct element schema
                let elem_schema = self.schema_map
                    .get(&field_type.base)
                    .and_then(|idx| self.schemas.get(*idx as usize));

                // If elem type resolves to Struct but no schema exists (e.g., "any"
                // pseudo-type from JSON inference), use heterogeneous encoding —
                // the reader can't decode struct format without a schema.
                if elem_tl_type == TLType::Struct && elem_schema.is_none() {
                    buf.push(0xFF);
                    for v in arr {
                        let (d, t, _, _) = self.encode_value(v, None)?;
                        buf.push(t as u8);
                        buf.extend(d);
                    }
                    return Ok(buf);
                }

                // Write element type byte (standard array format)
                buf.push(elem_tl_type as u8);

                // Encode each element with proper type
                for v in arr {
                    buf.extend(self.encode_typed_value(v, &elem_type, elem_schema)?);
                }
                return Ok(buf);
            }
            // Non-array value for array-typed field: encode as zero-length array
            // to maintain stream alignment (empty vec would corrupt subsequent fields)
            return Ok((0u32).to_le_bytes().to_vec());
        }

        let tl_type = field_type.to_tl_type();
        match tl_type {
            TLType::Null => Ok(vec![]),
            TLType::Bool => {
                if let Value::Bool(b) = value { Ok(vec![if *b { 1 } else { 0 }]) }
                else { Ok(vec![0]) }
            }
            TLType::Int8 => {
                let i = checked_int_value(value, i8::MIN as i64, i8::MAX as i64, "int8")?;
                Ok((i as i8).to_le_bytes().to_vec())
            }
            TLType::Int16 => {
                let i = checked_int_value(value, i16::MIN as i64, i16::MAX as i64, "int16")?;
                Ok((i as i16).to_le_bytes().to_vec())
            }
            TLType::Int32 => {
                let i = checked_int_value(value, i32::MIN as i64, i32::MAX as i64, "int32")?;
                Ok((i as i32).to_le_bytes().to_vec())
            }
            TLType::Int64 => {
                let i = checked_int_value(value, i64::MIN, i64::MAX, "int64")?;
                Ok(i.to_le_bytes().to_vec())
            }
            TLType::UInt8 => {
                let u = checked_uint_value(value, u8::MAX as u64, "uint8")?;
                Ok((u as u8).to_le_bytes().to_vec())
            }
            TLType::UInt16 => {
                let u = checked_uint_value(value, u16::MAX as u64, "uint16")?;
                Ok((u as u16).to_le_bytes().to_vec())
            }
            TLType::UInt32 => {
                let u = checked_uint_value(value, u32::MAX as u64, "uint32")?;
                Ok((u as u32).to_le_bytes().to_vec())
            }
            TLType::UInt64 => {
                let u = checked_uint_value(value, u64::MAX, "uint64")?;
                Ok(u.to_le_bytes().to_vec())
            }
            TLType::Float32 => {
                let f = match value { Value::Float(f) => *f, Value::Int(i) => *i as f64, Value::UInt(u) => *u as f64, _ => 0.0 };
                Ok((f as f32).to_le_bytes().to_vec())
            }
            TLType::Float64 => {
                let f = match value { Value::Float(f) => *f, Value::Int(i) => *i as f64, Value::UInt(u) => *u as f64, _ => 0.0 };
                Ok(f.to_le_bytes().to_vec())
            }
            TLType::String => {
                if let Value::String(s) = value { Ok(self.intern(s).to_le_bytes().to_vec()) }
                else { Ok(self.intern("").to_le_bytes().to_vec()) }
            }
            TLType::Bytes => {
                if let Value::Bytes(b) = value {
                    let mut buf = Vec::new();
                    write_varint(&mut buf, b.len() as u64);
                    buf.extend(b);
                    Ok(buf)
                } else { Ok(vec![0]) }
            }
            TLType::Timestamp => {
                if let Value::Timestamp(ts, tz) = value {
                    let mut buf = ts.to_le_bytes().to_vec();
                    buf.extend(tz.to_le_bytes());
                    Ok(buf)
                } else {
                    let mut buf = 0i64.to_le_bytes().to_vec();
                    buf.extend(0i16.to_le_bytes());
                    Ok(buf)
                }
            }
            TLType::Struct => {
                // Check if this is actually a union type resolved at encoding time
                if self.union_map.contains_key(&field_type.base) {
                    let (d, _, _, _) = self.encode_value(value, None)?;
                    return Ok(d);
                }
                // Nested struct - encode recursively
                if let (Value::Object(obj), Some(schema)) = (value, nested_schema) {
                    let mut buf = Vec::new();

                    // Write schema index
                    let schema_idx = *self.schema_map.get(&schema.name).unwrap_or(&0);
                    buf.extend(schema_idx.to_le_bytes());

                    let bms = (schema.fields.len() + 7) / 8;

                    // Bitmap (supports >64 fields)
                    let mut bitmap = vec![0u8; bms];
                    for (i, f) in schema.fields.iter().enumerate() {
                        if obj.get(&f.name).map(|v| v.is_null()).unwrap_or(true) {
                            bitmap[i / 8] |= 1 << (i % 8);
                        }
                    }
                    buf.extend_from_slice(&bitmap);

                    // Fields
                    for (i, f) in schema.fields.iter().enumerate() {
                        let is_null = bitmap[i / 8] & (1 << (i % 8)) != 0;
                        if !is_null {
                            if let Some(v) = obj.get(&f.name) {
                                let nested = self.schema_map
                                    .get(&f.field_type.base)
                                    .and_then(|idx| self.schemas.get(*idx as usize));
                                buf.extend(self.encode_typed_value(v, &f.field_type, nested)?);
                            }
                        }
                    }
                    Ok(buf)
                } else {
                    // No schema found — fall back to generic encoding
                    // (e.g., 'any' pseudo-type from JSON schema inference)
                    let (d, _, _, _) = self.encode_value(value, None)?;
                    Ok(d)
                }
            }
            _ => {
                // Fallback to generic encoding
                let (d, _, _, _) = self.encode_value(value, None)?;
                Ok(d)
            }
        }
    }

    fn encode_object(&mut self, obj: &ObjectMap<String, Value>) -> Result<(Vec<u8>, TLType, bool, u32)> {
        if obj.len() > MAX_OBJECT_FIELDS {
            return Err(crate::Error::ValueOutOfRange(
                format!("Object has {} fields, exceeds maximum {}", obj.len(), MAX_OBJECT_FIELDS)));
        }
        let mut buf = (obj.len() as u16).to_le_bytes().to_vec();
        for (k, v) in obj {
            buf.extend(self.intern(k).to_le_bytes());
            let (d, t, _, _) = self.encode_value(v, None)?;
            buf.push(t as u8);
            buf.extend(d);
        }
        Ok((buf, TLType::Object, false, 0))
    }
}

impl Default for Writer { fn default() -> Self { Self::new() } }

/// Extract an integer value with best-effort coercion for schema-typed fields.
/// Out-of-range and non-numeric values default to 0 (spec §2.5).
fn checked_int_value(value: &Value, min: i64, max: i64, _type_name: &str) -> Result<i64> {
    let i = match value {
        Value::Int(i) => *i,
        Value::UInt(u) if *u <= i64::MAX as u64 => *u as i64,
        Value::UInt(_) => 0,
        Value::Float(f) => {
            let f = *f;
            if f.is_finite() && f >= i64::MIN as f64 && f <= i64::MAX as f64 { f as i64 } else { 0 }
        }
        Value::JsonNumber(s) => s.parse::<i64>().unwrap_or(0),
        _ => 0,
    };
    if i < min || i > max { Ok(0) } else { Ok(i) }
}

/// Extract an unsigned integer value with best-effort coercion for schema-typed fields.
/// Out-of-range and non-numeric values default to 0 (spec §2.5).
fn checked_uint_value(value: &Value, max: u64, _type_name: &str) -> Result<u64> {
    let u = match value {
        Value::UInt(u) => *u,
        Value::Int(i) if *i >= 0 => *i as u64,
        Value::Int(_) => 0,
        Value::Float(f) => {
            let f = *f;
            if f.is_finite() && f >= 0.0 && f <= u64::MAX as f64 { f as u64 } else { 0 }
        }
        Value::JsonNumber(s) => s.parse::<u64>().unwrap_or(0),
        _ => 0,
    };
    if u > max { Ok(0) } else { Ok(u) }
}

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

fn compress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(data).map_err(crate::Error::Io)?;
    e.finish().map_err(crate::Error::Io)
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
        w.add_section("small", &Value::UInt(42), None).unwrap();
        w.add_section("medium", &Value::UInt(300), None).unwrap();
        w.add_section("large", &Value::UInt(100_000), None).unwrap();
        w.add_section("huge", &Value::UInt(5_000_000_000), None).unwrap();
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
        let mut obj = ObjectMap::new();
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
        w.add_section("records", &arr, Some(&schema)).unwrap();
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

        let mut obj = ObjectMap::new();
        obj.insert("name".to_string(), Value::String("Alice".into()));
        obj.insert("scores".to_string(), Value::Array(vec![Value::Int(90), Value::Int(85)]));

        let arr = Value::Array(vec![Value::Object(obj)]);
        w.add_section("users", &arr, Some(&schema)).unwrap();
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

        let mut obj = ObjectMap::new();
        obj.insert("x".to_string(), Value::Int(10));
        obj.insert("y".to_string(), Value::String("hello".into()));

        let mut w = Writer::new();
        w.add_section("data", &Value::Object(obj), None).unwrap();
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
        w.add_section("mapping", &Value::Map(pairs), None).unwrap();
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
        w.add_section("myref", &Value::Ref("some_ref".into()), None).unwrap();
        w.add_section("mytag", &Value::Tagged("label".into(), Box::new(Value::Int(42))), None).unwrap();
        w.add_section("myts", &Value::Timestamp(1700000000000, 0), None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();

        if let Value::Ref(s) = r.get("myref").unwrap() {
            assert_eq!(s, "some_ref");
        } else { panic!("Expected Ref"); }

        if let Value::Tagged(tag, inner) = r.get("mytag").unwrap() {
            assert_eq!(tag, "label");
            assert_eq!(inner.as_int(), Some(42));
        } else { panic!("Expected Tagged"); }

        if let Value::Timestamp(ts, _) = r.get("myts").unwrap() {
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
        w.add_section("numbers", &Value::Array(arr), None).unwrap();
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
        w.add_section("root", &Value::Array(vec![Value::Int(1)]), None).unwrap();
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
        w.add_section("blob", &Value::Bytes(vec![1, 2, 3, 4, 5]), None).unwrap();
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

        let mut addr = ObjectMap::new();
        addr.insert("city".to_string(), Value::String("Seattle".into()));
        addr.insert("zip".to_string(), Value::String("98101".into()));

        let mut person = ObjectMap::new();
        person.insert("name".to_string(), Value::String("Alice".into()));
        person.insert("home".to_string(), Value::Object(addr));

        let arr = Value::Array(vec![Value::Object(person)]);
        w.add_section("people", &arr, Some(&outer_schema)).unwrap();
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

        let mut obj = ObjectMap::new();
        obj.insert("name".to_string(), Value::String("deploy".into()));
        obj.insert("ts".to_string(), Value::Timestamp(1700000000000, 0));

        let arr = Value::Array(vec![Value::Object(obj)]);
        w.add_section("events", &arr, Some(&schema)).unwrap();
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

        let mut obj = ObjectMap::new();
        obj.insert("name".to_string(), Value::String("img".into()));
        obj.insert("data".to_string(), Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));

        let arr = Value::Array(vec![Value::Object(obj)]);
        w.add_section("blobs", &arr, Some(&schema)).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let blobs = r.get("blobs").unwrap();
        let items = blobs.as_array().unwrap();
        assert_eq!(items.len(), 1);
        std::fs::remove_file(&path).ok();
    }

    // =========================================================================
    // Issue 4: Checked numeric downcasting
    // =========================================================================

    #[test]
    fn test_checked_int_value_in_range() {
        assert_eq!(checked_int_value(&Value::Int(42), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), 42);
        assert_eq!(checked_int_value(&Value::Int(-128), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), -128);
        assert_eq!(checked_int_value(&Value::Int(127), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), 127);
        assert_eq!(checked_int_value(&Value::UInt(100), i16::MIN as i64, i16::MAX as i64, "int16").unwrap(), 100);
    }

    #[test]
    fn test_checked_int_value_overflow_defaults_to_zero() {
        // Spec §2.5: out-of-range defaults to 0, not error
        assert_eq!(checked_int_value(&Value::Int(128), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), 0);
    }

    #[test]
    fn test_checked_int_value_underflow_defaults_to_zero() {
        assert_eq!(checked_int_value(&Value::Int(-129), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), 0);
    }

    #[test]
    fn test_checked_int_value_float_coercion() {
        // Spec §2.5: floats coerce to integers
        assert_eq!(checked_int_value(&Value::Float(42.7), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), 42);
        assert_eq!(checked_int_value(&Value::Float(-3.9), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), -3);
        // NaN/Inf default to 0
        assert_eq!(checked_int_value(&Value::Float(f64::NAN), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), 0);
        assert_eq!(checked_int_value(&Value::Float(f64::INFINITY), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), 0);
        // Float that truncates out of range defaults to 0
        assert_eq!(checked_int_value(&Value::Float(200.0), i8::MIN as i64, i8::MAX as i64, "int8").unwrap(), 0);
    }

    #[test]
    fn test_checked_uint_value_in_range() {
        assert_eq!(checked_uint_value(&Value::UInt(255), u8::MAX as u64, "uint8").unwrap(), 255);
        assert_eq!(checked_uint_value(&Value::Int(42), u8::MAX as u64, "uint8").unwrap(), 42);
    }

    #[test]
    fn test_checked_uint_value_overflow_defaults_to_zero() {
        // Spec §2.5: out-of-range defaults to 0, not error
        assert_eq!(checked_uint_value(&Value::UInt(256), u8::MAX as u64, "uint8").unwrap(), 0);
    }

    #[test]
    fn test_checked_uint_value_negative_defaults_to_zero() {
        // Spec §2.5: negative for unsigned defaults to 0, not error
        assert_eq!(checked_uint_value(&Value::Int(-1), u8::MAX as u64, "uint8").unwrap(), 0);
    }

    #[test]
    fn test_checked_uint_value_float_coercion() {
        assert_eq!(checked_uint_value(&Value::Float(42.7), u8::MAX as u64, "uint8").unwrap(), 42);
        assert_eq!(checked_uint_value(&Value::Float(-1.0), u8::MAX as u64, "uint8").unwrap(), 0);
        assert_eq!(checked_uint_value(&Value::Float(f64::NAN), u8::MAX as u64, "uint8").unwrap(), 0);
        assert_eq!(checked_uint_value(&Value::Float(300.0), u8::MAX as u64, "uint8").unwrap(), 0);
    }

    // =========================================================================
    // Issue 1: Union/Enum round-trip via union_map
    // =========================================================================

    #[test]
    fn test_union_field_roundtrip() {
        // A struct with a field typed as a union should round-trip correctly
        let dir = std::env::temp_dir();
        let path = dir.join("test_union_field_rt.tlbx");

        let mut w = Writer::new();

        // Define a union
        let mut union_def = crate::Union::new("Shape");
        union_def.add_variant(crate::Variant::new("Circle").field("radius", FieldType::new("float64")));
        union_def.add_variant(crate::Variant::new("Rect").field("w", FieldType::new("float64")).field("h", FieldType::new("float64")));
        w.add_union(union_def);

        // Add a tagged value (as if from a union-typed field)
        let tagged = Value::Tagged(
            "Circle".to_string(),
            Box::new(Value::Object({
                let mut m = ObjectMap::new();
                m.insert("radius".to_string(), Value::Float(5.0));
                m
            })),
        );
        w.add_section("shape", &tagged, None).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let shape = r.get("shape").unwrap();
        if let Value::Tagged(tag, inner) = &shape {
            assert_eq!(tag, "Circle");
            let obj = inner.as_object().unwrap();
            assert_eq!(obj.get("radius").unwrap().as_float(), Some(5.0));
        } else {
            panic!("Expected Tagged value, got {:?}", shape);
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_union_typed_schema_field_roundtrip() {
        // A struct schema where one field is a union type
        let dir = std::env::temp_dir();
        let path = dir.join("test_union_schema_field.tlbx");

        let mut w = Writer::new();

        // Union: Status { Ok { code: int }, Err { msg: string } }
        let mut union_def = crate::Union::new("Status");
        union_def.add_variant(crate::Variant::new("Ok").field("code", FieldType::new("int32")));
        union_def.add_variant(crate::Variant::new("Err").field("msg", FieldType::new("string")));
        w.add_union(union_def);

        // Struct: Response { id: int, status: Status }
        let mut schema = Schema::new("Response");
        schema.add_field("id", FieldType::new("int32"));
        schema.add_field("status", FieldType::new("Status")); // Union-typed field

        let mut obj = ObjectMap::new();
        obj.insert("id".to_string(), Value::Int(1));
        obj.insert("status".to_string(), Value::Tagged(
            "Ok".to_string(),
            Box::new(Value::Object({
                let mut m = ObjectMap::new();
                m.insert("code".to_string(), Value::Int(200));
                m
            })),
        ));

        let arr = Value::Array(vec![Value::Object(obj)]);
        w.add_section("responses", &arr, Some(&schema)).unwrap();
        w.write(&path, false).unwrap();

        let r = Reader::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        // Verify the reader parsed the union and schema correctly
        assert!(!r.unions.is_empty(), "Reader should have unions");
        assert_eq!(r.unions[0].name, "Status");
        assert!(!r.schemas.is_empty(), "Reader should have schemas");
        assert_eq!(r.schemas[0].name, "Response");
        let responses = r.get("responses").unwrap();
        let items = responses.as_array().unwrap();
        assert_eq!(items.len(), 1);
        let resp = items[0].as_object().unwrap();
        assert_eq!(resp.get("id").unwrap().as_int(), Some(1));
        if let Value::Tagged(tag, inner) = resp.get("status").unwrap() {
            assert_eq!(tag, "Ok");
            let obj = inner.as_object().unwrap();
            assert_eq!(obj.get("code").unwrap().as_int(), Some(200));
        } else {
            panic!("Expected Tagged value for status field");
        }
        std::fs::remove_file(&path).ok();
    }

    // =========================================================================
    // Issue 7: Deterministic serialization (sorted object keys)
    // =========================================================================

    #[test]
    fn test_object_encoding_deterministic() {
        // Encoding the same object multiple times should produce identical bytes
        let mut obj = ObjectMap::new();
        obj.insert("zebra".to_string(), Value::Int(1));
        obj.insert("alpha".to_string(), Value::Int(2));
        obj.insert("middle".to_string(), Value::Int(3));

        let mut w1 = Writer::new();
        let (bytes1, _, _, _) = w1.encode_object(&obj).unwrap();

        let mut w2 = Writer::new();
        let (bytes2, _, _, _) = w2.encode_object(&obj).unwrap();

        assert_eq!(bytes1, bytes2, "Object encoding should be deterministic");
    }
}
