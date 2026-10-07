//! One workbench session. The session lives in the shared `workbench` crate; this
//! module re-exports it and keeps RunForge's tests of it, over RunForge's runs.

pub use workbench::{
    MAX_CALLS, MAX_CALLS_PER_ROUND, MAX_ROUNDS, NOTE_LIMIT, Phase, Step, Workbench,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::bench_board;
    use crate::series::{Board, Sample, Series};
    use serde_json::{Map, Value, json};
    use workbench::{BUILD_ROUND, direction_of};

    fn board() -> workbench::Board {
        bench_board(&runforge_board())
    }

    pub(super) fn runforge_board_for_snapshot() -> Board {
        runforge_board()
    }

    fn runforge_board() -> Board {
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

#[cfg(test)]
mod snapshot {
    /// RunForge's tool schema, byte for byte, as it was before hosts could word their own
    /// hints (`tests/fixtures/tool_specs.json`, written from the code at runforge 39c0ce7).
    #[test]
    fn runforge_tool_schema_is_byte_identical() {
        let bench = super::Workbench::new(
            crate::bench::bench_board(&super::tests::runforge_board_for_snapshot()),
            Vec::new(),
            Vec::new(),
            "2026-10-06",
        );
        let now = serde_json::to_string_pretty(&bench.tool_specs()).unwrap() + "\n";
        let then = include_str!("../tests/fixtures/tool_specs.json").replace("\r\n", "\n");
        assert_eq!(now, then);
    }
}
