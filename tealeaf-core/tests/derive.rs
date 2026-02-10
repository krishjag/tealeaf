//! Integration tests for the ToTeaLeaf/FromTeaLeaf derive macros.

use std::collections::HashMap;
use tealeaf::{FieldType, ObjectMap, TeaLeaf, TeaLeafBuilder, Value};
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
    let mut obj = ObjectMap::new();
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
        Some(Value::Timestamp(1700000000000, 0))
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
    let mut obj = ObjectMap::new();
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
    let mut obj = ObjectMap::new();
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

// =============================================================================
// Convert coverage: HashMap variants
// =============================================================================

#[test]
fn test_hashmap_string_vec_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let mut map: HashMap<String, Vec<i32>> = HashMap::new();
    map.insert("primes".to_string(), vec![2, 3, 5, 7]);
    map.insert("evens".to_string(), vec![2, 4, 6]);

    let val = map.to_tealeaf_value();
    let restored = HashMap::<String, Vec<i32>>::from_tealeaf_value(&val).unwrap();
    assert_eq!(restored.get("primes").unwrap(), &vec![2, 3, 5, 7]);
    assert_eq!(restored.get("evens").unwrap(), &vec![2, 4, 6]);
}

#[test]
fn test_hashmap_string_option_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let mut map: HashMap<String, Option<String>> = HashMap::new();
    map.insert("present".to_string(), Some("value".to_string()));
    map.insert("absent".to_string(), None);

    let val = map.to_tealeaf_value();
    let restored = HashMap::<String, Option<String>>::from_tealeaf_value(&val).unwrap();
    assert_eq!(restored.get("present").unwrap(), &Some("value".to_string()));
    assert_eq!(restored.get("absent").unwrap(), &None);
}

#[test]
fn test_hashmap_from_non_object_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = HashMap::<String, i64>::from_tealeaf_value(&Value::Int(42)).unwrap_err();
    match err {
        ConvertError::TypeMismatch { expected, .. } => assert_eq!(expected, "object"),
        _ => panic!("Expected TypeMismatch, got {:?}", err),
    }
}

#[test]
fn test_hashmap_nested_error_propagation() {
    use tealeaf::convert::FromTeaLeaf;
    let mut obj = ObjectMap::new();
    obj.insert("key".to_string(), Value::String("not_an_int".into()));
    let val = Value::Object(obj);
    let err = HashMap::<String, i64>::from_tealeaf_value(&val).unwrap_err();
    match err {
        ConvertError::Nested { path, .. } => assert_eq!(path, "key"),
        _ => panic!("Expected Nested, got {:?}", err),
    }
}

#[test]
fn test_hashmap_field_type() {
    use tealeaf::convert::ToTeaLeaf;
    let ft = HashMap::<String, i64>::tealeaf_field_type();
    assert_eq!(ft.base, "object");
}

#[test]
fn test_hashmap_collect_schemas() {
    use tealeaf::convert::ToTeaLeaf;
    let schemas = HashMap::<String, SimpleUser>::collect_schemas();
    assert!(schemas.contains_key("SimpleUser"));
}

// =============================================================================
// Convert coverage: Transparent wrappers (Arc, Rc)
// =============================================================================

#[test]
fn test_arc_string_roundtrip() {
    use std::sync::Arc;
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let arc_val = Arc::new("hello".to_string());
    let val = arc_val.to_tealeaf_value();
    assert_eq!(val, Value::String("hello".into()));
    let restored = Arc::<String>::from_tealeaf_value(&val).unwrap();
    assert_eq!(*restored, "hello");
}

#[test]
fn test_rc_i64_roundtrip() {
    use std::rc::Rc;
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let rc_val = Rc::new(42i64);
    let val = rc_val.to_tealeaf_value();
    assert_eq!(val, Value::Int(42));
    let restored = Rc::<i64>::from_tealeaf_value(&val).unwrap();
    assert_eq!(*restored, 42);
}

#[test]
fn test_box_vec_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let boxed = Box::new(vec![1i32, 2, 3]);
    let val = boxed.to_tealeaf_value();
    let restored = Box::<Vec<i32>>::from_tealeaf_value(&val).unwrap();
    assert_eq!(*restored, vec![1, 2, 3]);
}

