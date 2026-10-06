//! A folder of run-config series.
//!
//! Each sample stays a record. The recipe stays an object. `final_loss` on the
//! training summary is not appended to the curve. This module does not train
//! and does not call a model.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::HistoryError;
use crate::export::json_equal;
use crate::parse::parse_document;

/// One logged sample. Missing or non-finite loss and learning rate are gaps.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub x: Option<f64>,
    pub loss: Option<f64>,
    pub lr: Option<f64>,
    pub extra: Map<String, Value>,
}

/// One run-config file. `summary` is the training summary, kept off the curve.
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub name: String,
    pub seed: Option<i64>,
    pub model: String,
    pub file_name: String,
    pub samples: Vec<Sample>,
    pub recipe: Map<String, Value>,
    pub summary: Map<String, Value>,
}

/// The series in one opened folder. File order is seed, then name.
#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    pub series: Vec<Series>,
    pub skipped: usize,
    pub shared: Map<String, Value>,
    pub varying: Vec<String>,
}

/// A finite loss at its epoch or step.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub x: f64,
    pub loss: f64,
}

/// What one series did, measured from its own samples.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesRead {
    pub name: String,
    pub samples: usize,
    pub gaps: usize,
    pub unplaced: usize,
    pub first: Option<Mark>,
    pub low: Option<Mark>,
    pub last: Option<Mark>,
    pub climbed: bool,
    pub summary_final: Option<f64>,
}

/// The reading the sidecar cites. It does not replace the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub series: Vec<SeriesRead>,
    pub deepest: Vec<String>,
    pub quietest_end: Vec<String>,
    pub ends_differ: bool,
    pub varying_keys: Vec<String>,
    pub lever: String,
}

const RECIPE_ORDER: &[&str] = &[
    "method",
    "learning_rate",
    "lr_scheduler",
    "warmup_steps",
    "weight_decay",
    "max_grad_norm",
    "per_device_batch",
    "grad_accum",
    "effective_batch",
    "epochs",
    "max_seq_len",
    "prompt_loss_weight",
    "lora_r",
    "lora_alpha",
    "lora_dropout",
    "checkpoint_epochs",
    "target_modules",
];

const MEMORY_FILE: &str = "sidecar-memory.json";

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

/// Load run-config series from `folder` and its immediate child folders.
/// This does not walk further, and it does not merge a history file.
pub fn load_series_folder(folder: &Path) -> Result<Board, HistoryError> {
    let mut skipped = 0;
    let mut series = Vec::new();
    for path in series_files(folder) {
        match load_series_file(&path) {
            Ok(item) => series.push(item),
            Err(Skip) => skipped += 1,
        }
    }
    if series.is_empty() {
        return Err(HistoryError::NoSeries);
    }
    series.sort_by(|left, right| {
        seed_order(left.seed)
            .cmp(&seed_order(right.seed))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
    let (shared, varying) = split_recipe(&series);
    Ok(Board {
        series,
        skipped,
        shared,
        varying,
    })
}

struct Skip;

fn seed_order(seed: Option<i64>) -> (u8, i64) {
    match seed {
        Some(seed) => (0, seed),
        None => (1, 0),
    }
}

fn series_files(folder: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_configs(folder, &mut files);
    let mut children = Vec::new();
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                children.push(path);
            }
        }
    }
    children.sort();
    for child in children {
        collect_configs(&child, &mut files);
    }
    files.sort();
    files
}

fn collect_configs(folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with("run-config") && name.ends_with(".json") {
            out.push(path);
        }
    }
}

fn load_series_file(path: &Path) -> Result<Series, Skip> {
    let bytes = fs::read(path).map_err(|_| Skip)?;
    let value = parse_document(&bytes).map_err(|_| Skip)?;
    let object = value.as_object().ok_or(Skip)?;
    let curve = object
        .get("saturation_log")
        .and_then(Value::as_object)
        .and_then(|log| log.get("loss_curve"))
        .and_then(Value::as_array)
        .ok_or(Skip)?;
    let seed = object.get("seed").and_then(Value::as_i64);
    let model = object
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let name = seed.map(|seed| format!("seed {seed}")).unwrap_or_else(|| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("series")
            .to_string()
    });
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("run-config.json")
        .to_string();
    let recipe = object
        .get("hyperparameters")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let summary = object
        .get("training_summary")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let samples = curve.iter().map(sample_from).collect();
    Ok(Series {
        name,
        seed,
        model,
        file_name,
        samples,
        recipe,
        summary,
    })
}

