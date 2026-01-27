use super::structs::*;
use std::collections::HashMap;

// Import generated protobuf types
use crate::proto::benchmark as pb;

// ============================================================================
// Scenario 1: Small Object
// ============================================================================

pub fn small_config_pax_text() -> &'static str {
    r#"config: {
    name: "my-service",
    version: 42,
    enabled: true,
    threshold: 0.85
}"#
}

pub fn small_config_struct() -> SmallConfig {
    SmallConfig {
        name: "my-service".to_string(),
        version: 42,
        enabled: true,
        threshold: 0.85,
    }
}

pub fn small_config_proto() -> pb::SmallConfig {
    pb::SmallConfig {
        name: "my-service".to_string(),
        version: 42,
        enabled: true,
        threshold: 0.85,
    }
}

// ============================================================================
// Scenario 2: Large Array
// ============================================================================

pub fn large_array_pax_text(count: usize) -> String {
    let mut text = String::from("@struct point (x: float, y: float, z: float)\npoints: @table point [\n");
    for i in 0..count {
        let x = i as f64 * 0.1;
        let y = i as f64 * 0.2;
        let z = i as f64 * 0.3;
        text.push_str(&format!("    ({}, {}, {}),\n", x, y, z));
    }
    text.push_str("]\n");
    text
}

pub fn large_array_structs(count: usize) -> Vec<Point> {
    (0..count)
        .map(|i| Point {
            x: i as f64 * 0.1,
            y: i as f64 * 0.2,
            z: i as f64 * 0.3,
        })
        .collect()
}

pub fn large_array_proto(count: usize) -> pb::PointList {
    pb::PointList {
        points: (0..count)
            .map(|i| pb::Point {
                x: i as f64 * 0.1,
                y: i as f64 * 0.2,
                z: i as f64 * 0.3,
            })
            .collect(),
    }
}

// ============================================================================
// Scenario 3: Nested Structs
// ============================================================================

pub fn nested_struct_pax_text() -> &'static str {
    r#"@struct address (street: string, city: string, zip: string, country: string)
@struct company (name: string, address: address, employee_count: int)
@struct person (id: int64, name: string, email: string, age: int, employer: company)

people: @table person [
    (1, "Alice", "alice@example.com", 30, ("TechCorp", ("123 Main St", "San Francisco", "94102", "USA"), 500)),
    (2, "Bob", "bob@example.com", 25, ("DataInc", ("456 Oak Ave", "New York", "10001", "USA"), 1200)),
]"#
}

pub fn nested_struct_pax_text_scaled(count: usize) -> String {
    let mut text = String::from(
        r#"@struct address (street: string, city: string, zip: string, country: string)
@struct company (name: string, address: address, employee_count: int)
@struct person (id: int64, name: string, email: string, age: int, employer: company)

people: @table person [
"#,
    );
    for i in 0..count {
        let city = if i % 2 == 0 { "San Francisco" } else { "New York" };
        let zip = if i % 2 == 0 { "94102" } else { "10001" };
        text.push_str(&format!(
            "    ({}, \"User{}\", \"user{}@example.com\", {}, (\"Company{}\", (\"{} Main St\", \"{}\", \"{}\", \"USA\"), {})),\n",
            i, i, i, 20 + (i % 50), i, i, city, zip, 100 + i * 10
        ));
    }
    text.push_str("]\n");
    text
}

pub fn nested_struct_list() -> Vec<Person> {
    vec![
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
    ]
}

pub fn nested_struct_list_scaled(count: usize) -> Vec<Person> {
    (0..count)
        .map(|i| {
            let city = if i % 2 == 0 {
                "San Francisco"
            } else {
                "New York"
            };
            let zip = if i % 2 == 0 { "94102" } else { "10001" };
            Person {
                id: i as i64,
                name: format!("User{}", i),
                email: format!("user{}@example.com", i),
                age: 20 + (i % 50) as i32,
                employer: Company {
                    name: format!("Company{}", i),
                    address: Address {
                        street: format!("{} Main St", i),
                        city: city.to_string(),
                        zip: zip.to_string(),
                        country: "USA".to_string(),
                    },
                    employee_count: (100 + i * 10) as i32,
                },
            }
        })
        .collect()
}

pub fn nested_struct_proto() -> pb::PersonList {
    pb::PersonList {
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
    }
}

pub fn nested_struct_proto_scaled(count: usize) -> pb::PersonList {
    pb::PersonList {
        people: (0..count)
            .map(|i| {
                let city = if i % 2 == 0 { "San Francisco" } else { "New York" };
                let zip = if i % 2 == 0 { "94102" } else { "10001" };
                pb::Person {
                    id: i as i64,
                    name: format!("User{}", i),
                    email: format!("user{}@example.com", i),
                    age: 20 + (i % 50) as i32,
                    employer: Some(pb::Company {
                        name: format!("Company{}", i),
                        address: Some(pb::Address {
                            street: format!("{} Main St", i),
                            city: city.to_string(),
                            zip: zip.to_string(),
                            country: "USA".to_string(),
                        }),
                        employee_count: (100 + i * 10) as i32,
                    }),
                }
            })
            .collect(),
    }
}

// ============================================================================
// Scenario 4: Mixed Types
// ============================================================================

pub fn mixed_types_pax_text() -> &'static str {
    r#"record: {
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
}"#
}

pub fn mixed_types_struct() -> MixedData {
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "api".to_string());
    metadata.insert("version".to_string(), "v2".to_string());
    metadata.insert("region".to_string(), "us-west".to_string());

    MixedData {
        id: 12345678901234,
        name: "Test Record".to_string(),
        tags: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
        scores: vec![98.5, 87.3, 92.1, 88.8],
        metadata,
        active: true,
        raw_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
    }
}

pub fn mixed_types_proto() -> pb::MixedData {
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "api".to_string());
    metadata.insert("version".to_string(), "v2".to_string());
    metadata.insert("region".to_string(), "us-west".to_string());

    pb::MixedData {
        id: 12345678901234,
        name: "Test Record".to_string(),
        tags: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
        scores: vec![98.5, 87.3, 92.1, 88.8],
        metadata,
        active: true,
        raw_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
    }
}

// ============================================================================
// Scenario 5: Tabular Data
// ============================================================================

pub fn tabular_users_pax_text(count: usize) -> String {
    let mut text = String::from(
        "@struct user (id: int64, username: string, email: string, created_at: int64, is_admin: bool)\n\
         users: @table user [\n",
    );
    for i in 0..count {
        let admin = if i % 20 == 0 { "true" } else { "false" };
        text.push_str(&format!(
            "    ({}, \"user_{}\", \"user_{}@example.com\", {}, {}),\n",
            i,
            i,
            i,
            1700000000 + i * 1000,
            admin
        ));
    }
    text.push_str("]\n");
    text
}

pub fn tabular_users_structs(count: usize) -> Vec<User> {
    (0..count)
        .map(|i| User {
            id: i as i64,
            username: format!("user_{}", i),
            email: format!("user_{}@example.com", i),
            created_at: 1700000000 + (i as i64) * 1000,
            is_admin: i % 20 == 0,
        })
        .collect()
}

pub fn tabular_users_proto(count: usize) -> pb::UserList {
    pb::UserList {
        users: (0..count)
            .map(|i| pb::User {
                id: i as i64,
                username: format!("user_{}", i),
                email: format!("user_{}@example.com", i),
                created_at: 1700000000 + (i as i64) * 1000,
                is_admin: i % 20 == 0,
            })
            .collect(),
    }
}
