use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use runforge_core::{
    Board, Direction, Proposal, Sample, Series, State, compare_knob, evaluate, exact_p,
    experiment_for, learn_tool, note_use, propose, read_hypotheses, read_tools, reason_allowed,
    recall, record, record_weighing, remember, seed_noise, seeds_needed, test_all, test_hypothesis,
    weighed_now, write_hypotheses, write_tools,
};
use serde_json::{Map, Value};

fn scratch(name: &str) -> std::path::PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "runforge-bench-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A run whose curve falls to `low` at epoch 3 and ends at `low * 1.5`.
fn run(seed: i64, rank: i64, low: f64) -> Series {
    let mut recipe = Map::new();
    recipe.insert("method".into(), Value::from("bf16 LoRA"));
    recipe.insert("learning_rate".into(), Value::from(0.0001));
    recipe.insert("lora_r".into(), Value::from(rank));
    let samples = [
        (0.0, 5.0),
        (1.0, 2.0),
        (2.0, low * 2.0),
        (3.0, low),
        (4.0, low * 1.5),
    ]
    .iter()
    .map(|(x, loss)| Sample {
        x: Some(*x),
        loss: Some(*loss),
        lr: Some(0.0001),
        extra: Map::new(),
    })
    .collect();
    Series {
        name: format!("seed {seed} rank {rank}"),
        seed: Some(seed),
        model: String::new(),
        file_name: format!("{seed}-{rank}.json"),
        samples,
        recipe,
        summary: Map::new(),
    }
}

fn board(series: Vec<Series>) -> Board {
    let mut shared = Map::new();
    let mut varying = Vec::new();
    for key in series[0].recipe.keys() {
        let first = &series[0].recipe[key];
        if series.iter().all(|s| s.recipe.get(key) == Some(first)) {
            shared.insert(key.clone(), first.clone());
        } else {
            varying.push(key.clone());
        }
    }
    Board {
        series,
        skipped: 0,
        shared,
        varying,
    }
}

fn rank_board(low16: &[f64], low32: &[f64]) -> Board {
    let mut series = Vec::new();
    for (i, low) in low16.iter().enumerate() {
        series.push(run(i as i64 + 1, 16, *low));
    }
    for (i, low) in low32.iter().enumerate() {
        series.push(run(i as i64 + 1, 32, *low));
    }
    board(series)
}

#[test]
fn the_exact_rank_test_has_the_textbook_values() {
    assert!((exact_p(&[3.0, 4.0, 5.0], &[0.5, 1.0, 2.0], Direction::Lower) - 0.05).abs() < 1e-12);
    assert!((exact_p(&[3.0, 4.0], &[1.0, 2.0], Direction::Lower) - 1.0 / 6.0).abs() < 1e-12);
    assert!((exact_p(&[3.0, 4.0, 5.0], &[0.5, 1.0, 2.0], Direction::Higher) - 1.0).abs() < 1e-12);
    assert_eq!(seeds_needed(0.01, 0.01), Some(16));
    assert_eq!(seeds_needed(0.01, 0.0), None);
}

#[test]
fn a_shared_knob_is_refused_with_its_value() {
    let b = rank_board(&[0.1, 0.12], &[]);
    let column = evaluate(&b, "low", &[]).unwrap();
    let refused = compare_knob(&b, "learning_rate", &column).unwrap_err();
    assert!(refused.contains("Learning rate was 0.0001 on every run"));
    let noise = seed_noise(&b, &column).unwrap();
    assert_eq!(noise.runs, 2);
    assert!((noise.range() - 0.02).abs() < 1e-12);
}