#[test]
fn test_arc_collect_schemas() {
    use std::sync::Arc;
    use tealeaf::convert::ToTeaLeaf;
    let schemas = Arc::<SimpleUser>::collect_schemas();
    assert!(schemas.contains_key("SimpleUser"));
}

#[test]
fn test_rc_collect_schemas() {
    use std::rc::Rc;
    use tealeaf::convert::ToTeaLeaf;
    let schemas = Rc::<SimpleUser>::collect_schemas();
    assert!(schemas.contains_key("SimpleUser"));
}

#[test]
fn test_arc_field_type() {
    use std::sync::Arc;
    use tealeaf::convert::ToTeaLeaf;
    assert_eq!(Arc::<i64>::tealeaf_field_type(), FieldType::new("int64"));
}

#[test]
fn test_rc_field_type() {
    use std::rc::Rc;
    use tealeaf::convert::ToTeaLeaf;
    assert_eq!(Rc::<String>::tealeaf_field_type(), FieldType::new("string"));
}

// =============================================================================
// Convert coverage: Tuple sizes 3-6
// =============================================================================

#[test]
fn test_tuple3_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let t = (1i64, "hello".to_string(), true);
    let val = t.to_tealeaf_value();
    assert_eq!(
        val,
        Value::Array(vec![
            Value::Int(1),
            Value::String("hello".into()),
            Value::Bool(true)
        ])
    );
    let restored = <(i64, String, bool)>::from_tealeaf_value(&val).unwrap();
    assert_eq!(restored, (1, "hello".to_string(), true));
}

#[test]
fn test_tuple4_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let t = (1i64, 2u64, 3.0f64, "four".to_string());
    let val = t.to_tealeaf_value();
    let restored = <(i64, u64, f64, String)>::from_tealeaf_value(&val).unwrap();
    assert_eq!(restored, (1, 2, 3.0, "four".to_string()));
}

#[test]
fn test_tuple5_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let t = (1i64, 2u64, 3.0f64, "four".to_string(), true);
    let val = t.to_tealeaf_value();
    let restored = <(i64, u64, f64, String, bool)>::from_tealeaf_value(&val).unwrap();
    assert_eq!(restored, (1, 2, 3.0, "four".to_string(), true));
}

#[test]
fn test_tuple6_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let t = (1i64, 2u64, 3.0f64, "four".to_string(), true, 6i32);
    let val = t.to_tealeaf_value();
    let restored = <(i64, u64, f64, String, bool, i32)>::from_tealeaf_value(&val).unwrap();
    assert_eq!(restored, (1, 2, 3.0, "four".to_string(), true, 6));
}

#[test]
fn test_tuple_collect_schemas() {
    use tealeaf::convert::ToTeaLeaf;
    let schemas = <(i64, String)>::collect_schemas();
    assert!(schemas.is_empty());
}

#[test]
fn test_tuple_field_type() {
    use tealeaf::convert::ToTeaLeaf;
    let ft = <(i64, String)>::tealeaf_field_type();
    assert_eq!(ft.base, "tuple");
}

// =============================================================================
// Convert coverage: Tuple error paths
// =============================================================================

#[test]
fn test_tuple_from_non_array_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = <(i64, String)>::from_tealeaf_value(&Value::Int(42)).unwrap_err();
    match err {
        ConvertError::TypeMismatch { expected, .. } => assert_eq!(expected, "tuple (array)"),
        _ => panic!("Expected TypeMismatch, got {:?}", err),
    }
}

#[test]
fn test_tuple_from_short_array_error() {
    use tealeaf::convert::FromTeaLeaf;
    let val = Value::Array(vec![Value::Int(1)]);
    let err = <(i64, String)>::from_tealeaf_value(&val).unwrap_err();
    match err {
        ConvertError::MissingField { struct_name, field } => {
            assert_eq!(struct_name, "tuple");
            assert_eq!(field, "index 1");
        }
        _ => panic!("Expected MissingField, got {:?}", err),
    }
}

#[test]
fn test_tuple_nested_type_error() {
    use tealeaf::convert::FromTeaLeaf;
    let val = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let err = <(i64, String)>::from_tealeaf_value(&val).unwrap_err();
    match err {
        ConvertError::Nested { path, .. } => assert_eq!(path, "[1]"),
        _ => panic!("Expected Nested, got {:?}", err),
    }
}

