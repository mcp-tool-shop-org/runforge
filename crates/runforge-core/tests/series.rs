use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use runforge_core::{
    HistoryError, Sample, band_segments, comparison_report, comparison_report_with,
    earlier_readings, epoch_floor, fingerprint, format_measure, ledger_for, load_series_folder,
    loss_segments, low_band, read_board, recall, recipe_keys, record_weighing, remember,
    sidecar_prompt, spikes_above, weighed_now,
};

fn scratch(name: &str) -> std::path::PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "runforge-series-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn curve(seed: i64, last: f64, low: f64) -> String {
    format!(
        r#"{{
            "seed": {seed},
            "model": "Qwen/Qwen2.5-7B-Instruct",
            "hyperparameters": {{
                "method": "bf16 LoRA",
                "learning_rate": 0.00015,
                "lr_scheduler": "cosine",
                "warmup_steps": 10,
                "lora_r": 16,
                "lora_alpha": 32,
                "lora_dropout": 0.1,
                "target_modules": ["q_proj", "v_proj"],
                "extra_flag": true
            }},
            "training_summary": {{"final_loss": 0.7}},
            "saturation_log": {{
                "loss_curve": [
                    {{"epoch": 0.0, "loss": 12.0, "lr": 0.0}},
                    {{"epoch": 1.0, "loss": null, "lr": 0.0001}},
                    {{"epoch": 7.0, "loss": {low}, "lr": 0.000001, "grad_norm": 0.4}},
                    {{"epoch": 8.0, "loss": {last}, "lr": 0.000000001}}
                ]
            }}
        }}"#
    )
}

fn write_pair(root: &Path) {
    let pod_a = root.join("podA");
    let pod_b = root.join("podB");
    fs::create_dir_all(&pod_a).unwrap();
    fs::create_dir_all(&pod_b).unwrap();
    fs::write(pod_a.join("run-config-seed13.json"), curve(13, 0.08, 0.02)).unwrap();
    fs::write(
        pod_b.join("run-config-seed1024.json"),
        curve(1024, 0.05, 0.04),
    )
    .unwrap();
    let buried = pod_a.join("nested");
    fs::create_dir_all(&buried).unwrap();
    fs::write(buried.join("run-config-seed99.json"), curve(99, 0.01, 0.01)).unwrap();
    fs::write(pod_a.join("notes.json"), "{}").unwrap();
    fs::write(pod_a.join("run-config-broken.json"), "{").unwrap();
}

