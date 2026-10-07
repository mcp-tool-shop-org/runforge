//! The workbench: tools the sidecar can call, tools it has learned, and hypotheses
//! about what a knob does.
//!
//! Every number comes from here, computed from the stored samples. The model
//! chooses which tool to run, proposes formulas and hypotheses, and words them.
//! It does not compute, and it does not decide a verdict. A learned tool is a
//! formula in the language of `expr`, kept only after it passes the gates below.
//! A hypothesis has a state the program sets from a test declared when the
//! hypothesis was proposed, not from the data it is then tested on.

use std::path::Path;

use serde_json::{Map, Value};

use crate::board::Board;
use crate::expr::{self, Expr};
use crate::format::{format_measure, knob_text as recipe_text, read_memory, write_memory};

const TOOLS_KEY: &str = "tools";
const HYPOTHESES_KEY: &str = "hypotheses";
const MAX_TOOLS: usize = 50;
const MAX_HYPOTHESES: usize = 60;
const MAX_EVALUATIONS: usize = 40;
/// The tilt of the permutation e-value for a folder with these group sizes.
///
/// Fixed at design time from group sizes alone, so it is known before any folder
/// opens and the e-value stays valid. Each entry maximizes the expected log
/// e-value (growth rate) under a one-standard-deviation shift, by simulation
/// (3,000 draws per size, a grid of 0.5 to 8). A single fixed 8 shrank the
/// product on average for small folders even under a real effect (Kimi K3 review,
/// 2026-10-06, confirmed by that simulation).
pub fn lambda_for(n_low: usize, n_high: usize) -> f64 {
    let (small, large) = if n_low <= n_high {
        (n_low, n_high)
    } else {
        (n_high, n_low)
    };
    match (small, large) {
        (1, 1) => 1.0,
        (1, 2) => 1.5,
        (1, 3) => 2.0,
        (1, _) => 2.5,
        (2, 2) | (2, 3) => 3.0,
        (2, 4) | (3, 3) => 4.0,
        (2, _) | (3, 4) => 5.0,
        (3, 5) | (4, 4) => 6.0,
        _ => 8.0,
    }
}

/// How many new folders open between checkpoints. Fixed in advance: a checkpoint
/// is the only time a verdict is issued, so the reporting time is never chosen.
pub const CHECKPOINT_EVERY: u32 = 5;
/// The false discovery rate the bench's verdicts are held to.
pub const FDR: f64 = 0.05;
/// One-sided level for the exact rank test. With three runs per setting, 1/20 is the smallest p there is.
pub const ALPHA: f64 = 0.05;
/// z for a two-sided 0.05 test plus z for 80% power, squared and doubled (Colas et al. 2018).
const POWER_FACTOR: f64 = 2.0 * (1.959_964 + 0.841_621) * (1.959_964 + 0.841_621);

/// A formula the sidecar learned. It stays provisional until it has been useful on two folders.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnedTool {
    pub name: String,
    pub formula: String,
    pub meaning: String,
    pub created: String,
    pub uses: u32,
    /// Board keys this tool has been run on. Two different ones make it kept.
    pub boards: Vec<String>,
}

impl LearnedTool {
    pub fn kept(&self) -> bool {
        self.boards.len() >= 2
    }
}

/// One formula evaluated on every run of the board.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub formula: String,
    pub values: Vec<(String, Result<f64, String>)>,
}

impl Column {
    pub fn finite(&self) -> Vec<(String, f64)> {
        self.values
            .iter()
            .filter_map(|(name, value)| value.as_ref().ok().map(|value| (name.clone(), *value)))
            .collect()
    }

    /// One line per run, written by the program.
    pub fn lines(&self) -> Vec<String> {
        self.values
            .iter()
            .map(|(name, value)| match value {
                Ok(value) => format!("{name}: {}", format_measure(*value)),
                Err(reason) => format!("{name}: no value. {reason}"),
            })
            .collect()
    }
}

/// Parse a formula, with learned tool names standing for their own formulas.
pub fn parse_with_library(
    board: &Board,
    text: &str,
    library: &[LearnedTool],
) -> Result<Expr, String> {
    let expr = expr::parse_open(text, board.host.measures(), &|name| {
        library
            .iter()
            .find(|tool| tool.name == name)
            .map(|tool| tool.formula.clone())
    })?;
    Ok(expr)
}

/// Evaluate a formula (or a learned tool's name) on every run.
pub fn evaluate(board: &Board, text: &str, library: &[LearnedTool]) -> Result<Column, String> {
    let parsed = parse_with_library(board, text, library)?;
    Ok(Column {
        formula: text.trim().to_string(),
        values: board
            .runs
            .iter()
            .enumerate()
            .map(|(index, run)| (run.name.clone(), expr::eval(&parsed, board, index)))
            .collect(),
    })
}

/// How much one measure moves between runs of one recipe, when only the seed changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Noise {
    pub runs: usize,
    pub low: f64,
    pub high: f64,
    /// Sample standard deviation across runs, the pilot sigma for a seed count.
    pub sigma: f64,
}

impl Noise {
    pub fn range(&self) -> f64 {
        self.high - self.low
    }
}

/// The spread of a measure across runs that share the whole recipe. `None` when the recipe varies or fewer than two runs have a value.
pub fn seed_noise(board: &Board, column: &Column) -> Option<Noise> {
    if !board.varying.is_empty() {
        return None;
    }
    noise_of(
        &column
            .finite()
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>(),
    )
}

fn noise_of(values: &[f64]) -> Option<Noise> {
    if values.len() < 2 {
        return None;
    }
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    Some(Noise {
        runs: values.len(),
        low,
        high,
        sigma: var.sqrt(),
    })
}

