//! Size comparison report for various serialization formats
//!
//! Run with: cargo run --example size_report
//!
//! Compares serialized sizes of PAX against JSON, MessagePack, CBOR, Bincode, and Protobuf.

use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tempfile::NamedTempFile;

// Include generated protobuf code
pub mod proto {
    pub mod benchmark {
        include!(concat!(env!("OUT_DIR"), "/benchmark.rs"));
    }
}

use proto::benchmark as pb;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SmallConfig {
    name: String,
    version: i32,
    enabled: bool,
    threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
    zip: String,
    country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Company {
    name: String,
    address: Address,
    employee_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Person {
    id: i64,
    name: String,
    email: String,
    age: i32,
    employer: Company,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MixedData {
    id: i64,
    name: String,
    tags: Vec<String>,
    scores: Vec<f64>,
    metadata: HashMap<String, String>,
    active: bool,
    raw_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    username: String,
    email: String,
    created_at: i64,
    is_admin: bool,
}

fn main() {
    println!("\n{:=<80}", "");
    println!("SERIALIZATION SIZE COMPARISON REPORT");
    println!("{:=<80}", "");

    report_small_object();
    report_large_array(100);
    report_large_array(1000);
    report_large_array(10000);
    report_nested_structs();
    report_mixed_types();
    report_tabular_users(100);
    report_tabular_users(1000);
    report_tabular_users(5000);
}

fn print_header() {
    println!(
        "{:<20} {:>12} {:>12} {:>10}",
        "Format", "Size (B)", "vs JSON", "Ratio"
    );
    println!("{:-<56}", "");
}

fn print_row(name: &str, size: usize, baseline: usize) {
    let ratio = size as f64 / baseline as f64;
    let diff = if size < baseline {
        format!("-{:.1}%", (1.0 - ratio) * 100.0)
    } else if size > baseline {
        format!("+{:.1}%", (ratio - 1.0) * 100.0)
    } else {
        "0.0%".to_string()
    };
    println!(
        "{:<20} {:>12} {:>12} {:>10.2}x",
        name, size, diff, ratio
    );
}

fn report_small_object() {
    println!("\n--- Small Object (Config) ---\n");

    let serde_data = SmallConfig {
        name: "my-service".to_string(),
        version: 42,
        enabled: true,
        threshold: 0.85,
    };

    let proto_data = pb::SmallConfig {
        name: "my-service".to_string(),
        version: 42,
        enabled: true,
        threshold: 0.85,
    };

    let pax_text = r#"config: {
    name: "my-service",
    version: 42,
    enabled: true,
    threshold: 0.85
}"#;

    let json_size = serde_json::to_vec(&serde_data).unwrap().len();
    let msgpack_size = rmp_serde::to_vec(&serde_data).unwrap().len();
    let cbor_size = {
        let mut buf = Vec::new();
        ciborium::into_writer(&serde_data, &mut buf).unwrap();
        buf.len()
    };
    let bincode_size = bincode::serialize(&serde_data).unwrap().len();
    let protobuf_size = proto_data.encode_to_vec().len();

    let pax_doc = pax::Pax::parse(pax_text).unwrap();
    let pax_tmp = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp.path(), false).unwrap();
    let pax_size = std::fs::metadata(pax_tmp.path()).unwrap().len() as usize;

    let pax_tmp_compressed = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp_compressed.path(), true).unwrap();
    let pax_compressed_size = std::fs::metadata(pax_tmp_compressed.path()).unwrap().len() as usize;

    let pax_text_size = pax_text.len();

    print_header();
    print_row("JSON", json_size, json_size);
    print_row("MessagePack", msgpack_size, json_size);
    print_row("CBOR", cbor_size, json_size);
    print_row("Bincode", bincode_size, json_size);
    print_row("Protobuf", protobuf_size, json_size);
    print_row("Pax Text", pax_text_size, json_size);
    print_row("Pax Binary", pax_size, json_size);
    print_row("Pax Compressed", pax_compressed_size, json_size);
}

