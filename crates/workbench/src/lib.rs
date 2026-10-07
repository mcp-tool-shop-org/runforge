//! The sidecar workbench, shared by RunForge and ScalarScope.
//!
//! A local model investigates the open runs by calling tools. It builds formula tools
//! and proposes hypotheses about what each knob does. The program computes every
//! number, sets every verdict, and writes every sentence that carries a number.
//!
//! The crate knows nothing about losses or latencies: a host implements [`Host`]
//! to name its measures and compute them, and hands over a [`Board`] of runs.
//! It does not open a window, read a run file, or call a model other than a local
//! Ollama on 127.0.0.1.

mod bench;
mod board;
mod expr;
mod format;
pub mod ollama;
mod session;
#[cfg(test)]
mod testing;

pub use bench::{
    ALPHA, Arm, Book, CHECKPOINT_EVERY, Checkpoint, Column, Direction, Evaluation, Evidence,
    Experiment, FDR, Hypothesis, Judged, KnobComparison, LearnedTool, Noise, Proposal, State,
    Verdict, allowed_names, compare_knob, evaluate, evidence, exact_p, experiment_for, lambda_for,
    learn_tool, note_new_folder, note_use, parse_with_library, permutation_e, propose, read_book,
    read_hypotheses, read_tools, reason_allowed, record, seed_noise, seeds_needed,
    test as test_hypothesis, test_all, threshold, verdicts, wording_problem,
    wording_problem_naming, write_book, write_hypotheses, write_tools,
};
pub use board::{Board, Host, Run, split_knobs};
pub use expr::{BUILTINS, Expr, Measure, canonical, catalogue, eval, parse, parse_open, quantile};
pub use format::{format_measure, knob_text, read_memory, write_memory};
pub use ollama::{BenchReply, start_bench, start_bench_on};
pub use session::{
    BUILD_ROUND, MAX_CALLS, MAX_CALLS_PER_ROUND, MAX_ROUNDS, NOTE_LIMIT, Phase, Step, Workbench,
    direction_of,
};
