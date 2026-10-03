//! The window. It draws what `runforge-core` already decided.

use std::path::PathBuf;

use eframe::egui::{self, Color32, RichText};
use egui_plot::{Line, Plot, PlotPoints, Points};
use runforge_core::{
    EvalSummary, History, HyperDiff, LossSample, Prefs, RunEntry, Theme, VERSION, curve_csv,
    curve_segments, entry_json, finite_points, format_f64, hyperparameter_diffs, list_csv,
    load_folder, read_prefs, write_prefs,
};

const CURVE: Color32 = Color32::from_rgb(0x4e, 0xcd, 0xc4);
const CURVE_B: Color32 = Color32::from_rgb(0xff, 0x6b, 0x6b);

struct Row {
    file_index: usize,
    label: String,
}

type AskFolder = Box<dyn FnMut(&Prefs) -> Option<PathBuf>>;
type AskSave = Box<dyn FnMut(&str) -> Option<PathBuf>>;

pub struct RunForgeApp {
    prefs_dir: PathBuf,
    prefs: Prefs,
    history: Option<History>,
    opened_file: Option<PathBuf>,
    note: String,
    selected: Option<usize>,
    compare: Option<usize>,
    ask_folder: AskFolder,
    ask_save: AskSave,
}

impl RunForgeApp {
    pub fn open(prefs_dir: PathBuf) -> Self {
        let prefs = read_prefs(&prefs_dir);
        let mut app = Self {
            prefs_dir,
            prefs,
            history: None,
            opened_file: None,
            note: String::new(),
            selected: None,
            compare: None,
            ask_folder: Box::new(rfd_folder),
            ask_save: Box::new(rfd_save),
        };
        if let Some(folder) = app.prefs.last_folder.clone() {
            app.load_folder(folder);
        }
        app
    }

    fn load_folder(&mut self, folder: PathBuf) {
        match load_folder(&folder) {
            Ok((path, history)) => {
                self.note.clear();
                self.opened_file = Some(path);
                self.selected = history
                    .display_order()
                    .first()
                    .map(|&position| history.entries[position].file_index);
                self.compare = None;
                self.history = Some(history);
                self.prefs.set_folder(&folder);
                if let Err(error) = write_prefs(&self.prefs_dir, &self.prefs) {
                    self.note = format!("could not save preferences: {error}");
                }
            }
            Err(error) => {
                self.history = None;
                self.opened_file = None;
                self.selected = None;
                self.compare = None;
                self.note = error.to_string();
            }
        }
    }

    fn pick_folder(&mut self) {
        let prefs = self.prefs.clone();
        let Some(folder) = (self.ask_folder)(&prefs) else {
            return;
        };
        self.load_folder(folder);
    }

    fn set_theme(&mut self, theme: Theme) {
        self.prefs.set_theme(theme);
        if let Err(error) = write_prefs(&self.prefs_dir, &self.prefs) {
            self.note = format!("could not save preferences: {error}");
        }
    }

    fn save_text(&mut self, suggested: &str, text: &str) {
        let Some(path) = (self.ask_save)(suggested) else {
            return;
        };
        match std::fs::write(&path, text) {
            Ok(()) => self.note = format!("Wrote {}", path.display()),
            Err(error) => self.note = format!("could not write the export: {error}"),
        }
    }
}

impl eframe::App for RunForgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().set_visuals(match self.prefs.theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        });
        egui::Panel::top("bar").show(ui, |ui| self.toolbar(ui));
        if !self.note.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new(&self.note).color(Color32::from_rgb(0xff, 0xd9, 0x3d)));
        }
        if self.history.is_none() {
            ui.add_space(24.0);
            ui.label("Open the folder where backpropagate wrote run_history.json.");
            return;
        }
        self.bench(ui);
    }
}