fn report_large_array(count: usize) {
    println!("\n--- Large Array ({} Points) ---\n", count);

    let serde_data: Vec<Point> = (0..count)
        .map(|i| Point {
            x: i as f64 * 0.1,
            y: i as f64 * 0.2,
            z: i as f64 * 0.3,
        })
        .collect();

    let proto_data = pb::PointList {
        points: (0..count)
            .map(|i| pb::Point {
                x: i as f64 * 0.1,
                y: i as f64 * 0.2,
                z: i as f64 * 0.3,
            })
            .collect(),
    };

    let mut pax_text =
        String::from("@struct point (x: float, y: float, z: float)\npoints: @table point [\n");
    for i in 0..count {
        pax_text.push_str(&format!(
            "    ({}, {}, {}),\n",
            i as f64 * 0.1,
            i as f64 * 0.2,
            i as f64 * 0.3
        ));
    }
    pax_text.push_str("]\n");

    let json_size = serde_json::to_vec(&serde_data).unwrap().len();
    let msgpack_size = rmp_serde::to_vec(&serde_data).unwrap().len();
    let cbor_size = {
        let mut buf = Vec::new();
        ciborium::into_writer(&serde_data, &mut buf).unwrap();
        buf.len()
    };
    let bincode_size = bincode::serialize(&serde_data).unwrap().len();
    let protobuf_size = proto_data.encode_to_vec().len();

    let pax_doc = pax::Pax::parse(&pax_text).unwrap();
    let pax_tmp = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp.path(), false).unwrap();
    let pax_size = std::fs::metadata(pax_tmp.path()).unwrap().len() as usize;

    let pax_tmp_compressed = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp_compressed.path(), true).unwrap();
    let pax_compressed_size = std::fs::metadata(pax_tmp_compressed.path()).unwrap().len() as usize;

    let pax_text_size = pax_text.len();

    print_header();
    print_row("JSON", json_size, json_size);
    print_row("MessagePack", msgpack_size, json_size);
    print_row("CBOR", cbor_size, json_size);
    print_row("Bincode", bincode_size, json_size);
    print_row("Protobuf", protobuf_size, json_size);
    print_row("Pax Text", pax_text_size, json_size);
    print_row("Pax Binary", pax_size, json_size);
    print_row("Pax Compressed", pax_compressed_size, json_size);
}

fn report_nested_structs() {
    println!("\n--- Nested Structs (2 People) ---\n");

    let serde_data = vec![
        Person {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            age: 30,
            employer: Company {
                name: "TechCorp".to_string(),
                address: Address {
                    street: "123 Main St".to_string(),
                    city: "San Francisco".to_string(),
                    zip: "94102".to_string(),
                    country: "USA".to_string(),
                },
                employee_count: 500,
            },
        },
        Person {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            age: 25,
            employer: Company {
                name: "DataInc".to_string(),
                address: Address {
                    street: "456 Oak Ave".to_string(),
                    city: "New York".to_string(),
                    zip: "10001".to_string(),
                    country: "USA".to_string(),
                },
                employee_count: 1200,
            },
        },
    ];

    let proto_data = pb::PersonList {
        people: vec![
            pb::Person {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                age: 30,
                employer: Some(pb::Company {
                    name: "TechCorp".to_string(),
                    address: Some(pb::Address {
                        street: "123 Main St".to_string(),
                        city: "San Francisco".to_string(),
                        zip: "94102".to_string(),
                        country: "USA".to_string(),
                    }),
                    employee_count: 500,
                }),
            },
            pb::Person {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
                age: 25,
                employer: Some(pb::Company {
                    name: "DataInc".to_string(),
                    address: Some(pb::Address {
                        street: "456 Oak Ave".to_string(),
                        city: "New York".to_string(),
                        zip: "10001".to_string(),
                        country: "USA".to_string(),
                    }),
                    employee_count: 1200,
                }),
            },
        ],
    };

    let pax_text = r#"@struct address (street: string, city: string, zip: string, country: string)
@struct company (name: string, address: address, employee_count: int)
@struct person (id: int64, name: string, email: string, age: int, employer: company)

people: @table person [
    (1, "Alice", "alice@example.com", 30, ("TechCorp", ("123 Main St", "San Francisco", "94102", "USA"), 500)),
    (2, "Bob", "bob@example.com", 25, ("DataInc", ("456 Oak Ave", "New York", "10001", "USA"), 1200)),
]"#;

    let json_size = serde_json::to_vec(&serde_data).unwrap().len();
    let msgpack_size = rmp_serde::to_vec(&serde_data).unwrap().len();
    let cbor_size = {
        let mut buf = Vec::new();
        ciborium::into_writer(&serde_data, &mut buf).unwrap();
        buf.len()
    };
    let bincode_size = bincode::serialize(&serde_data).unwrap().len();
    let protobuf_size = proto_data.encode_to_vec().len();

    let pax_doc = pax::Pax::parse(pax_text).unwrap();
    let pax_tmp = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp.path(), false).unwrap();
    let pax_size = std::fs::metadata(pax_tmp.path()).unwrap().len() as usize;

    let pax_tmp_compressed = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp_compressed.path(), true).unwrap();
    let pax_compressed_size = std::fs::metadata(pax_tmp_compressed.path()).unwrap().len() as usize;

    let pax_text_size = pax_text.len();

    print_header();
    print_row("JSON", json_size, json_size);
    print_row("MessagePack", msgpack_size, json_size);
    print_row("CBOR", cbor_size, json_size);
    print_row("Bincode", bincode_size, json_size);
    print_row("Protobuf", protobuf_size, json_size);
    print_row("Pax Text", pax_text_size, json_size);
    print_row("Pax Binary", pax_size, json_size);
    print_row("Pax Compressed", pax_compressed_size, json_size);
}