/// Runs per setting to detect a difference `delta`, from a pilot sigma (Colas, Sigaud, and Oudeyer 2018).
///
/// Guidance for planning, not a test. Two-sided 0.05, power 0.8, normal approximation.
pub fn seeds_needed(sigma: f64, delta: f64) -> Option<u32> {
    if !(sigma.is_finite() && delta.is_finite()) || delta == 0.0 || sigma < 0.0 {
        return None;
    }
    let n = POWER_FACTOR * sigma * sigma / (delta * delta);
    Some(n.ceil().max(2.0) as u32)
}

/// Which way the measure moves when the knob goes up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Lower,
    Higher,
}

impl Direction {
    pub fn word(self) -> &'static str {
        match self {
            Direction::Lower => "lower",
            Direction::Higher => "higher",
        }
    }

    pub fn parse(text: &str) -> Option<Direction> {
        match text.trim().to_ascii_lowercase().as_str() {
            "lower" | "down" | "decrease" | "decreases" => Some(Direction::Lower),
            "higher" | "up" | "increase" | "increases" => Some(Direction::Higher),
            _ => None,
        }
    }
}

/// The runs at one value of the knob.
#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    pub level: String,
    pub sort: Option<f64>,
    pub values: Vec<(String, f64)>,
}

/// A knob's two outer settings, compared on one measure.
#[derive(Clone, Debug, PartialEq)]
pub struct KnobComparison {
    pub knob: String,
    pub low: Arm,
    pub high: Arm,
    /// Settings between the outer two, which this comparison does not use.
    pub middle: usize,
    /// Other recipe fields that also differ between runs.
    pub confounds: Vec<String>,
    /// Median of the high setting minus median of the low setting.
    pub gap: f64,
    /// The widest seed spread inside one setting, when each setting has two runs or more.
    pub noise: Option<f64>,
    /// Pairs (low run, high run) where the high run's value is larger, with ties as half.
    pub pairs_higher: f64,
    pub pairs: usize,
    /// One-sided exact p for "the measure is higher at the high setting", and for "lower".
    pub p_higher: f64,
    pub p_lower: f64,
}

impl KnobComparison {
    pub fn inside_noise(&self) -> Option<bool> {
        self.noise.map(|noise| self.gap.abs() <= noise)
    }
}

/// Compare a knob that varies on this board. Refused, with the reason, when it did not vary.
pub fn compare_knob(board: &Board, knob: &str, column: &Column) -> Result<KnobComparison, String> {
    let label = board.label(knob);
    if !board.varying.iter().any(|key| key == knob) {
        return Err(match board.shared.get(knob) {
            Some(value) => format!(
                "{} was {} on every run, so these runs hold no evidence about it.",
                capitalized(&label),
                recipe_text(value)
            ),
            None => format!("{} is not in these recipes.", capitalized(&label)),
        });
    }
    let mut arms: Vec<Arm> = Vec::new();
    for (series, (_, value)) in board.runs.iter().zip(&column.values) {
        let Ok(value) = value else { continue };
        let Some(setting) = series.knobs.get(knob) else {
            continue;
        };
        let level = recipe_text(setting);
        match arms.iter_mut().find(|arm| arm.level == level) {
            Some(arm) => arm.values.push((series.name.clone(), *value)),
            None => arms.push(Arm {
                level,
                sort: setting.as_f64(),
                values: vec![(series.name.clone(), *value)],
            }),
        }
    }
    if arms.len() < 2 {
        return Err(format!(
            "Fewer than two settings of {label} have a value for this measure."
        ));
    }
    arms.sort_by(|a, b| match (a.sort, b.sort) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        _ => a.level.cmp(&b.level),
    });
    let middle = arms.len() - 2;
    let high = arms.pop().expect("two arms");
    let low = arms.swap_remove(0);
    let confounds = board
        .varying
        .iter()
        .filter(|key| key.as_str() != knob)
        .map(|key| board.label(key))
        .collect();
    let low_values: Vec<f64> = low.values.iter().map(|(_, v)| *v).collect();
    let high_values: Vec<f64> = high.values.iter().map(|(_, v)| *v).collect();
    let gap = median(&high_values) - median(&low_values);
    let noise = if low_values.len() >= 2 && high_values.len() >= 2 {
        Some(range(&low_values).max(range(&high_values)))
    } else {
        None
    };
    let pairs_higher = pairs_above(&high_values, &low_values);
    Ok(KnobComparison {
        knob: knob.to_string(),
        pairs: low_values.len() * high_values.len(),
        p_higher: exact_p(&low_values, &high_values, Direction::Higher),
        p_lower: exact_p(&low_values, &high_values, Direction::Lower),
        low,
        high,
        middle,
        confounds,
        gap,
        noise,
        pairs_higher,
    })
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    expr::quantile(&sorted, 0.5).unwrap_or(f64::NAN)
}