impl RunForgeApp {
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(format!("RunForge {VERSION}"));
            if ui.button("Open folder").clicked() {
                self.pick_folder();
            }
            let history = self.history.clone();
            if let Some(history) = history.as_ref() {
                if ui.button("Export list").clicked() {
                    let text = list_csv(history);
                    self.save_text("runs.csv", &text);
                }
                if let Some(entry) = self.selected.and_then(|index| history.get(index)).cloned() {
                    if ui.button("Export curve").clicked() {
                        self.save_text(&format!("{}.csv", entry.run_id), &curve_csv(&entry));
                    }
                    if ui.button("Export entry").clicked() {
                        self.save_text(&format!("{}.json", entry.run_id), &entry_json(&entry));
                    }
                }
            }
            let theme_label = match self.prefs.theme {
                Theme::Dark => "Light",
                Theme::Light => "Dark",
            };
            if ui.button(theme_label).clicked() {
                let next = match self.prefs.theme {
                    Theme::Dark => Theme::Light,
                    Theme::Light => Theme::Dark,
                };
                self.set_theme(next);
            }
        });
        if let Some(path) = &self.opened_file {
            ui.label(path.display().to_string());
        }
    }

    fn bench(&mut self, ui: &mut egui::Ui) {
        let Some(history) = self.history.clone() else {
            return;
        };
        let rows: Vec<Row> = history
            .display_order()
            .into_iter()
            .map(|position| {
                let entry = &history.entries[position];
                Row {
                    file_index: entry.file_index,
                    label: row_label(entry),
                }
            })
            .collect();
        let notes = notes(&history);
        let primary = self.selected.and_then(|index| history.get(index)).cloned();
        let compare = self
            .compare
            .and_then(|index| history.get(index))
            .filter(|entry| Some(entry.file_index) != self.selected)
            .cloned();
        let diffs = match (&primary, &compare) {
            (Some(left), Some(right)) => {
                hyperparameter_diffs(&left.hyperparameters, &right.hyperparameters)
            }
            _ => Vec::new(),
        };

        egui::Panel::left("runs").exact_size(460.0).show(ui, |ui| {
            ui.label("Status, model, final loss, started. Newest first.");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for row in &rows {
                    ui.horizontal(|ui| {
                        let chosen = self.selected == Some(row.file_index);
                        if ui.selectable_label(chosen, &row.label).clicked() {
                            self.selected = Some(row.file_index);
                        }
                        if ui.small_button("Compare").clicked() {
                            self.arm_compare(row.file_index);
                        }
                    });
                }
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            for line in &notes {
                ui.label(line);
            }
            if self.compare.is_some() && ui.button("Clear compare").clicked() {
                self.compare = None;
            }
            match &primary {
                Some(entry) => {
                    draw_chart(ui, entry, compare.as_ref());
                    draw_entry(ui, entry);
                    if let Some(other) = &compare {
                        ui.add_space(8.0);
                        ui.heading(format!("Compared with {}", other.run_id));
                        draw_entry(ui, other);
                        draw_diffs(ui, &diffs);
                    }
                }
                None => {
                    ui.label("Select a run.");
                }
            }
        });
    }

    fn arm_compare(&mut self, file_index: usize) {
        if self.selected == Some(file_index) {
            return;
        }
        if self.selected.is_none() {
            self.selected = Some(file_index);
            return;
        }
        self.compare = Some(file_index);
    }
}

fn rfd_folder(prefs: &Prefs) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(folder) = &prefs.last_folder {
        dialog = dialog.set_directory(folder);
    }
    dialog.pick_folder()
}

fn rfd_save(suggested: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_file_name(suggested).save_file()
}

fn row_label(entry: &RunEntry) -> String {
    let loss = entry
        .final_loss
        .map(format_f64)
        .unwrap_or_else(|| "—".to_string());
    let model = dash(&entry.model_name);
    let status = dash(&entry.status);
    let started = dash(&entry.started_at);
    format!("{status}   {model}   {loss}   {started}")
}

fn dash(text: &str) -> &str {
    if text.is_empty() { "—" } else { text }
}

