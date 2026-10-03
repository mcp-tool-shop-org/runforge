//! JSON parse that refuses a duplicate key.
//!
//! serde_json keeps the last value when an object repeats a key. A history
//! file that repeats `run_id` would then show the second run and hide the
//! first. This reader refuses the whole file instead.

use std::fmt;

use serde::Deserialize;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::error::HistoryError;

pub(crate) fn parse_document(bytes: &[u8]) -> Result<Value, HistoryError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Strict
        .deserialize(&mut deserializer)
        .map_err(|error| HistoryError::Parse(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| HistoryError::Parse(error.to_string()))?;
    Ok(value)
}

struct Strict;

impl<'de> DeserializeSeed<'de> for Strict {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
        Ok(Number::from_f64(value).map_or(Value::Null, Value::Number))
    }

    fn visit_i128<E>(self, value: i128) -> Result<Value, E>
    where
        E: de::Error,
    {
        let inner = de::value::I128Deserializer::<E>::new(value);
        Number::deserialize(inner).map(Value::Number)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Value, E>
    where
        E: de::Error,
    {
        let inner = de::value::U128Deserializer::<E>::new(value);
        Number::deserialize(inner).map(Value::Number)
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Strict.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element_seed(Strict)? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate key {key}")));
            }
            let value = map.next_value_seed(Strict)?;
            object.insert(key, value);
        }
        Ok(Value::Object(object))
    }
}