fn range(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn pairs_above(high: &[f64], low: &[f64]) -> f64 {
    let mut count = 0.0;
    for h in high {
        for l in low {
            if h > l {
                count += 1.0;
            } else if h == l {
                count += 0.5;
            }
        }
    }
    count
}

/// Exact one-sided p of the Mann-Whitney statistic, by enumerating every relabeling (Mann and Whitney 1947).
///
/// `Higher` asks how often a relabeled high group sits at least this far above the low group.
pub fn exact_p(low: &[f64], high: &[f64], direction: Direction) -> f64 {
    let pool: Vec<f64> = low.iter().chain(high).copied().collect();
    let n = pool.len();
    let k = high.len();
    if low.is_empty() || high.is_empty() || n > 20 {
        return 1.0;
    }
    let statistic = |high: &[f64], low: &[f64]| match direction {
        Direction::Higher => pairs_above(high, low),
        Direction::Lower => pairs_above(low, high),
    };
    let observed = statistic(high, low);
    let mut at_least = 0u64;
    let mut total = 0u64;
    let mut pick = Vec::with_capacity(k);
    combinations(n, k, 0, &mut pick, &mut |chosen| {
        let (mut h, mut l) = (Vec::with_capacity(k), Vec::with_capacity(n - k));
        for (index, value) in pool.iter().enumerate() {
            if chosen.contains(&index) {
                h.push(*value);
            } else {
                l.push(*value);
            }
        }
        total += 1;
        if statistic(&h, &l) >= observed - 1e-12 {
            at_least += 1;
        }
    });
    at_least as f64 / total as f64
}

fn combinations(
    n: usize,
    k: usize,
    start: usize,
    pick: &mut Vec<usize>,
    visit: &mut dyn FnMut(&[usize]),
) {
    if pick.len() == k {
        visit(pick);
        return;
    }
    for index in start..n {
        if n - index < k - pick.len() {
            break;
        }
        pick.push(index);
        combinations(n, k, index + 1, pick, visit);
        pick.pop();
    }
}

/// Where a hypothesis stands on one board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The knob did not vary, or is not in the recipe.
    Untestable,
    /// Another setting changed with it.
    Confounded,
    /// The data cannot decide: one run per setting, a gap inside the seed spread, or too few runs for the test.
    Inconclusive,
    Supported,
    Refuted,
}

impl State {
    pub fn word(&self) -> &'static str {
        match self {
            State::Untestable => "not testable here",
            State::Confounded => "confounded",
            State::Inconclusive => "inconclusive",
            State::Supported => "passes its test on these runs alone",
            State::Refuted => "goes the other way on these runs alone",
        }
    }

    fn key(&self) -> &'static str {
        match self {
            State::Untestable => "untestable",
            State::Confounded => "confounded",
            State::Inconclusive => "inconclusive",
            State::Supported => "supported",
            State::Refuted => "refuted",
        }
    }

    fn from_key(text: &str) -> Option<State> {
        Some(match text {
            "untestable" => State::Untestable,
            "confounded" => State::Confounded,
            "inconclusive" => State::Inconclusive,
            "supported" => State::Supported,
            "refuted" => State::Refuted,
            _ => return None,
        })
    }
}

/// One test of a hypothesis on one board.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub date: String,
    pub board: String,
    pub state: State,
    /// The program's sentence: what was compared and why the state is what it is.
    pub detail: String,
    /// This folder's permutation e-value for the declared direction, and for the opposite one.
    /// `None` when the folder gives no test: the knob did not change, or another one changed with it.
    pub e_for: Option<f64>,
    pub e_against: Option<f64>,
    /// Fingerprints of the runs tested, so a run is never counted twice across folders.
    pub runs: Vec<String>,
}

/// A claim about what one knob does to one measure, with its test fixed when proposed.
#[derive(Clone, Debug, PartialEq)]
pub struct Hypothesis {
    pub id: String,
    /// The method the hypothesis was proposed under. Evidence from another method does not count.
    pub method: String,
    pub knob: String,
    pub formula: String,
    pub direction: Direction,
    /// The proposer's reason, fenced: no digit, no markdown, no verdict.
    pub why: String,
    pub proposed: String,
    /// The folder the hypothesis was proposed on. Its data shaped the claim, so it is not evidence for it.
    pub proposed_on: String,
    /// Every run RunForge knew when the hypothesis was registered. Any of them may
    /// have shaped the claim, so a folder holding one of them never counts.
    pub registered_runs: Vec<String>,
    pub evaluations: Vec<Evaluation>,
}

impl Hypothesis {
    /// The program's wording of the claim, with the knob called `label`.
    pub fn statement_with(&self, label: &str) -> String {
        format!(
            "When {} goes up, {} goes {}.",
            label,
            self.formula,
            self.direction.word()
        )
    }

    /// The program's wording of the claim, with the knob called what the host calls it.
    pub fn statement_on(&self, board: &Board) -> String {
        self.statement_with(&board.label(&self.knob))
    }

    /// The latest state, or `None` before its first test.
    pub fn state(&self) -> Option<&State> {
        self.evaluations.first().map(|evaluation| &evaluation.state)
    }
}

/// Why a piece of the model's wording is refused, or `None` when it may stand.
///
/// Wording may explain. It may not carry a digit, markdown, or a verdict word, and
/// it may not run past `limit` characters. Numbers belong to the program.
pub fn wording_problem(text: &str, limit: usize) -> Option<String> {
    if text.chars().any(|ch| ch.is_ascii_digit()) {
        return Some("it carried a digit, and numbers come only from the tools".to_string());
    }
    if text.contains("**") || text.contains('#') {
        return Some("it carried markdown".to_string());
    }
    if text.chars().count() > limit {
        return Some(format!("it ran past {limit} characters"));
    }
    const VERDICT: &[&str] = &[
        "proves",
        "proven",
        "best",
        "winner",
        "should",
        "recommend",
        "always",
        "never",
    ];
    let lower = text.to_lowercase();
    lower
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .find(|word| VERDICT.contains(word))
        .map(|word| format!("it carried the verdict word \"{word}\""))
}

/// A hypothesis's reason: at most 280 characters, no digit, markdown, or verdict.
pub fn reason_allowed(text: &str) -> bool {
    wording_problem(text, 280).is_none()
}