fn report_mixed_types() {
    println!("\n--- Mixed Types ---\n");

    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "api".to_string());
    metadata.insert("version".to_string(), "v2".to_string());
    metadata.insert("region".to_string(), "us-west".to_string());

    let serde_data = MixedData {
        id: 12345678901234,
        name: "Test Record".to_string(),
        tags: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
        scores: vec![98.5, 87.3, 92.1, 88.8],
        metadata: metadata.clone(),
        active: true,
        raw_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };

    let proto_data = pb::MixedData {
        id: 12345678901234,
        name: "Test Record".to_string(),
        tags: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
        scores: vec![98.5, 87.3, 92.1, 88.8],
        metadata,
        active: true,
        raw_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };

    let pax_text = r#"record: {
    id: 12345678901234,
    name: "Test Record",
    tags: ["alpha", "beta", "gamma"],
    scores: [98.5, 87.3, 92.1, 88.8],
    metadata: {
        source: "api",
        version: "v2",
        region: "us-west"
    },
    active: true,
    raw_bytes: [222, 173, 190, 239]
}"#;

    let json_size = serde_json::to_vec(&serde_data).unwrap().len();
    let msgpack_size = rmp_serde::to_vec(&serde_data).unwrap().len();
    let cbor_size = {
        let mut buf = Vec::new();
        ciborium::into_writer(&serde_data, &mut buf).unwrap();
        buf.len()
    };
    let bincode_size = bincode::serialize(&serde_data).unwrap().len();
    let protobuf_size = proto_data.encode_to_vec().len();

    let pax_doc = pax::Pax::parse(pax_text).unwrap();
    let pax_tmp = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp.path(), false).unwrap();
    let pax_size = std::fs::metadata(pax_tmp.path()).unwrap().len() as usize;

    let pax_tmp_compressed = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp_compressed.path(), true).unwrap();
    let pax_compressed_size = std::fs::metadata(pax_tmp_compressed.path()).unwrap().len() as usize;

    let pax_text_size = pax_text.len();

    print_header();
    print_row("JSON", json_size, json_size);
    print_row("MessagePack", msgpack_size, json_size);
    print_row("CBOR", cbor_size, json_size);
    print_row("Bincode", bincode_size, json_size);
    print_row("Protobuf", protobuf_size, json_size);
    print_row("Pax Text", pax_text_size, json_size);
    print_row("Pax Binary", pax_size, json_size);
    print_row("Pax Compressed", pax_compressed_size, json_size);
}

fn report_tabular_users(count: usize) {
    println!("\n--- Tabular Users ({}) ---\n", count);

    let serde_data: Vec<User> = (0..count)
        .map(|i| User {
            id: i as i64,
            username: format!("user_{}", i),
            email: format!("user_{}@example.com", i),
            created_at: 1700000000 + (i as i64) * 1000,
            is_admin: i % 20 == 0,
        })
        .collect();

    let proto_data = pb::UserList {
        users: (0..count)
            .map(|i| pb::User {
                id: i as i64,
                username: format!("user_{}", i),
                email: format!("user_{}@example.com", i),
                created_at: 1700000000 + (i as i64) * 1000,
                is_admin: i % 20 == 0,
            })
            .collect(),
    };

    let mut pax_text = String::from(
        "@struct user (id: int64, username: string, email: string, created_at: int64, is_admin: bool)\n\
         users: @table user [\n",
    );
    for i in 0..count {
        let admin = if i % 20 == 0 { "true" } else { "false" };
        pax_text.push_str(&format!(
            "    ({}, \"user_{}\", \"user_{}@example.com\", {}, {}),\n",
            i,
            i,
            i,
            1700000000 + i * 1000,
            admin
        ));
    }
    pax_text.push_str("]\n");

    let json_size = serde_json::to_vec(&serde_data).unwrap().len();
    let msgpack_size = rmp_serde::to_vec(&serde_data).unwrap().len();
    let cbor_size = {
        let mut buf = Vec::new();
        ciborium::into_writer(&serde_data, &mut buf).unwrap();
        buf.len()
    };
    let bincode_size = bincode::serialize(&serde_data).unwrap().len();
    let protobuf_size = proto_data.encode_to_vec().len();

    let pax_doc = pax::Pax::parse(&pax_text).unwrap();
    let pax_tmp = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp.path(), false).unwrap();
    let pax_size = std::fs::metadata(pax_tmp.path()).unwrap().len() as usize;

    let pax_tmp_compressed = NamedTempFile::new().unwrap();
    pax_doc.compile(pax_tmp_compressed.path(), true).unwrap();
    let pax_compressed_size = std::fs::metadata(pax_tmp_compressed.path()).unwrap().len() as usize;

    let pax_text_size = pax_text.len();

    print_header();
    print_row("JSON", json_size, json_size);
    print_row("MessagePack", msgpack_size, json_size);
    print_row("CBOR", cbor_size, json_size);
    print_row("Bincode", bincode_size, json_size);
    print_row("Protobuf", protobuf_size, json_size);
    print_row("Pax Text", pax_text_size, json_size);
    print_row("Pax Binary", pax_size, json_size);
    print_row("Pax Compressed", pax_compressed_size, json_size);
}
