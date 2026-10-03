use serde_json::Value;

use crate::history::{History, LossSample, RunEntry};

/// List CSV in the same order as the window: newest first.
pub fn list_csv(history: &History) -> String {
    let mut out = String::from(
        "run_id,status,session_kind,model_name,final_loss,steps,duration_seconds,started_at,failure_reason\n",
    );
    for index in history.display_order() {
        let entry = &history.entries[index];
        let fields = [
            entry.run_id.as_str(),
            entry.status.as_str(),
            entry.session_kind.as_str(),
            entry.model_name.as_str(),
            &format_optional(entry.final_loss),
            entry.steps.as_str(),
            &format_optional(entry.duration_seconds),
            entry.started_at.as_str(),
            entry.failure_reason.as_str(),
        ];
        push_row(&mut out, &fields);
    }
    out
}

/// Curve CSV. A gap is an empty loss cell. The index is the stored sample index.
pub fn curve_csv(entry: &RunEntry) -> String {
    let mut out = String::from("index,loss\n");
    for (index, sample) in entry.loss.iter().enumerate() {
        let loss = match sample {
            LossSample::Point(value) => format_f64(*value),
            LossSample::Gap => String::new(),
        };
        push_row(&mut out, &[&index.to_string(), &loss]);
    }
    out
}

/// Pretty JSON of the original entry, unknown keys included.
pub fn entry_json(entry: &RunEntry) -> String {
    let mut bytes = serde_json::to_vec_pretty(&entry.raw).expect("a JSON value should serialize");
    bytes.push(b'\n');
    String::from_utf8(bytes).expect("serde_json writes UTF-8")
}

pub fn format_f64(value: f64) -> String {
    serde_json::to_string(&value).expect("a finite f64 should serialize")
}

fn format_optional(value: Option<f64>) -> String {
    value.map(format_f64).unwrap_or_default()
}

fn push_row(out: &mut String, fields: &[&str]) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&csv_field(field));
    }
    out.push('\n');
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// Contiguous finite samples. A gap starts a new segment so the line does not bridge it.
pub fn curve_segments(loss: &[LossSample]) -> Vec<Vec<[f64; 2]>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for (index, sample) in loss.iter().enumerate() {
        match sample {
            LossSample::Point(y) => current.push([index as f64, *y]),
            LossSample::Gap => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

pub fn finite_points(loss: &[LossSample]) -> Vec<[f64; 2]> {
    loss.iter()
        .enumerate()
        .filter_map(|(index, sample)| match sample {
            LossSample::Point(y) => Some([index as f64, *y]),
            LossSample::Gap => None,
        })
        .collect()
}

/// `true` when two parsed JSON values are the same. Numbers compare by f64 bits.
pub fn json_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => x.to_bits() == y.to_bits(),
            _ => a.to_string() == b.to_string(),
        },
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| json_equal(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| json_equal(value, other)))
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}