/// Test one hypothesis on this board. Holm's adjustment is applied across a batch by `test_all`.
pub fn test(
    board: &Board,
    hypothesis: &Hypothesis,
    library: &[LearnedTool],
    date: &str,
) -> (Evaluation, Option<KnobComparison>) {
    let key = board.key.clone();
    let runs: Vec<String> = board.runs.iter().map(|run| run.identity.clone()).collect();
    let done = |state: State, detail: String| Evaluation {
        date: date.to_string(),
        board: key.clone(),
        state,
        detail,
        e_for: None,
        e_against: None,
        runs: runs.clone(),
    };
    if board.method.clone() != hypothesis.method {
        return (
            done(
                State::Untestable,
                "These runs use a different method.".to_string(),
            ),
            None,
        );
    }
    let column = match evaluate(board, &hypothesis.formula, library) {
        Ok(column) => column,
        Err(reason) => return (done(State::Untestable, reason), None),
    };
    let comparison = match compare_knob(board, &hypothesis.knob, &column) {
        Ok(comparison) => comparison,
        Err(reason) => return (done(State::Untestable, reason), None),
    };
    let label = board.label(&hypothesis.knob);
    let head = format!(
        "{label} {} against {}: {} {} against {}, a gap of {}.",
        comparison.high.level,
        comparison.low.level,
        hypothesis.formula,
        format_measure(median(
            &comparison
                .high
                .values
                .iter()
                .map(|v| v.1)
                .collect::<Vec<_>>()
        )),
        format_measure(median(
            &comparison
                .low
                .values
                .iter()
                .map(|v| v.1)
                .collect::<Vec<_>>()
        )),
        format_measure(comparison.gap)
    );
    if !comparison.confounds.is_empty() {
        let detail = format!(
            "{head} {} changed as well, so the gap does not belong to {label} alone.",
            join(&comparison.confounds)
        );
        return (done(State::Confounded, detail), Some(comparison));
    }
    let smallest = comparison
        .low
        .values
        .len()
        .min(comparison.high.values.len());
    let (p_for, p_against) = match hypothesis.direction {
        Direction::Higher => (comparison.p_higher, comparison.p_lower),
        Direction::Lower => (comparison.p_lower, comparison.p_higher),
    };
    let state_detail = if smallest < 2 {
        (
            State::Inconclusive,
            "One run per setting, so there is no seed spread to weigh the gap against.".to_string(),
        )
    } else if comparison.inside_noise() == Some(true) {
        (
            State::Inconclusive,
            format!(
                "The gap is inside the seed spread within one setting ({}).",
                format_measure(comparison.noise.unwrap_or(f64::NAN))
            ),
        )
    } else if smallest < 3 {
        (
            State::Inconclusive,
            "The gap is wider than the seed spread, but a one-in-twenty test needs at least three runs per setting.".to_string(),
        )
    } else if p_for <= ALPHA {
        (
            State::Supported,
            format!(
                "Every comparison of runs agrees: exact rank test p = {}.",
                format_measure(p_for)
            ),
        )
    } else if p_against <= ALPHA {
        (
            State::Refuted,
            format!(
                "The runs move the other way: exact rank test p = {}.",
                format_measure(p_against)
            ),
        )
    } else {
        (
            State::Inconclusive,
            format!(
                "The rank test does not reach one in twenty (p = {}).",
                format_measure(p_for)
            ),
        )
    };
    (
        {
            let mut evaluation = done(state_detail.0, format!("{head} {}", state_detail.1));
            let low: Vec<f64> = comparison.low.values.iter().map(|v| v.1).collect();
            let high: Vec<f64> = comparison.high.values.iter().map(|v| v.1).collect();
            let opposite = match hypothesis.direction {
                Direction::Lower => Direction::Higher,
                Direction::Higher => Direction::Lower,
            };
            evaluation.e_for = permutation_e(&low, &high, hypothesis.direction);
            evaluation.e_against = permutation_e(&low, &high, opposite);
            evaluation
        },
        Some(comparison),
    )
}

/// Test every hypothesis that applies to this board. A supported or refuted verdict must survive Holm's adjustment across the batch (Holm 1979).
pub fn test_all(
    board: &Board,
    hypotheses: &[Hypothesis],
    library: &[LearnedTool],
    date: &str,
) -> Vec<Evaluation> {
    let mut results: Vec<(Evaluation, Option<f64>)> = hypotheses
        .iter()
        .map(|hypothesis| {
            let (evaluation, comparison) = test(board, hypothesis, library, date);
            let p = comparison.map(|c| match (&evaluation.state, hypothesis.direction) {
                (State::Refuted, Direction::Higher) | (State::Supported, Direction::Lower) => {
                    c.p_lower
                }
                _ => c.p_higher,
            });
            let p = match evaluation.state {
                State::Supported | State::Refuted => p,
                _ => None,
            };
            (evaluation, p)
        })
        .collect();
    let mut decided: Vec<(usize, f64)> = results
        .iter()
        .enumerate()
        .filter_map(|(index, (_, p))| p.map(|p| (index, p)))
        .collect();
    decided.sort_by(|a, b| a.1.total_cmp(&b.1));
    let m = hypotheses
        .iter()
        .zip(&results)
        .filter(|(_, (evaluation, _))| !matches!(evaluation.state, State::Untestable))
        .count();
    let mut failed = false;
    for (rank, (index, p)) in decided.into_iter().enumerate() {
        let limit = ALPHA / (m - rank).max(1) as f64;
        if failed || p > limit {
            failed = true;
            let evaluation = &mut results[index].0;
            evaluation.state = State::Inconclusive;
            evaluation.detail.push_str(&format!(
                " With {m} hypotheses tested on these runs, Holm's adjustment needs p at or under {}, so this stays inconclusive.",
                format_measure(limit)
            ));
        }
    }
    results
        .into_iter()
        .map(|(evaluation, _)| evaluation)
        .collect()
}

/// The smallest run plan that could settle a hypothesis: one knob, two settings, matched recipes.
#[derive(Clone, Debug, PartialEq)]
pub struct Experiment {
    pub knob: String,
    pub levels: Vec<String>,
    pub seeds_per_level: u32,
    pub runs: u32,
    pub matched: Vec<String>,
    pub settles: String,
    pub refutes_if: String,
}

