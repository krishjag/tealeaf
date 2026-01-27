use criterion::{measurement::WallTime, BenchmarkGroup, BenchmarkId};
use prost::Message;
use std::hint::black_box;
use tempfile::NamedTempFile;

use crate::common::data;
use crate::common::structs::MixedData;
use crate::proto::benchmark as pb;

pub fn bench_encode(group: &mut BenchmarkGroup<WallTime>) {
    let pax_text = data::mixed_types_pax_text();
    let serde_data = data::mixed_types_struct();

    // Pax: Text Parse
    group.bench_function(BenchmarkId::new("encode", "pax_parse"), |b| {
        b.iter(|| pax::Pax::parse(black_box(pax_text)).unwrap());
    });

    // Pax: Binary Encode (from pre-parsed)
    let pax_doc = pax::Pax::parse(pax_text).unwrap();
    group.bench_function(BenchmarkId::new("encode", "pax_binary"), |b| {
        b.iter(|| {
            let tmp = NamedTempFile::new().unwrap();
            pax_doc.compile(tmp.path(), false).unwrap();
        });
    });

    // JSON
    group.bench_function(BenchmarkId::new("encode", "json"), |b| {
        b.iter(|| serde_json::to_vec(black_box(&serde_data)).unwrap());
    });

    // MessagePack
    group.bench_function(BenchmarkId::new("encode", "msgpack"), |b| {
        b.iter(|| rmp_serde::to_vec(black_box(&serde_data)).unwrap());
    });

    // CBOR
    group.bench_function(BenchmarkId::new("encode", "cbor"), |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            ciborium::into_writer(black_box(&serde_data), &mut buf).unwrap();
            buf
        });
    });

    // Protobuf
    let proto_data = data::mixed_types_proto();
    group.bench_function(BenchmarkId::new("encode", "protobuf"), |b| {
        b.iter(|| black_box(&proto_data).encode_to_vec());
    });
}

pub fn bench_decode(group: &mut BenchmarkGroup<WallTime>) {
    let serde_data = data::mixed_types_struct();

    // Pre-encode all formats
    let json_bytes = serde_json::to_vec(&serde_data).unwrap();
    let msgpack_bytes = rmp_serde::to_vec(&serde_data).unwrap();
    let mut cbor_bytes = Vec::new();
    ciborium::into_writer(&serde_data, &mut cbor_bytes).unwrap();

    // Create Pax binary file
    let pax_text = data::mixed_types_pax_text();
    let pax_doc = pax::Pax::parse(pax_text).unwrap();
    let pax_tmp = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp.path(), false).unwrap();
    let pax_bytes = std::fs::read(pax_tmp.path()).unwrap();

    // Pax Binary Decode
    group.bench_function(BenchmarkId::new("decode", "pax_binary"), |b| {
        b.iter(|| {
            let reader = pax::Reader::from_bytes(black_box(pax_bytes.clone())).unwrap();
            reader.get("record").unwrap()
        });
    });

    // JSON
    group.bench_function(BenchmarkId::new("decode", "json"), |b| {
        b.iter(|| serde_json::from_slice::<MixedData>(black_box(&json_bytes)).unwrap());
    });

    // MessagePack
    group.bench_function(BenchmarkId::new("decode", "msgpack"), |b| {
        b.iter(|| rmp_serde::from_slice::<MixedData>(black_box(&msgpack_bytes)).unwrap());
    });

    // CBOR
    group.bench_function(BenchmarkId::new("decode", "cbor"), |b| {
        b.iter(|| {
            ciborium::from_reader::<MixedData, _>(black_box(cbor_bytes.as_slice())).unwrap()
        });
    });

    // Protobuf
    let proto_data = data::mixed_types_proto();
    let proto_bytes = proto_data.encode_to_vec();
    group.bench_function(BenchmarkId::new("decode", "protobuf"), |b| {
        b.iter(|| pb::MixedData::decode(black_box(proto_bytes.as_slice())).unwrap());
    });
}