#[test]
fn a_hypothesis_moves_through_its_states_with_the_evidence() {
    let date = "2026-10-06";
    let base = rank_board(&[0.3, 0.32, 0.31], &[0.1, 0.12, 0.11]);
    let h = propose(
        &base,
        &[],
        &[],
        &Proposal {
            knob: "lora_r",
            formula: "low",
            direction: "lower",
            why: "A larger adapter can fit more.",
        },
        date,
    )
    .unwrap();
    assert_eq!(h.statement(), "When LoRA rank goes up, low goes lower.");
    assert!(
        propose(
            &base,
            std::slice::from_ref(&h),
            &[],
            &Proposal {
                knob: "lora_r",
                formula: "low",
                direction: "lower",
                why: "again"
            },
            date
        )
        .is_err()
    );
    assert!(
        propose(
            &base,
            &[],
            &[],
            &Proposal {
                knob: "lora_r",
                formula: "low",
                direction: "lower",
                why: "It is the best by 3."
            },
            date
        )
        .is_err()
    );
    assert!(
        propose(
            &base,
            &[],
            &[],
            &Proposal {
                knob: "dropout",
                formula: "low",
                direction: "lower",
                why: ""
            },
            date
        )
        .is_err()
    );

    let (supported, _) = test_hypothesis(&base, &h, &[], date);
    assert_eq!(supported.state, State::Supported, "{}", supported.detail);
    assert!(supported.detail.contains("p = 0.05"));

    let flipped = rank_board(&[0.1, 0.12, 0.11], &[0.3, 0.32, 0.31]);
    assert_eq!(
        test_hypothesis(&flipped, &h, &[], date).0.state,
        State::Refuted
    );

    let single = rank_board(&[0.3], &[0.1]);
    let (one, _) = test_hypothesis(&single, &h, &[], date);
    assert_eq!(one.state, State::Inconclusive);
    assert!(one.detail.contains("One run per setting"));

    let pairs = rank_board(&[0.3, 0.32], &[0.1, 0.12]);
    let (two, _) = test_hypothesis(&pairs, &h, &[], date);
    assert_eq!(two.state, State::Inconclusive);
    assert!(two.detail.contains("at least three runs per setting"));

    let noisy = rank_board(&[0.3, 0.1, 0.5], &[0.2, 0.4, 0.05]);
    let (inside, _) = test_hypothesis(&noisy, &h, &[], date);
    assert_eq!(inside.state, State::Inconclusive);
    assert!(inside.detail.contains("inside the seed spread"));

    let shared = rank_board(&[0.3, 0.32], &[]);
    assert_eq!(
        test_hypothesis(&shared, &h, &[], date).0.state,
        State::Untestable
    );

    let mut confounded = base.clone();
    for series in confounded.series.iter_mut().skip(3) {
        series
            .recipe
            .insert("learning_rate".into(), Value::from(0.0002));
    }
    confounded.shared.remove("learning_rate");
    confounded.varying.push("learning_rate".into());
    let (mixed, _) = test_hypothesis(&confounded, &h, &[], date);
    assert_eq!(mixed.state, State::Confounded);
    assert!(mixed.detail.contains("learning rate changed as well"));
}

#[test]
fn holm_keeps_two_borderline_verdicts_inconclusive() {
    let date = "2026-10-06";
    let b = rank_board(&[0.3, 0.32, 0.31], &[0.1, 0.12, 0.11]);
    let first = propose(
        &b,
        &[],
        &[],
        &Proposal {
            knob: "lora_r",
            formula: "low",
            direction: "lower",
            why: "",
        },
        date,
    )
    .unwrap();
    let second = propose(
        &b,
        std::slice::from_ref(&first),
        &[],
        &Proposal {
            knob: "lora_r",
            formula: "last",
            direction: "lower",
            why: "",
        },
        date,
    )
    .unwrap();
    let results = test_all(&b, &[first, second], &[], date);
    assert!(results.iter().all(|e| e.state == State::Inconclusive));
    assert!(results[0].detail.contains("Holm"));
}

