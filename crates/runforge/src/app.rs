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

pub struct RunForgeApp {
    prefs_dir: PathBuf,
    prefs: Prefs,
    history: Option<History>,
    opened_file: Option<PathBuf>,
    note: String,
    selected: Option<usize>,
    compare: Option<usize>,
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
        let mut dialog = rfd::FileDialog::new();
        if let Some(folder) = &self.prefs.last_folder {
            dialog = dialog.set_directory(folder);
        }
        if let Some(folder) = dialog.pick_folder() {
            self.load_folder(folder);
        }
    }

    fn set_theme(&mut self, theme: Theme) {
        self.prefs.set_theme(theme);
        if let Err(error) = write_prefs(&self.prefs_dir, &self.prefs) {
            self.note = format!("could not save preferences: {error}");
        }
    }

    fn save_text(&mut self, suggested: &str, text: &str) {
        let Some(path) = rfd::FileDialog::new().set_file_name(suggested).save_file() else {
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