// =============================================================================
// Convert coverage: Vec/Option error paths
// =============================================================================

#[test]
fn test_vec_from_non_array_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = Vec::<i64>::from_tealeaf_value(&Value::String("nope".into())).unwrap_err();
    match err {
        ConvertError::TypeMismatch { expected, .. } => assert_eq!(expected, "array"),
        _ => panic!("Expected TypeMismatch, got {:?}", err),
    }
}

#[test]
fn test_vec_nested_element_error() {
    use tealeaf::convert::FromTeaLeaf;
    let val = Value::Array(vec![Value::Int(1), Value::String("bad".into())]);
    let err = Vec::<i64>::from_tealeaf_value(&val).unwrap_err();
    match err {
        ConvertError::Nested { path, .. } => assert_eq!(path, "[1]"),
        _ => panic!("Expected Nested, got {:?}", err),
    }
}

#[test]
fn test_option_propagates_inner_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = Option::<i64>::from_tealeaf_value(&Value::String("nope".into())).unwrap_err();
    match err {
        ConvertError::TypeMismatch { expected, .. } => assert_eq!(expected, "int64"),
        _ => panic!("Expected TypeMismatch, got {:?}", err),
    }
}

// =============================================================================
// Convert coverage: ConvertError Display (Nested, Custom) + Error conversion
// =============================================================================

#[test]
fn test_convert_error_nested_display() {
    let inner = ConvertError::TypeMismatch {
        expected: "int".into(),
        got: "String".into(),
        path: "".into(),
    };
    let err = ConvertError::Nested {
        path: "users[0].age".into(),
        source: Box::new(inner),
    };
    let msg = err.to_string();
    assert!(msg.contains("At 'users[0].age'"));
    assert!(msg.contains("Type mismatch"));
}

#[test]
fn test_convert_error_custom_display() {
    let err = ConvertError::Custom("something went wrong".into());
    assert_eq!(err.to_string(), "something went wrong");
}

#[test]
fn test_convert_error_to_tealeaf_error() {
    let convert_err = ConvertError::Custom("test error".into());
    let tl_err: tealeaf::Error = convert_err.into();
    let msg = tl_err.to_string();
    assert!(msg.contains("test error"));
}

// =============================================================================
// Convert coverage: Additional primitive type coverage
// =============================================================================

#[test]
fn test_i8_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let val = 42i8;
    let tl = val.to_tealeaf_value();
    assert_eq!(tl, Value::Int(42));
    assert_eq!(i8::from_tealeaf_value(&tl).unwrap(), 42);
    assert_eq!(i8::tealeaf_field_type().base, "int8");
}

#[test]
fn test_i16_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let val = 1000i16;
    let tl = val.to_tealeaf_value();
    assert_eq!(tl, Value::Int(1000));
    assert_eq!(i16::from_tealeaf_value(&tl).unwrap(), 1000);
    assert_eq!(i16::tealeaf_field_type().base, "int16");
}

#[test]
fn test_i32_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let val = 100_000i32;
    let tl = val.to_tealeaf_value();
    assert_eq!(tl, Value::Int(100_000));
    assert_eq!(i32::from_tealeaf_value(&tl).unwrap(), 100_000);
    assert_eq!(i32::tealeaf_field_type().base, "int");
}

#[test]
fn test_u8_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let val = 200u8;
    let tl = val.to_tealeaf_value();
    assert_eq!(tl, Value::UInt(200));
    assert_eq!(u8::from_tealeaf_value(&tl).unwrap(), 200);
    assert_eq!(u8::tealeaf_field_type().base, "uint8");
}

#[test]
fn test_u16_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let val = 50000u16;
    let tl = val.to_tealeaf_value();
    assert_eq!(tl, Value::UInt(50000));
    assert_eq!(u16::from_tealeaf_value(&tl).unwrap(), 50000);
    assert_eq!(u16::tealeaf_field_type().base, "uint16");
}

#[test]
fn test_u32_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let val = 3_000_000u32;
    let tl = val.to_tealeaf_value();
    assert_eq!(tl, Value::UInt(3_000_000));
    assert_eq!(u32::from_tealeaf_value(&tl).unwrap(), 3_000_000);
    assert_eq!(u32::tealeaf_field_type().base, "uint");
}