fn sample_from(value: &Value) -> Sample {
    let Some(object) = value.as_object() else {
        return Sample {
            x: None,
            loss: None,
            lr: None,
            extra: Map::new(),
        };
    };
    let x = finite_number(object.get("epoch")).or_else(|| finite_number(object.get("step")));
    let mut extra = object.clone();
    for key in ["epoch", "step", "loss", "lr"] {
        extra.remove(key);
    }
    Sample {
        x,
        loss: finite_number(object.get("loss")),
        lr: finite_number(object.get("lr")),
        extra,
    }
}

fn finite_number(value: Option<&Value>) -> Option<f64> {
    value
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
}

fn split_recipe(series: &[Series]) -> (Map<String, Value>, Vec<String>) {
    let mut keys = BTreeSet::new();
    for item in series {
        keys.extend(item.recipe.keys().cloned());
    }
    let mut shared = Map::new();
    let mut varying = Vec::new();
    for key in keys {
        let values: Vec<Option<&Value>> = series.iter().map(|item| item.recipe.get(&key)).collect();
        let same = values.windows(2).all(|pair| match (pair[0], pair[1]) {
            (Some(left), Some(right)) => json_equal(left, right),
            _ => false,
        }) && values.first().is_some_and(|value| value.is_some());
        if same {
            if let Some(value) = values[0].cloned() {
                shared.insert(key, value);
            }
        } else {
            varying.push(key);
        }
    }
    (shared, varying)
}

/// Recipe keys in board order, then any key the file added.
pub fn recipe_keys(recipe: &Map<String, Value>) -> Vec<&str> {
    let mut keys: Vec<&str> = RECIPE_ORDER
        .iter()
        .copied()
        .filter(|key| recipe.contains_key(*key))
        .collect();
    let mut rest: Vec<&str> = recipe
        .keys()
        .map(String::as_str)
        .filter(|key| !RECIPE_ORDER.contains(key))
        .collect();
    rest.sort_unstable();
    keys.extend(rest);
    keys
}

