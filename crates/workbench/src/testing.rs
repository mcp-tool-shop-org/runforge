//! A host for the crate's own tests: each run has one value per named measure.

use std::sync::Arc;

use serde_json::{Map, Value};

use crate::board::{Board, Host, Run, split_knobs};
use crate::expr::Measure;

pub(crate) struct Table {
    pub values: Vec<Vec<(&'static str, f64)>>,
}

const MEASURES: &[Measure] = &[
    Measure {
        name: "low",
        args: "",
        means: "the lowest value",
    },
    Measure {
        name: "last",
        args: "",
        means: "the last value",
    },
    Measure {
        name: "between",
        args: "a, b",
        means: "a windowed value, a + b",
    },
];

impl Host for Table {
    fn program(&self) -> &str {
        "Test"
    }
    fn subject(&self) -> &str {
        "test runs"
    }
    fn build_example(&self) -> &str {
        "For example, last / low."
    }
    fn measures(&self) -> &[Measure] {
        MEASURES
    }
    fn measure(&self, run: usize, name: &str, args: &[f64]) -> Result<f64, String> {
        if name == "between" {
            return Ok(args[0] + args[1]);
        }
        self.values[run]
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value)
            .ok_or_else(|| format!("run {run} has no {name}."))
    }
}

/// One run: name, seed, knobs, low, last.
pub(crate) type Row<'a> = (&'a str, i64, &'a [(&'a str, Value)], f64, f64);

/// A board from rows.
pub(crate) fn board(runs: &[Row<'_>]) -> Board {
    let built: Vec<Run> = runs
        .iter()
        .map(|(name, seed, knobs, _, _)| Run {
            name: (*name).to_string(),
            seed: Some(*seed),
            knobs: knobs
                .iter()
                .map(|(key, value)| ((*key).to_string(), value.clone()))
                .collect::<Map<String, Value>>(),
            identity: format!("{name}-{seed}"),
        })
        .collect();
    let (shared, varying) = split_knobs(&built);
    let key = built
        .iter()
        .map(|run| run.identity.clone())
        .collect::<Vec<_>>()
        .join("\n");
    Board {
        runs: built,
        shared,
        varying,
        key,
        method: "test".to_string(),
        host: Arc::new(Table {
            values: runs
                .iter()
                .map(|(_, _, _, low, last)| vec![("low", *low), ("last", *last)])
                .collect(),
        }),
    }
}

/// Two seeds of one recipe: only the seed differs.
pub(crate) fn seeds_board() -> Board {
    let method: &[(&str, Value)] = &[("method", Value::from("bf16 LoRA"))];
    board(&[
        ("seed 1", 1, method, 0.5, 0.6),
        ("seed 2", 2, method, 1.0, 1.1),
    ])
}