#[test]
fn test_u64_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let val = u64::MAX;
    let tl = val.to_tealeaf_value();
    assert_eq!(tl, Value::UInt(u64::MAX));
    assert_eq!(u64::from_tealeaf_value(&tl).unwrap(), u64::MAX);
    assert_eq!(u64::tealeaf_field_type().base, "uint64");
}

#[test]
fn test_f32_roundtrip() {
    use tealeaf::convert::{ToTeaLeaf, FromTeaLeaf};
    let val = 1.5f32;
    let tl = val.to_tealeaf_value();
    // f32 is widened to f64
    assert_eq!(tl, Value::Float(1.5));
    assert_eq!(f32::from_tealeaf_value(&tl).unwrap(), 1.5);
    assert_eq!(f32::tealeaf_field_type().base, "float32");
}

#[test]
fn test_bool_from_non_bool_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = bool::from_tealeaf_value(&Value::Int(1)).unwrap_err();
    assert!(matches!(err, ConvertError::TypeMismatch { .. }));
}

#[test]
fn test_string_from_non_string_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = String::from_tealeaf_value(&Value::Int(42)).unwrap_err();
    assert!(matches!(err, ConvertError::TypeMismatch { .. }));
}

#[test]
fn test_f32_from_non_float_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = f32::from_tealeaf_value(&Value::Bool(true)).unwrap_err();
    assert!(matches!(err, ConvertError::TypeMismatch { .. }));
}

#[test]
fn test_u64_from_non_uint_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = u64::from_tealeaf_value(&Value::String("no".into())).unwrap_err();
    assert!(matches!(err, ConvertError::TypeMismatch { .. }));
}

#[test]
fn test_vec_u8_from_non_bytes_error() {
    use tealeaf::convert::FromTeaLeaf;
    let err = Vec::<u8>::from_tealeaf_value(&Value::Int(42)).unwrap_err();
    assert!(matches!(err, ConvertError::TypeMismatch { .. }));
}

#[test]
fn test_str_to_tealeaf() {
    use tealeaf::convert::ToTeaLeaf;
    let val = "world".to_tealeaf_value();
    assert_eq!(val, Value::String("world".into()));
    assert_eq!(<&str>::tealeaf_field_type().base, "string");
}

#[test]
fn test_option_collect_schemas() {
    use tealeaf::convert::ToTeaLeaf;
    let schemas = Option::<SimpleUser>::collect_schemas();
    assert!(schemas.contains_key("SimpleUser"));
}

#[test]
fn test_vec_collect_schemas() {
    use tealeaf::convert::ToTeaLeaf;
    // Vec<String> uses the generic Vec<T: NotU8> impl
    let schemas = Vec::<String>::collect_schemas();
    assert!(schemas.is_empty()); // primitives have no schemas
}

#[test]
fn test_vec_field_type() {
    use tealeaf::convert::ToTeaLeaf;
    let ft = Vec::<i32>::tealeaf_field_type();
    assert_eq!(ft.base, "int");
    assert!(ft.is_array);
}

// =============================================================================
// Phase 4: Flatten attribute tests
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct PersonWithAddress {
    name: String,
    #[tealeaf(flatten)]
    address: Address,
}

