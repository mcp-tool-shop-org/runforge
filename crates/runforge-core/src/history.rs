use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::HistoryError;
use crate::export::json_equal;
use crate::parse::parse_document;
use crate::time::timestamp_ord;

/// One stored loss sample. A gap is a null or non-finite value, and it is not drawn as zero.
#[derive(Clone, Debug, PartialEq)]
pub enum LossSample {
    Point(f64),
    Gap,
}

/// Eval fields the bench shows. Generation text stays in the raw entry and is not rendered.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalSummary {
    pub held_out_loss: Option<f64>,
    pub perplexity: Option<f64>,
    pub task_metrics: Vec<(String, f64)>,
    pub eval_n: Option<i64>,
    pub n_prompts: Option<i64>,
}

/// One row of `run_history.json`. `file_index` is the position in the file, which compare uses.
#[derive(Clone, Debug)]
pub struct RunEntry {
    pub file_index: usize,
    pub run_id: String,
    pub status: String,
    pub session_kind: String,
    pub model_name: String,
    pub dataset_info: String,
    pub started_at: String,
    pub completed_at: String,
    pub duration_seconds: Option<f64>,
    pub steps: String,
    pub final_loss: Option<f64>,
    pub loss: Vec<LossSample>,
    pub hyperparameters: Map<String, Value>,
    pub failure_reason: String,
    pub checkpoint_path: String,
    pub export_paths: Vec<String>,
    pub dataset_hash: String,
    pub eval_summary: Option<EvalSummary>,
    pub schema_version: Option<String>,
    pub raw: Value,
    started_ord: Option<i128>,
}

/// A hyperparameter whose parsed value differs between two rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperDiff {
    pub key: String,
    pub left: String,
    pub right: String,
}

/// The runs the bench can show. Entries stay in file order.
#[derive(Clone, Debug)]
pub struct History {
    pub entries: Vec<RunEntry>,
    pub skipped: usize,
    pub duplicate_ids: usize,
    pub schema_notes: Vec<String>,
}

impl History {
    /// Newest parsed timestamp first. A row with no parseable time sorts last.
    /// Equal times keep file order.
    pub fn display_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        order.sort_by(|&left, &right| {
            let a = &self.entries[left];
            let b = &self.entries[right];
            match (a.started_ord, b.started_ord) {
                (Some(a_ord), Some(b_ord)) => b_ord.cmp(&a_ord).then(left.cmp(&right)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => left.cmp(&right),
            }
        });
        order
    }

    /// File index of the lowest finite `final_loss`. A missing or non-finite loss is never chosen.
    pub fn best_loss_index(&self) -> Option<usize> {
        pick_best_loss(
            &self
                .entries
                .iter()
                .map(|entry| entry.final_loss)
                .collect::<Vec<_>>(),
        )
        .map(|index| self.entries[index].file_index)
    }

    pub fn get(&self, file_index: usize) -> Option<&RunEntry> {
        self.entries
            .iter()
            .find(|entry| entry.file_index == file_index)
    }
}

/// Index into `losses` of the best finite loss. Non-finite values sort after every finite loss.
pub fn pick_best_loss(losses: &[Option<f64>]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (index, loss) in losses.iter().enumerate() {
        let Some(loss) = loss.filter(|value| value.is_finite()) else {
            continue;
        };
        match best {
            None => best = Some((index, loss)),
            Some((_, current)) if loss.total_cmp(&current).is_lt() => best = Some((index, loss)),
            _ => {}
        }
    }
    best.map(|(index, _)| index)
}

/// Keys whose parsed values differ. Numbers compare by f64 bits, so `0.0002` and `2e-4` agree.
pub fn hyperparameter_diffs(
    left: &Map<String, Value>,
    right: &Map<String, Value>,
) -> Vec<HyperDiff> {
    let mut keys = BTreeSet::new();
    keys.extend(left.keys().cloned());
    keys.extend(right.keys().cloned());
    let mut diffs = Vec::new();
    for key in keys {
        let left_value = left.get(&key);
        let right_value = right.get(&key);
        if let (Some(left_value), Some(right_value)) = (left_value, right_value)
            && json_equal(left_value, right_value)
        {
            continue;
        }
        diffs.push(HyperDiff {
            key,
            left: left_value.map(value_text).unwrap_or_default(),
            right: right_value.map(value_text).unwrap_or_default(),
        });
    }
    diffs
}

fn value_text(value: &Value) -> String {
    serde_json::to_string(value).expect("a JSON value should serialize")
}

/// `run_history.json` in `folder`, or `output/run_history.json` one level down.
/// The file in the opened folder wins when both exist. This does not walk the disk.
pub fn history_file(folder: &Path) -> Result<PathBuf, HistoryError> {
    let direct = folder.join("run_history.json");
    if direct.is_file() {
        return Ok(direct);
    }
    let nested = folder.join("output").join("run_history.json");
    if nested.is_file() {
        return Ok(nested);
    }
    Err(HistoryError::NotFound)
}

