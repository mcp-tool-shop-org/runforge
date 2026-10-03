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

#[cfg(test)]
mod tests {
    use super::{Strict, StrictVisitor};
    use serde::de::{DeserializeSeed, Visitor};
    use serde_json::{Number, Value};

    #[test]
    fn the_visitor_accepts_every_json_shape() {
        struct Show;
        impl std::fmt::Display for Show {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let visitor = StrictVisitor;
                visitor.expecting(formatter)
            }
        }
        assert_eq!(Show.to_string(), "a JSON value");

        assert_eq!(
            StrictVisitor.visit_bool::<serde_json::Error>(true).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            StrictVisitor.visit_unit::<serde_json::Error>().unwrap(),
            Value::Null
        );
        assert_eq!(
            StrictVisitor.visit_none::<serde_json::Error>().unwrap(),
            Value::Null
        );
        assert_eq!(
            StrictVisitor
                .visit_string::<serde_json::Error>("kept".to_string())
                .unwrap(),
            Value::String("kept".to_string())
        );
        assert_eq!(
            StrictVisitor.visit_i64::<serde_json::Error>(-3).unwrap(),
            Value::Number((-3i64).into())
        );
        assert_eq!(
            StrictVisitor.visit_u64::<serde_json::Error>(3).unwrap(),
            Value::Number(3u64.into())
        );
        assert!(matches!(
            StrictVisitor.visit_f64::<serde_json::Error>(-0.0).unwrap(),
            Value::Number(_)
        ));
        assert!(matches!(
            StrictVisitor
                .visit_f64::<serde_json::Error>(f64::NAN)
                .unwrap(),
            Value::Null
        ));
        assert_eq!(
            StrictVisitor.visit_i128::<serde_json::Error>(9).unwrap(),
            Value::Number(Number::from(9i64))
        );
        assert!(
            StrictVisitor
                .visit_i128::<serde_json::Error>(i128::MAX)
                .is_err()
        );
        assert_eq!(
            StrictVisitor.visit_u128::<serde_json::Error>(9).unwrap(),
            Value::Number(Number::from(9u64))
        );
        assert!(
            StrictVisitor
                .visit_u128::<serde_json::Error>(u128::MAX)
                .is_err()
        );
        assert_eq!(
            StrictVisitor
                .visit_str::<serde_json::Error>("kept")
                .unwrap(),
            Value::String("kept".to_string())
        );
        let mut inner = serde_json::Deserializer::from_str("true");
        assert_eq!(
            StrictVisitor.visit_some(&mut inner).unwrap(),
            Value::Bool(true)
        );

        let mut deserializer = serde_json::Deserializer::from_str("[true, null]");
        let value = Strict.deserialize(&mut deserializer).unwrap();
        assert_eq!(value, serde_json::json!([true, null]));
    }
}
