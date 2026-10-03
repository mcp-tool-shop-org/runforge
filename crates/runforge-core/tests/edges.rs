use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use runforge_core::{
    HistoryError, LossSample, Theme, curve_csv, curve_segments, entry_json, finite_points,
    format_f64, hyperparameter_diffs, list_csv, load_bytes, load_folder, load_text, pick_best_loss,
    read_prefs, write_prefs,
};

fn scratch(name: &str) -> std::path::PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "runforge-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_missing_folder_file_is_not_found() {
    let dir = scratch("missing");
    let error = load_folder(&dir).unwrap_err();
    assert_eq!(error.to_string(), "no run_history.json in this folder");
    assert!(matches!(error, HistoryError::NotFound));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn broken_json_names_the_parse_error() {
    let error = load_text("{").unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("run history is not valid JSON")
    );
    let trailing = load_text("[] []").unwrap_err();
    assert!(matches!(trailing, HistoryError::Parse(_)));
    let bytes = load_bytes(&[0xff, 0xfe]).unwrap_err();
    assert!(matches!(bytes, HistoryError::Parse(_)));
}

#[test]
fn a_nested_duplicate_key_refuses_the_file() {
    let error = load_text(r#"[{"run_id":"a","hyperparameters":{"lr":1,"lr":2}}]"#).unwrap_err();
    assert!(error.to_string().contains("duplicate key"));
}

#[test]
fn rows_without_a_run_id_are_skipped() {
    let history = load_text(
        r#"[null, true, 1, "x", {"run_id":""}, {"run_id":"ok","loss_history":[true,"no",0.5]}]"#,
    )
    .unwrap();
    assert_eq!(history.skipped, 5);
    assert_eq!(history.entries[0].loss.len(), 3);
    assert!(matches!(history.entries[0].loss[0], LossSample::Gap));
    assert!(matches!(history.entries[0].loss[1], LossSample::Gap));
    assert!(matches!(history.entries[0].loss[2], LossSample::Point(_)));
}

#[test]
fn text_fields_keep_numbers_bools_and_a_string_step_count() {
    let history = load_text(
        r#"[{
            "run_id":"wide",
            "status": true,
            "steps": "12",
            "dataset_info": 7,
            "timestamp": "2026-05-21T00:00:00Z",
            "started_at": "",
            "final_loss": "n/a",
            "duration_seconds": "soon",
            "hyperparameters": [],
            "export_paths": "model.gguf",
            "schema_version": 2,
            "eval": []
        }]"#,
    )
    .unwrap();
    let entry = &history.entries[0];
    assert_eq!(entry.status, "true");
    assert_eq!(entry.steps, "12");
    assert_eq!(entry.dataset_info, "7");
    assert_eq!(entry.started_at, "2026-05-21T00:00:00Z");
    assert!(entry.final_loss.is_none());
    assert!(entry.duration_seconds.is_none());
    assert!(entry.hyperparameters.is_empty());
    assert_eq!(entry.export_paths, vec!["model.gguf".to_string()]);
    assert_eq!(entry.schema_version.as_deref(), Some("2"));
    assert_eq!(history.schema_notes, vec!["2".to_string()]);
    assert!(entry.eval_summary.is_none());
    let ordered = load_text(
        r#"[{"run_id":"bad","started_at":"yesterday"},{"run_id":"wide","timestamp":"2026-05-21T00:00:00Z","started_at":""}]"#,
    )
    .unwrap();
    let ids: Vec<_> = ordered
        .display_order()
        .into_iter()
        .map(|index| ordered.entries[index].run_id.as_str())
        .collect();
    assert_eq!(ids, vec!["wide", "bad"]);
}

#[test]
fn eval_keeps_whole_floats_and_drops_a_bad_metric() {
    let history = load_text(
        r#"[{
            "run_id":"ev",
            "export_paths": ["a", 1, "b"],
            "eval": {
                "held_out_loss": "x",
                "eval_n": 8.0,
                "n_prompts": 8.5,
                "task_metrics": { "zeta": 1.0, "alpha": 0.5, "skip": "no", "keep": 0.25 }
            }
        }]"#,
    )
    .unwrap();
    let entry = &history.entries[0];
    assert_eq!(entry.export_paths, vec!["a".to_string(), "b".to_string()]);
    let eval = entry.eval_summary.as_ref().unwrap();
    assert!(eval.held_out_loss.is_none());
    assert_eq!(eval.eval_n, Some(8));
    assert!(eval.n_prompts.is_none());
    assert_eq!(
        eval.task_metrics,
        vec![
            ("alpha".to_string(), 0.5),
            ("keep".to_string(), 0.25),
            ("zeta".to_string(), 1.0)
        ]
    );
    let text_count = load_text(r#"[{"run_id":"ev","eval":{"eval_n":"8"}}]"#).unwrap();
    assert!(
        text_count.entries[0]
            .eval_summary
            .as_ref()
            .unwrap()
            .eval_n
            .is_none()
    );
}

#[test]
fn equal_times_keep_file_order_and_the_best_tie_keeps_the_first() {
    let history = load_text(
        r#"[
            {"run_id":"a","started_at":"2026-05-21T01:00:00","final_loss":0.5},
            {"run_id":"b","started_at":"2026-05-21T01:00:00","final_loss":0.5},
            {"run_id":"c","started_at":"not-a-time","final_loss":-1.0}
        ]"#,
    )
    .unwrap();
    let ids: Vec<_> = history
        .display_order()
        .into_iter()
        .map(|index| history.entries[index].run_id.as_str())
        .collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
    let undated = load_text(r#"[{"run_id":"a","started_at":"nope"},{"run_id":"b"}]"#).unwrap();
    let undated_ids: Vec<_> = undated
        .display_order()
        .into_iter()
        .map(|index| undated.entries[index].run_id.as_str())
        .collect();
    assert_eq!(undated_ids, vec!["a", "b"]);
    assert_eq!(history.best_loss_index(), Some(2));
    assert_eq!(pick_best_loss(&[Some(0.5), Some(0.5), Some(-1.0)]), Some(2));
    assert_eq!(pick_best_loss(&[Some(0.4), Some(0.4)]), Some(0));
    assert!(history.get(9).is_none());
    assert_eq!(history.get(1).unwrap().run_id, "b");
}