fn notes(history: &History) -> Vec<String> {
    let mut lines = Vec::new();
    if history.skipped > 0 {
        lines.push(format!(
            "{} entries skipped because they had no run id",
            history.skipped
        ));
    }
    if history.duplicate_ids > 0 {
        lines.push(format!(
            "{} run ids appear more than once",
            history.duplicate_ids
        ));
    }
    if !history.schema_notes.is_empty() {
        lines.push(format!(
            "Some entries use schema {}. Known fields are still shown.",
            history.schema_notes.join(", ")
        ));
    }
    lines
}

fn draw_chart(ui: &mut egui::Ui, primary: &RunEntry, compare: Option<&RunEntry>) {
    let primary_empty = finite_points(&primary.loss).is_empty();
    let compare_empty = compare.is_none_or(|entry| finite_points(&entry.loss).is_empty());
    if primary_empty && compare_empty {
        ui.label("No stored loss for this run.");
    } else {
        Plot::new("loss")
            .height(360.0)
            .legend(egui_plot::Legend::default())
            .x_axis_label("stored sample")
            .y_axis_label("loss")
            .show(ui, |plot| {
                draw_series(plot, &primary.run_id, CURVE, &primary.loss);
                if let Some(other) = compare {
                    draw_series(plot, &other.run_id, CURVE_B, &other.loss);
                }
            });
    }
    ui.label("The file holds the trainer's stored samples, at most 100.");
}

fn draw_series(plot: &mut egui_plot::PlotUi<'_>, name: &str, color: Color32, loss: &[LossSample]) {
    for (index, segment) in curve_segments(loss).into_iter().enumerate() {
        let label = if index == 0 { name } else { "" };
        plot.line(
            Line::new(label, PlotPoints::new(segment))
                .color(color)
                .width(2.0),
        );
    }
    let points = finite_points(loss);
    if !points.is_empty() {
        plot.points(
            Points::new("", PlotPoints::new(points))
                .color(color)
                .radius(3.0),
        );
    }
}

fn draw_entry(ui: &mut egui::Ui, entry: &RunEntry) {
    ui.add_space(8.0);
    ui.heading(&entry.run_id);
    line(ui, "Status", &entry.status);
    line(ui, "Model", &entry.model_name);
    line(ui, "Dataset", &entry.dataset_info);
    line(ui, "Kind", &entry.session_kind);
    line(ui, "Steps", &entry.steps);
    if let Some(duration) = entry.duration_seconds {
        line(ui, "Duration (seconds)", &format_f64(duration));
    }
    line(ui, "Started", &entry.started_at);
    line(ui, "Completed", &entry.completed_at);
    if let Some(loss) = entry.final_loss {
        line(ui, "Final loss", &format_f64(loss));
    }
    line(ui, "Dataset hash", &entry.dataset_hash);
    line(ui, "Checkpoint", &entry.checkpoint_path);
    if !entry.export_paths.is_empty() {
        line(ui, "Exports", &entry.export_paths.join(", "));
    }
    if !entry.failure_reason.is_empty() {
        ui.label(RichText::new(&entry.failure_reason).color(Color32::from_rgb(0xff, 0x8a, 0x80)));
    }
    if let Some(eval) = &entry.eval_summary {
        draw_eval(ui, eval);
    }
}

fn draw_eval(ui: &mut egui::Ui, eval: &EvalSummary) {
    ui.label("Eval");
    if let Some(loss) = eval.held_out_loss {
        line(ui, "Held-out loss", &format_f64(loss));
    }
    if let Some(perplexity) = eval.perplexity {
        line(ui, "Perplexity", &format_f64(perplexity));
    }
    if let Some(n) = eval.eval_n {
        line(ui, "Scored items", &n.to_string());
    }
    if let Some(n) = eval.n_prompts {
        line(ui, "Prompts", &n.to_string());
    }
    for (name, score) in &eval.task_metrics {
        line(ui, name, &format_f64(*score));
    }
}