#[test]
fn a_series_folder_keeps_every_sample_and_the_shared_recipe() {
    let root = scratch("pair");
    write_pair(&root);
    let board = load_series_folder(&root).unwrap();
    assert_eq!(board.series.len(), 2);
    assert_eq!(board.skipped, 1);
    assert_eq!(board.series[0].name, "seed 13");
    assert_eq!(board.series[1].seed, Some(1024));
    assert_eq!(board.series[0].samples.len(), 4);
    assert!(board.series[0].samples[1].loss.is_none());
    assert_eq!(
        board.series[0].samples[2]
            .extra
            .get("grad_norm")
            .and_then(|value| value.as_f64()),
        Some(0.4)
    );
    assert!(board.shared.contains_key("lora_r"));
    assert!(board.varying.is_empty());
    assert!(recipe_keys(&board.shared).contains(&"extra_flag"));
    let segments = loss_segments(&board.series[0].samples, false);
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0], vec![[0.0, 12.0]]);
    assert!(
        !segments
            .iter()
            .flatten()
            .any(|point| point[1] == 0.0 && point[0] == 1.0)
    );
    assert!(
        !board.series[0]
            .samples
            .iter()
            .any(|sample| sample.loss == Some(0.7))
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_reading_names_the_climb_and_the_two_rankings() {
    let root = scratch("read");
    write_pair(&root);
    let board = load_series_folder(&root).unwrap();
    let reading = read_board(&board);
    let lines = reading.lines().join("\n");
    assert!(lines.contains("Seed is the lever that changes"));
    assert!(lines.contains("seed 13"));
    assert!(lines.contains("The line climbs after the low"));
    assert!(lines.contains("training_summary.final_loss is 0.7"));
    assert!(lines.contains("not a sample on the curve"));
    assert!(reading.ends_differ);
    assert_eq!(reading.deepest, vec!["seed 13".to_string()]);
    assert_eq!(reading.quietest_end, vec!["seed 1024".to_string()]);
    assert!(lines.contains("would hide the deeper low"));
    let (start, end, top) = low_band(&board).unwrap();
    assert_eq!(start, 6.0);
    assert_eq!(end, 8.0);
    assert!(top > 0.08 && top < 0.2);
    let prompt = sidecar_prompt(&reading, &board, &[]);
    assert!(prompt.contains("Use no digit"));
    assert!(prompt.contains("Do not name a winner"));
    assert!(!prompt.contains("0.02"));
    assert!(!prompt.contains("seed 13"));
    assert!(!prompt.contains("run-config"));
    assert!(!prompt.contains('\\'));
    let report = comparison_report(&board, None);
    assert!(report.contains("same run, seed 13"));
    assert!(!report.contains("same run, seed 1024"));
    assert!(report.contains("lowest last sample"));
    assert!(report.contains("seed 1024"));
    assert!(report.contains("LoRA scale (alpha/r) is 2"));
    assert!(report.contains("twentieth"));
    assert!(report.contains("half an epoch"));
    assert!(report.contains("middle half from"));
    assert!(!report.contains("placeholder"));
    assert!(report.contains("2106.09685"));
    assert!(!report.contains("1706.02677"));
    assert!(!report.contains("would disappear"));
    assert!(!report.contains("This comparison has no winner"));
    assert!(!report.contains("move the learning rate"));
    assert!(!report.contains("**"));
    assert!(report.lines().all(|line| !line.starts_with('#')));
    assert!(!report.contains('\\'));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_grandchild_is_not_a_series_and_an_empty_folder_is_refused() {
    let root = scratch("empty");
    let error = load_series_folder(&root).unwrap_err();
    assert!(matches!(error, HistoryError::NoSeries));
    let buried = root.join("pod").join("nested");
    fs::create_dir_all(&buried).unwrap();
    fs::write(buried.join("run-config-seed99.json"), curve(99, 0.01, 0.01)).unwrap();
    let error = load_series_folder(&root).unwrap_err();
    assert!(matches!(error, HistoryError::NoSeries));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_duplicate_key_refuses_that_file_only() {
    let root = scratch("dup");
    fs::write(root.join("run-config-seed13.json"), curve(13, 0.08, 0.02)).unwrap();
    fs::write(
        root.join("run-config-dup.json"),
        r#"{"seed": 1, "seed": 2, "saturation_log": {"loss_curve": []}}"#,
    )
    .unwrap();
    let board = load_series_folder(&root).unwrap();
    assert_eq!(board.series.len(), 1);
    assert_eq!(board.skipped, 1);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn memory_recalls_the_recipe_and_not_another_one() {
    let root = scratch("mem");
    write_pair(&root);
    let board = load_series_folder(&root).unwrap();
    let prefs = scratch("mem-prefs");
    assert!(recall(&prefs, &board).is_none());
    remember(&prefs, &board, "seed 13 climbed after epoch 7").unwrap();
    assert_eq!(
        recall(&prefs, &board).as_deref(),
        Some("seed 13 climbed after epoch 7")
    );
    let other_root = scratch("mem-other");
    fs::create_dir_all(&other_root).unwrap();
    let mut body = curve(13, 0.08, 0.02);
    body = body.replace("\"lora_r\": 16", "\"lora_r\": 32");
    fs::write(other_root.join("run-config-seed13.json"), body).unwrap();
    let other = load_series_folder(&other_root).unwrap();
    assert_ne!(fingerprint(&board), fingerprint(&other));
    assert!(recall(&prefs, &other).is_none());
    fs::remove_dir_all(&other_root).unwrap();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&prefs).unwrap();
}

#[test]
fn earlier_readings_keep_three_model_notes_for_the_same_method() {
    let prefs = scratch("early-prefs");
    let root = scratch("early-board");
    write_pair(&root);
    let board = load_series_folder(&root).unwrap();
    remember(&prefs, &board, "same\n\nmodel: this recipe").unwrap();
    let mut roots = vec![root];
    for rank in [4_i64, 8, 64, 32] {
        let folder = scratch(&format!("early-rank-{rank}"));
        let body = curve(13, 0.08, 0.02).replace("\"lora_r\": 16", &format!("\"lora_r\": {rank}"));
        fs::write(folder.join("run-config-seed13.json"), body).unwrap();
        let other = load_series_folder(&folder).unwrap();
        remember(
            &prefs,
            &other,
            &format!("reading\n\nmodel: rank {rank} held"),
        )
        .unwrap();
        roots.push(folder);
    }
    let bare = scratch("early-bare");
    let bare_body = curve(13, 0.08, 0.02).replace("\"lora_r\": 16", "\"lora_r\": 128");
    fs::write(bare.join("run-config-seed13.json"), bare_body).unwrap();
    remember(&prefs, &load_series_folder(&bare).unwrap(), "no model half").unwrap();
    roots.push(bare);
    let swapped = scratch("early-method");
    let swapped_body =
        curve(13, 0.08, 0.02).replace("\"method\": \"bf16 LoRA\"", "\"method\": \"full\"");
    fs::write(swapped.join("run-config-seed13.json"), swapped_body).unwrap();
    remember(
        &prefs,
        &load_series_folder(&swapped).unwrap(),
        "reading\n\nmodel: full finetune",
    )
    .unwrap();
    roots.push(swapped);
    let earlier = earlier_readings(&prefs, &board);
    assert_eq!(earlier.len(), 3);
    assert!(earlier[0].contains("rank 32"));
    assert!(earlier[1].contains("rank 64"));
    assert!(earlier[2].contains("rank 8"));
    assert!(earlier.iter().all(|line| !line.contains("this recipe")));
    assert!(earlier.iter().all(|line| !line.contains("full finetune")));
    assert!(earlier.iter().all(|line| !line.contains("rank 4")));
    fs::remove_dir_all(&prefs).unwrap();
    for folder in roots {
        fs::remove_dir_all(folder).unwrap();
    }
}

#[test]
fn the_low_row_gaps_a_spike_and_keeps_the_epoch_floor() {
    let samples = vec![
        Sample {
            x: Some(6.0),
            loss: Some(0.05),
            lr: None,
            extra: serde_json::Map::new(),
        },
        Sample {
            x: Some(6.2),
            loss: Some(0.4),
            lr: None,
            extra: serde_json::Map::new(),
        },
        Sample {
            x: Some(6.4),
            loss: Some(0.03),
            lr: None,
            extra: serde_json::Map::new(),
        },
        Sample {
            x: Some(7.1),
            loss: Some(0.08),
            lr: None,
            extra: serde_json::Map::new(),
        },
        Sample {
            x: Some(7.4),
            loss: Some(0.02),
            lr: None,
            extra: serde_json::Map::new(),
        },
    ];
    assert_eq!(spikes_above(&samples, 6.0, 0.1), 1);
    assert_eq!(
        band_segments(&samples, 6.0, 0.1),
        vec![
            vec![[6.0, 0.05]],
            vec![[6.4, 0.03], [7.1, 0.08], [7.4, 0.02]],
        ]
    );
    assert_eq!(
        epoch_floor(&samples, 6.0),
        vec![vec![[6.4, 0.03], [7.4, 0.02]]]
    );
    let root = scratch("one");
    fs::write(
        root.join("run-config-seed3.json"),
        r#"{"seed":3,"saturation_log":{"loss_curve":[{"epoch":4,"loss":0.2,"lr":0.1}]}}"#,
    )
    .unwrap();
    let board = load_series_folder(&root).unwrap();
    let (start, end, top) = low_band(&board).unwrap();
    assert!(end > start);
    assert!(top > 0.2);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn measures_keep_small_rates_and_drop_trailing_zeros() {
    assert_eq!(format_measure(12.1289), "12.1289");
    assert_eq!(format_measure(0.0176), "0.0176");
    assert_eq!(format_measure(0.00015), "0.00015");
    assert_eq!(format_measure(7.0), "7");
    assert_eq!(format_measure(0.0), "0");
    assert_eq!(format_measure(12288.0), "12288");
    assert!(format_measure(1.57e-9).contains('e'));
}

#[test]
fn the_ledger_keeps_the_numbers_beside_the_notes_and_groups_by_recipe_and_method() {
    let root = scratch("ledger");
    write_pair(&root);
    let board = load_series_folder(&root).unwrap();
    let prefs = scratch("ledger-prefs");
    assert!(ledger_for(&prefs, &board).is_empty());
    remember(&prefs, &board, "a note").unwrap();
    record_weighing(&prefs, &weighed_now(&board, "2026-10-01")).unwrap();
    // The note survived the weighing, and the weighing survives a later note.
    assert_eq!(recall(&prefs, &board).as_deref(), Some("a note"));
    remember(&prefs, &board, "a later note").unwrap();
    let ledger = ledger_for(&prefs, &board);
    let first = ledger.same_runs.as_ref().unwrap();
    assert_eq!(first.date, "2026-10-01");
    assert_eq!(first.runs.len(), 2);
    assert!(
        first
            .runs
            .iter()
            .all(|run| run.q1 <= run.median && run.median <= run.q3)
    );
    // The same runs with the same numbers keep their first date.
    record_weighing(&prefs, &weighed_now(&board, "2026-10-05")).unwrap();
    assert_eq!(
        ledger_for(&prefs, &board).same_runs.unwrap().date,
        "2026-10-01"
    );
    let report = comparison_report_with(&board, Some("2026-10-05"), &ledger_for(&prefs, &board));
    assert!(report.contains("first weighed on 2026-10-01"));

    // Another set of seeds on the same recipe, and a recipe that only shares the method.
    let again = scratch("ledger-again");
    fs::write(again.join("run-config-seed7.json"), curve(7, 0.06, 0.03)).unwrap();
    fs::write(again.join("run-config-seed8.json"), curve(8, 0.07, 0.035)).unwrap();
    let other = scratch("ledger-other");
    let rank32 =
        |seed, last, low| curve(seed, last, low).replace("\"lora_r\": 16", "\"lora_r\": 32");
    fs::write(other.join("run-config-seed13.json"), rank32(13, 0.08, 0.02)).unwrap();
    let again_board = load_series_folder(&again).unwrap();
    let other_board = load_series_folder(&other).unwrap();
    record_weighing(&prefs, &weighed_now(&other_board, "2026-10-02")).unwrap();
    let ledger = ledger_for(&prefs, &again_board);
    assert!(ledger.same_runs.is_none());
    assert_eq!(ledger.same_recipe.len(), 1);
    assert_eq!(ledger.same_method.len(), 1);
    assert_eq!(ledger.same_method[0].date, "2026-10-02");
    let report = comparison_report_with(&again_board, Some("2026-10-06"), &ledger);
    assert!(report.contains("On 2026-10-01, other runs of this same recipe: two runs"));
    assert!(report.contains(
        "On 2026-10-02, a different recipe with the same method (LoRA rank 32 where this has 16)"
    ));
    assert!(report.contains("Today's middles run from"));
    assert!(!report.contains(&root.to_string_lossy().to_string()));
    for dir in [root, prefs, again, other] {
        fs::remove_dir_all(dir).unwrap();
    }
}
