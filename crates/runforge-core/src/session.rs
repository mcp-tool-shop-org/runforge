//! One workbench session: the tools a local model may call, and what each call does.
//!
//! The model is told what the runs are and which knobs moved. It calls tools; the
//! program computes every result and writes every sentence that carries a number.
//! The model's own words are limited to a tool's meaning, a hypothesis's reason,
//! and a closing note, each fenced. The session never trains and never fetches.
//!
//! Sources for the shape of the loop: program-aided reasoning (Gao et al. 2022,
//! arXiv:2211.10435; Chen et al. 2022, arXiv:2211.12588), few tools per turn
//! (Paramanayakam et al. 2024, arXiv:2411.15399), a short loop with an abstain
//! path (Patil et al., BFCL, ICML 2025), and tools kept only after checks
//! (Yuan et al. 2023, arXiv:2309.17428; Wang et al. 2024, arXiv:2401.12869).

use serde_json::{Value, json};

use crate::bench::{
    Hypothesis, LearnedTool, Proposal, State, compare_knob, evaluate, experiment_for, learn_tool,
    propose, record, seed_noise, test, wording_problem,
};
use crate::expr::MEASURES;
use crate::series::{Board, format_measure, recipe_keys, recipe_label, recipe_text};

/// Chat requests per session.
pub const MAX_ROUNDS: usize = 6;
/// Tool calls per session.
pub const MAX_CALLS: usize = 10;
/// Tool calls run per round. A model that lists every measure at once would
/// otherwise spend the whole session before it builds or proposes anything.
pub const MAX_CALLS_PER_ROUND: usize = 3;
/// The longest closing note kept.
pub const NOTE_LIMIT: usize = 600;
/// Rounds spent looking before the model is asked to build a tool.
const LOOK_ROUNDS: usize = 2;
/// The round in which the model is asked to build a tool.
const BUILD_ROUND: usize = LOOK_ROUNDS;

/// What the model is asked to do in a round. The program narrows the tools it offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// measure, compare_knob, learn_tool. Finish is not offered yet.
    Look,
    /// learn_tool, measure: name something the listed measures do not capture.
    Build,
    /// propose_hypothesis, learn_tool, measure, finish.
    Propose,
    /// finish only.
    Close,
}

impl Phase {
    /// The phase of a zero-based round.
    pub fn of(round: usize) -> Phase {
        if round + 1 >= MAX_ROUNDS {
            Phase::Close
        } else if round == BUILD_ROUND {
            Phase::Build
        } else if round > BUILD_ROUND {
            Phase::Propose
        } else {
            Phase::Look
        }
    }

    /// The message that opens this phase, when it differs from the round before.
    pub fn prompt(self) -> &'static str {
        match self {
            Phase::Look => "Investigate. Start with measure.",
            Phase::Build => {
                "Now build at least one tool with learn_tool: a formula for something the listed measures do not capture in these curves. For example, how far a curve climbs after its low (last / low), or how steep the last epoch is (slope_between(end_epoch - 1, end_epoch)). Pick what these runs make you curious about, name it, and say what it means."
            }
            Phase::Propose => {
                "Now propose hypotheses from what you measured: a knob, a formula, and a direction, with the mechanism you suspect. A knob that did not change is fine; the program will plan the runs that would test it. You may still build a tool or measure once more."
            }
            Phase::Close => {
                "Call finish now, with a note in words on what you looked at and what is still open."
            }
        }
    }

    fn offers(self, tool: &str) -> bool {
        match self {
            // No finish while looking: a session that stops before building a tool or
            // proposing a hypothesis leaves the bench where it was.
            Phase::Look => matches!(tool, "measure" | "compare_knob" | "learn_tool"),
            Phase::Build => matches!(tool, "learn_tool" | "measure"),
            Phase::Propose => matches!(
                tool,
                "propose_hypothesis" | "learn_tool" | "measure" | "finish"
            ),
            Phase::Close => tool == "finish",
        }
    }
}

/// One tool call and what the program answered.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub tool: String,
    pub args: String,
    pub ok: bool,
    pub result: String,
}

