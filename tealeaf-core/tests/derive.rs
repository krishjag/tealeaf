//! Integration tests for the ToTeaLeaf/FromTeaLeaf derive macros.

use std::collections::HashMap;
use tealeaf::{FieldType, Schema, TeaLeaf, TeaLeafBuilder, Value};
use tealeaf::convert::{ConvertError, FromTeaLeaf, ToTeaLeaf, ToTeaLeafExt};
use tealeaf_derive::{FromTeaLeaf, ToTeaLeaf};

// =============================================================================
// Simple struct round-trip
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct SimpleUser {
    id: i64,
    name: String,
    active: bool,
}

#[test]
fn test_simple_struct_to_value() {
    let user = SimpleUser {
        id: 1,
        name: "Alice".into(),
        active: true,
    };
    let value = user.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("id").unwrap().as_int(), Some(1));
    assert_eq!(obj.get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(obj.get("active").unwrap().as_bool(), Some(true));
}

#[test]
fn test_simple_struct_from_value() {
    let user = SimpleUser {
        id: 1,
        name: "Alice".into(),
        active: true,
    };
    let value = user.to_tealeaf_value();
    let restored = SimpleUser::from_tealeaf_value(&value).unwrap();
    assert_eq!(user, restored);
}

#[test]
fn test_simple_struct_schema() {
    let schemas = SimpleUser::collect_schemas();
    let schema = schemas.get("SimpleUser").unwrap();
    assert_eq!(schema.name, "SimpleUser");
    assert_eq!(schema.fields.len(), 3);
    assert_eq!(schema.fields[0].name, "id");
    assert_eq!(schema.fields[0].field_type.base, "int64");
    assert_eq!(schema.fields[1].name, "name");
    assert_eq!(schema.fields[1].field_type.base, "string");
    assert_eq!(schema.fields[2].name, "active");
    assert_eq!(schema.fields[2].field_type.base, "bool");
}

#[test]
fn test_simple_struct_field_type() {
    let ft = SimpleUser::tealeaf_field_type();
    assert_eq!(ft.base, "SimpleUser");
    assert!(!ft.nullable);
    assert!(!ft.is_array);
}

// =============================================================================
// Nested struct
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct Address {
    city: String,
    zip: String,
}

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct Person {
    name: String,
    home: Address,
}

#[test]
fn test_nested_struct_roundtrip() {
    let person = Person {
        name: "Bob".into(),
        home: Address {
            city: "Berlin".into(),
            zip: "10115".into(),
        },
    };
    let value = person.to_tealeaf_value();
    let restored = Person::from_tealeaf_value(&value).unwrap();
    assert_eq!(person, restored);
}

#[test]
fn test_nested_struct_schemas() {
    let schemas = Person::collect_schemas();
    assert!(schemas.contains_key("Address"));
    assert!(schemas.contains_key("Person"));
    let person_schema = schemas.get("Person").unwrap();
    let home_field = person_schema.get_field("home").unwrap();
    assert_eq!(home_field.field_type.base, "Address");
}

// =============================================================================
// Optional fields
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct Profile {
    name: String,
    bio: Option<String>,
    age: Option<i32>,
}

#[test]
fn test_optional_fields_some() {
    let profile = Profile {
        name: "Alice".into(),
        bio: Some("Hello world".into()),
        age: Some(30),
    };
    let value = profile.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("bio").unwrap().as_str(), Some("Hello world"));
    assert_eq!(obj.get("age").unwrap().as_int(), Some(30));

    let restored = Profile::from_tealeaf_value(&value).unwrap();
    assert_eq!(profile, restored);
}

#[test]
fn test_optional_fields_none() {
    let profile = Profile {
        name: "Bob".into(),
        bio: None,
        age: None,
    };
    let value = profile.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert!(obj.get("bio").unwrap().is_null());
    assert!(obj.get("age").unwrap().is_null());

    let restored = Profile::from_tealeaf_value(&value).unwrap();
    assert_eq!(profile, restored);
}

#[test]
fn test_optional_field_schema() {
    let schemas = Profile::collect_schemas();
    let schema = schemas.get("Profile").unwrap();
    let bio_field = schema.get_field("bio").unwrap();
    assert!(bio_field.field_type.nullable);
    assert_eq!(bio_field.field_type.base, "string");
}

// =============================================================================
// Rename attribute
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(rename = "usr")]
struct RenamedUser {
    #[tealeaf(rename = "user_id")]
    id: i64,
    name: String,
}

#[test]
fn test_rename_container() {
    let schemas = RenamedUser::collect_schemas();
    assert!(schemas.contains_key("usr"));
    assert!(!schemas.contains_key("RenamedUser"));
    assert_eq!(RenamedUser::tealeaf_field_type().base, "usr");
}