#[test]
fn hyperparameter_shapes_compare_by_parsed_value() {
    let history = load_text(
        r#"[
            {"run_id":"a","hyperparameters":{
                "flag": true, "same_flag": true, "tags": ["x"], "nest": {"k": 1}, "only": 1, "same": null, "name": "ada", "note": "a"
            }},
            {"run_id":"b","hyperparameters":{
                "flag": false, "same_flag": true, "tags": ["x","y"], "nest": {"k": 1}, "same": null, "extra": "z", "name": "ada", "note": "b"
            }}
        ]"#,
    )
    .unwrap();
    let diffs = hyperparameter_diffs(
        &history.entries[0].hyperparameters,
        &history.entries[1].hyperparameters,
    );
    let keys: Vec<_> = diffs.iter().map(|diff| diff.key.as_str()).collect();
    assert_eq!(keys, vec!["extra", "flag", "note", "only", "tags"]);
    assert_eq!(diffs[0].left, "");
    assert_eq!(diffs[3].right, "");
}

#[test]
fn list_csv_quotes_and_orders_newest_first() {
    let history = load_text(
        r#"[
            {"run_id":"old","started_at":"2026-01-01T00:00:00","failure_reason":"say \"hi\", now","final_loss":0.5,"duration_seconds":1.5,"steps":3},
            {"run_id":"new","started_at":"2026-02-01T00:00:00"}
        ]"#,
    )
    .unwrap();
    let csv = list_csv(&history);
    assert!(csv.starts_with(
        "run_id,status,session_kind,model_name,final_loss,steps,duration_seconds,started_at,failure_reason\n"
    ));
    let lines: Vec<_> = csv.lines().collect();
    assert!(lines[1].starts_with("new,"));
    assert!(lines[2].contains("\"say \"\"hi\"\", now\""));
    assert!(lines[2].contains("0.5"));
    assert!(lines[2].contains("1.5"));
    assert!(curve_csv(&history.entries[0]).ends_with('\n'));
    assert!(entry_json(&history.entries[0]).ends_with('\n'));
    assert_eq!(format_f64(-0.0), "-0.0");
}

#[test]
fn a_leading_gap_does_not_invent_a_segment() {
    let history =
        load_text(r#"[{"run_id":"g","loss_history":[null, 1.0, null, null, 2.0]}]"#).unwrap();
    let segments = curve_segments(&history.entries[0].loss);
    assert_eq!(segments, vec![vec![[1.0, 1.0]], vec![[4.0, 2.0]]]);
    assert_eq!(
        finite_points(&history.entries[0].loss),
        vec![[1.0, 1.0], [4.0, 2.0]]
    );
    let csv = curve_csv(&history.entries[0]);
    assert!(csv.lines().any(|line| line == "0,"));
    assert!(csv.lines().any(|line| line == "2,"));
}

#[test]
fn two_duplicate_ids_are_both_counted() {
    let history = load_text(
        r#"[{"run_id":"a"},{"run_id":"a"},{"run_id":"b"},{"run_id":"b"},{"run_id":"c","schema_version":"9"},{"run_id":"d","schema_version":"2.0"}]"#,
    )
    .unwrap();
    assert_eq!(history.duplicate_ids, 2);
    assert_eq!(
        history.schema_notes,
        vec!["2.0".to_string(), "9".to_string()]
    );
}

#[test]
fn prefs_ignore_a_bad_file_and_a_blank_folder() {
    let dir = scratch("prefs");
    assert_eq!(read_prefs(&dir).theme, Theme::Dark);
    fs::write(dir.join("preferences.json"), "[]").unwrap();
    assert!(read_prefs(&dir).last_folder.is_none());
    fs::write(
        dir.join("preferences.json"),
        r#"{"theme":" Light ","last_folder":"","future":true}"#,
    )
    .unwrap();
    let prefs = read_prefs(&dir);
    assert_eq!(prefs.theme, Theme::Light);
    assert!(prefs.last_folder.is_none());

    fs::write(dir.join("preferences.json"), r#"{"theme":"nope"}"#).unwrap();
    assert_eq!(read_prefs(&dir).theme, Theme::Dark);

    let nested = dir.join("made");
    let mut prefs = read_prefs(&dir);
    prefs.set_theme(Theme::Dark);
    prefs.set_folder(Path::new("runs"));
    write_prefs(&nested, &prefs).unwrap();
    assert_eq!(read_prefs(&nested).theme, Theme::Dark);
    assert_eq!(read_prefs(&nested).last_folder.unwrap().as_os_str(), "runs");
    fs::remove_dir_all(&dir).unwrap();
}

#[cfg(windows)]
#[test]
fn a_locked_history_file_is_a_read_error() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let dir = scratch("read");
    let file = dir.join("run_history.json");
    fs::write(&file, b"[]").unwrap();
    let _lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    let error = load_folder(&dir).unwrap_err();
    assert!(matches!(error, HistoryError::Read { .. }));
    assert!(
        error
            .to_string()
            .starts_with("could not read run_history.json")
    );
    drop(_lock);
    fs::remove_dir_all(&dir).unwrap();
}