/// The state of one session over one board.
pub struct Workbench {
    board: Board,
    library: Vec<LearnedTool>,
    hypotheses: Vec<Hypothesis>,
    date: String,
    pub steps: Vec<Step>,
    /// Tools learned in this session, already past their gates.
    pub learned: Vec<LearnedTool>,
    /// Hypotheses proposed in this session, each with its first test on this board.
    pub proposed: Vec<Hypothesis>,
    /// Learned tools this session used, by name.
    pub used: Vec<String>,
    /// The closing note, when it passed the fence.
    pub note: Option<String>,
    /// A closing note that crossed the fence was dropped.
    pub note_dropped: bool,
    pub finished: bool,
    /// Runs RunForge had seen before this session; a hypothesis registered now excludes them.
    known_runs: Vec<String>,
}

impl Workbench {
    /// Name the runs already seen, so a hypothesis registered in this session cannot count them.
    pub fn knowing(mut self, runs: Vec<String>) -> Self {
        self.known_runs = runs;
        self
    }

    pub fn new(
        board: Board,
        library: Vec<LearnedTool>,
        hypotheses: Vec<Hypothesis>,
        date: &str,
    ) -> Self {
        Workbench {
            board,
            library,
            hypotheses,
            date: date.to_string(),
            steps: Vec::new(),
            learned: Vec::new(),
            proposed: Vec::new(),
            used: Vec::new(),
            note: None,
            note_dropped: false,
            finished: false,
            known_runs: Vec::new(),
        }
    }

    fn all_tools(&self) -> Vec<LearnedTool> {
        self.library.iter().chain(&self.learned).cloned().collect()
    }

    fn calls_left(&self) -> usize {
        MAX_CALLS.saturating_sub(self.steps.len())
    }