/// Contiguous finite loss points. A gap starts a new segment.
/// On a log scale, a non-positive loss is a gap because it has no logarithm.
pub fn loss_segments(samples: &[Sample], log_scale: bool) -> Vec<Vec<[f64; 2]>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for sample in samples {
        let point = plotted(sample, log_scale);
        match point {
            Some(point) => current.push(point),
            None => {
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

/// Samples on the low row, from `start` onward, at or under `ceiling`.
///
/// A non-positive loss is a gap. A loss above the ceiling is a gap on this
/// row. That spike is still a sample, and the main chart draws it.
pub fn band_segments(samples: &[Sample], start: f64, ceiling: f64) -> Vec<Vec<[f64; 2]>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for sample in samples {
        let Some(x) = sample.x else {
            continue;
        };
        if !x.is_finite() || x + f64::EPSILON < start {
            continue;
        }
        let drawable = sample
            .loss
            .filter(|loss| loss.is_finite() && *loss > 0.0 && *loss <= ceiling);
        match drawable {
            Some(loss) => current.push([x, loss]),
            None => {
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

/// Finite losses above the low-row ceiling, from `start` onward.
pub fn spikes_above(samples: &[Sample], start: f64, ceiling: f64) -> usize {
    samples
        .iter()
        .filter(|sample| match (sample.x, sample.loss) {
            (Some(x), Some(loss)) => {
                x.is_finite() && loss.is_finite() && x + f64::EPSILON >= start && loss > ceiling
            }
            _ => false,
        })
        .count()
}

/// The lowest finite loss in each nearest whole epoch, from `start` onward.
///
/// The point keeps the epoch of that sample. Consecutive epochs stay on one
/// line. A missing epoch is a gap. The raw samples are not replaced by this.
pub fn epoch_floor(samples: &[Sample], start: f64) -> Vec<Vec<[f64; 2]>> {
    let mut bins: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for sample in samples {
        let (Some(x), Some(loss)) = (sample.x, sample.loss) else {
            continue;
        };
        if !x.is_finite() || !loss.is_finite() || x + f64::EPSILON < start {
            continue;
        }
        let epoch = x.round() as i64;
        bins.entry(epoch)
            .and_modify(|(at, best)| {
                if loss < *best {
                    *at = x;
                    *best = loss;
                }
            })
            .or_insert((x, loss));
    }
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut previous: Option<i64> = None;
    for (epoch, (x, loss)) in bins {
        if previous.is_some_and(|last| epoch != last + 1) && !current.is_empty() {
            segments.push(std::mem::take(&mut current));
        }
        current.push([x, loss]);
        previous = Some(epoch);
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

fn plotted(sample: &Sample, log_scale: bool) -> Option<[f64; 2]> {
    let x = sample.x?;
    let loss = sample.loss?;
    if log_scale {
        if loss <= 0.0 {
            return None;
        }
        Some([x, loss.log10()])
    } else {
        Some([x, loss])
    }
}

/// The epoch stretch around the lows, and a linear ceiling that keeps a climb readable.
///
/// The ceiling is above the highest low and the highest last sample. A spike
/// above it is still in the samples. The main chart draws it.
pub fn low_band(board: &Board) -> Option<(f64, f64, f64)> {
    let mut earliest_low = f64::MAX;
    let mut latest = f64::MIN;
    let mut earliest = f64::MAX;
    let mut crest = 0.0_f64;
    let mut any = false;
    for series in &board.series {
        let placed = placed_marks(&series.samples);
        let Some(low) = placed
            .iter()
            .min_by(|left, right| left.loss.total_cmp(&right.loss))
        else {
            continue;
        };
        let Some(last) = placed.last() else {
            continue;
        };
        earliest_low = earliest_low.min(low.x);
        latest = latest.max(last.x);
        if let Some(first) = placed.first() {
            earliest = earliest.min(first.x);
        }
        crest = crest.max(low.loss).max(last.loss);
        any = true;
    }
    if !any {
        return None;
    }
    let start = (earliest_low - 1.0).max(earliest);
    let end = if latest > start { latest } else { start + 1.0 };
    let top = (crest * 2.2).max(1e-9);
    Some((start, end, top))
}

fn placed_marks(samples: &[Sample]) -> Vec<Mark> {
    samples
        .iter()
        .filter_map(|sample| {
            Some(Mark {
                x: sample.x?,
                loss: sample.loss?,
            })
        })
        .collect()
}

pub fn read_board(board: &Board) -> Reading {
    let series = board.series.iter().map(read_series).collect::<Vec<_>>();
    let deepest = lowest_names(&series, |item| item.low.as_ref().map(|mark| mark.loss));
    let quietest_end = lowest_names(&series, |item| item.last.as_ref().map(|mark| mark.loss));
    let ends_differ = !deepest.is_empty() && !quietest_end.is_empty() && deepest != quietest_end;
    let lever = lever_line(board);
    Reading {
        series,
        deepest,
        quietest_end,
        ends_differ,
        varying_keys: board.varying.clone(),
        lever,
    }
}

fn read_series(series: &Series) -> SeriesRead {
    let mut placed = Vec::new();
    let mut gaps = 0;
    let mut unplaced = 0;
    for sample in &series.samples {
        if sample.x.is_none() {
            unplaced += 1;
        }
        if sample.loss.is_none() {
            gaps += 1;
        }
        if let (Some(x), Some(loss)) = (sample.x, sample.loss) {
            placed.push(Mark { x, loss });
        }
    }
    let first = placed.first().cloned();
    let last = placed.last().cloned();
    let low = placed
        .iter()
        .min_by(|left, right| left.loss.total_cmp(&right.loss))
        .cloned();
    let climbed = match (&low, &last) {
        (Some(low), Some(last)) => last.x > low.x && last.loss.total_cmp(&low.loss).is_gt(),
        _ => false,
    };
    SeriesRead {
        name: series.name.clone(),
        samples: series.samples.len(),
        gaps,
        unplaced,
        first,
        low,
        last,
        climbed,
        summary_final: finite_number(series.summary.get("final_loss")),
    }
}

fn lowest_names(series: &[SeriesRead], pick: impl Fn(&SeriesRead) -> Option<f64>) -> Vec<String> {
    let mut best: Option<f64> = None;
    for item in series {
        let Some(value) = pick(item) else {
            continue;
        };
        best = Some(match best {
            None => value,
            Some(current) if value.total_cmp(&current).is_lt() => value,
            Some(current) => current,
        });
    }
    let Some(best) = best else {
        return Vec::new();
    };
    series
        .iter()
        .filter(|item| pick(item).is_some_and(|value| value.to_bits() == best.to_bits()))
        .map(|item| item.name.clone())
        .collect()
}

fn lever_line(board: &Board) -> String {
    let seeds: BTreeSet<i64> = board
        .series
        .iter()
        .filter_map(|series| series.seed)
        .collect();
    let seed_varies = seeds.len() > 1;
    let recipe_varies = !board.varying.is_empty();
    match (seed_varies, recipe_varies) {
        (true, false) => "Seed is the lever that changes. The recipe is shared.".to_string(),
        (false, true) => format!("The recipe changes in: {}.", board.varying.join(", ")),
        (true, true) => format!(
            "Seed changes, and the recipe changes in: {}.",
            board.varying.join(", ")
        ),
        (false, false) => "Nothing in the recipe or the seed differs.".to_string(),
    }
}

impl Reading {
    /// The ranking sentence, when the deepest low and the quietest end disagree.
    pub fn finding(&self) -> Option<String> {
        if !self.ends_differ {
            return None;
        }
        Some(format!(
            "The lowest sample is {}. The lowest last sample is {}. A ranking by the last sample would hide the deeper low.",
            self.deepest.join(", "),
            self.quietest_end.join(", ")
        ))
    }

    /// Sentences the sidecar cites. Each one names a series and a range.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![self.lever.clone()];
        for series in &self.series {
            lines.push(series_line(series));
            if let Some(final_loss) = series.summary_final {
                lines.push(format!(
                    "{}: training_summary.final_loss is {}. It is not a sample on the curve.",
                    series.name,
                    format_measure(final_loss)
                ));
            }
        }
        if let Some(finding) = self.finding() {
            lines.push(finding);
        }
        lines
    }
}

fn series_line(series: &SeriesRead) -> String {
    let mut line = format!("{}: {} samples", series.name, series.samples);
    if series.gaps > 0 {
        line.push_str(&format!(", {} gaps", series.gaps));
    }
    if series.unplaced > 0 {
        line.push_str(&format!(", {} without an epoch or step", series.unplaced));
    }
    if let Some(first) = &series.first {
        line.push_str(&format!(
            ". Loss {} at epoch {}",
            format_measure(first.loss),
            format_measure(first.x)
        ));
    }
    if let Some(low) = &series.low {
        line.push_str(&format!(
            ", lowest {} at epoch {}",
            format_measure(low.loss),
            format_measure(low.x)
        ));
    }
    if let Some(last) = &series.last {
        line.push_str(&format!(
            ", last sample {} at epoch {}",
            format_measure(last.loss),
            format_measure(last.x)
        ));
    }
    if series.climbed {
        line.push_str(". The line climbs after the low");
    }
    line.push('.');
    line
}

/// Stable identity of the shared recipe. Paths are not part of it.
pub fn fingerprint(board: &Board) -> String {
    let mut object = Map::new();
    let mut keys: Vec<&String> = board.shared.keys().collect();
    keys.sort();
    for key in keys {
        if let Some(value) = board.shared.get(key) {
            object.insert(key.clone(), value.clone());
        }
    }
    object.insert(
        "varying".to_string(),
        Value::Array(board.varying.iter().cloned().map(Value::String).collect()),
    );
    serde_json::to_string(&object).expect("a recipe should serialize")
}

/// Remember a reading for this recipe. The note stays in the preferences directory.
pub fn remember(directory: &Path, board: &Board, text: &str) -> Result<(), std::io::Error> {
    let mut notes = read_notes(directory);
    let key = fingerprint(board);
    notes.retain(|note| note.fingerprint != key);
    notes.insert(
        0,
        Note {
            fingerprint: key,
            text: text.to_string(),
        },
    );
    notes.truncate(20);
    let mut memory = read_memory(directory);
    memory.insert("notes".to_string(), notes_value(&notes));
    write_memory(directory, &memory)
}

/// The memory file as one object. Each part (notes, weighings) keeps its own key.
pub(crate) fn read_memory(directory: &Path) -> Map<String, Value> {
    fs::read(directory.join(MEMORY_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| match value {
            Value::Object(object) => Some(object),
            _ => None,
        })
        .unwrap_or_default()
}

/// Write the whole memory object back. A part this call did not change is kept.
pub(crate) fn write_memory(
    directory: &Path,
    memory: &Map<String, Value>,
) -> Result<(), std::io::Error> {
    fs::create_dir_all(directory)?;
    let body = serde_json::to_string_pretty(&Value::Object(memory.clone()))
        .expect("memory should serialize");
    fs::write(directory.join(MEMORY_FILE), body + "\n")
}

/// The last reading stored for this recipe, when the fingerprint matches.
pub fn recall(directory: &Path, board: &Board) -> Option<String> {
    let key = fingerprint(board);
    read_notes(directory)
        .into_iter()
        .find(|note| note.fingerprint == key)
        .map(|note| note.text)
}

/// Model notes from other recipes that used the same method. At most three.
/// The open recipe is skipped. A note with no model half is skipped.
pub fn earlier_readings(directory: &Path, board: &Board) -> Vec<String> {
    let key = fingerprint(board);
    let Some(method) = fingerprint_method(&key) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for note in read_notes(directory) {
        if note.fingerprint == key {
            continue;
        }
        let Some(other) = fingerprint_method(&note.fingerprint) else {
            continue;
        };
        if !json_equal(&method, &other) {
            continue;
        }
        let Some((_, rest)) = note.text.split_once("\n\n") else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }
        found.push(rest.to_string());
        if found.len() == 3 {
            break;
        }
    }
    found
}

fn fingerprint_method(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| value.get("method").cloned())
}

struct Note {
    fingerprint: String,
    text: String,
}

fn notes_value(notes: &[Note]) -> Value {
    Value::Array(
        notes
            .iter()
            .map(|note| {
                Value::Object(Map::from_iter([
                    (
                        "fingerprint".to_string(),
                        Value::String(note.fingerprint.clone()),
                    ),
                    ("text".to_string(), Value::String(note.text.clone())),
                ]))
            })
            .collect(),
    )
}

fn read_notes(directory: &Path) -> Vec<Note> {
    let memory = read_memory(directory);
    let Some(items) = memory.get("notes").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            Some(Note {
                fingerprint: object.get("fingerprint")?.as_str()?.to_string(),
                text: object.get("text")?.as_str()?.to_string(),
            })
        })
        .collect()
}

/// The question for the optional orientation note. It carries no sample and no path.
///
/// The report already states the measurements. This question asks for one paragraph
/// a newcomer can use, with no digit and no advice.
pub fn sidecar_prompt(_reading: &Reading, _board: &Board, _earlier: &[String]) -> String {
    let mut prompt = String::new();
    prompt.push_str("The report is already written from the measurements. You may add one short paragraph on how to read a comparison of training runs.\n");
    prompt.push_str(
        "Use no digit. Do not name a setting. Do not name a winner. Do not give advice.\n",
    );
    prompt.push_str("If you cannot do that, say nothing.\n");
    prompt
}

/// Display text for a recipe value. Arrays of names stay separate marks.
pub fn recipe_marks(value: &Value) -> Option<Vec<String>> {
    let items = value.as_array()?;
    if items.is_empty() || !items.iter().all(|item| item.as_str().is_some()) {
        return None;
    }
    Some(
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
    )
}

pub fn recipe_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number
            .as_f64()
            .map(format_measure)
            .unwrap_or_else(|| number.to_string()),
        Value::Null => "—".to_string(),
        Value::Array(items) => items.iter().map(recipe_text).collect::<Vec<_>>().join("  "),
        Value::Object(_) => serde_json::to_string(value).expect("a JSON value should serialize"),
    }
}

pub fn recipe_label(key: &str) -> &str {
    match key {
        "learning_rate" => "learning rate",
        "lr_scheduler" => "schedule",
        "warmup_steps" => "warmup",
        "weight_decay" => "weight decay",
        "max_grad_norm" => "grad clip",
        "per_device_batch" => "batch",
        "grad_accum" => "accumulation",
        "effective_batch" => "effective batch",
        "max_seq_len" => "sequence",
        "prompt_loss_weight" => "prompt loss",
        "lora_r" => "LoRA rank",
        "lora_alpha" => "LoRA alpha",
        "lora_dropout" => "LoRA dropout",
        "checkpoint_epochs" => "checkpoints",
        "target_modules" => "target modules",
        other => other,
    }
}
