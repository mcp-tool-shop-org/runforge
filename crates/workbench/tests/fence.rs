//! The wording fence with a host whose measure names carry digits (ScalarScope's p50, p99):
//! an exact name passes, a number does not.

use std::sync::Arc;

use serde_json::{Map, Value};
use workbench::{
    Board, Host, LearnedTool, Measure, Proposal, Run, Workbench, allowed_names, propose,
    split_knobs, wording_problem, wording_problem_naming,
};

const MEASURES: &[Measure] = &[
    Measure {
        name: "p50",
        args: "",
        means: "the median",
    },
    Measure {
        name: "p99",
        args: "",
        means: "the 99th percentile",
    },
    Measure {
        name: "p99_over_p50",
        args: "",
        means: "the tail ratio",
    },
];

struct Latency;

impl Host for Latency {
    fn program(&self) -> &str {
        "Test"
    }
    fn subject(&self) -> &str {
        "latency runs"
    }
    fn build_example(&self) -> &str {
        "For example, p99 / p50."
    }
    fn measures(&self) -> &[Measure] {
        MEASURES
    }
    fn reason_hint(&self) -> &str {
        "in words; name a measure by its name, such as p99, and write no other numbers"
    }
    fn measure(&self, run: usize, name: &str, _args: &[f64]) -> Result<f64, String> {
        let base = 10.0 + run as f64;
        Ok(match name {
            "p50" => base,
            "p99" => base * 2.0,
            _ => 2.0,
        })
    }
}

fn board() -> Board {
    let runs: Vec<Run> = [1, 1, 8, 8]
        .iter()
        .enumerate()
        .map(|(index, batch)| Run {
            name: format!("run {index}"),
            seed: Some(index as i64),
            knobs: [("batch".to_string(), Value::from(*batch))]
                .into_iter()
                .collect::<Map<_, _>>(),
            identity: format!("id{index}"),
        })
        .collect();
    let (shared, varying) = split_knobs(&runs);
    Board {
        runs,
        shared,
        varying,
        key: "k".into(),
        method: "m".into(),
        host: Arc::new(Latency),
    }
}

const NAMES: &[&str] = &["p50", "p99", "p99_over_p50", "low"];

#[test]
fn an_exact_measure_name_passes_and_a_number_does_not() {
    assert_eq!(
        wording_problem_naming("A bigger batch lifts p99 more than p50.", 280, NAMES),
        None
    );
    assert_eq!(
        wording_problem_naming("The p99_over_p50 ratio grows.", 280, NAMES),
        None
    );
    for smuggled in [
        "p99x grows",
        "the 99th percentile grows",
        "p 99 grows",
        "p99.5 grows",
        "p990 grows",
    ] {
        let problem = wording_problem_naming(smuggled, 280, NAMES)
            .unwrap_or_else(|| panic!("{smuggled} passed"));
        assert!(
            problem.contains("name the measure (p50, p99, p99_over_p50) and write no other digits"),
            "{problem}"
        );
    }
    // Other rules still apply to a text that names measures.
    assert!(
        wording_problem_naming("p99 proves it.", 280, NAMES)
            .unwrap()
            .contains("verdict word")
    );
    assert!(
        wording_problem_naming(&format!("{} p99", "a".repeat(280)), 280, NAMES)
            .unwrap()
            .contains("ran past")
    );
}

#[test]
fn a_host_without_digit_names_reads_exactly_as_before() {
    let names = ["low", "median", "slope_between"];
    for text in [
        "the 99th percentile",
        "low is lower",
        "it proves it",
        "**bold**",
    ] {
        assert_eq!(
            wording_problem_naming(text, 280, &names),
            wording_problem(text, 280),
            "{text}"
        );
    }
}

#[test]
fn a_reason_and_a_note_may_name_measures_and_learned_tools() {
    let board = board();
    let library = vec![LearnedTool {
        name: "tail_p99".into(),
        formula: "p99 / p50".into(),
        meaning: "tail".into(),
        created: "2026-10-07".into(),
        uses: 0,
        boards: Vec::new(),
    }];
    assert!(allowed_names(&board, &library).contains(&"tail_p99"));
    let proposal = Proposal {
        knob: "batch",
        formula: "tail_p99",
        direction: "higher",
        why: "A bigger batch stretches p99 past p50, so tail_p99 rises.",
    };
    assert!(propose(&board, &[], &library, &proposal, "2026-10-07").is_ok());
    let refused = Proposal {
        why: "The 99th percentile rises.",
        ..proposal
    };
    let error = propose(&board, &[], &library, &refused, "2026-10-07").unwrap_err();
    assert!(
        error.starts_with("The reason was refused: it carried a digit"),
        "{error}"
    );
    assert!(
        error.contains("name the measure (p50, p99, p99_over_p50)"),
        "{error}"
    );

    let mut bench = Workbench::new(board, Vec::new(), Vec::new(), "2026-10-07");
    let kept = bench.call(
        "finish",
        &serde_json::json!({"note": "p99 rose with the batch; p50 too."}),
    );
    assert_eq!(kept, "Finished. Your note is kept.");
}

#[test]
fn a_host_words_its_own_reason_hint_and_others_keep_the_default() {
    let bench = Workbench::new(board(), Vec::new(), Vec::new(), "2026-10-07");
    let specs = bench.tool_specs();
    let why = &specs[3]["function"]["parameters"]["properties"]["why"]["description"];
    assert_eq!(
        why,
        "in words; name a measure by its name, such as p99, and write no other numbers"
    );
    let note = &specs[4]["function"]["parameters"]["properties"]["note"]["description"];
    assert_eq!(
        note,
        "What you looked at and what is still open, in words, with no numbers."
    );
}
