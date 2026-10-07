//! RunForge's side of the shared workbench.
//!
//! The formulas, hypotheses, evidence and session live in the `workbench` crate,
//! which ScalarScope shares. This module hands it RunForge's runs: a [`Board`] of
//! series becomes a workbench board whose measures are read from the loss curves
//! (`expr::LossHost`), whose knobs are the recipes, and whose memory is
//! `sidecar-memory.json` beside the preferences. The functions here keep the
//! names and signatures RunForge had before the move.

use std::path::Path;
use std::sync::Arc;

pub use workbench::{
    ALPHA, Arm, Book, CHECKPOINT_EVERY, Checkpoint, Column, Direction, Evaluation, Evidence,
    Experiment, FDR, Hypothesis, KnobComparison, LearnedTool, Noise, Proposal, State, Verdict,
    evidence, exact_p, lambda_for, note_new_folder, permutation_e, reason_allowed, record,
    seeds_needed, threshold, verdicts, wording_problem,
};

use crate::expr::LossHost;
use crate::ledger::board_key;
use crate::series::{Board, MEMORY_FILE, Series, recipe_label, recipe_text};

/// The workbench's view of this board: one run per series, keyed and filed as RunForge files them.
pub fn bench_board(board: &Board) -> workbench::Board {
    workbench::Board {
        runs: board
            .series
            .iter()
            .map(|series| workbench::Run {
                name: series.name.clone(),
                seed: series.seed,
                knobs: series.recipe.clone(),
                identity: run_fingerprint(series),
            })
            .collect(),
        shared: board.shared.clone(),
        varying: board.varying.clone(),
        key: board_key(board),
        method: board_method(board),
        host: Arc::new(LossHost::new(board.series.clone())),
    }
}

/// A hypothesis worded with RunForge's knob labels.
pub trait Statement {
    fn statement(&self) -> String;
}

impl Statement for Hypothesis {
    fn statement(&self) -> String {
        self.statement_with(recipe_label(&self.knob))
    }
}

fn memory(directory: &Path) -> std::path::PathBuf {
    directory.join(MEMORY_FILE)
}

/// Evaluate a formula (or a learned tool's name) on every run.
pub fn evaluate(board: &Board, text: &str, library: &[LearnedTool]) -> Result<Column, String> {
    workbench::evaluate(&bench_board(board), text, library)
}

/// The spread of a measure across runs that share the whole recipe.
pub fn seed_noise(board: &Board, column: &Column) -> Option<Noise> {
    workbench::seed_noise(&bench_board(board), column)
}

/// Compare a knob that varies on this board.
pub fn compare_knob(board: &Board, knob: &str, column: &Column) -> Result<KnobComparison, String> {
    workbench::compare_knob(&bench_board(board), knob, column)
}

/// Test one hypothesis on this board.
pub fn test(
    board: &Board,
    hypothesis: &Hypothesis,
    library: &[LearnedTool],
    date: &str,
) -> (Evaluation, Option<KnobComparison>) {
    workbench::test_hypothesis(&bench_board(board), hypothesis, library, date)
}

/// Test every hypothesis that applies to this board.
pub fn test_all(
    board: &Board,
    hypotheses: &[Hypothesis],
    library: &[LearnedTool],
    date: &str,
) -> Vec<Evaluation> {
    workbench::test_all(&bench_board(board), hypotheses, library, date)
}

/// The smallest run plan that could settle a hypothesis.
pub fn experiment_for(
    board: &Board,
    hypothesis: &Hypothesis,
    noise: Option<&Noise>,
    delta: Option<f64>,
) -> Experiment {
    workbench::experiment_for(&bench_board(board), hypothesis, noise, delta)
}

/// The method field of the board's recipe, as a key for hypotheses.
pub fn board_method(board: &Board) -> String {
    board
        .shared
        .get("method")
        .map(recipe_text)
        .unwrap_or_else(|| "unknown method".to_string())
}

/// Keep a formula as a learned tool, after its gates.
pub fn learn_tool(
    board: &Board,
    library: &[LearnedTool],
    name: &str,
    formula: &str,
    meaning: &str,
    date: &str,
) -> Result<LearnedTool, String> {
    workbench::learn_tool(&bench_board(board), library, name, formula, meaning, date)
}

/// Count a use of a learned tool on a board. A second board makes it kept.
pub fn note_use(library: &mut [LearnedTool], name: &str, board: &Board) {
    workbench::note_use(library, name, &bench_board(board));
}

/// Register a hypothesis proposed on this board.
pub fn propose(
    board: &Board,
    existing: &[Hypothesis],
    library: &[LearnedTool],
    proposal: &Proposal<'_>,
    date: &str,
) -> Result<Hypothesis, String> {
    workbench::propose(&bench_board(board), existing, library, proposal, date)
}

/// The learned tools, newest first.
pub fn read_tools(directory: &Path) -> Vec<LearnedTool> {
    workbench::read_tools(&memory(directory))
}

pub fn write_tools(directory: &Path, tools: &[LearnedTool]) -> Result<(), std::io::Error> {
    workbench::write_tools(&memory(directory), tools)
}

pub fn read_hypotheses(directory: &Path) -> Vec<Hypothesis> {
    workbench::read_hypotheses(&memory(directory))
}

pub fn write_hypotheses(directory: &Path, hypotheses: &[Hypothesis]) -> Result<(), std::io::Error> {
    workbench::write_hypotheses(&memory(directory), hypotheses)
}

pub fn read_book(directory: &Path) -> Book {
    workbench::read_book(&memory(directory))
}

pub fn write_book(directory: &Path, book: &Book) -> Result<(), std::io::Error> {
    workbench::write_book(&memory(directory), book)
}

/// Samples hashed into a run's identity: enough to tell runs apart, few enough
/// that a run still training keeps its identity as it grows.
const IDENTITY_SAMPLES: usize = 32;

/// A run's identity: an FNV-1a hash of its seed and its first stored epochs and losses.
///
/// The same run opened from two folders, or again after it trained further, has
/// one identity, so its evidence is counted once. Two runs that share a seed and
/// a recipe but not their data have different identities.
pub fn run_fingerprint(series: &Series) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(&series.seed.unwrap_or(i64::MIN).to_le_bytes());
    for sample in series.samples.iter().take(IDENTITY_SAMPLES) {
        feed(&sample.x.unwrap_or(f64::NAN).to_bits().to_le_bytes());
        feed(&sample.loss.unwrap_or(f64::NAN).to_bits().to_le_bytes());
    }
    format!("{hash:016x}")
}
