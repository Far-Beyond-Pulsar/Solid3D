mod common;
use solid_rs::value::Value;
use std::collections::HashMap;

// ── Null ──────────────────────────────────────────────────────────────────────

#[test]
fn value_null_is_null() {
    assert!(Value::Null.is_null());
}
#[test]
fn value_bool_not_null() {
    assert!(!Value::Bool(true).is_null());
}
#[test]
fn value_int_not_null() {
    assert!(!Value::Int(0).is_null());
}

// ── Bool ─────────────────────────────────────────────────────────────────────

#[test]
fn value_bool_true_as_bool() {
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
}
#[test]
fn value_bool_false_as_bool() {
    assert_eq!(Value::Bool(false).as_bool(), Some(false));
}
#[test]
fn value_int_as_bool_is_none() {
    assert!(Value::Int(1).as_bool().is_none());
}

// ── Int ───────────────────────────────────────────────────────────────────────

#[test]
fn value_int_as_int() {
    assert_eq!(Value::Int(42).as_int(), Some(42));
}
#[test]
fn value_int_negative() {
    assert_eq!(Value::Int(-100).as_int(), Some(-100));
}
#[test]
fn value_bool_as_int_is_none() {
    assert!(Value::Bool(true).as_int().is_none());
}

// ── Float ────────────────────────────────────────────────────────────────────

#[test]
fn value_float_as_float() {
    assert_eq!(Value::Float(3.14).as_float(), Some(3.14));
}
#[test]
fn value_float_negative() {
    assert_eq!(Value::Float(-2.5).as_float(), Some(-2.5));
}
#[test]
fn value_int_as_float_none() {
    assert!(Value::Int(1).as_float().is_none());
}

// ── String ───────────────────────────────────────────────────────────────────

#[test]
fn value_string_as_str() {
    assert_eq!(Value::String("hi".into()).as_str(), Some("hi"));
}
#[test]
fn value_string_empty() {
    assert_eq!(Value::String("".into()).as_str(), Some(""));
}
#[test]
fn value_null_as_str_none() {
    assert!(Value::Null.as_str().is_none());
}

// ── Vec2/3/4 ─────────────────────────────────────────────────────────────────

#[test]
fn value_vec2_stored() {
    let v = Value::Vec2([1.0, 2.0]);
    assert!(matches!(v, Value::Vec2(_)));
}
#[test]
fn value_vec3_stored() {
    let v = Value::Vec3([1.0, 2.0, 3.0]);
    assert!(matches!(v, Value::Vec3(_)));
}
#[test]
fn value_vec4_stored() {
    let v = Value::Vec4([1.0, 2.0, 3.0, 4.0]);
    assert!(matches!(v, Value::Vec4(_)));
}

// ── Array ─────────────────────────────────────────────────────────────────────

#[test]
fn value_array_as_array() {
    let a = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    assert_eq!(a.as_array().unwrap().len(), 2);
}

#[test]
fn value_empty_array() {
    let a = Value::Array(vec![]);
    assert_eq!(a.as_array().unwrap().len(), 0);
}

#[test]
fn value_null_as_array_is_none() {
    assert!(Value::Null.as_array().is_none());
}

// ── Map ───────────────────────────────────────────────────────────────────────

#[test]
fn value_map_as_map() {
    let mut m = HashMap::new();
    m.insert("key".into(), Value::Int(99));
    let v = Value::Map(m);
    assert_eq!(v.as_map().unwrap()["key"].as_int(), Some(99));
}

#[test]
fn value_null_as_map_is_none() {
    assert!(Value::Null.as_map().is_none());
}

// ── Bytes ────────────────────────────────────────────────────────────────────

#[test]
fn value_bytes_stored() {
    let v = Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]);
    assert!(matches!(v, Value::Bytes(_)));
}

// ── From impls ────────────────────────────────────────────────────────────────

#[test]
fn from_bool_true() {
    let v: Value = true.into();
    assert_eq!(v.as_bool(), Some(true));
}
#[test]
fn from_bool_false() {
    let v: Value = false.into();
    assert_eq!(v.as_bool(), Some(false));
}
#[test]
fn from_i32() {
    let v: Value = 7i32.into();
    assert_eq!(v.as_int(), Some(7));
}
#[test]
fn from_i64() {
    let v: Value = 100i64.into();
    assert_eq!(v.as_int(), Some(100));
}
#[test]
fn from_u32() {
    let v: Value = 5u32.into();
    assert_eq!(v.as_int(), Some(5));
}
#[test]
fn from_f32() {
    let v: Value = 1.5f32.into();
    assert!(v.as_float().is_some());
}
#[test]
fn from_f64() {
    let v: Value = 2.71f64.into();
    assert_eq!(v.as_float(), Some(2.71));
}
#[test]
fn from_string() {
    let v: Value = "hi".to_owned().into();
    assert_eq!(v.as_str(), Some("hi"));
}
#[test]
fn from_str_ref() {
    let v: Value = "world".into();
    assert_eq!(v.as_str(), Some("world"));
}
#[test]
fn from_bytes() {
    let v: Value = vec![1u8, 2, 3].into();
    assert!(matches!(v, Value::Bytes(_)));
}
#[test]
fn from_vec_value() {
    let v: Value = vec![Value::Int(1), Value::Int(2)].into();
    assert_eq!(v.as_array().unwrap().len(), 2);
}
#[test]
fn from_hashmap() {
    let mut m = HashMap::new();
    m.insert("k".into(), Value::Bool(true));
    let v: Value = m.into();
    assert!(v.as_map().is_some());
}

// ── Clone & PartialEq ─────────────────────────────────────────────────────────

#[test]
fn value_clone_null() {
    assert_eq!(Value::Null.clone(), Value::Null);
}
#[test]
fn value_clone_int() {
    let v = Value::Int(5);
    assert_eq!(v.clone(), v);
}
#[test]
fn value_clone_string() {
    let v = Value::String("abc".into());
    assert_eq!(v.clone(), v);
}
#[test]
fn value_eq_null() {
    assert_eq!(Value::Null, Value::Null);
}
#[test]
fn value_ne_types() {
    assert_ne!(Value::Null, Value::Bool(false));
}
#[test]
fn value_eq_int() {
    assert_eq!(Value::Int(3), Value::Int(3));
}
#[test]
fn value_ne_int() {
    assert_ne!(Value::Int(3), Value::Int(4));
}

// ── Nested structures ─────────────────────────────────────────────────────────

#[test]
fn value_nested_map_in_array() {
    let mut inner = HashMap::new();
    inner.insert("x".into(), Value::Float(1.0));
    let v = Value::Array(vec![Value::Map(inner)]);
    let arr = v.as_array().unwrap();
    assert_eq!(arr[0].as_map().unwrap()["x"].as_float(), Some(1.0));
}

#[test]
fn value_deeply_nested() {
    let deep = Value::Array(vec![Value::Array(vec![Value::Int(42)])]);
    let inner = deep.as_array().unwrap()[0].as_array().unwrap();
    assert_eq!(inner[0].as_int(), Some(42));
}