#[test]
fn test_flatten_to_value() {
    let person = PersonWithAddress {
        name: "Alice".into(),
        address: Address {
            city: "Boston".into(),
            zip: "02101".into(),
        },
    };
    let value = person.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    // Flattened fields should be merged into parent
    assert_eq!(obj.get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(obj.get("city").unwrap().as_str(), Some("Boston"));
    assert_eq!(obj.get("zip").unwrap().as_str(), Some("02101"));
    // "address" key should NOT exist
    assert!(obj.get("address").is_none(), "Flattened field should not appear as key");
}

#[test]
fn test_flatten_from_value() {
    // Build a flat object (as flatten produces)
    let mut obj = ObjectMap::new();
    obj.insert("name".to_string(), Value::String("Alice".into()));
    obj.insert("city".to_string(), Value::String("Boston".into()));
    obj.insert("zip".to_string(), Value::String("02101".into()));

    let person = PersonWithAddress::from_tealeaf_value(&Value::Object(obj)).unwrap();
    assert_eq!(person.name, "Alice");
    assert_eq!(person.address.city, "Boston");
    assert_eq!(person.address.zip, "02101");
}

#[test]
fn test_flatten_roundtrip() {
    let person = PersonWithAddress {
        name: "Bob".into(),
        address: Address {
            city: "Paris".into(),
            zip: "75001".into(),
        },
    };
    let value = person.to_tealeaf_value();
    let restored = PersonWithAddress::from_tealeaf_value(&value).unwrap();
    assert_eq!(person, restored);
}

#[test]
fn test_flatten_schema() {
    let schemas = PersonWithAddress::collect_schemas();
    // Should have schemas for both PersonWithAddress and Address
    assert!(schemas.get("PersonWithAddress").is_some(), "Should have parent schema");
    assert!(schemas.get("Address").is_some(), "Should have flattened type schema");
}

// =============================================================================
// Phase 4: Default expression attribute tests
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct ConfigWithDefaults {
    name: String,
    #[tealeaf(default = "42")]
    port: i64,
    #[tealeaf(default = "String::from(\"localhost\")")]
    host: String,
    #[tealeaf(default)]
    retries: i64,
}

#[test]
fn test_default_expr_from_missing_fields() {
    // Only provide "name", other fields should use defaults
    let mut obj = ObjectMap::new();
    obj.insert("name".to_string(), Value::String("myapp".into()));

    let config = ConfigWithDefaults::from_tealeaf_value(&Value::Object(obj)).unwrap();
    assert_eq!(config.name, "myapp");
    assert_eq!(config.port, 42, "Should use default expression '42'");
    assert_eq!(config.host, "localhost", "Should use default expression 'String::from(\"localhost\")'");
    assert_eq!(config.retries, 0, "Should use Default::default() for i64");
}

#[test]
fn test_default_expr_from_null_fields() {
    let mut obj = ObjectMap::new();
    obj.insert("name".to_string(), Value::String("myapp".into()));
    obj.insert("port".to_string(), Value::Null);
    obj.insert("host".to_string(), Value::Null);
    obj.insert("retries".to_string(), Value::Null);

    let config = ConfigWithDefaults::from_tealeaf_value(&Value::Object(obj)).unwrap();
    assert_eq!(config.port, 42, "Null should trigger default expr");
    assert_eq!(config.host, "localhost", "Null should trigger default expr");
    assert_eq!(config.retries, 0, "Null should trigger bare default");
}

#[test]
fn test_default_expr_from_present_fields() {
    let mut obj = ObjectMap::new();
    obj.insert("name".to_string(), Value::String("myapp".into()));
    obj.insert("port".to_string(), Value::Int(8080));
    obj.insert("host".to_string(), Value::String("example.com".into()));
    obj.insert("retries".to_string(), Value::Int(3));

    let config = ConfigWithDefaults::from_tealeaf_value(&Value::Object(obj)).unwrap();
    assert_eq!(config.port, 8080, "Present value should override default");
    assert_eq!(config.host, "example.com", "Present value should override default");
    assert_eq!(config.retries, 3, "Present value should override default");
}

// =============================================================================
// Phase 4: Skip + Default combined
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct WithSkipDefault {
    name: String,
    #[tealeaf(skip, default = "String::from(\"computed\")")]
    internal: String,
}

#[test]
fn test_skip_with_default_expr() {
    let item = WithSkipDefault {
        name: "test".into(),
        internal: "something".into(),
    };

    // Serialization: skip field should be absent
    let value = item.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert!(obj.get("internal").is_none(), "Skipped field should not be serialized");

    // Deserialization: should use default expression
    let restored = WithSkipDefault::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored.name, "test");
    assert_eq!(restored.internal, "computed", "Should use default expr for skipped field");
}

// =============================================================================
// Phase 4: Optional timestamp with type override
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct EventWithOptionalTimestamp {
    name: String,
    #[tealeaf(type = "timestamp")]
    created_at: Option<i64>,
}

#[test]
fn test_optional_timestamp_some() {
    let event = EventWithOptionalTimestamp {
        name: "deploy".into(),
        created_at: Some(1705315800000),
    };
    let value = event.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("created_at").unwrap().as_timestamp(), Some((1705315800000, 0)));

    let restored = EventWithOptionalTimestamp::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored.created_at, Some(1705315800000));
}

