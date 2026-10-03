use std::fs;

use runforge_core::{
    HistoryError, LossSample, Prefs, Theme, choose_prefs_dir, curve_csv, curve_segments,
    entry_json, hyperparameter_diffs, list_csv, load_folder, load_text, pick_best_loss,
    write_prefs,
};

fn failed_empty() -> &'static str {
    r#"[
      {
        "run_id": "failed-empty",
        "status": "failed",
        "model_name": "Qwen/Qwen2.5-7B-Instruct",
        "started_at": "2026-05-21T04:54:14.646724",
        "final_loss": null,
        "loss_history": [],
        "failure_reason": "TrainingError: GPU error during training",
        "session_kind": "single_run"
      }
    ]"#
}

#[test]
fn failed_entry_with_an_empty_curve_stays_in_the_list() {
    let history = load_text(failed_empty()).unwrap();
    assert_eq!(history.entries.len(), 1);
    assert!(history.entries[0].loss.is_empty());
    assert!(history.entries[0].final_loss.is_none());
    assert_eq!(history.entries[0].status, "failed");
    assert!(curve_segments(&history.entries[0].loss).is_empty());
    assert!(list_csv(&history).contains("failed-empty"));
}

#[test]
fn completed_curve_does_not_append_final_loss() {
    let history = load_text(
        r#"[
          {
            "run_id": "done",
            "status": "completed",
            "final_loss": 0.11,
            "loss_history": [0.9, 0.4, 0.2],
            "started_at": "2026-05-21T01:00:00"
          }
        ]"#,
    )
    .unwrap();
    let points: Vec<f64> = history.entries[0]
        .loss
        .iter()
        .map(|sample| match sample {
            LossSample::Point(value) => *value,
            LossSample::Gap => panic!("this curve has no gap"),
        })
        .collect();
    assert_eq!(points, vec![0.9, 0.4, 0.2]);
    assert_eq!(history.entries[0].final_loss, Some(0.11));
    let csv = curve_csv(&history.entries[0]);
    assert!(!csv.contains("0.11"));
    assert!(csv.contains("0.2"));
}

#[test]
fn more_than_one_hundred_stored_points_are_kept() {
    let points = (0..101)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let history = load_text(&format!(
        r#"[{{"run_id":"long","loss_history":[{points}]}}]"#
    ))
    .unwrap();
    assert_eq!(history.entries[0].loss.len(), 101);
}

#[test]
fn eval_summary_is_read_and_generations_are_not_required() {
    let history = load_text(
        r#"[
          {
            "run_id": "scored",
            "status": "completed",
            "eval": {
              "held_out_loss": 1.5,
              "perplexity": 4.48,
              "task_metrics": { "token_f1": 0.5, "contains": 1.0 },
              "eval_n": 8,
              "n_prompts": 5,
              "generations": [{ "prompt": "hi", "completion": "there" }]
            }
          }
        ]"#,
    )
    .unwrap();
    let eval = history.entries[0].eval_summary.as_ref().unwrap();
    assert_eq!(eval.held_out_loss, Some(1.5));
    assert_eq!(eval.perplexity, Some(4.48));
    assert_eq!(eval.eval_n, Some(8));
    assert_eq!(eval.n_prompts, Some(5));
    assert_eq!(
        eval.task_metrics,
        vec![("contains".to_string(), 1.0), ("token_f1".to_string(), 0.5)]
    );
}

#[test]
fn unknown_key_survives_export() {
    let history =
        load_text(r#"[{"run_id":"kept","trainer_build":"abc","loss_history":[0.5]}]"#).unwrap();
    let json = entry_json(&history.entries[0]);
    assert!(json.contains("trainer_build"));
    assert!(json.contains("abc"));
}

#[test]
fn null_loss_sample_is_a_gap() {
    let history = load_text(r#"[{"run_id":"gap","loss_history":[1.0, null, 0.25]}]"#).unwrap();
    assert_eq!(
        history.entries[0].loss,
        vec![
            LossSample::Point(1.0),
            LossSample::Gap,
            LossSample::Point(0.25)
        ]
    );
    let segments = curve_segments(&history.entries[0].loss);
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0], vec![[0.0, 1.0]]);
    assert_eq!(segments[1], vec![[2.0, 0.25]]);
    let csv = curve_csv(&history.entries[0]);
    assert!(csv.contains("1,\n") || csv.contains("1,\r\n") || csv.lines().any(|line| line == "1,"));
}

