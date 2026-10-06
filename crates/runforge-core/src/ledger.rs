//! The weighing ledger. What was measured, kept beside the model notes.
//!
//! A model note is a paraphrase. The ledger keeps the numbers the weighing
//! measured, so a later report can set these runs beside earlier runs of the
//! same recipe or the same method. It lives in the same memory file as the
//! notes, under its own key. It is not a database, and nothing leaves the disk.

use std::path::Path;

use serde_json::{Map, Value};

use crate::series::{Board, fingerprint, read_board, read_memory, write_memory};
use crate::weigh::weigh;

const KEY: &str = "weighings";
const KEEP: usize = 40;
const SHOWN: usize = 3;

/// One run as the weighing measured it.
#[derive(Clone, Debug, PartialEq)]
pub struct RunMark {
    pub name: String,
    pub low: f64,
    pub at: f64,
    pub median: f64,
    pub q1: f64,
    pub q3: f64,
    pub last: Option<f64>,
}

/// One weighing of one set of runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Weighed {
    /// The UTC day this set was first weighed with these numbers.
    pub date: String,
    pub fingerprint: String,
    pub runs: Vec<RunMark>,
    pub abstain: bool,
}

impl Weighed {
    /// The recipe plus the run names. The same folder opened twice has the same key.
    fn key(&self) -> String {
        let mut names: Vec<&str> = self.runs.iter().map(|run| run.name.as_str()).collect();
        names.sort_unstable();
        format!("{}\n{}", self.fingerprint, names.join("\n"))
    }

    fn same_numbers(&self, other: &Weighed) -> bool {
        self.runs == other.runs && self.abstain == other.abstain
    }

    pub fn method(&self) -> Option<Value> {
        recipe_field(&self.fingerprint, "method")
    }

    /// The recipe fields that changed between these runs. Empty when only the seed did.
    pub fn varying(&self) -> Vec<String> {
        recipe_field(&self.fingerprint, "varying")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect()
    }

    /// The run with the deepest single sample.
    pub fn deepest(&self) -> Option<&RunMark> {
        self.runs.iter().min_by(|a, b| a.low.total_cmp(&b.low))
    }

    /// The run with the lowest window middle.
    pub fn quietest(&self) -> Option<&RunMark> {
        self.runs
            .iter()
            .min_by(|a, b| a.median.total_cmp(&b.median))
    }

    /// Lowest and highest window middle.
    pub fn middles(&self) -> Option<(f64, f64)> {
        let low = self
            .runs
            .iter()
            .map(|run| run.median)
            .fold(f64::INFINITY, f64::min);
        let high = self
            .runs
            .iter()
            .map(|run| run.median)
            .fold(f64::NEG_INFINITY, f64::max);
        (low.is_finite() && high.is_finite()).then_some((low, high))
    }
}

/// Earlier weighings that bear on this board. Read it before recording the board.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ledger {
    /// The same runs, weighed before.
    pub same_runs: Option<Weighed>,
    /// Other sets of runs with this exact recipe. Newest first, at most three.
    pub same_recipe: Vec<Weighed>,
    /// Other recipes with the same method. Newest first, at most three.
    pub same_method: Vec<Weighed>,
}

impl Ledger {
    pub fn is_empty(&self) -> bool {
        self.same_runs.is_none() && self.same_recipe.is_empty() && self.same_method.is_empty()
    }
}

/// The recipe plus the run names: the same folder opened twice has the same key.
pub fn board_key(board: &Board) -> String {
    let mut names: Vec<&str> = board
        .series
        .iter()
        .map(|series| series.name.as_str())
        .collect();
    names.sort_unstable();
    format!("{}\n{}", fingerprint(board), names.join("\n"))
}