#[test]
fn a_learned_tool_passes_its_gates_and_grows_from_provisional_to_kept() {
    let date = "2026-10-06";
    let b = rank_board(&[0.3, 0.32], &[0.1, 0.12]);
    assert!(learn_tool(&b, &[], "X", "low", "bad name", date).is_err());
    assert!(learn_tool(&b, &[], "low", "low", "clashes with a measure", date).is_err());
    assert!(learn_tool(&b, &[], "rebound", "last / low", "", date).is_err());
    let refused = learn_tool(
        &b,
        &[],
        "late_tail",
        "median_between(9, 10)",
        "nothing there",
        date,
    )
    .unwrap_err();
    assert!(refused.contains("no value on"));
    let rebound = learn_tool(
        &b,
        &[],
        "rebound",
        "last / low",
        "how far the end sits above the low",
        date,
    )
    .unwrap();
    assert!(!rebound.kept());
    let library = vec![rebound];
    assert!(
        learn_tool(
            &b,
            &library,
            "rebound_two",
            "last/low",
            "same formula",
            date
        )
        .unwrap_err()
        .contains("formula of rebound")
    );
    assert!(
        learn_tool(
            &b,
            &library,
            "rebound_three",
            "1.5 + 0 * low",
            "same values",
            date
        )
        .unwrap_err()
        .contains("same values as rebound")
    );
    let column = evaluate(&b, "rebound * 2", &library).unwrap();
    assert!(
        column
            .finite()
            .iter()
            .all(|(_, v)| (*v - 3.0).abs() < 1e-12)
    );

    let mut library = library;
    note_use(&mut library, "rebound", &b);
    assert!(!library[0].kept());
    let other = rank_board(&[0.4, 0.42], &[0.2, 0.21, 0.22]);
    note_use(&mut library, "rebound", &other);
    assert!(library[0].kept());
    assert_eq!(library[0].uses, 2);
}

#[test]
fn tools_hypotheses_notes_and_weighings_share_one_memory_file() {
    let date = "2026-10-06";
    let dir = scratch("share");
    let b = rank_board(&[0.3, 0.32, 0.31], &[0.1, 0.12, 0.11]);
    remember(&dir, &b, "a note").unwrap();
    record_weighing(&dir, &weighed_now(&b, date)).unwrap();
    let tool = learn_tool(&b, &[], "rebound", "last / low", "end over low", date).unwrap();
    write_tools(&dir, std::slice::from_ref(&tool)).unwrap();
    let mut h = propose(
        &b,
        &[],
        std::slice::from_ref(&tool),
        &Proposal {
            knob: "lora_r",
            formula: "rebound",
            direction: "higher",
            why: "",
        },
        date,
    )
    .unwrap();
    let (evaluation, _) = test_hypothesis(&b, &h, std::slice::from_ref(&tool), date);
    record(&mut h, evaluation);
    write_hypotheses(&dir, std::slice::from_ref(&h)).unwrap();
    assert_eq!(read_tools(&dir), vec![tool]);
    assert_eq!(read_hypotheses(&dir), vec![h.clone()]);
    assert_eq!(recall(&dir, &b).as_deref(), Some("a note"));
    let plan = experiment_for(&b, &h, None, None);
    assert_eq!(plan.seeds_per_level, 3);
    assert_eq!(plan.runs, 6);
    assert_eq!(plan.levels, vec!["16".to_string(), "32".to_string()]);
    assert!(plan.matched.iter().any(|label| label == "learning rate"));
    assert!(!plan.matched.iter().any(|label| label == "LoRA rank"));
    fs::remove_dir_all(dir).unwrap();
    assert!(reason_allowed(
        "A wider adapter has more room to fit the late data."
    ));
    assert!(!reason_allowed("It always wins."));
}