/// Plan the next runs for a hypothesis. The app does not press Train; this is a proposal.
pub fn experiment_for(
    board: &Board,
    hypothesis: &Hypothesis,
    noise: Option<&Noise>,
    delta: Option<f64>,
) -> Experiment {
    let mut levels: Vec<String> = Vec::new();
    for series in &board.runs {
        if let Some(value) = series.knobs.get(&hypothesis.knob) {
            let text = recipe_text(value);
            if !levels.contains(&text) {
                levels.push(text);
            }
        }
    }
    if levels.len() < 2 {
        let current = levels
            .first()
            .cloned()
            .unwrap_or_else(|| "its current value".to_string());
        levels = vec![current, "a second value of your choice".to_string()];
    }
    let planned = match (noise, delta) {
        (Some(noise), Some(delta)) => seeds_needed(noise.sigma, delta).unwrap_or(3),
        _ => 3,
    };
    let seeds = planned.max(3);
    let label = board.label(&hypothesis.knob);
    let opposite = match hypothesis.direction {
        Direction::Lower => Direction::Higher,
        Direction::Higher => Direction::Lower,
    };
    Experiment {
        knob: hypothesis.knob.clone(),
        runs: seeds * 2,
        seeds_per_level: seeds,
        matched: board
            .shared
            .keys()
            .filter(|key| key.as_str() != hypothesis.knob)
            .map(|key| board.label(key))
            .collect(),
        settles: hypothesis.statement_on(board),
        refutes_if: format!(
            "the {} {label} setting gives the {} {} on every seed",
            "higher",
            opposite.word(),
            hypothesis.formula
        ),
        levels: levels.into_iter().take(2).collect(),
    }
}

fn join(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => format!(
            "{}, and {}",
            items[..items.len() - 1].join(", "),
            items[items.len() - 1]
        ),
    }
}

/// Why a learned tool was refused, or the tool as kept.
pub fn learn_tool(
    board: &Board,
    library: &[LearnedTool],
    name: &str,
    formula: &str,
    meaning: &str,
    date: &str,
) -> Result<LearnedTool, String> {
    let name = name.trim();
    if name.len() < 3
        || name.len() > 32
        || !name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch == '_' || ch.is_ascii_digit())
        || !name.starts_with(|ch: char| ch.is_ascii_lowercase())
    {
        return Err(
            "A tool name is 3 to 32 lowercase letters, digits, or _, starting with a letter."
                .to_string(),
        );
    }
    if expr::catalogue(board.host.measures())
        .iter()
        .any(|measure| measure.name == name)
        || library.iter().any(|tool| tool.name == name)
    {
        return Err(format!("{name} is already a measure or a tool."));
    }
    let meaning = meaning.trim();
    if meaning.is_empty()
        || meaning.chars().count() > 160
        || meaning.contains("**")
        || meaning.contains('#')
    {
        return Err("A tool needs a plain meaning of at most 160 characters.".to_string());
    }
    let parsed = parse_with_library(board, formula, library)?;
    let canonical = expr::canonical(&parsed);
    let column = evaluate(board, formula, library)?;
    if let Some((run, Err(reason))) = column.values.iter().find(|(_, value)| value.is_err()) {
        return Err(format!("Refused: it has no value on {run}. {reason}"));
    }
    for tool in library {
        let Ok(existing) = parse_with_library(board, &tool.formula, library) else {
            continue;
        };
        if expr::canonical(&existing) == canonical {
            return Err(format!("Refused: that is the formula of {}.", tool.name));
        }
        if let Ok(theirs) = evaluate(board, &tool.formula, library)
            && same_values(&theirs, &column)
        {
            return Err(format!(
                "Refused: on these runs it gives the same values as {}. Use that tool, or write a formula that differs.",
                tool.name
            ));
        }
    }
    Ok(LearnedTool {
        name: name.to_string(),
        formula: formula.trim().to_string(),
        meaning: meaning.to_string(),
        created: date.to_string(),
        uses: 0,
        boards: vec![board.key.clone()],
    })
}

fn same_values(a: &Column, b: &Column) -> bool {
    a.values.len() == b.values.len()
        && a.values
            .iter()
            .zip(&b.values)
            .all(|(x, y)| match (&x.1, &y.1) {
                (Ok(x), Ok(y)) => (x - y).abs() <= 1e-12 * x.abs().max(y.abs()).max(1.0),
                _ => false,
            })
}

/// Count a use of a learned tool on a board. A second board makes it kept.
pub fn note_use(library: &mut [LearnedTool], name: &str, board: &Board) {
    let key = board.key.clone();
    if let Some(tool) = library.iter_mut().find(|tool| tool.name == name) {
        tool.uses += 1;
        if !tool.boards.contains(&key) {
            tool.boards.push(key);
            tool.boards.truncate(8);
        }
    }
}

/// The learned tools, newest first.
pub fn read_tools(file: &Path) -> Vec<LearnedTool> {
    read_memory(file)
        .get(TOOLS_KEY)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(tool_from).collect())
        .unwrap_or_default()
}

/// Keep the library. Past the cap, the provisional tool used least goes first (TroVE-style trimming).
pub fn write_tools(file: &Path, tools: &[LearnedTool]) -> Result<(), std::io::Error> {
    let mut tools = tools.to_vec();
    while tools.len() > MAX_TOOLS {
        let drop = tools
            .iter()
            .enumerate()
            .min_by_key(|(_, tool)| (tool.kept(), tool.uses))
            .map(|(index, _)| index)
            .expect("a tool");
        tools.remove(drop);
    }
    let mut memory = read_memory(file);
    memory.insert(
        TOOLS_KEY.to_string(),
        Value::Array(tools.iter().map(tool_to).collect()),
    );
    write_memory(file, &memory)
}