#[test]
fn missing_run_id_is_skipped_and_the_rest_remain() {
    let history =
        load_text(r#"[{"status":"completed"}, {"run_id":"kept","status":"completed"}, "nope"]"#)
            .unwrap();
    assert_eq!(history.skipped, 2);
    assert_eq!(history.entries.len(), 1);
    assert_eq!(history.entries[0].run_id, "kept");
}

#[test]
fn a_file_that_is_not_an_array_is_refused() {
    let error = load_text(r#"{"runs":[]}"#).unwrap_err();
    assert!(matches!(error, HistoryError::NotArray));
}

#[test]
fn duplicate_json_key_refuses_the_file() {
    let error = load_text(r#"[{"run_id":"a","run_id":"b"}]"#).unwrap_err();
    assert!(matches!(error, HistoryError::Parse(_)));
}

#[test]
fn best_loss_puts_a_non_finite_loss_last() {
    let history = load_text(
        r#"[
          {"run_id":"missing","final_loss":null},
          {"run_id":"higher","final_loss":0.4},
          {"run_id":"best","final_loss":0.2}
        ]"#,
    )
    .unwrap();
    assert_eq!(history.best_loss_index(), Some(2));
    assert_eq!(
        pick_best_loss(&[Some(1.0), Some(f64::NAN), Some(0.5), None]),
        Some(2)
    );
    assert_eq!(pick_best_loss(&[Some(f64::INFINITY), None]), None);
}

#[test]
fn equal_learning_rates_are_not_a_difference() {
    let history = load_text(
        r#"[
          {"run_id":"a","hyperparameters":{"learning_rate":0.0002,"seed":1}},
          {"run_id":"b","hyperparameters":{"learning_rate":2e-4,"seed":2}}
        ]"#,
    )
    .unwrap();
    let diffs = hyperparameter_diffs(
        &history.entries[0].hyperparameters,
        &history.entries[1].hyperparameters,
    );
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].key, "seed");
}

#[test]
fn newest_row_is_first_and_a_bad_timestamp_is_last() {
    let history = load_text(
        r#"[
          {"run_id":"old","started_at":"2026-05-21T01:00:00"},
          {"run_id":"new","started_at":"2026-05-22T01:00:00"},
          {"run_id":"bad","started_at":"yesterday"}
        ]"#,
    )
    .unwrap();
    let order = history.display_order();
    let ids: Vec<_> = order
        .iter()
        .map(|index| history.entries[*index].run_id.as_str())
        .collect();
    assert_eq!(ids, vec!["new", "old", "bad"]);
}

#[test]
fn duplicate_ids_both_stay() {
    let history = load_text(r#"[{"run_id":"same"},{"run_id":"same"},{"run_id":"other"}]"#).unwrap();
    assert_eq!(history.entries.len(), 3);
    assert_eq!(history.duplicate_ids, 1);
    assert_eq!(history.entries[0].file_index, 0);
    assert_eq!(history.entries[1].file_index, 1);
}

#[test]
fn a_schema_other_than_1_0_is_a_note() {
    let history = load_text(
        r#"[{"run_id":"a","schema_version":"1.0"},{"run_id":"b","schema_version":"2.0"}]"#,
    )
    .unwrap();
    assert_eq!(history.schema_notes, vec!["2.0".to_string()]);
}

#[test]
fn direct_history_wins_over_the_nested_output_file() {
    let root = std::env::temp_dir().join(format!("runforge-history-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("output")).unwrap();
    fs::write(root.join("run_history.json"), r#"[{"run_id":"direct"}]"#).unwrap();
    fs::write(
        root.join("output").join("run_history.json"),
        r#"[{"run_id":"nested"}]"#,
    )
    .unwrap();
    let (path, history) = load_folder(&root).unwrap();
    assert!(path.ends_with("run_history.json"));
    assert!(!path.ends_with(std::path::Path::new("output").join("run_history.json")));
    assert_eq!(history.entries[0].run_id, "direct");

    fs::remove_file(root.join("run_history.json")).unwrap();
    let (_, nested) = load_folder(&root).unwrap();
    assert_eq!(nested.entries[0].run_id, "nested");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn negative_zero_round_trips_in_the_export() {
    let history = load_text(r#"[{"run_id":"signed","final_loss":-0.0}]"#).unwrap();
    let json = entry_json(&history.entries[0]);
    assert!(json.contains("-0.0"), "{json}");
}

#[test]
fn packaged_prefs_dir_wins_and_unknown_keys_survive() {
    let exe = std::env::temp_dir().join("runforge-exe");
    let local = std::env::temp_dir().join("runforge-localstate");
    assert_eq!(choose_prefs_dir(Some(&local), &exe), local);
    assert_eq!(choose_prefs_dir(None, &exe), exe);

    let dir = std::env::temp_dir().join(format!("runforge-prefs-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("preferences.json"),
        "{\n  \"theme\": \"light\",\n  \"future\": 1\n}\n",
    )
    .unwrap();
    let mut prefs = runforge_core::read_prefs(&dir);
    assert_eq!(prefs.theme, Theme::Light);
    prefs.set_folder(&exe);
    write_prefs(&dir, &prefs).unwrap();
    let saved = fs::read_to_string(dir.join("preferences.json")).unwrap();
    assert!(saved.contains("\"future\""));
    assert!(saved.contains("light"));
    let again = runforge_core::read_prefs(&dir);
    assert_eq!(again.last_folder.unwrap(), exe);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn default_prefs_are_dark() {
    let prefs = Prefs::default();
    assert_eq!(prefs.theme, Theme::Dark);
    assert!(prefs.last_folder.is_none());
}