#[test]
fn evidence_gathers_across_folders_until_a_checkpoint_decides() {
    use runforge_core::{
        Book, CHECKPOINT_EVERY, Verdict, comparison_report_full, note_new_folder, verdicts,
    };
    let date = "2026-10-06";
    let first = rank_board(&[0.3, 0.32, 0.31], &[0.1, 0.12, 0.11]);
    let proposal = Proposal {
        knob: "lora_r",
        formula: "low",
        direction: "lower",
        why: "A larger adapter can fit more.",
    };
    let mut h = propose(&first, &[], &[], &proposal, date).unwrap();
    let (evaluation, _) = test_hypothesis(&first, &h, &[], date);
    record(&mut h, evaluation);
    // Seen at registration, so this folder does not count, however clean it is.
    assert!(evidence_of(&h).counted.is_empty());
    let later = [
        (
            rank_board(&[0.29, 0.33, 0.35], &[0.13, 0.09, 0.105]),
            "2026-10-07",
        ),
        (
            rank_board(&[0.36, 0.305, 0.34], &[0.125, 0.14, 0.08]),
            "2026-10-08",
        ),
        (
            rank_board(&[0.37, 0.31, 0.38], &[0.15, 0.085, 0.095]),
            "2026-10-09",
        ),
    ];
    for (board, day) in &later[..2] {
        let (evaluation, _) = test_hypothesis(board, &h, &[], day);
        record(&mut h, evaluation);
    }
    // Two clean folders give about 4.5 squared, short of the 40 one direction needs alone.
    assert_eq!(verdicts(std::slice::from_ref(&h))[0].1, Verdict::Open);
    let (evaluation, _) = test_hypothesis(&later[2].0, &h, &[], later[2].1);
    record(&mut h, evaluation);
    let three = verdicts(std::slice::from_ref(&h));
    assert_eq!(three[0].1, Verdict::Supported);
    assert_eq!(
        three[0].0.counted,
        vec!["2026-10-07", "2026-10-08", "2026-10-09"]
    );

    // The report shows evidence between checkpoints, and a verdict only once one runs.
    let mut book = Book::default();
    let before = comparison_report_full(
        &later[2].0,
        None,
        &Default::default(),
        std::slice::from_ref(&h),
        &[],
        &book,
    );
    assert!(
        before.contains("No checkpoint has judged it yet."),
        "{before}"
    );
    assert!(before.contains("from three folders"));
    for _ in 1..CHECKPOINT_EVERY {
        assert!(note_new_folder(&mut book, std::slice::from_ref(&h), "2026-10-09").is_none());
    }
    let checkpoint = note_new_folder(&mut book, std::slice::from_ref(&h), "2026-10-09").unwrap();
    assert_eq!(checkpoint.number, 1);
    let after = comparison_report_full(
        &later[2].0,
        None,
        &Default::default(),
        std::slice::from_ref(&h),
        &[],
        &book,
    );
    assert!(
        after.contains("At checkpoint 1 (2026-10-09, one hypothesis): supported."),
        "{after}"
    );
    let first_report = comparison_report_full(
        &first,
        None,
        &Default::default(),
        std::slice::from_ref(&h),
        &[],
        &book,
    );
    assert!(
        first_report
            .contains("Not counted: these runs were seen before the hypothesis was registered.")
    );
}

fn evidence_of(h: &runforge_core::Hypothesis) -> runforge_core::Evidence {
    runforge_core::evidence(h)
}