pub fn read_hypotheses(file: &Path) -> Vec<Hypothesis> {
    read_memory(file)
        .get(HYPOTHESES_KEY)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(hypothesis_from).collect())
        .unwrap_or_default()
}

pub fn write_hypotheses(file: &Path, hypotheses: &[Hypothesis]) -> Result<(), std::io::Error> {
    let mut memory = read_memory(file);
    let kept: Vec<Value> = hypotheses
        .iter()
        .take(MAX_HYPOTHESES)
        .map(hypothesis_to)
        .collect();
    memory.insert(HYPOTHESES_KEY.to_string(), Value::Array(kept));
    write_memory(file, &memory)
}

/// What a proposer supplies. The program states the claim and fixes its test.
pub struct Proposal<'a> {
    pub knob: &'a str,
    pub formula: &'a str,
    pub direction: &'a str,
    pub why: &'a str,
}

/// A new hypothesis, refused with a reason when it cannot be stated or tested as written.
pub fn propose(
    board: &Board,
    existing: &[Hypothesis],
    library: &[LearnedTool],
    proposal: &Proposal<'_>,
    date: &str,
) -> Result<Hypothesis, String> {
    let Proposal {
        knob,
        formula,
        direction,
        why,
    } = *proposal;
    let knob = knob.trim();
    let in_recipe = board
        .runs
        .iter()
        .any(|series| series.knobs.contains_key(knob));
    if !in_recipe {
        let mut names: Vec<&str> = Vec::new();
        for series in &board.runs {
            for (key, value) in &series.knobs {
                if value.is_number() && !names.contains(&key.as_str()) {
                    names.push(key);
                }
            }
        }
        return Err(format!(
            "{knob} is not a field of these recipes. The seed is not a knob. Knobs you can name: {}.",
            names.join(", ")
        ));
    }
    let Some(direction) = Direction::parse(direction) else {
        return Err("The direction is lower or higher.".to_string());
    };
    parse_with_library(board, formula, library)?;
    if let Some(problem) = wording_problem(why, 280) {
        return Err(format!("The reason was refused: {problem}."));
    }
    let method = board.method.clone();
    let formula = formula.trim().to_string();
    if existing.iter().any(|h| {
        h.method == method && h.knob == knob && h.formula == formula && h.direction == direction
    }) {
        return Err("That hypothesis is already on the bench.".to_string());
    }
    // A number never reused, even after old hypotheses are trimmed, so a checkpoint's ids stay unique.
    let next = existing
        .iter()
        .filter_map(|h| h.id.strip_prefix('h').and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0)
        + 1;
    let id = format!("h{next}");
    Ok(Hypothesis {
        id,
        method,
        knob: knob.to_string(),
        formula,
        direction,
        why: why.trim().to_string(),
        proposed: date.to_string(),
        proposed_on: board.key.clone(),
        registered_runs: board.runs.iter().map(|run| run.identity.clone()).collect(),
        evaluations: Vec::new(),
    })
}

/// Record a test, newest first, replacing an older test of the same board.
///
/// A folder tested again keeps its place, so the order folders are counted in
/// depends only on when each was first opened, never on its result.
pub fn record(hypothesis: &mut Hypothesis, evaluation: Evaluation) {
    match hypothesis
        .evaluations
        .iter_mut()
        .find(|old| old.board == evaluation.board)
    {
        Some(old) => *old = evaluation,
        None => hypothesis.evaluations.insert(0, evaluation),
    }
    hypothesis.evaluations.truncate(MAX_EVALUATIONS);
}

fn text(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_string)
}

fn tool_to(tool: &LearnedTool) -> Value {
    serde_json::json!({
        "name": tool.name,
        "formula": tool.formula,
        "meaning": tool.meaning,
        "created": tool.created,
        "uses": tool.uses,
        "boards": tool.boards,
    })
}

