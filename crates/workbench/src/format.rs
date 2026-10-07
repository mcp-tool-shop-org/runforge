//! How the program writes a number or a knob value into a sentence, and the memory file.

use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

/// A short label for a finite measurement. Trailing zeros are dropped.
pub fn format_measure(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let abs = value.abs();
    let whole = (1000.0..1.0e7).contains(&abs) && value.fract() == 0.0;
    let text = if whole {
        format!("{value:.0}")
    } else if abs >= 1.0e7 || abs < 1e-4 {
        format!("{value:.3e}")
    } else if abs >= 0.01 {
        format!("{value:.4}")
    } else {
        format!("{value:.6}")
    };
    trim_zeros(&text)
}

fn trim_zeros(text: &str) -> String {
    let Some((head, frac)) = text.split_once('.') else {
        return text.to_string();
    };
    if frac.contains('e') || frac.contains('E') {
        return text.to_string();
    }
    let frac = frac.trim_end_matches('0');
    if frac.is_empty() {
        head.to_string()
    } else {
        format!("{head}.{frac}")
    }
}

/// A knob's value as it reads in a sentence.
pub fn knob_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number
            .as_f64()
            .map(format_measure)
            .unwrap_or_else(|| number.to_string()),
        Value::Null => "—".to_string(),
        Value::Array(items) => items.iter().map(knob_text).collect::<Vec<_>>().join("  "),
        Value::Object(_) => serde_json::to_string(value).expect("a JSON value should serialize"),
    }
}

/// The memory file as an object. A missing or unreadable file is empty.
pub fn read_memory(file: &Path) -> Map<String, Value> {
    fs::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| match value {
            Value::Object(object) => Some(object),
            _ => None,
        })
        .unwrap_or_default()
}

/// Write the whole memory object back. A part this call did not change is kept.
pub fn write_memory(file: &Path, memory: &Map<String, Value>) -> Result<(), std::io::Error> {
    if let Some(directory) = file.parent() {
        fs::create_dir_all(directory)?;
    }
    let body = serde_json::to_string_pretty(&Value::Object(memory.clone()))
        .expect("memory should serialize");
    fs::write(file, body + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_knob_values_read_as_sentences() {
        assert_eq!(format_measure(f64::NAN), "—");
        assert_eq!(format_measure(0.0), "0");
        assert_eq!(format_measure(2500.0), "2500");
        assert_eq!(format_measure(0.125), "0.125");
        assert_eq!(format_measure(0.00512), "0.00512");
        assert_eq!(format_measure(3.0e-6), "3.000e-6");
        assert_eq!(knob_text(&Value::from("fp16")), "fp16");
        assert_eq!(knob_text(&Value::from(true)), "true");
        assert_eq!(knob_text(&Value::Null), "—");
        assert_eq!(knob_text(&serde_json::json!([1, "a"])), "1  a");
        assert_eq!(knob_text(&serde_json::json!({"k": 1})), r#"{"k":1}"#);
    }

    #[test]
    fn the_memory_file_round_trips_and_a_bad_one_reads_empty() {
        let dir = std::env::temp_dir().join(format!("workbench-memory-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let file = dir.join("nested").join("workbench.json");
        assert!(read_memory(&file).is_empty());
        let mut memory = Map::new();
        memory.insert("notes".into(), Value::from("kept"));
        write_memory(&file, &memory).unwrap();
        assert_eq!(read_memory(&file), memory);
        fs::write(&file, "[1, 2]").unwrap();
        assert!(read_memory(&file).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