#[test]
fn test_rename_field() {
    let user = RenamedUser {
        id: 42,
        name: "Test".into(),
    };
    let value = user.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert!(obj.contains_key("user_id"));
    assert!(!obj.contains_key("id"));
    assert_eq!(obj.get("user_id").unwrap().as_int(), Some(42));
}

#[test]
fn test_rename_field_roundtrip() {
    let user = RenamedUser {
        id: 42,
        name: "Test".into(),
    };
    let value = user.to_tealeaf_value();
    let restored = RenamedUser::from_tealeaf_value(&value).unwrap();
    assert_eq!(user, restored);
}

// =============================================================================
// Skip attribute
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct WithSkipped {
    name: String,
    #[tealeaf(skip)]
    internal: String,
}

#[test]
fn test_skip_field() {
    let val = WithSkipped {
        name: "test".into(),
        internal: "secret".into(),
    };
    let value = val.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert!(!obj.contains_key("internal"));
    assert!(obj.contains_key("name"));
}

#[test]
fn test_skip_field_from() {
    let mut obj = HashMap::new();
    obj.insert("name".to_string(), Value::String("test".into()));
    let value = Value::Object(obj);
    let restored = WithSkipped::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored.name, "test");
    assert_eq!(restored.internal, ""); // Default
}

// =============================================================================
// Type override
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct EventLog {
    name: String,
    #[tealeaf(type = "timestamp")]
    created_at: i64,
}

#[test]
fn test_type_override_timestamp() {
    let event = EventLog {
        name: "deploy".into(),
        created_at: 1700000000000,
    };
    let value = event.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert!(matches!(
        obj.get("created_at"),
        Some(Value::Timestamp(1700000000000))
    ));
}

#[test]
fn test_type_override_schema() {
    let schemas = EventLog::collect_schemas();
    let schema = schemas.get("EventLog").unwrap();
    let ts_field = schema.get_field("created_at").unwrap();
    assert_eq!(ts_field.field_type.base, "timestamp");
}

#[test]
fn test_type_override_roundtrip() {
    let event = EventLog {
        name: "deploy".into(),
        created_at: 1700000000000,
    };
    let value = event.to_tealeaf_value();
    let restored = EventLog::from_tealeaf_value(&value).unwrap();
    assert_eq!(event, restored);
}

// =============================================================================
// Enum support
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    Point,
}

#[test]
fn test_enum_struct_variant() {
    let shape = Shape::Circle { radius: 5.0 };
    let value = shape.to_tealeaf_value();
    if let Value::Tagged(tag, inner) = &value {
        assert_eq!(tag, "Circle");
        let obj = inner.as_object().unwrap();
        assert_eq!(obj.get("radius").unwrap().as_float(), Some(5.0));
    } else {
        panic!("Expected Tagged value");
    }
}

#[test]
fn test_enum_unit_variant() {
    let shape = Shape::Point;
    let value = shape.to_tealeaf_value();
    if let Value::Tagged(tag, inner) = &value {
        assert_eq!(tag, "Point");
        assert!(inner.is_null());
    } else {
        panic!("Expected Tagged value");
    }
}

#[test]
fn test_enum_roundtrip() {
    let shapes = vec![
        Shape::Circle { radius: 5.0 },
        Shape::Rectangle {
            width: 10.0,
            height: 20.0,
        },
        Shape::Point,
    ];

    for shape in &shapes {
        let value = shape.to_tealeaf_value();
        let restored = Shape::from_tealeaf_value(&value).unwrap();
        assert_eq!(shape, &restored);
    }
}

// =============================================================================
// Enum with tuple variants
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
enum Message {
    Text(String),
    Number(i64),
    Pair(String, i64),
    Empty,
}

#[test]
fn test_enum_single_tuple_variant() {
    let msg = Message::Text("hello".into());
    let value = msg.to_tealeaf_value();
    let restored = Message::from_tealeaf_value(&value).unwrap();
    assert_eq!(msg, restored);
}

#[test]
fn test_enum_multi_tuple_variant() {
    let msg = Message::Pair("key".into(), 42);
    let value = msg.to_tealeaf_value();
    let restored = Message::from_tealeaf_value(&value).unwrap();
    assert_eq!(msg, restored);
}

// =============================================================================
// Document creation convenience methods
// =============================================================================