#[test]
fn test_optional_timestamp_none() {
    let event = EventWithOptionalTimestamp {
        name: "deploy".into(),
        created_at: None,
    };
    let value = event.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert!(obj.get("created_at").unwrap().is_null());

    let restored = EventWithOptionalTimestamp::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored.created_at, None);
}

// =============================================================================
// Phase 4: Enum with named-field variant roundtrip
// =============================================================================

#[test]
fn test_enum_named_variant_circle() {
    let shape = Shape::Circle { radius: 5.0 };
    let value = shape.to_tealeaf_value();

    let (tag, inner) = value.as_tagged().unwrap();
    assert_eq!(tag, "Circle");
    let obj = inner.as_object().unwrap();
    assert_eq!(obj.get("radius").unwrap().as_float(), Some(5.0));

    let restored = Shape::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored, Shape::Circle { radius: 5.0 });
}

#[test]
fn test_enum_named_variant_rectangle() {
    let shape = Shape::Rectangle { width: 10.0, height: 20.0 };
    let value = shape.to_tealeaf_value();

    let (tag, inner) = value.as_tagged().unwrap();
    assert_eq!(tag, "Rectangle");
    let obj = inner.as_object().unwrap();
    assert_eq!(obj.get("width").unwrap().as_float(), Some(10.0));
    assert_eq!(obj.get("height").unwrap().as_float(), Some(20.0));

    let restored = Shape::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored, Shape::Rectangle { width: 10.0, height: 20.0 });
}

#[test]
fn test_enum_named_variant_unit() {
    let shape = Shape::Point;
    let value = shape.to_tealeaf_value();
    let restored = Shape::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored, Shape::Point);
}

// =============================================================================
// Phase 4: Enum with multi-tuple variant deserialization
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
enum MultiResult {
    Pair(String, i64),
    Single(String),
    Empty,
}

#[test]
fn test_enum_multi_tuple_variant_roundtrip() {
    let val = MultiResult::Pair("hello".into(), 42);
    let value = val.to_tealeaf_value();

    let (tag, inner) = value.as_tagged().unwrap();
    assert_eq!(tag, "Pair");
    let arr = inner.as_array().unwrap();
    assert_eq!(arr[0].as_str(), Some("hello"));
    assert_eq!(arr[1].as_int(), Some(42));

    let restored = MultiResult::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored, MultiResult::Pair("hello".into(), 42));
}

#[test]
fn test_enum_single_variant_roundtrip() {
    let val = MultiResult::Single("test".into());
    let value = val.to_tealeaf_value();
    let restored = MultiResult::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored, MultiResult::Single("test".into()));
}

#[test]
fn test_enum_unit_variant_roundtrip() {
    let val = MultiResult::Empty;
    let value = val.to_tealeaf_value();
    let restored = MultiResult::from_tealeaf_value(&value).unwrap();
    assert_eq!(restored, MultiResult::Empty);
}

// =============================================================================
// Phase 4: Type override with non-timestamp (generic path)
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct WithGenericTypeOverride {
    name: String,
    #[tealeaf(type = "string")]
    label: String,
}

#[test]
fn test_generic_type_override_roundtrip() {
    let item = WithGenericTypeOverride {
        name: "test".into(),
        label: "my-label".into(),
    };
    let value = item.to_tealeaf_value();
    let restored = WithGenericTypeOverride::from_tealeaf_value(&value).unwrap();
    assert_eq!(item, restored);
}

#[test]
fn test_generic_type_override_schema() {
    let schemas = WithGenericTypeOverride::collect_schemas();
    let schema = schemas.get("WithGenericTypeOverride").unwrap();
    let label_field = schema.fields.iter().find(|f| f.name == "label").unwrap();
    assert_eq!(label_field.field_type.base, "string");
}

// =============================================================================
// Phase 4: Optional field with type override (nullable schema)
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct WithOptionalOverride {
    name: String,
    #[tealeaf(type = "int", optional)]
    priority: i64,
}