pub fn load_folder(folder: &Path) -> Result<(PathBuf, History), HistoryError> {
    let path = history_file(folder)?;
    let bytes = fs::read(&path).map_err(|source| HistoryError::Read { source })?;
    let history = load_bytes(&bytes)?;
    Ok((path, history))
}

pub fn load_text(text: &str) -> Result<History, HistoryError> {
    load_bytes(text.as_bytes())
}

pub fn load_bytes(bytes: &[u8]) -> Result<History, HistoryError> {
    let value = parse_document(bytes)?;
    let Value::Array(items) = value else {
        return Err(HistoryError::NotArray);
    };
    Ok(project(items))
}

fn project(items: Vec<Value>) -> History {
    let mut entries = Vec::new();
    let mut skipped = 0usize;
    let mut schema_versions = BTreeSet::new();
    for (file_index, item) in items.into_iter().enumerate() {
        match project_entry(file_index, item) {
            Some(entry) => {
                if let Some(version) = entry.schema_version.as_deref()
                    && version != "1.0"
                {
                    schema_versions.insert(version.to_string());
                }
                entries.push(entry);
            }
            None => skipped += 1,
        }
    }
    let duplicate_ids = duplicate_id_count(&entries);
    History {
        entries,
        skipped,
        duplicate_ids,
        schema_notes: schema_versions.into_iter().collect(),
    }
}

fn duplicate_id_count(entries: &[RunEntry]) -> usize {
    let mut counts = BTreeMap::<&str, usize>::new();
    for entry in entries {
        *counts.entry(&entry.run_id).or_insert(0) += 1;
    }
    counts.values().filter(|count| **count > 1).count()
}

fn project_entry(file_index: usize, item: Value) -> Option<RunEntry> {
    let Value::Object(map) = &item else {
        return None;
    };
    let run_id = match map.get("run_id") {
        Some(Value::String(id)) if !id.is_empty() => id.clone(),
        _ => return None,
    };
    let started_at = match map.get("started_at") {
        Some(Value::String(text)) if !text.is_empty() => text.clone(),
        _ => text_of(map.get("timestamp")),
    };
    let loss = loss_samples(map.get("loss_history"));
    Some(RunEntry {
        file_index,
        run_id,
        status: text_of(map.get("status")),
        session_kind: text_of(map.get("session_kind")),
        model_name: text_of(map.get("model_name")),
        dataset_info: text_of(map.get("dataset_info")),
        started_ord: timestamp_ord(&started_at),
        started_at,
        completed_at: text_of(map.get("completed_at")),
        duration_seconds: map.get("duration_seconds").and_then(finite_f64),
        steps: text_of(map.get("steps")),
        final_loss: map.get("final_loss").and_then(finite_f64),
        loss,
        hyperparameters: match map.get("hyperparameters") {
            Some(Value::Object(object)) => object.clone(),
            _ => Map::new(),
        },
        failure_reason: text_of(map.get("failure_reason")),
        checkpoint_path: text_of(map.get("checkpoint_path")),
        export_paths: export_paths(map.get("export_paths")),
        dataset_hash: text_of(map.get("dataset_hash")),
        eval_summary: map.get("eval").and_then(eval_summary),
        schema_version: schema_version(map.get("schema_version")),
        raw: item,
    })
}

fn loss_samples(value: Option<&Value>) -> Vec<LossSample> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| match finite_f64(item) {
            Some(sample) => LossSample::Point(sample),
            None => LossSample::Gap,
        })
        .collect()
}

fn eval_summary(value: &Value) -> Option<EvalSummary> {
    let Value::Object(map) = value else {
        return None;
    };
    let mut task_metrics = Vec::new();
    if let Some(Value::Object(metrics)) = map.get("task_metrics") {
        for (name, metric) in metrics {
            if let Some(score) = finite_f64(metric) {
                task_metrics.push((name.clone(), score));
            }
        }
        task_metrics.sort_by(|left, right| left.0.cmp(&right.0));
    }
    Some(EvalSummary {
        held_out_loss: map.get("held_out_loss").and_then(finite_f64),
        perplexity: map.get("perplexity").and_then(finite_f64),
        task_metrics,
        eval_n: map.get("eval_n").and_then(integer),
        n_prompts: map.get("n_prompts").and_then(integer),
    })
}

fn export_paths(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(path)) => vec![path.clone()],
        _ => Vec::new(),
    }
}

fn schema_version(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) if !text.is_empty() => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}

fn text_of(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        _ => String::new(),
    }
}

fn finite_f64(value: &Value) -> Option<f64> {
    value.as_f64().filter(|number| number.is_finite())
}

fn integer(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64().or_else(|| {
            let float = number.as_f64()?;
            if float.is_finite()
                && float.fract() == 0.0
                && float >= i64::MIN as f64
                && float <= i64::MAX as f64
            {
                Some(float as i64)
            } else {
                None
            }
        }),
        _ => None,
    }
}