fn draw_diffs(ui: &mut egui::Ui, diffs: &[HyperDiff]) {
    ui.add_space(8.0);
    if diffs.is_empty() {
        ui.label("Hyperparameters match.");
        return;
    }
    ui.label("Hyperparameters that differ");
    for diff in diffs {
        let left = if diff.left.is_empty() {
            "absent"
        } else {
            diff.left.as_str()
        };
        let right = if diff.right.is_empty() {
            "absent"
        } else {
            diff.right.as_str()
        };
        ui.label(format!("{}: {left}  →  {right}", diff.key));
    }
}

fn line(ui: &mut egui::Ui, name: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    ui.label(format!("{name}: {value}"));
}

#[cfg(test)]
mod tests {
    use super::RunForgeApp;
    use eframe::App;
    use eframe::egui::{self, Event, Modifiers, PointerButton};
    use runforge_core::{Theme, write_prefs};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn scratch(name: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "runforge-ui-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct Harness {
        ctx: egui::Context,
        frame: eframe::Frame,
        tick: f64,
    }

    impl Harness {
        fn new() -> Self {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            Self {
                ctx,
                frame: eframe::Frame::_new_kittest(),
                tick: 0.0,
            }
        }

        fn show(&mut self, app: &mut RunForgeApp, events: Vec<Event>) -> egui::FullOutput {
            self.tick += 1.0 / 60.0;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 780.0),
                )),
                time: Some(self.tick),
                events,
                ..Default::default()
            };
            let frame = &mut self.frame;
            self.ctx.run_ui(input, |ui| app.ui(ui, frame))
        }

        fn texts(output: &egui::FullOutput) -> Vec<String> {
            let Some(update) = &output.platform_output.accesskit_update else {
                return Vec::new();
            };
            let mut texts = Vec::new();
            for (_, node) in &update.nodes {
                if let Some(text) = node.label() {
                    texts.push(text.to_string());
                }
                if let Some(text) = node.value() {
                    texts.push(text.to_string());
                }
            }
            texts
        }

        fn center(output: &egui::FullOutput, needle: &str) -> egui::Pos2 {
            let update = output
                .platform_output
                .accesskit_update
                .as_ref()
                .expect("accesskit");
            let mut exact = Vec::new();
            let mut loose = Vec::new();
            for (_, node) in &update.nodes {
                let label = node.label().unwrap_or("");
                let value = node.value().unwrap_or("");
                let Some(bounds) = node.bounds() else {
                    continue;
                };
                let width = bounds.x1 - bounds.x0;
                let height = bounds.y1 - bounds.y0;
                if width <= 0.0 || height <= 0.0 || width * height > 80_000.0 {
                    continue;
                }
                let pos = egui::pos2(
                    ((bounds.x0 + bounds.x1) / 2.0) as f32,
                    ((bounds.y0 + bounds.y1) / 2.0) as f32,
                );
                let hit = (bounds.y0, bounds.x0, pos);
                if label == needle || value == needle {
                    exact.push(hit);
                } else if label.contains(needle) || value.contains(needle) {
                    loose.push(hit);
                }
            }
            let mut hits = if exact.is_empty() { loose } else { exact };
            hits.sort_by(|left, right| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.total_cmp(&right.1))
            });
            hits.first().map(|(_, _, pos)| *pos).unwrap_or_else(|| {
                panic!("missing control");
            })
        }

        fn click(&mut self, app: &mut RunForgeApp, needle: &str) {
            let output = self.show(app, Vec::new());
            let pos = Self::center(&output, needle);
            output.drop_without_applying_deltas();
            self.show(app, vec![Event::PointerMoved(pos)])
                .drop_without_applying_deltas();
            self.show(
                app,
                vec![Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                }],
            )
            .drop_without_applying_deltas();
            self.show(
                app,
                vec![Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                }],
            )
            .drop_without_applying_deltas();
        }
    }

    fn sample_history() -> String {
        r#"[
            {"run_id":"older","status":"failed","model_name":"Alpha","started_at":"2026-01-01T00:00:00","final_loss":0.2,"loss_history":[1.0, null, 0.4],"failure_reason":"boom","hyperparameters":{"lr":0.1,"only":1},"schema_version":"2.0","eval":{"held_out_loss":1.5,"perplexity":4.0,"eval_n":8,"n_prompts":5,"task_metrics":{"f1":0.5}}},
            {"status":"skipped"},
            {"run_id":"newer","status":"completed","model_name":"Beta","session_kind":"single_run","started_at":"2026-02-01T00:00:00","completed_at":"2026-02-01T01:00:00","steps":"10","duration_seconds":3.5,"final_loss":0.4,"loss_history":[],"dataset_info":"notes","dataset_hash":"abc","checkpoint_path":"ckpt","export_paths":["out.gguf"],"hyperparameters":{"lr":0.2,"extra":1},"schema_version":"1.0"}
        ]"#
        .to_string()
    }

    fn write_history(dir: &Path) {
        std::fs::write(dir.join("run_history.json"), sample_history()).unwrap();
    }

    #[test]
    fn an_empty_window_asks_for_the_folder() {
        let dir = scratch("empty");
        let mut app = RunForgeApp::open(dir);
        let mut ui = Harness::new();
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        assert!(texts.iter().any(|text| text.contains("run_history.json")));
        assert!(texts.iter().any(|text| text.contains("RunForge")));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn the_bench_draws_compares_and_exports() {
        let dir = scratch("bench");
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        write_history(&folder);
        let export = dir.join("export");
        std::fs::create_dir(&export).unwrap();
        let mut app = RunForgeApp::open(dir.join("prefs"));
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        let export_dir = export.clone();
        app.ask_save = Box::new(move |name| Some(export_dir.join(name)));

        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        assert_eq!(
            app.history.as_ref().map(|history| history.entries.len()),
            Some(2)
        );
        assert_eq!(app.selected, Some(2));
        ui.click(&mut app, "Compare");
        assert!(app.compare.is_none());
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        assert!(texts.iter().any(|text| text.contains("No stored loss")));
        assert!(texts.iter().any(|text| text.contains("skipped")));
        assert!(texts.iter().any(|text| text.contains("schema")));
        output.drop_without_applying_deltas();

        ui.click(&mut app, "Alpha");
        assert_eq!(app.selected, Some(0));
        ui.click(&mut app, "Compare");
        assert_eq!(app.compare, Some(2));
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        assert!(texts.iter().any(|text| text.contains("boom")));
        assert!(texts.iter().any(|text| text.contains("Held-out loss")));
        assert!(
            texts
                .iter()
                .any(|text| text.contains("Hyperparameters that differ") || text.contains("lr"))
        );
        assert!(texts.iter().any(|text| text.contains("absent")));
        assert!(texts.iter().any(|text| text.contains("stored samples")));
        output.drop_without_applying_deltas();

        ui.click(&mut app, "Clear compare");
        assert!(app.compare.is_none());

        ui.click(&mut app, "Export list");
        assert!(export.join("runs.csv").is_file());
        ui.click(&mut app, "Export curve");
        assert!(export.join("older.csv").is_file());
        ui.click(&mut app, "Export entry");
        assert!(export.join("older.json").is_file());

        ui.click(&mut app, "Light");
        assert_eq!(app.prefs.theme, Theme::Light);
        ui.click(&mut app, "Dark");
        assert_eq!(app.prefs.theme, Theme::Dark);

        app.selected = None;
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        assert!(texts.iter().any(|text| text.contains("Select a run")));
        output.drop_without_applying_deltas();
        app.selected = None;
        ui.click(&mut app, "Compare");
        assert_eq!(app.selected, Some(2));
    }

    #[test]
    fn a_cancelled_dialog_leaves_the_bench_alone() {
        let dir = scratch("cancel");
        let mut app = RunForgeApp::open(dir.join("prefs"));
        app.ask_folder = Box::new(|_| None);
        app.ask_save = Box::new(|_| None);
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        assert!(app.history.is_none());
        assert!(app.note.is_empty());

        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        write_history(&folder);
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        ui.click(&mut app, "Open folder");
        ui.click(&mut app, "Export list");
        assert!(app.note.is_empty());
        assert!(app.history.is_some());
    }

    #[test]
    fn a_bad_folder_and_a_failed_save_are_sentences() {
        let dir = scratch("bad");
        let empty = dir.join("empty");
        std::fs::create_dir(&empty).unwrap();
        let mut app = RunForgeApp::open(dir.join("prefs"));
        let chosen = empty.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        app.ask_save = Box::new(|_| Some(PathBuf::from("runs.csv")));
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        assert!(app.history.is_none());
        assert!(app.note.contains("run_history.json"));
        let output = ui.show(&mut app, Vec::new());
        assert!(
            Harness::texts(&output)
                .iter()
                .any(|text| text.contains("run_history.json"))
        );
        output.drop_without_applying_deltas();

        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        write_history(&folder);
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        ui.click(&mut app, "Open folder");
        let blocked = dir.join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        app.ask_save = Box::new(move |_| Some(blocked.clone()));
        ui.click(&mut app, "Export list");
        assert!(app.note.contains("could not write the export"));
    }

    #[test]
    fn opening_reloads_the_last_folder() {
        let dir = scratch("reload");
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        write_history(&folder);
        let prefs_dir = dir.join("prefs");
        let mut prefs = runforge_core::Prefs::default();
        prefs.set_folder(&folder);
        prefs.set_theme(Theme::Light);
        write_prefs(&prefs_dir, &prefs).unwrap();
        let app = RunForgeApp::open(prefs_dir);
        assert_eq!(
            app.history.as_ref().map(|history| history.entries.len()),
            Some(2)
        );
        assert_eq!(app.prefs.theme, Theme::Light);

        let missing = dir.join("missing-prefs");
        let mut prefs = runforge_core::Prefs::default();
        prefs.set_folder(&dir.join("nope"));
        write_prefs(&missing, &prefs).unwrap();
        let app = RunForgeApp::open(missing);
        assert!(app.history.is_none());
        assert!(!app.note.is_empty());
    }

    #[test]
    fn matching_hyperparameters_say_so() {
        let dir = scratch("match");
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(
            folder.join("run_history.json"),
            r#"[{"run_id":"a","model_name":"Alpha","started_at":"2026-02-01T00:00:00","hyperparameters":{"lr":1},"loss_history":[0.5]},{"run_id":"b","model_name":"Beta","started_at":"2026-01-01T00:00:00","hyperparameters":{"lr":1},"loss_history":[0.4,0.2]}]"#,
        )
        .unwrap();
        let mut app = RunForgeApp::open(dir.join("prefs"));
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        ui.click(&mut app, "Beta");
        ui.click(&mut app, "Compare");
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        assert!(
            texts
                .iter()
                .any(|text| text.contains("Hyperparameters match"))
        );
        output.drop_without_applying_deltas();
    }

    #[test]
    fn a_file_for_a_prefs_dir_is_reported() {
        let dir = scratch("prefs-file");
        let blocker = dir.join("not-a-dir");
        std::fs::write(&blocker, b"x").unwrap();
        let mut app = RunForgeApp::open(blocker);
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        write_history(&folder);
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        assert!(app.note.contains("could not save preferences"));
        assert!(app.history.is_some());
        ui.click(&mut app, "Light");
        assert_eq!(app.prefs.theme, Theme::Light);
        assert!(app.note.contains("could not save preferences"));
    }

    #[test]
    fn duplicate_run_ids_are_named() {
        let dir = scratch("dup");
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(
            folder.join("run_history.json"),
            r#"[{"run_id":"same","model_name":"Alpha"},{"run_id":"same","model_name":"Beta"}]"#,
        )
        .unwrap();
        let mut app = RunForgeApp::open(dir.join("prefs"));
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        assert!(
            texts
                .iter()
                .any(|text| text.contains("appear more than once"))
        );
        output.drop_without_applying_deltas();
    }
}
