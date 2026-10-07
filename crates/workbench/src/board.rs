//! What a host hands the workbench: its runs, their knobs, and how to measure one.
//!
//! The workbench knows nothing about losses or latencies. A host (RunForge, ScalarScope)
//! names its measures and computes them; the workbench parses formulas over them, tests
//! hypotheses about knobs, and runs the session.

use std::fmt;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::expr::Measure;

/// What a host supplies. Everything here is about the host's own data and words.
pub trait Host: Send + Sync {
    /// The program's name, as the model is told it, such as "RunForge".
    fn program(&self) -> &str;
    /// What the runs are, in a few words, such as "fine-tuning runs".
    fn subject(&self) -> &str;
    /// The build phase's example: one sentence naming two tools in this host's measures.
    fn build_example(&self) -> &str;
    /// The host's own measures. `knob` and the math functions are added by the workbench.
    fn measures(&self) -> &[Measure];
    /// One host measure on one run, with its arguments already evaluated.
    fn measure(&self, run: usize, name: &str, args: &[f64]) -> Result<f64, String>;
    /// A knob's label in sentences.
    fn knob_label(&self, key: &str) -> String {
        key.to_string()
    }
    /// How the `why` of a hypothesis is asked for, in the tool schema.
    fn reason_hint(&self) -> &str {
        "The mechanism you suspect, in words, with no numbers."
    }
    /// How the closing note is asked for, in the tool schema.
    fn note_hint(&self) -> &str {
        "What you looked at and what is still open, in words, with no numbers."
    }
    /// Knob keys in the order the opening lists them.
    fn knob_keys<'a>(&self, knobs: &'a Map<String, Value>) -> Vec<&'a str> {
        let mut keys: Vec<&str> = knobs.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys
    }
}

/// One run, as the workbench sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub name: String,
    pub seed: Option<i64>,
    /// The run's knobs. A numeric one can be read in a formula with `knob('name')`.
    pub knobs: Map<String, Value>,
    /// A stable identity, computed by the host from the run's own data.
    pub identity: String,
}

/// The runs open together.
#[derive(Clone)]
pub struct Board {
    pub runs: Vec<Run>,
    /// Knobs with one value on every run.
    pub shared: Map<String, Value>,
    /// Knobs that differ between runs.
    pub varying: Vec<String>,
    /// The board's identity: a learned tool used on two keys is kept, and a hypothesis
    /// keeps one evaluation per key.
    pub key: String,
    /// The family hypotheses are filed under, such as a training method.
    pub method: String,
    pub host: Arc<dyn Host>,
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Board")
            .field("runs", &self.runs)
            .field("shared", &self.shared)
            .field("varying", &self.varying)
            .field("key", &self.key)
            .field("method", &self.method)
            .field("host", &self.host.program())
            .finish()
    }
}

impl Board {
    pub fn label(&self, knob: &str) -> String {
        self.host.knob_label(knob)
    }
}

/// Split knobs into the ones every run shares and the ones that vary, in first-seen order.
pub fn split_knobs(runs: &[Run]) -> (Map<String, Value>, Vec<String>) {
    let mut keys: Vec<String> = Vec::new();
    for run in runs {
        for key in run.knobs.keys() {
            if !keys.contains(key) {
                keys.push(key.clone());
            }
        }
    }
    let mut shared = Map::new();
    let mut varying = Vec::new();
    for key in keys {
        let first = runs.first().and_then(|run| run.knobs.get(&key));
        if runs.iter().all(|run| run.knobs.get(&key) == first) {
            if let Some(value) = first {
                shared.insert(key, value.clone());
            }
        } else {
            varying.push(key);
        }
    }
    (shared, varying)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(name: &str, knobs: &[(&str, Value)]) -> Run {
        Run {
            name: name.to_string(),
            seed: None,
            knobs: knobs
                .iter()
                .map(|(key, value)| ((*key).to_string(), value.clone()))
                .collect(),
            identity: name.to_string(),
        }
    }

    #[test]
    fn knobs_split_into_shared_and_varying_in_first_seen_order() {
        let runs = vec![
            run(
                "a",
                &[
                    ("batch", Value::from(1)),
                    ("precision", Value::from("fp16")),
                ],
            ),
            run(
                "b",
                &[
                    ("batch", Value::from(8)),
                    ("precision", Value::from("fp16")),
                ],
            ),
            run(
                "c",
                &[
                    ("batch", Value::from(8)),
                    ("precision", Value::from("fp16")),
                    ("trt", Value::from(true)),
                ],
            ),
        ];
        let (shared, varying) = split_knobs(&runs);
        assert_eq!(shared.len(), 1);
        assert_eq!(shared["precision"], Value::from("fp16"));
        // A knob missing on some runs varies: absent is a setting of its own.
        assert_eq!(varying, vec!["batch".to_string(), "trt".to_string()]);
        assert_eq!(split_knobs(&[]), (Map::new(), Vec::new()));
    }

    #[test]
    fn a_board_prints_its_host_by_name_and_labels_through_it() {
        let board = crate::testing::seeds_board();
        let printed = format!("{board:?}");
        assert!(printed.contains("host: \"Test\""), "{printed}");
        assert_eq!(board.label("method"), "method");
        assert_eq!(board.host.knob_keys(&board.shared), vec!["method"]);
    }
}
