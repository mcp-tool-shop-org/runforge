//! What RunForge reads.
//!
//! A backpropagate `run_history.json`, or a folder of run-config series.
//! The library does not train, spawn a process, open a window, or call a model.
//! A bad file is a [`HistoryError`]. It is not a panic.

mod bench;
mod error;
mod export;
mod expr;
mod history;
mod ledger;
mod parse;
mod prefs;
mod report;
mod series;
mod session;
mod time;
mod weigh;

pub use bench::{
    ALPHA, Arm, Column, Direction, Evaluation, Evidence, Experiment, FDR, Hypothesis,
    KnobComparison, LearnedTool, Noise, Proposal, State, Verdict, board_method, compare_knob,
    evaluate, evidence, exact_p, experiment_for, learn_tool, note_use, permutation_e, propose,
    read_hypotheses, read_tools, reason_allowed, record, run_fingerprint, seed_noise, seeds_needed,
    test as test_hypothesis, test_all, threshold, verdicts, wording_problem, write_hypotheses,
    write_tools,
};
pub use bench::{
    Book, CHECKPOINT_EVERY, Checkpoint, lambda_for, note_new_folder, read_book, write_book,
};
pub use error::HistoryError;
pub use export::{curve_csv, curve_segments, entry_json, finite_points, format_f64, list_csv};
pub use expr::{Expr, MEASURES, Measure, canonical, parse as parse_formula};
pub use history::{
    EvalSummary, History, HyperDiff, LossSample, RunEntry, hyperparameter_diffs, load_bytes,
    load_folder, load_text, pick_best_loss,
};
pub use ledger::{
    Ledger, RunMark, Weighed, board_key, known_runs, ledger_for, record_weighing, weighed_now,
};
pub use prefs::{PREFS_FILE, Prefs, Theme, choose_prefs_dir, read_prefs, write_prefs};
pub use report::{
    ORIENTATION_OMITTED, comparison_report, comparison_report_full, comparison_report_with,
    is_heading, orientation_allowed, orientation_omission, report_file_name, utc_date,
};
pub use series::{
    Board, Mark, Reading, Sample, Series, SeriesRead, band_segments, earlier_readings, epoch_floor,
    fingerprint, format_measure, load_series_folder, loss_segments, low_band, normalize_recipe,
    read_board, recall, recipe_keys, recipe_label, recipe_marks, recipe_text, remember,
    sidecar_prompt, spikes_above,
};
pub use session::{MAX_CALLS, MAX_ROUNDS, NOTE_LIMIT, Phase, Step, Workbench};
pub use weigh::{Card, Neighborhood, Separation, Spread, Weighing, spread, weigh};

pub const VERSION: &str = "2.0.0";