/// The board as weighed today.
pub fn weighed_now(board: &Board, date: &str) -> Weighed {
    let weighing = weigh(board);
    let reading = read_board(board);
    let runs = weighing
        .neighborhoods
        .iter()
        .map(|near| RunMark {
            name: near.name.clone(),
            low: near.low,
            at: near.at,
            median: near.median,
            q1: near.q1,
            q3: near.q3,
            last: reading
                .series
                .iter()
                .find(|series| series.name == near.name)
                .and_then(|series| series.last.as_ref())
                .map(|mark| mark.loss),
        })
        .collect();
    Weighed {
        date: date.to_string(),
        fingerprint: fingerprint(board),
        runs,
        abstain: weighing.abstain,
    }
}

/// What the ledger already holds about this board.
pub fn ledger_for(directory: &Path, board: &Board) -> Ledger {
    let now = weighed_now(board, "");
    let key = now.key();
    let method = now.method();
    let mut ledger = Ledger::default();
    for item in read_ledger(directory) {
        if item.key() == key {
            ledger.same_runs.get_or_insert(item);
        } else if item.fingerprint == now.fingerprint {
            if ledger.same_recipe.len() < SHOWN {
                ledger.same_recipe.push(item);
            }
        } else if method.is_some() && item.method() == method && ledger.same_method.len() < SHOWN {
            ledger.same_method.push(item);
        }
    }
    ledger
}

/// Keep this weighing. The same runs with the same numbers keep their first date.
pub fn record_weighing(directory: &Path, weighed: &Weighed) -> Result<(), std::io::Error> {
    if weighed.runs.is_empty() {
        return Ok(());
    }
    let mut items = read_ledger(directory);
    let key = weighed.key();
    if items
        .iter()
        .any(|item| item.key() == key && item.same_numbers(weighed))
    {
        return Ok(());
    }
    items.retain(|item| item.key() != key);
    items.insert(0, weighed.clone());
    items.truncate(KEEP);
    let mut memory = read_memory(directory);
    memory.insert(
        KEY.to_string(),
        Value::Array(items.iter().map(to_value).collect()),
    );
    write_memory(directory, &memory)
}

fn read_ledger(directory: &Path) -> Vec<Weighed> {
    read_memory(directory)
        .get(KEY)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(from_value).collect())
        .unwrap_or_default()
}

fn recipe_field(fingerprint: &str, field: &str) -> Option<Value> {
    serde_json::from_str::<Value>(fingerprint)
        .ok()
        .and_then(|value| value.get(field).cloned())
}

fn number(value: f64) -> Value {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn to_value(item: &Weighed) -> Value {
    let runs = item
        .runs
        .iter()
        .map(|run| {
            let mut object = Map::new();
            object.insert("name".into(), Value::String(run.name.clone()));
            object.insert("low".into(), number(run.low));
            object.insert("at".into(), number(run.at));
            object.insert("median".into(), number(run.median));
            object.insert("q1".into(), number(run.q1));
            object.insert("q3".into(), number(run.q3));
            object.insert("last".into(), run.last.map(number).unwrap_or(Value::Null));
            Value::Object(object)
        })
        .collect();
    let mut object = Map::new();
    object.insert("date".into(), Value::String(item.date.clone()));
    object.insert(
        "fingerprint".into(),
        Value::String(item.fingerprint.clone()),
    );
    object.insert("abstain".into(), Value::Bool(item.abstain));
    object.insert("runs".into(), Value::Array(runs));
    Value::Object(object)
}

fn from_value(value: &Value) -> Option<Weighed> {
    let object = value.as_object()?;
    let runs = object
        .get("runs")?
        .as_array()?
        .iter()
        .map(|run| {
            let run = run.as_object()?;
            let field = |name: &str| run.get(name).and_then(Value::as_f64);
            Some(RunMark {
                name: run.get("name")?.as_str()?.to_string(),
                low: field("low")?,
                at: field("at")?,
                median: field("median")?,
                q1: field("q1")?,
                q3: field("q3")?,
                last: field("last"),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Weighed {
        date: object.get("date")?.as_str()?.to_string(),
        fingerprint: object.get("fingerprint")?.as_str()?.to_string(),
        runs,
        abstain: object.get("abstain")?.as_bool()?,
    })
}