#[test]
fn test_optional_flag_schema() {
    let schemas = WithOptionalOverride::collect_schemas();
    let schema = schemas.get("WithOptionalOverride").unwrap();
    let priority_field = schema.fields.iter().find(|f| f.name == "priority").unwrap();
    assert!(priority_field.field_type.nullable, "optional flag should make field nullable in schema");
}

// =============================================================================
// Phase 4+: Container attribute tests (root_array, key)
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(root_array)]
struct RootArrayItem {
    id: i64,
    name: String,
}

#[test]
fn test_root_array_attribute_roundtrip() {
    let item = RootArrayItem { id: 1, name: "test".into() };
    let value = item.to_tealeaf_value();
    let restored = RootArrayItem::from_tealeaf_value(&value).unwrap();
    assert_eq!(item, restored);
}

#[test]
fn test_root_array_schema() {
    let schemas = RootArrayItem::collect_schemas();
    assert!(schemas.get("RootArrayItem").is_some());
}

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(key = "custom_key")]
struct WithCustomKey {
    value: String,
}

#[test]
fn test_key_attribute_roundtrip() {
    let item = WithCustomKey { value: "hello".into() };
    let value = item.to_tealeaf_value();
    let restored = WithCustomKey::from_tealeaf_value(&value).unwrap();
    assert_eq!(item, restored);
}

// =============================================================================
// Phase 4+: Skip without default expr (bare default)
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct WithBareSkip {
    name: String,
    #[tealeaf(skip)]
    internal: i64,  // Will use Default::default() = 0
}

#[test]
fn test_bare_skip_to_value() {
    let item = WithBareSkip { name: "test".into(), internal: 999 };
    let value = item.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    // Skipped field should not appear in serialized output
    assert!(obj.get("internal").is_none());
    assert_eq!(obj.get("name").unwrap().as_str(), Some("test"));
}

#[test]
fn test_bare_skip_from_value() {
    let mut obj = ObjectMap::new();
    obj.insert("name".to_string(), Value::String("test".into()));
    // No "internal" field — should default to 0
    let restored = WithBareSkip::from_tealeaf_value(&Value::Object(obj)).unwrap();
    assert_eq!(restored.internal, 0); // Default::default()
    assert_eq!(restored.name, "test");
}

// =============================================================================
// Phase 4+: Flatten on Option<T> field
// =============================================================================

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct OptionalAddress {
    city: String,
    zip: String,
}

#[derive(Debug, Clone, PartialEq, ToTeaLeaf, FromTeaLeaf)]
struct PersonWithOptionalAddress {
    name: String,
    #[tealeaf(flatten)]
    address: Option<OptionalAddress>,
}

#[test]
fn test_flatten_optional_some() {
    let person = PersonWithOptionalAddress {
        name: "Alice".into(),
        address: Some(OptionalAddress { city: "NYC".into(), zip: "10001".into() }),
    };
    let value = person.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(obj.get("city").unwrap().as_str(), Some("NYC"));
}

#[test]
fn test_flatten_optional_none() {
    let person = PersonWithOptionalAddress {
        name: "Bob".into(),
        address: None,
    };
    let value = person.to_tealeaf_value();
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("name").unwrap().as_str(), Some("Bob"));
}

#[test]
fn test_flatten_optional_schema() {
    let schemas = PersonWithOptionalAddress::collect_schemas();
    assert!(schemas.get("PersonWithOptionalAddress").is_some());
    assert!(schemas.get("OptionalAddress").is_some());
}

// =============================================================================
// Builder + schema-aware serialization: @table output with PascalCase schemas
// =============================================================================

use tealeaf::convert::NotU8;

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
struct StockEntry {
    warehouse: String,
    qty: i32,
    backordered: bool,
}
impl NotU8 for StockEntry {}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
struct Pricing {
    base_price: f64,
    currency: String,
}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
struct ProductInfo {
    id: String,
    name: String,
    pricing: Pricing,
    stock: Vec<StockEntry>,
}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
struct LineItem {
    line: i32,
    product: ProductInfo,
    quantity: i32,
}
impl NotU8 for LineItem {}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
struct SalesOrder {
    order_id: String,
    items: Vec<LineItem>,
    total: f64,
}
impl NotU8 for SalesOrder {}