    /// The schemas offered in one phase: a few tools at a time.
    pub fn tool_specs_for(&self, phase: Phase) -> Value {
        Value::Array(
            self.tool_specs()
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|spec| {
                    spec["function"]["name"]
                        .as_str()
                        .is_some_and(|name| phase.offers(name))
                })
                .collect(),
        )
    }

    /// Every function schema. Knob names are enumerated from these recipes.
    pub fn tool_specs(&self) -> Value {
        let mut knobs: Vec<String> = Vec::new();
        for series in &self.board.series {
            for key in series.recipe.keys() {
                if !knobs.contains(key) && series.recipe.get(key).and_then(Value::as_f64).is_some()
                {
                    knobs.push(key.clone());
                }
            }
        }
        let formula = json!({"type": "string", "description": "A formula in the RunForge formula language, or a learned tool's name."});
        let mut tools = vec![json!({
            "type": "function",
            "function": {
                "name": "measure",
                "description": "Evaluate a formula on every run and see the values.",
                "parameters": {"type": "object", "properties": {"formula": formula}, "required": ["formula"]}
            }
        })];
        let varying: Vec<&String> = self
            .board
            .varying
            .iter()
            .filter(|key| knobs.contains(key))
            .collect();
        if !varying.is_empty() {
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": "compare_knob",
                    "description": "Compare the runs at the lowest and highest setting of a knob that changed, on one formula.",
                    "parameters": {"type": "object", "properties": {
                        "knob": {"type": "string", "enum": varying},
                        "formula": formula
                    }, "required": ["knob", "formula"]}
                }
            }));
        }
        tools.push(json!({
            "type": "function",
            "function": {
                "name": "learn_tool",
                "description": "Keep a new formula as a named tool, when no measure says what you need. The program checks it first.",
                "parameters": {"type": "object", "properties": {
                    "name": {"type": "string", "description": "lowercase_with_underscores"},
                    "formula": formula,
                    "meaning": {"type": "string", "description": "What it measures, in one plain sentence."}
                }, "required": ["name", "formula", "meaning"]}
            }
        }));
        tools.push(json!({
            "type": "function",
            "function": {
                "name": "propose_hypothesis",
                "description": "Propose what a knob does to a formula. The program tests it and plans the runs that would settle it.",
                "parameters": {"type": "object", "properties": {
                    "knob": {"type": "string", "enum": knobs},
                    "formula": formula,
                    "knob_change": {"type": "string", "enum": ["raise", "lower"], "description": "The change to the knob you have in mind."},
                    "formula_moves": {"type": "string", "enum": ["up", "down"], "description": "Which way you expect the formula to move after that change."},
                    "why": {"type": "string", "description": "The mechanism you suspect, in words, with no numbers."}
                }, "required": ["knob", "formula", "knob_change", "formula_moves", "why"]}
            }
        }));
        tools.push(json!({
            "type": "function",
            "function": {
                "name": "finish",
                "description": "End the session. Use it when you are done, or when the tools cannot answer.",
                "parameters": {"type": "object", "properties": {
                    "note": {"type": "string", "description": "What you looked at and what is still open, in words, with no numbers."}
                }, "required": ["note"]}
            }
        }));
        Value::Array(tools)
    }

    /// The system message: the role, the rules, the formula language, and the bench as it stands.
    pub fn system_prompt(&self) -> String {
        let mut text = String::from(
            "You are the RunForge workbench. You investigate fine-tuning runs by calling tools. \
You never compute or quote numbers yourself: the tools compute, and the program prints every number. \
Your job is to find what these runs can and cannot say about what each knob does. \
Look at the data with measure. Building tools is part of the job: when a question needs a measure that is not listed, or you ask the same thing of the curves twice, define it with learn_tool. A kept tool is there for every later session. \
Propose hypotheses as a knob, a formula, and a direction; the program tests them. \
A knob that had the same value on every run was not tested here. Do not suggest changing it. You may still propose a hypothesis about it; the program will mark it not testable and plan the runs that would test it. \
Use at most ",
        );
        text.push_str(&format!(
            "{MAX_CALLS} tool calls, then call finish. Your own words may not contain digits.\n\nThe formula language: + - * / ^ and parentheses over these names, evaluated once per run.\n"
        ));
        for measure in MEASURES {
            if measure.args.is_empty() {
                text.push_str(&format!("- {}: {}\n", measure.name, measure.means));
            } else {
                text.push_str(&format!(
                    "- {}({}): {}\n",
                    measure.name, measure.args, measure.means
                ));
            }
        }
        let tools = self.all_tools();
        if !tools.is_empty() {
            text.push_str("\nLearned tools (use them by name inside formulas):\n");
            for tool in &tools {
                text.push_str(&format!("- {}: {}\n", tool.name, tool.meaning));
            }
        }
        text
    }

    /// The opening user message: the runs and their recipes, and the open hypotheses for this method.
    pub fn opening(&self) -> String {
        let mut text = format!("{} runs are open.\n", self.board.series.len());
        let changed: Vec<String> = self
            .board
            .varying
            .iter()
            .map(|key| {
                let values: Vec<String> = self
                    .board
                    .series
                    .iter()
                    .map(|series| {
                        format!(
                            "{} {}",
                            series.name,
                            series
                                .recipe
                                .get(key)
                                .map(recipe_text)
                                .unwrap_or_else(|| "none".into())
                        )
                    })
                    .collect();
                format!("{key} ({})", values.join(", "))
            })
            .collect();
        let seeds: Vec<String> = self
            .board
            .series
            .iter()
            .map(|series| {
                series
                    .seed
                    .map(|seed| seed.to_string())
                    .unwrap_or_else(|| "none".into())
            })
            .collect();
        text.push_str(&format!("Seeds: {}.\n", seeds.join(", ")));
        if changed.is_empty() {
            text.push_str("Knobs that changed: none. Only the seed differs.\n");
        } else {
            text.push_str(&format!("Knobs that changed: {}.\n", changed.join("; ")));
        }
        let shared: Vec<String> = recipe_keys(&self.board.shared)
            .into_iter()
            .map(|key| format!("{key} {}", recipe_text(&self.board.shared[key])))
            .collect();
        if !shared.is_empty() {
            text.push_str(&format!(
                "Shared by every run, so not tested here: {}.\n",
                shared.join(", ")
            ));
        }
        let open: Vec<String> = self
            .hypotheses
            .iter()
            .filter(|h| h.method == crate::bench::board_method(&self.board))
            .map(|h| {
                let so_far = crate::bench::evidence(h);
                format!(
                    "- {} (evidence so far: {} for, {} against)",
                    h.statement(),
                    format_measure(so_far.e_for),
                    format_measure(so_far.e_against)
                )
            })
            .collect();
        if !open.is_empty() {
            text.push_str("Hypotheses already on the bench for this method:\n");
            text.push_str(&open.join("\n"));
            text.push('\n');
        }
        text.push_str(Phase::Look.prompt());
        text
    }

    /// Run one tool call. The returned text goes back to the model as the tool's answer.
    pub fn call(&mut self, name: &str, args: &Value) -> String {
        if self.finished {
            return "The session has finished.".to_string();
        }
        if self.calls_left() == 0 {
            self.finished = true;
            return "No tool calls are left. The session has finished.".to_string();
        }
        let field = |key: &str| {
            args.get(key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let (ok, result) = match name {
            "measure" => self.measure(&field("formula")),
            "compare_knob" => self.compare(&field("knob"), &field("formula")),
            "learn_tool" => self.learn(&field("name"), &field("formula"), &field("meaning")),
            "propose_hypothesis" => {
                let direction = direction_of(&field("knob_change"), &field("formula_moves"))
                    .unwrap_or_else(|| field("direction"));
                self.hypothesize(&field("knob"), &field("formula"), &direction, &field("why"))
            }
            "finish" => {
                let note = field("note");
                self.finished = true;
                if note.trim().is_empty() {
                    (true, "Finished.".to_string())
                } else if let Some(problem) = wording_problem(&note, NOTE_LIMIT) {
                    self.note_dropped = true;
                    (
                        false,
                        format!("Finished. Your note was dropped: {problem}."),
                    )
                } else {
                    self.note = Some(note.trim().to_string());
                    (true, "Finished. Your note is kept.".to_string())
                }
            }
            other => (
                false,
                format!(
                    "{other} is not a tool. The tools are measure, compare_knob, learn_tool, propose_hypothesis, and finish."
                ),
            ),
        };
        self.steps.push(Step {
            tool: name.to_string(),
            args: serde_json::to_string(args).unwrap_or_default(),
            ok,
            result: result.clone(),
        });
        if self.calls_left() == 0 && !self.finished {
            self.finished = true;
            return format!("{result}\nThat was the last tool call. The session has finished.");
        }
        result
    }

    fn note_learned_use(&mut self, formula: &str) {
        for tool in self.all_tools() {
            let used = formula
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .any(|word| word == tool.name);
            if used && !self.used.contains(&tool.name) {
                self.used.push(tool.name.clone());
            }
        }
    }

    fn measure(&mut self, formula: &str) -> (bool, String) {
        let tools = self.all_tools();
        match evaluate(&self.board, formula, &tools) {
            Ok(column) => {
                self.note_learned_use(formula);
                let mut lines = column.lines();
                if let Some(noise) = seed_noise(&self.board, &column) {
                    lines.push(format!(
                        "Across these {} runs of one recipe it spans {} (from {} to {}). That spread is seed noise.",
                        noise.runs,
                        format_measure(noise.range()),
                        format_measure(noise.low),
                        format_measure(noise.high)
                    ));
                }
                (true, lines.join("\n"))
            }
            Err(reason) => (false, reason),
        }
    }

    fn compare(&mut self, knob: &str, formula: &str) -> (bool, String) {
        let tools = self.all_tools();
        let column = match evaluate(&self.board, formula, &tools) {
            Ok(column) => column,
            Err(reason) => return (false, reason),
        };
        self.note_learned_use(formula);
        match compare_knob(&self.board, knob, &column) {
            Ok(c) => {
                let label = recipe_label(knob);
                let values = |arm: &crate::bench::Arm| {
                    arm.values
                        .iter()
                        .map(|(name, value)| format!("{name} {}", format_measure(*value)))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let mut text = format!(
                    "{label} {}: {}.\n{label} {}: {}.\nGap (high minus low, medians): {}.",
                    c.low.level,
                    values(&c.low),
                    c.high.level,
                    values(&c.high),
                    format_measure(c.gap)
                );
                match c.noise {
                    Some(noise) => text.push_str(&format!(
                        " Widest seed spread inside one setting: {}, so the gap is {} it.",
                        format_measure(noise),
                        if c.gap.abs() <= noise { "inside" } else { "wider than" }
                    )),
                    None => text.push_str(" One run at some setting, so there is no seed spread to weigh the gap against."),
                }
                text.push_str(&format!(
                    "\nA high-setting run is above a low-setting run in {} of {} pairs. Exact rank test, one-sided: p = {} for higher, {} for lower.",
                    format_measure(c.pairs_higher),
                    c.pairs,
                    format_measure(c.p_higher),
                    format_measure(c.p_lower)
                ));
                if c.middle > 0 {
                    text.push_str(&format!(" {} settings in between are not used.", c.middle));
                }
                if !c.confounds.is_empty() {
                    text.push_str(&format!(
                        "\nAlso changed: {}. The gap does not belong to {label} alone.",
                        c.confounds.join(", ")
                    ));
                }
                (true, text)
            }
            Err(reason) => (false, reason),
        }
    }

    fn learn(&mut self, name: &str, formula: &str, meaning: &str) -> (bool, String) {
        let tools = self.all_tools();
        match learn_tool(&self.board, &tools, name, formula, meaning, &self.date) {
            Ok(tool) => {
                let column = evaluate(&self.board, &tool.formula, &self.all_tools());
                let mut text = format!(
                    "Kept {} as a provisional tool. It becomes a kept tool once it is used on a second folder.",
                    tool.name
                );
                self.learned.push(tool);
                if let Ok(column) = column {
                    text.push('\n');
                    text.push_str(&column.lines().join("\n"));
                }
                (true, text)
            }
            Err(reason) => (false, reason),
        }
    }

    fn hypothesize(
        &mut self,
        knob: &str,
        formula: &str,
        direction: &str,
        why: &str,
    ) -> (bool, String) {
        let tools = self.all_tools();
        let existing: Vec<Hypothesis> = self
            .hypotheses
            .iter()
            .chain(&self.proposed)
            .cloned()
            .collect();
        let proposal = Proposal {
            knob,
            formula,
            direction,
            why,
        };
        let mut hypothesis = match propose(&self.board, &existing, &tools, &proposal, &self.date) {
            Ok(hypothesis) => hypothesis,
            Err(reason) => return (false, reason),
        };
        for run in &self.known_runs {
            if !hypothesis.registered_runs.contains(run) {
                hypothesis.registered_runs.push(run.clone());
            }
        }
        self.note_learned_use(formula);
        let (evaluation, comparison) = test(&self.board, &hypothesis, &tools, &self.date);
        let mut text = format!(
            "Recorded {}: {} On these runs it is {}. {}",
            hypothesis.id,
            hypothesis.statement(),
            evaluation.state.word(),
            evaluation.detail
        );
        let settled = matches!(evaluation.state, State::Supported | State::Refuted);
        if !settled {
            let noise = evaluate(&self.board, formula, &tools)
                .ok()
                .and_then(|column| seed_noise(&self.board, &column));
            let delta = comparison
                .as_ref()
                .map(|c| c.gap.abs())
                .filter(|gap| *gap > 0.0);
            let plan = experiment_for(&self.board, &hypothesis, noise.as_ref(), delta);
            text.push_str(&format!(
                "\nTo settle it: {} runs, {} at {} and {}, {} seeds each, everything else as here. It is refuted if {}.",
                plan.runs,
                recipe_label(&plan.knob),
                plan.levels[0],
                plan.levels[1],
                plan.seeds_per_level,
                plan.refutes_if
            ));
        }
        record(&mut hypothesis, evaluation);
        self.proposed.push(hypothesis);
        (true, text)
    }
}

/// The program's direction (the formula when the knob goes up) from the proposer's own framing.
///
/// "Lower the learning rate and the low goes down" is the direction "higher":
/// raising the rate would raise the low. Asking this way keeps a proposer from
/// stating one claim in the reason and the opposite in the direction.
pub fn direction_of(knob_change: &str, formula_moves: &str) -> Option<String> {
    let raise = match knob_change.trim().to_ascii_lowercase().as_str() {
        "raise" | "up" | "increase" | "higher" => true,
        "lower" | "down" | "decrease" | "reduce" => false,
        _ => return None,
    };
    let up = match formula_moves.trim().to_ascii_lowercase().as_str() {
        "up" | "higher" | "increase" | "increases" => true,
        "down" | "lower" | "decrease" | "decreases" => false,
        _ => return None,
    };
    Some(if raise == up { "higher" } else { "lower" }.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::series::{Sample, Series};
    use serde_json::Map;

    fn board() -> Board {
        let run = |seed: i64, rank: i64, low: f64| {
            let mut recipe = Map::new();
            recipe.insert("method".into(), Value::from("bf16 LoRA"));
            recipe.insert("learning_rate".into(), Value::from(0.0001));
            recipe.insert("lora_r".into(), Value::from(rank));
            Series {
                name: format!("seed {seed}"),
                seed: Some(seed),
                model: String::new(),
                file_name: String::new(),
                samples: [(0.0, 4.0), (1.0, low), (2.0, low * 1.4)]
                    .iter()
                    .map(|(x, loss)| Sample {
                        x: Some(*x),
                        loss: Some(*loss),
                        lr: Some(0.0001),
                        extra: Map::new(),
                    })
                    .collect(),
                recipe,
                summary: Map::new(),
            }
        };
        let series = vec![run(1, 16, 0.3), run(2, 32, 0.1)];
        let mut shared = Map::new();
        shared.insert("method".into(), Value::from("bf16 LoRA"));
        shared.insert("learning_rate".into(), Value::from(0.0001));
        Board {
            series,
            skipped: 0,
            shared,
            varying: vec!["lora_r".into()],
        }
    }

    #[test]
    fn the_specs_offer_five_tools_and_enumerate_the_knobs() {
        let bench = Workbench::new(board(), Vec::new(), Vec::new(), "2026-10-06");
        let specs = bench.tool_specs();
        let names: Vec<&str> = specs
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "measure",
                "compare_knob",
                "learn_tool",
                "propose_hypothesis",
                "finish"
            ]
        );
        assert_eq!(
            specs[1]["function"]["parameters"]["properties"]["knob"]["enum"],
            json!(["lora_r"])
        );
        let prompt = bench.system_prompt();
        assert!(prompt.contains("slope_between(a, b)"));
        assert!(bench.opening().contains(
            "Shared by every run, so not tested here: method bf16 LoRA, learning_rate 0.0001"
        ));
    }

    #[test]
    fn each_phase_offers_a_few_tools_and_the_last_offers_only_finish() {
        let bench = Workbench::new(board(), Vec::new(), Vec::new(), "2026-10-06");
        let names = |phase| -> Vec<String> {
            bench
                .tool_specs_for(phase)
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t["function"]["name"].as_str().unwrap().to_string())
                .collect()
        };
        assert_eq!(
            names(Phase::of(0)),
            vec!["measure", "compare_knob", "learn_tool"]
        );
        assert_eq!(names(Phase::of(BUILD_ROUND)), vec!["measure", "learn_tool"]);
        assert_eq!(
            names(Phase::of(BUILD_ROUND + 1)),
            vec!["measure", "learn_tool", "propose_hypothesis", "finish"]
        );
        assert_eq!(names(Phase::of(MAX_ROUNDS - 1)), vec!["finish"]);
    }

    #[test]
    fn calls_are_computed_by_the_program_and_capped() {
        let mut bench = Workbench::new(board(), Vec::new(), Vec::new(), "2026-10-06");
        assert!(
            bench
                .call("measure", &json!({"formula": "low"}))
                .contains("seed 1: 0.3")
        );
        assert!(
            bench
                .call("compare_knob", &json!({"knob": "lora_r", "formula": "low"}))
                .contains("Gap (high minus low, medians): -0.2")
        );
        assert!(
            bench
                .call("measure", &json!({"formula": "open('x')"}))
                .contains("not a measure")
        );
        assert!(
            bench
                .call(
                    "learn_tool",
                    &json!({"name": "rebound", "formula": "last / low", "meaning": "end over low"})
                )
                .contains("Kept rebound")
        );
        assert_eq!(direction_of("lower", "down").as_deref(), Some("higher"));
        assert_eq!(direction_of("raise", "down").as_deref(), Some("lower"));
        assert_eq!(direction_of("sideways", "down"), None);
        let answer = bench.call("propose_hypothesis", &json!({"knob": "learning_rate", "formula": "rebound", "knob_change": "lower", "formula_moves": "down", "why": "A slower step settles deeper."}));
        assert!(
            answer.contains("When learning rate goes up, rebound goes higher."),
            "{answer}"
        );
        assert!(answer.contains("not testable here"), "{answer}");
        assert!(answer.contains("To settle it: 6 runs"), "{answer}");
        assert_eq!(bench.used, vec!["rebound".to_string()]);
        assert!(
            bench
                .call("delete_files", &json!({}))
                .contains("is not a tool")
        );
        assert!(
            bench
                .call(
                    "finish",
                    &json!({"note": "Rank looks like it matters by 3 points."})
                )
                .contains("dropped")
        );
        assert!(bench.finished && bench.note.is_none() && bench.note_dropped);
        assert_eq!(
            bench.call("measure", &json!({"formula": "low"})),
            "The session has finished."
        );
        let mut capped = Workbench::new(board(), Vec::new(), Vec::new(), "2026-10-06");
        for _ in 0..MAX_CALLS {
            capped.call("measure", &json!({"formula": "low"}));
        }
        assert!(capped.finished);
        assert_eq!(capped.steps.len(), MAX_CALLS);
    }
}
