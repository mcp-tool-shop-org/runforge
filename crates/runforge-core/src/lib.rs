//! The bench's reading of a backpropagate `run_history.json`.
//!
//! The library does not train, spawn a process, or open a window.
//! A bad file is a [`HistoryError`]. It is not a panic.

mod error;
mod export;
mod history;
mod parse;
mod prefs;
mod time;

pub use error::HistoryError;
pub use export::{curve_csv, curve_segments, entry_json, finite_points, format_f64, list_csv};
pub use history::{
    EvalSummary, History, HyperDiff, LossSample, RunEntry, hyperparameter_diffs, load_bytes,
    load_folder, load_text, pick_best_loss,
};
pub use prefs::{PREFS_FILE, Prefs, Theme, choose_prefs_dir, read_prefs, write_prefs};

pub const VERSION: &str = "2.0.0";