#[test]
fn test_from_dto() {
    let user = SimpleUser {
        id: 1,
        name: "Alice".into(),
        active: true,
    };
    let doc = TeaLeaf::from_dto("user", &user);
    assert!(doc.schema("SimpleUser").is_some());
    assert!(doc.get("user").is_some());
    let restored: SimpleUser = doc.to_dto("user").unwrap();
    assert_eq!(user, restored);
}

#[test]
fn test_from_dto_array() {
    let users = vec![
        SimpleUser {
            id: 1,
            name: "Alice".into(),
            active: true,
        },
        SimpleUser {
            id: 2,
            name: "Bob".into(),
            active: false,
        },
    ];
    let doc = TeaLeaf::from_dto_array("users", &users);
    assert!(doc.schema("SimpleUser").is_some());
    let restored: Vec<SimpleUser> = doc.to_dto_vec("users").unwrap();
    assert_eq!(users, restored);
}

#[test]
fn test_builder_with_dtos() {
    let users = vec![
        SimpleUser {
            id: 1,
            name: "Alice".into(),
            active: true,
        },
    ];
    let addr = Address {
        city: "NYC".into(),
        zip: "10001".into(),
    };

    let doc = TeaLeafBuilder::new()
        .add_vec("users", &users)
        .add("office", &addr)
        .build();

    assert!(doc.schema("SimpleUser").is_some());
    assert!(doc.schema("Address").is_some());
    assert!(doc.get("users").is_some());
    assert!(doc.get("office").is_some());
}

// =============================================================================
// Extension trait methods
// =============================================================================

#[test]
fn test_to_tl_string() {
    let user = SimpleUser {
        id: 1,
        name: "Alice".into(),
        active: true,
    };
    let tl = user.to_tl_string("user");
    assert!(tl.contains("@struct SimpleUser"));
    assert!(tl.contains("user:"));
}

#[test]
fn test_to_tealeaf_json() {
    let user = SimpleUser {
        id: 1,
        name: "Alice".into(),
        active: true,
    };
    let json = user.to_tealeaf_json("user").unwrap();
    assert!(json.contains("\"name\": \"Alice\""));
    assert!(json.contains("\"id\": 1"));
}

// =============================================================================
// Default attribute
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct WithDefaults {
    name: String,
    #[tealeaf(default)]
    count: i32,
}

#[test]
fn test_default_field() {
    let mut obj = HashMap::new();
    obj.insert("name".to_string(), Value::String("test".into()));
    // count is missing
    let value = Value::Object(obj);
    let restored = WithDefaults::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored.name, "test");
    assert_eq!(restored.count, 0); // i32 default
}

// =============================================================================
// Vec fields
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct WithVec {
    tags: Vec<String>,
    scores: Vec<i32>,
}

#[test]
fn test_vec_fields_roundtrip() {
    let val = WithVec {
        tags: vec!["rust".into(), "tealeaf".into()],
        scores: vec![100, 200, 300],
    };
    let value = val.to_tealeaf_value();
    let restored = WithVec::from_tealeaf_value(&value).unwrap();
    assert_eq!(val, restored);
}

// =============================================================================
// HashMap fields
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct WithMap {
    name: String,
    metadata: HashMap<String, String>,
}

#[test]
fn test_hashmap_field_roundtrip() {
    let mut metadata = HashMap::new();
    metadata.insert("version".to_string(), "1.0".to_string());
    metadata.insert("author".to_string(), "Alice".to_string());

    let val = WithMap {
        name: "test".into(),
        metadata,
    };
    let value = val.to_tealeaf_value();
    let restored = WithMap::from_tealeaf_value(&value).unwrap();
    assert_eq!(val, restored);
}

// =============================================================================
// Error cases
// =============================================================================

#[test]
fn test_missing_field_error() {
    let mut obj = HashMap::new();
    obj.insert("name".to_string(), Value::String("test".into()));
    // id is missing (required)
    let value = Value::Object(obj);
    let err = SimpleUser::from_tealeaf_value(&value).unwrap_err();
    match err {
        ConvertError::MissingField { field, .. } => assert_eq!(field, "id"),
        _ => panic!("Expected MissingField, got {:?}", err),
    }
}

#[test]
fn test_type_mismatch_error() {
    let err = SimpleUser::from_tealeaf_value(&Value::Int(42)).unwrap_err();
    assert!(matches!(err, ConvertError::TypeMismatch { .. }));
}

#[test]
fn test_unknown_enum_variant() {
    let value = Value::Tagged("Unknown".into(), Box::new(Value::Null));
    let err = Shape::from_tealeaf_value(&value).unwrap_err();
    match err {
        ConvertError::Custom(msg) => assert!(msg.contains("Unknown Shape variant")),
        _ => panic!("Expected Custom error, got {:?}", err),
    }
}
