//! What RunForge reads.
//!
//! A backpropagate `run_history.json`, or a folder of run-config series.
//! The library does not train, spawn a process, open a window, or call a model.
//! A bad file is a [`HistoryError`]. It is not a panic.

mod error;
mod export;
mod history;
mod ledger;
mod parse;
mod prefs;
mod report;
mod series;
mod time;
mod weigh;

pub use error::HistoryError;
pub use export::{curve_csv, curve_segments, entry_json, finite_points, format_f64, list_csv};
pub use history::{
    EvalSummary, History, HyperDiff, LossSample, RunEntry, hyperparameter_diffs, load_bytes,
    load_folder, load_text, pick_best_loss,
};
pub use ledger::{Ledger, RunMark, Weighed, ledger_for, record_weighing, weighed_now};
pub use prefs::{PREFS_FILE, Prefs, Theme, choose_prefs_dir, read_prefs, write_prefs};
pub use report::{
    ORIENTATION_OMITTED, comparison_report, comparison_report_with, is_heading,
    orientation_allowed, orientation_omission, report_file_name, utc_date,
};
pub use series::{
    Board, Mark, Reading, Sample, Series, SeriesRead, band_segments, earlier_readings, epoch_floor,
    fingerprint, format_measure, load_series_folder, loss_segments, low_band, read_board, recall,
    recipe_keys, recipe_label, recipe_marks, recipe_text, remember, sidecar_prompt, spikes_above,
};
pub use weigh::{Card, Neighborhood, Separation, Spread, Weighing, spread, weigh};

pub const VERSION: &str = "2.0.0";