#[test]
fn checkpoints_and_the_tool_cap_survive_a_round_trip() {
    use runforge_core::{Book, LearnedTool, note_new_folder, read_book, write_book};
    let dir = scratch("book");
    assert_eq!(read_book(&dir), Book::default());
    let b = rank_board(&[0.3, 0.32, 0.31], &[0.1, 0.12, 0.11]);
    let proposal = Proposal {
        knob: "lora_r",
        formula: "low",
        direction: "lower",
        why: "",
    };
    let h = propose(&b, &[], &[], &proposal, "2026-10-06").unwrap();
    let mut book = Book::default();
    for _ in 0..runforge_core::CHECKPOINT_EVERY + 2 {
        note_new_folder(&mut book, std::slice::from_ref(&h), "2026-10-06");
    }
    write_book(&dir, &book).unwrap();
    let back = read_book(&dir);
    assert_eq!(back, book);
    assert_eq!(back.since, 2);
    assert_eq!(back.until_next(), runforge_core::CHECKPOINT_EVERY - 2);
    assert_eq!(back.latest_for(&h.id).unwrap().1.1.word(), "open");
    assert!(back.latest_for("h999").is_none());

    // Past the cap, the provisional tool used least is trimmed first.
    let tool = |n: usize, uses: u32, boards: usize| LearnedTool {
        name: format!("tool_{n}"),
        formula: format!("low * {n}"),
        meaning: "a scaled low".into(),
        created: "2026-10-06".into(),
        uses,
        boards: (0..boards).map(|k| format!("board {k}")).collect(),
    };
    let mut tools: Vec<LearnedTool> = (0..50).map(|n| tool(n, 5, 2)).collect();
    tools.push(tool(50, 0, 1));
    write_tools(&dir, &tools).unwrap();
    let kept = read_tools(&dir);
    assert_eq!(kept.len(), 50);
    assert!(!kept.iter().any(|t| t.name == "tool_50"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_test_on_the_wrong_method_or_a_broken_formula_is_not_testable() {
    let b = rank_board(&[0.3, 0.32, 0.31], &[0.1, 0.12, 0.11]);
    let proposal = Proposal {
        knob: "lora_r",
        formula: "low",
        direction: "lower",
        why: "",
    };
    let mut h = propose(&b, &[], &[], &proposal, "2026-10-06").unwrap();
    let mut other = b.clone();
    other
        .shared
        .insert("method".into(), Value::from("full fine-tune"));
    let (wrong, _) = test_hypothesis(&other, &h, &[], "2026-10-06");
    assert_eq!(wrong.state, State::Untestable);
    assert!(wrong.detail.contains("different method"));
    h.formula = "gone_tool * 2".into();
    let (broken, _) = test_hypothesis(&b, &h, &[], "2026-10-06");
    assert_eq!(broken.state, State::Untestable);
    assert!(broken.detail.contains("not a measure"));
    // A rank test that does not reach one in twenty stays inconclusive.
    let h = propose(&b, &[], &[], &proposal, "2026-10-06").unwrap();
    let mixed = rank_board(&[0.30, 0.10, 0.32], &[0.12, 0.31, 0.105]);
    let (stays, _) = test_hypothesis(&mixed, &h, &[], "2026-10-06");
    assert_eq!(stays.state, State::Inconclusive, "{}", stays.detail);
    for state in [
        State::Untestable,
        State::Confounded,
        State::Inconclusive,
        State::Supported,
        State::Refuted,
    ] {
        assert!(!state.word().is_empty());
    }
    for verdict in [
        runforge_core::Verdict::Supported,
        runforge_core::Verdict::Refuted,
        runforge_core::Verdict::Open,
    ] {
        assert!(!verdict.word().is_empty());
    }
}

#[test]
fn a_column_names_the_run_it_could_not_measure_and_known_runs_lists_the_ledger() {
    let mut b = rank_board(&[0.3, 0.32], &[0.1, 0.12]);
    b.series[0].samples.clear();
    let column = evaluate(&b, "low", &[]).unwrap();
    assert!(column.lines()[0].contains("no value."));
    let dir = scratch("known");
    assert!(runforge_core::known_runs(&dir).is_empty());
    let full = rank_board(&[0.3, 0.32], &[0.1, 0.12]);
    record_weighing(&dir, &weighed_now(&full, "2026-10-06")).unwrap();
    record_weighing(&dir, &weighed_now(&full, "2026-10-07")).unwrap();
    assert_eq!(runforge_core::known_runs(&dir).len(), 4);
    fs::remove_dir_all(dir).unwrap();
}