#[test]
fn test_builder_schema_aware_table_output() {
    // Build a document via the Builder API with derive-generated PascalCase schemas.
    // The serializer must produce @table annotations even though the schema names
    // (SalesOrder, LineItem, ProductInfo, etc.) don't match singularize("orders") = "order".
    let orders = vec![
        SalesOrder {
            order_id: "ORD-001".into(),
            items: vec![
                LineItem {
                    line: 1,
                    product: ProductInfo {
                        id: "P-100".into(),
                        name: "Widget".into(),
                        pricing: Pricing { base_price: 9.99, currency: "USD".into() },
                        stock: vec![
                            StockEntry { warehouse: "W1".into(), qty: 100, backordered: false },
                        ],
                    },
                    quantity: 2,
                },
            ],
            total: 19.98,
        },
        SalesOrder {
            order_id: "ORD-002".into(),
            items: vec![
                LineItem {
                    line: 1,
                    product: ProductInfo {
                        id: "P-200".into(),
                        name: "Gadget".into(),
                        pricing: Pricing { base_price: 24.50, currency: "USD".into() },
                        stock: vec![
                            StockEntry { warehouse: "W1".into(), qty: 50, backordered: false },
                            StockEntry { warehouse: "W2".into(), qty: 0, backordered: true },
                        ],
                    },
                    quantity: 1,
                },
            ],
            total: 24.50,
        },
    ];

    let doc = TeaLeafBuilder::new()
        .add_vec("orders", &orders)
        .build();

    let tl = doc.to_tl_with_schemas();

    // Must contain @table annotation — the whole point of this fix
    assert!(tl.contains("@table SalesOrder"), "Missing @table SalesOrder:\n{tl}");

    // Must contain @struct declarations for all schemas
    assert!(tl.contains("@struct SalesOrder"), "Missing @struct SalesOrder:\n{tl}");
    assert!(tl.contains("@struct LineItem"), "Missing @struct LineItem:\n{tl}");
    assert!(tl.contains("@struct ProductInfo"), "Missing @struct ProductInfo:\n{tl}");
    assert!(tl.contains("@struct Pricing"), "Missing @struct Pricing:\n{tl}");
    assert!(tl.contains("@struct StockEntry"), "Missing @struct StockEntry:\n{tl}");

    // Must use tuple encoding inside @table (parentheses, not braces)
    assert!(tl.contains("(ORD-001,"), "Missing tuple encoding for ORD-001:\n{tl}");
    assert!(tl.contains("(ORD-002,"), "Missing tuple encoding for ORD-002:\n{tl}");

    // Round-trip: parse TL back and verify data integrity
    let reparsed = TeaLeaf::parse(&tl)
        .unwrap_or_else(|e| panic!("Re-parse failed: {e}\nTL:\n{tl}"));
    let orders_val = reparsed.get("orders").expect("missing 'orders' key");
    let arr = orders_val.as_array().expect("orders should be array");
    assert_eq!(arr.len(), 2, "should have 2 orders");

    // Verify first order round-trips correctly
    let first = arr[0].as_object().unwrap();
    assert_eq!(first.get("order_id").unwrap().as_str(), Some("ORD-001"));
    assert_eq!(first.get("total").unwrap().as_float(), Some(19.98));
    let items = first.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 1);
    let item = items[0].as_object().unwrap();
    let product = item.get("product").unwrap().as_object().unwrap();
    assert_eq!(product.get("name").unwrap().as_str(), Some("Widget"));
    let pricing = product.get("pricing").unwrap().as_object().unwrap();
    assert_eq!(pricing.get("base_price").unwrap().as_float(), Some(9.99));
    let stock = product.get("stock").unwrap().as_array().unwrap();
    assert_eq!(stock.len(), 1);
    assert_eq!(stock[0].as_object().unwrap().get("warehouse").unwrap().as_str(), Some("W1"));
}

#[path = "fixtures/retail_orders_different_shape.rs"]
mod retail_data;

#[test]
fn gen_retail_orders_api_tl() {
    let orders = retail_data::sample_orders();
    let doc = TeaLeafBuilder::new()
        .add_vec("orders", &orders)
        .build();
    let tl = doc.to_tl_with_schemas();
    let out_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .join("examples/retail_orders_different_shape_api.tl");
    std::fs::write(&out_path, &tl).unwrap();
    eprintln!("Wrote {} bytes to {}", tl.len(), out_path.display());
}

