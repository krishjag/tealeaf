use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Scenario 1: Small Object (Config-like data)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SmallConfig {
    pub name: String,
    pub version: i32,
    pub enabled: bool,
    pub threshold: f64,
}

// Scenario 2: Large Array
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// Scenario 3: Nested Structs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub zip: String,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Company {
    pub name: String,
    pub address: Address,
    pub employee_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: i32,
    pub employer: Company,
}

// Scenario 4: Mixed Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MixedData {
    pub id: i64,
    pub name: String,
    pub tags: Vec<String>,
    pub scores: Vec<f64>,
    pub metadata: HashMap<String, String>,
    pub active: bool,
    #[serde(with = "serde_bytes")]
    pub raw_bytes: Vec<u8>,
}

mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer)
    }
}

// Scenario 5: Tabular Data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: i64,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Product {
    pub sku: String,
    pub name: String,
    pub price: f64,
    pub quantity: i32,
    pub category: String,
}