fn tool_from(value: &Value) -> Option<LearnedTool> {
    let object = value.as_object()?;
    Some(LearnedTool {
        name: text(object, "name")?,
        formula: text(object, "formula")?,
        meaning: text(object, "meaning")?,
        created: text(object, "created").unwrap_or_default(),
        uses: object.get("uses").and_then(Value::as_u64).unwrap_or(0) as u32,
        boards: object
            .get("boards")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn hypothesis_to(h: &Hypothesis) -> Value {
    serde_json::json!({
        "id": h.id,
        "method": h.method,
        "knob": h.knob,
        "formula": h.formula,
        "direction": h.direction.word(),
        "why": h.why,
        "proposed": h.proposed,
        "proposed_on": h.proposed_on,
        "registered_runs": h.registered_runs,
        "evaluations": h.evaluations.iter().map(|e| serde_json::json!({
            "date": e.date,
            "board": e.board,
            "state": e.state.key(),
            "detail": e.detail,
            "e_for": e.e_for,
            "e_against": e.e_against,
            "runs": e.runs,
        })).collect::<Vec<_>>(),
    })
}

fn hypothesis_from(value: &Value) -> Option<Hypothesis> {
    let object = value.as_object()?;
    Some(Hypothesis {
        id: text(object, "id")?,
        method: text(object, "method")?,
        knob: text(object, "knob")?,
        formula: text(object, "formula")?,
        direction: Direction::parse(&text(object, "direction")?)?,
        why: text(object, "why").unwrap_or_default(),
        proposed: text(object, "proposed").unwrap_or_default(),
        proposed_on: text(object, "proposed_on").unwrap_or_default(),
        registered_runs: object
            .get("registered_runs")
            .and_then(Value::as_array)
            .map(|runs| {
                runs.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        evaluations: object
            .get("evaluations")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let item = item.as_object()?;
                        Some(Evaluation {
                            date: text(item, "date")?,
                            board: text(item, "board")?,
                            state: State::from_key(&text(item, "state")?)?,
                            detail: text(item, "detail")?,
                            e_for: item.get("e_for").and_then(Value::as_f64),
                            e_against: item.get("e_against").and_then(Value::as_f64),
                            runs: item
                                .get("runs")
                                .and_then(Value::as_array)
                                .map(|runs| {
                                    runs.iter()
                                        .filter_map(Value::as_str)
                                        .map(str::to_string)
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// A permutation e-value for "the measure moves this way when the knob goes up".
///
/// S is the share of (low-setting run, high-setting run) pairs that move in the
/// declared direction, ties counting half. The e-value is exp(lambda * S) divided
/// by its average over every way of relabeling the pooled runs into two groups of
/// the same sizes. If the knob does nothing, the runs are exchangeable, every
/// relabeling is equally likely, and the e-value averages exactly 1 (Koning,
/// arXiv:2310.01153, on e-values for exchangeability). `None` past 20 runs, where
/// the relabelings are not enumerated.
pub fn permutation_e(low: &[f64], high: &[f64], direction: Direction) -> Option<f64> {
    let n = low.len() + high.len();
    if low.is_empty() || high.is_empty() || n > 20 {
        return None;
    }
    let pool: Vec<f64> = low.iter().chain(high).copied().collect();
    let k = high.len();
    let pairs = (low.len() * high.len()) as f64;
    let share = |high: &[f64], low: &[f64]| {
        let above = pairs_above(high, low);
        match direction {
            Direction::Higher => above / pairs,
            Direction::Lower => (pairs - above) / pairs,
        }
    };
    let observed = share(high, low);
    let lambda = lambda_for(low.len(), high.len());
    // Log-sum-exp over the relabelings, so a large lambda cannot overflow.
    let mut exponents = Vec::new();
    let mut pick = Vec::with_capacity(k);
    combinations(n, k, 0, &mut pick, &mut |chosen| {
        let (mut h, mut l) = (Vec::with_capacity(k), Vec::with_capacity(n - k));
        for (index, value) in pool.iter().enumerate() {
            if chosen.contains(&index) {
                h.push(*value);
            } else {
                l.push(*value);
            }
        }
        exponents.push(lambda * share(&h, &l));
    });
    let top = exponents.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean = exponents.iter().map(|x| (x - top).exp()).sum::<f64>() / exponents.len() as f64;
    Some((lambda * observed - top).exp() / mean)
}

/// One hypothesis's evidence gathered across folders.
#[derive(Clone, Debug, PartialEq)]
pub struct Evidence {
    /// The product of the counted folders' e-values for the declared direction, and against it.
    pub e_for: f64,
    pub e_against: f64,
    /// Folders whose e-values were multiplied in, oldest first, with their dates.
    pub counted: Vec<String>,
    /// Folders left out and why: the one it was proposed on, or one sharing a run with a counted folder.
    pub left_out: Vec<(String, &'static str)>,
}

/// Multiply the folders' e-values, oldest folder first.
///
/// A folder counts when it gave a test, holds no run RunForge knew when the
/// hypothesis was registered (those runs may have shaped it), and shares no run
/// with a folder already counted. Which folders
/// count depends only on that order and on run identity, never on the values,
/// so the product of e-values from new runs stays valid however the folders
/// were chosen (Grunwald, de Heide, and Koolen 2024, "Safe testing", JRSS-B 86(5);
/// Ramdas, Grunwald, Vovk, and Shafer 2023, arXiv:2210.01948).
pub fn evidence(hypothesis: &Hypothesis) -> Evidence {
    let mut e_for = 1.0;
    let mut e_against = 1.0;
    let mut counted = Vec::new();
    let mut left_out = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for evaluation in hypothesis.evaluations.iter().rev() {
        let (Some(for_here), Some(against_here)) = (evaluation.e_for, evaluation.e_against) else {
            continue;
        };
        let before = evaluation
            .runs
            .iter()
            .any(|run| hypothesis.registered_runs.contains(run));
        if before
            || (!hypothesis.proposed_on.is_empty() && evaluation.board == hypothesis.proposed_on)
        {
            left_out.push((
                evaluation.date.clone(),
                "holds runs seen before the hypothesis was registered",
            ));
            continue;
        }
        if evaluation
            .runs
            .iter()
            .any(|run| seen.contains(&run.as_str()))
        {
            left_out.push((
                evaluation.date.clone(),
                "shares runs with a folder already counted",
            ));
            continue;
        }
        seen.extend(evaluation.runs.iter().map(String::as_str));
        e_for *= for_here;
        e_against *= against_here;
        counted.push(evaluation.date.clone());
    }
    Evidence {
        e_for,
        e_against,
        counted,
        left_out,
    }
}

/// Where a hypothesis stands across every folder it was tested on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Supported,
    Refuted,
    Open,
}

impl Verdict {
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Supported => "supported",
            Verdict::Refuted => "refuted",
            Verdict::Open => "open",
        }
    }
}

/// The bench's verdicts: e-BH at FDR over the 2K directional e-values of K
/// hypotheses (Wang and Ramdas 2022, JRSS-B 84(3)).
///
/// Each hypothesis contributes two e-values for "no effect": its evidence for the
/// declared direction and its evidence against. Sort all 2K from largest; the
/// largest k for which the k-th is at least 2K / (FDR * k) marks those k as
/// discoveries. A discovered "for" is supported; a discovered "against" is
/// refuted. Both are directional discoveries held to the same false discovery
/// rate, under any dependence (Kimi K3 review, 2026-10-06).
pub fn verdicts(hypotheses: &[Hypothesis]) -> Vec<(Evidence, Verdict)> {
    let gathered: Vec<Evidence> = hypotheses.iter().map(evidence).collect();
    let family = 2 * gathered.len();
    let mut entries: Vec<(usize, bool, f64)> = Vec::with_capacity(family);
    for (index, evidence) in gathered.iter().enumerate() {
        entries.push((index, true, evidence.e_for));
        entries.push((index, false, evidence.e_against));
    }
    entries.sort_by(|a, b| b.2.total_cmp(&a.2));
    let mut discovered = 0;
    for (rank, entry) in entries.iter().enumerate() {
        let k = rank + 1;
        if entry.2 >= family as f64 / (FDR * k as f64) {
            discovered = k;
        }
    }
    let mut out: Vec<(Evidence, Verdict)> = gathered
        .iter()
        .map(|evidence| (evidence.clone(), Verdict::Open))
        .collect();
    for (index, declared, _) in entries.into_iter().take(discovered) {
        out[index].1 = match (out[index].1, declared) {
            (Verdict::Open, true) => Verdict::Supported,
            (Verdict::Open, false) => Verdict::Refuted,
            // Both directions discovered cannot settle a direction.
            _ => Verdict::Open,
        };
    }
    out
}

/// The e-value a single direction needs, alone at the top of a bench of `k_total` hypotheses.
pub fn threshold(k_total: usize) -> f64 {
    (2 * k_total.max(1)) as f64 / FDR
}

/// One hypothesis at a checkpoint: its id, verdict, and evidence for and against.
pub type Judged = (String, Verdict, f64, f64);

/// One frozen batch of verdicts. Its family is every hypothesis registered by then.
#[derive(Clone, Debug, PartialEq)]
pub struct Checkpoint {
    pub number: u32,
    pub date: String,
    /// The number of hypotheses in the family.
    pub family: usize,
    /// Per hypothesis id: the verdict, and the evidence for and against at the time.
    pub results: Vec<Judged>,
}

impl Checkpoint {
    pub fn verdict_of(&self, id: &str) -> Option<&Judged> {
        self.results.iter().find(|result| result.0 == id)
    }
}

/// The checkpoints so far, and how many new folders have opened since the last one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Book {
    pub checkpoints: Vec<Checkpoint>,
    pub since: u32,
}

impl Book {
    pub fn latest_for(&self, id: &str) -> Option<(&Checkpoint, &Judged)> {
        self.checkpoints
            .iter()
            .rev()
            .find_map(|checkpoint| checkpoint.verdict_of(id).map(|result| (checkpoint, result)))
    }

    pub fn until_next(&self) -> u32 {
        CHECKPOINT_EVERY.saturating_sub(self.since)
    }
}

/// Count a newly opened folder. Every `CHECKPOINT_EVERY` new folders, freeze the
/// verdicts of every registered hypothesis as one batch e-BH. A checkpoint's verdicts
/// are final for it; between checkpoints the report shows evidence, not verdicts.
/// Reporting only at a schedule fixed in advance keeps the batch guarantee, which a
/// re-run over a growing family would not (Kimi K3 review, 2026-10-06).
pub fn note_new_folder(
    book: &mut Book,
    hypotheses: &[Hypothesis],
    date: &str,
) -> Option<Checkpoint> {
    book.since += 1;
    if book.since < CHECKPOINT_EVERY {
        return None;
    }
    book.since = 0;
    let judged = verdicts(hypotheses);
    let checkpoint = Checkpoint {
        number: book.checkpoints.last().map(|c| c.number).unwrap_or(0) + 1,
        date: date.to_string(),
        family: hypotheses.len(),
        results: hypotheses
            .iter()
            .zip(judged)
            .map(|(h, (evidence, verdict))| {
                (h.id.clone(), verdict, evidence.e_for, evidence.e_against)
            })
            .collect(),
    };
    book.checkpoints.push(checkpoint.clone());
    Some(checkpoint)
}

const BOOK_KEY: &str = "checkpoints";

pub fn read_book(file: &Path) -> Book {
    let memory = read_memory(file);
    let Some(object) = memory.get(BOOK_KEY).and_then(Value::as_object) else {
        return Book::default();
    };
    let checkpoints = object
        .get("list")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let item = item.as_object()?;
                    Some(Checkpoint {
                        number: item.get("number")?.as_u64()? as u32,
                        date: text(item, "date")?,
                        family: item.get("family")?.as_u64()? as usize,
                        results: item
                            .get("results")?
                            .as_array()?
                            .iter()
                            .filter_map(|result| {
                                let result = result.as_object()?;
                                let verdict = match result.get("verdict")?.as_str()? {
                                    "supported" => Verdict::Supported,
                                    "refuted" => Verdict::Refuted,
                                    _ => Verdict::Open,
                                };
                                Some((
                                    text(result, "id")?,
                                    verdict,
                                    result.get("e_for")?.as_f64()?,
                                    result.get("e_against")?.as_f64()?,
                                ))
                            })
                            .collect(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Book {
        checkpoints,
        since: object.get("since").and_then(Value::as_u64).unwrap_or(0) as u32,
    }
}

pub fn write_book(file: &Path, book: &Book) -> Result<(), std::io::Error> {
    let list: Vec<Value> = book
        .checkpoints
        .iter()
        .map(|checkpoint| {
            serde_json::json!({
                "number": checkpoint.number,
                "date": checkpoint.date,
                "family": checkpoint.family,
                "results": checkpoint.results.iter().map(|(id, verdict, e_for, e_against)| serde_json::json!({
                    "id": id,
                    "verdict": verdict.word(),
                    "e_for": e_for,
                    "e_against": e_against,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let mut memory = read_memory(file);
    memory.insert(
        BOOK_KEY.to_string(),
        serde_json::json!({"since": book.since, "list": list}),
    );
    write_memory(file, &memory)
}
