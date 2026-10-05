//! The window. It draws what `runforge-core` already decided.
//!
//! Train, Eval, and Export model start an already-installed `backprop`. The trainer is not in
//! this package.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, RichText};
use egui_plot::{Line, Plot, PlotPoints, Points};
use runforge_core::{
    EvalSummary, History, HyperDiff, LossSample, Prefs, RunEntry, Theme, VERSION, curve_csv,
    curve_segments, entry_json, finite_points, format_f64, hyperparameter_diffs, list_csv,
    load_folder, read_prefs, write_prefs,
};

use crate::launch::{
    ALREADY_RUNNING, LaunchRequest, MISSING_TOOL, NEED_DATA, NO_CHECKPOINT, NOTHING_RUNNING,
    OPEN_FOLDER, SELECT_RUN, Session, ToolAnswer, apply_log_update, bust_tool_cache, eval_args,
    exit_note, export_args, start_installed, tool_answer, train_args,
};

struct Ink {
    note: Color32,
    failure: Color32,
    curve: Color32,
    curve_b: Color32,
}

/// Dark keeps the colors that already clear the dark fills. Light is a
/// separate pair: text at 4.5:1 on panel gray 248, lines at 3:1 on white.
fn ink(dark: bool) -> Ink {
    if dark {
        Ink {
            note: Color32::from_rgb(0xff, 0xd9, 0x3d),
            failure: Color32::from_rgb(0xff, 0x8a, 0x80),
            curve: Color32::from_rgb(0x4e, 0xcd, 0xc4),
            curve_b: Color32::from_rgb(0xff, 0x6b, 0x6b),
        }
    } else {
        Ink {
            note: Color32::from_rgb(0x6b, 0x44, 0x00),
            failure: Color32::from_rgb(0xa3, 0x20, 0x20),
            curve: Color32::from_rgb(0x0e, 0x6b, 0x66),
            curve_b: Color32::from_rgb(0xb4, 0x23, 0x18),
        }
    }
}

struct Row {
    file_index: usize,
    label: String,
}

type AskFolder = Box<dyn FnMut(&Prefs) -> Option<PathBuf>>;
type AskSave = Box<dyn FnMut(&str) -> Option<PathBuf>>;
type AskData = Box<dyn FnMut() -> Option<PathBuf>>;
type FindTool = Box<dyn FnMut() -> ToolAnswer>;
type StartCommand = Box<dyn FnMut(LaunchRequest) -> Result<Box<dyn Session>, &'static str>>;

enum Begin {
    Now(PathBuf, PathBuf),
    Later(PathBuf),
}

struct HeldLaunch {
    args: Vec<String>,
    cwd: PathBuf,
}

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
    ask_data: AskData,
    find_tool: FindTool,
    start: StartCommand,
    model: String,
    data_file: Option<PathBuf>,
    steps: String,
    log: VecDeque<String>,
    /// True while the last log line is a carriage-return progress revision.
    log_open: bool,
    session: Option<Box<dyn Session>>,
    /// Train, Eval, or Export captured while the PATH walk is still running.
    held: Option<HeldLaunch>,
    /// The latest tool answer had no finished result yet.
    tool_pending: bool,
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
            ask_data: Box::new(rfd_data),
            find_tool: Box::new(tool_answer),
            start: Box::new(start_installed),
            model: String::new(),
            data_file: None,
            steps: String::new(),
            log: VecDeque::new(),
            log_open: false,
            session: None,
            held: None,
            tool_pending: false,
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
        self.poll_session();
        self.resume_held();
        let visuals = match self.prefs.theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        };
        // The root Ui is built before this runs, so the context update only
        // reaches the next frame. This frame's widgets read the Ui copy.
        ui.ctx().set_visuals(visuals.clone());
        *ui.visuals_mut() = visuals;
        egui::Panel::top("bar").show(ui, |ui| self.toolbar(ui));
        if !self.note.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new(&self.note).color(ink(ui.visuals().dark_mode).note));
        }
        if self.history.is_none() && self.session.is_none() {
            ui.add_space(24.0);
            ui.label("Open the folder where backpropagate wrote run_history.json.");
        } else {
            egui::Panel::bottom("log").show(ui, |ui| self.log_panel(ui));
            self.bench(ui);
        }
        if self.session.is_some() || self.held.is_some() || self.tool_pending {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(200));
        }
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
        if self.history.is_some() || self.session.is_some() {
            self.launch_form(ui);
        }
    }

    fn launch_form(&mut self, ui: &mut egui::Ui) {
        let answer = (self.find_tool)();
        self.tool_pending = matches!(answer, ToolAnswer::Pending);
        let missing = matches!(answer, ToolAnswer::Ready(None));
        let data_label = self
            .data_file
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("No data file")
            .to_string();
        let mut output_field = self
            .output_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Model");
            ui.add(egui::TextEdit::singleline(&mut self.model).desired_width(160.0));
            ui.label("Data file");
            ui.label(&data_label);
            if ui.button("Browse").clicked() {
                self.on_browse();
            }
            ui.label("Steps");
            ui.add(egui::TextEdit::singleline(&mut self.steps).desired_width(72.0));
            ui.label("Output folder");
            ui.add(
                egui::TextEdit::singleline(&mut output_field)
                    .desired_width(220.0)
                    .interactive(false),
            );
        });
        if missing {
            ui.label(MISSING_TOOL);
        }
        ui.horizontal(|ui| {
            if ui.button("Train").clicked() {
                self.on_train();
            }
            if ui.button("Eval").clicked() {
                self.on_eval();
            }
            if ui.button("Export model").clicked() {
                self.on_export();
            }
            if ui.button("Stop").clicked() {
                self.on_stop();
            }
        });
    }

    fn log_panel(&self, ui: &mut egui::Ui) {
        ui.label("Log");
        egui::ScrollArea::vertical()
            .max_height(120.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.log.is_empty() {
                    ui.label("No log yet.");
                    return;
                }
                for line in &self.log {
                    ui.label(line);
                }
            });
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
                    // The list is 460px and does not scroll sideways. The label
                    // gives up width so Compare stays inside the panel.
                    let chosen = self.selected == Some(row.file_index);
                    let mut select = false;
                    let mut compare = false;
                    egui::Sides::new().shrink_left().truncate().show(
                        ui,
                        |ui| {
                            if ui
                                .add(egui::Button::selectable(chosen, &row.label).truncate())
                                .clicked()
                            {
                                select = true;
                            }
                        },
                        |ui| {
                            if ui.small_button("Compare").clicked() {
                                compare = true;
                            }
                        },
                    );
                    if select {
                        self.selected = Some(row.file_index);
                    }
                    if compare {
                        self.arm_compare(row.file_index);
                    }
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

    fn poll_session(&mut self) {
        let (lines, code) = {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let mut lines = session.take_lines();
            let code = session.finished();
            if code.is_some() {
                lines.extend(session.take_lines());
            }
            (lines, code)
        };
        for update in lines {
            apply_log_update(&mut self.log, &mut self.log_open, update);
        }
        let Some(code) = code else {
            return;
        };
        self.session = None;
        if let Some(folder) = self.output_dir() {
            self.load_folder(folder);
        }
        if self.note.is_empty() {
            self.note = exit_note(code);
        }
    }

    fn output_dir(&self) -> Option<PathBuf> {
        self.opened_file
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
    }

    fn selected_entry(&self) -> Option<RunEntry> {
        let history = self.history.as_ref()?;
        let index = self.selected?;
        history.get(index).cloned()
    }

    fn begin(&mut self) -> Result<Begin, &'static str> {
        bust_tool_cache();
        if self.session.is_some() {
            return Err(ALREADY_RUNNING);
        }
        let program = match (self.find_tool)() {
            ToolAnswer::Ready(Some(program)) => Some(program),
            ToolAnswer::Ready(None) => return Err(MISSING_TOOL),
            ToolAnswer::Pending => None,
        };
        let Some(output) = self.output_dir() else {
            return Err(OPEN_FOLDER);
        };
        Ok(match program {
            Some(program) => Begin::Now(program, output),
            None => Begin::Later(output),
        })
    }

    fn finish_begin(&mut self, begin: Begin, args: Vec<String>) {
        match begin {
            Begin::Now(program, cwd) => {
                self.held = None;
                self.launch(program, args, cwd);
            }
            Begin::Later(cwd) => {
                self.held = Some(HeldLaunch { args, cwd });
                self.note.clear();
            }
        }
    }

    fn resume_held(&mut self) {
        if self.held.is_none() || self.session.is_some() {
            return;
        }
        let program = match (self.find_tool)() {
            ToolAnswer::Pending => return,
            ToolAnswer::Ready(None) => {
                self.held = None;
                self.note = MISSING_TOOL.to_string();
                return;
            }
            ToolAnswer::Ready(Some(program)) => program,
        };
        let Some(held) = self.held.take() else {
            return;
        };
        self.launch(program, held.args, held.cwd);
    }

    fn on_browse(&mut self) {
        let Some(path) = (self.ask_data)() else {
            return;
        };
        self.data_file = Some(path);
    }

    fn on_train(&mut self) {
        let begin = match self.begin() {
            Ok(begin) => begin,
            Err(text) => {
                self.note = text.to_string();
                return;
            }
        };
        let Some(data) = self.data_file.clone() else {
            self.note = NEED_DATA.to_string();
            return;
        };
        let output = match &begin {
            Begin::Now(_, output) | Begin::Later(output) => output.clone(),
        };
        match train_args(&self.model, &data, &self.steps, &output) {
            Ok(args) => self.finish_begin(begin, args),
            Err(text) => self.note = text.to_string(),
        }
    }

    fn on_eval(&mut self) {
        let begin = match self.begin() {
            Ok(begin) => begin,
            Err(text) => {
                self.note = text.to_string();
                return;
            }
        };
        let Some(entry) = self.selected_entry() else {
            self.note = SELECT_RUN.to_string();
            return;
        };
        let output = match &begin {
            Begin::Now(_, output) | Begin::Later(output) => output.clone(),
        };
        match eval_args(&entry.run_id, &output) {
            Ok(args) => self.finish_begin(begin, args),
            Err(text) => self.note = text.to_string(),
        }
    }

    fn on_export(&mut self) {
        let begin = match self.begin() {
            Ok(begin) => begin,
            Err(text) => {
                self.note = text.to_string();
                return;
            }
        };
        let Some(entry) = self.selected_entry() else {
            self.note = SELECT_RUN.to_string();
            return;
        };
        if entry.checkpoint_path.is_empty() {
            self.note = NO_CHECKPOINT.to_string();
            return;
        }
        let output = match &begin {
            Begin::Now(_, output) | Begin::Later(output) => output.clone(),
        };
        match export_args(&entry.checkpoint_path, &output) {
            Ok(args) => self.finish_begin(begin, args),
            Err(text) => self.note = text.to_string(),
        }
    }

    fn on_stop(&mut self) {
        let Some(session) = self.session.as_mut() else {
            self.note = NOTHING_RUNNING.to_string();
            return;
        };
        session.stop();
    }

    fn launch(&mut self, program: PathBuf, args: Vec<String>, cwd: PathBuf) {
        match (self.start)(LaunchRequest { program, args, cwd }) {
            Ok(session) => {
                self.session = Some(session);
                self.log.clear();
                self.log_open = false;
                self.note.clear();
            }
            Err(text) => self.note = text.to_string(),
        }
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

fn rfd_data() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_file()
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
    let ink = ink(ui.visuals().dark_mode);
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
                draw_series(plot, &primary.run_id, ink.curve, &primary.loss);
                if let Some(other) = compare {
                    draw_series(plot, &other.run_id, ink.curve_b, &other.loss);
                }
            });
    }
    ui.label("The chart is the stored samples, in file order.");
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
        ui.label(RichText::new(&entry.failure_reason).color(ink(ui.visuals().dark_mode).failure));
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
    use super::{RunForgeApp, ink};
    use crate::launch::{
        ALREADY_RUNNING, BAD_ARGUMENT, BAD_STEPS, LaunchRequest, LogUpdate, MISSING_TOOL,
        NEED_DATA, NO_CHECKPOINT, NOTHING_RUNNING, OPEN_FOLDER, SELECT_RUN, START_FAILED, Session,
        ToolAnswer,
    };
    use eframe::App;
    use eframe::egui::{self, Event, Modifiers, PointerButton};
    use runforge_core::{Theme, write_prefs};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

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
        assert!(texts.iter().all(|text| text != "Train"));
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
        assert!(texts.iter().any(|text| text.contains("in file order")));
        assert!(texts.iter().all(|text| !text.contains("at most 100")));
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

    struct Seen {
        verb: String,
        data_ok: bool,
        output_ok: bool,
        cwd_ok: bool,
        has_model: bool,
        has_steps: bool,
        run_ok: bool,
        checkpoint_ok: bool,
    }

    fn inspect(request: &LaunchRequest, folder_name: &str) -> Seen {
        let args = &request.args;
        Seen {
            verb: args.first().cloned().unwrap_or_default(),
            data_ok: args
                .windows(2)
                .any(|pair| pair[0] == "--data" && pair[1] == "notes.json"),
            output_ok: args.windows(2).any(|pair| {
                pair[0] == "--output"
                    && Path::new(&pair[1])
                        .file_name()
                        .and_then(|name| name.to_str())
                        == Some(folder_name)
            }),
            cwd_ok: request.cwd.file_name().and_then(|name| name.to_str()) == Some(folder_name),
            has_model: args.iter().any(|arg| arg == "--model"),
            has_steps: args.iter().any(|arg| arg == "--steps"),
            run_ok: args.get(1).map(String::as_str) == Some("newer"),
            checkpoint_ok: args.get(1).map(String::as_str) == Some("ckpt"),
        }
    }

    struct Scripted {
        lines: Vec<String>,
        running: bool,
        stopped: Arc<AtomicBool>,
    }

    impl Session for Scripted {
        fn take_lines(&mut self) -> Vec<LogUpdate> {
            std::mem::take(&mut self.lines)
                .into_iter()
                .map(LogUpdate::Commit)
                .collect()
        }

        fn finished(&mut self) -> Option<i32> {
            if self.running { None } else { Some(0) }
        }

        fn stop(&mut self) {
            self.stopped.store(true, Ordering::Relaxed);
            self.running = false;
        }
    }

    fn open_runs(name: &str) -> (PathBuf, RunForgeApp, Harness) {
        let dir = scratch(name);
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        write_history(&folder);
        let mut app = RunForgeApp::open(dir.join("prefs"));
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        (folder, app, ui)
    }

    fn folder_name(folder: &Path) -> String {
        match folder.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => panic!("folder"),
        }
    }

    #[test]
    fn a_missing_tool_is_named_on_the_form() {
        let (_folder, mut app, mut ui) = open_runs("missing-tool");
        app.find_tool = Box::new(|| ToolAnswer::Ready(None));
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        assert!(texts.iter().any(|text| text == MISSING_TOOL));
        assert!(texts.iter().any(|text| text == "No data file"));
        assert!(texts.iter().any(|text| text == "No log yet."));
        assert!(texts.iter().any(|text| text == "Train"));
        output.drop_without_applying_deltas();
        ui.click(&mut app, "Train");
        assert_eq!(app.note, MISSING_TOOL);
        ui.click(&mut app, "Stop");
        assert_eq!(app.note, NOTHING_RUNNING);
    }

    #[test]
    fn train_sends_the_fields_and_stop_ends_it() {
        let (folder, mut app, mut ui) = open_runs("train");
        let name = folder_name(&folder);
        let seen = Arc::new(Mutex::new(Vec::<Seen>::new()));
        let seen_start = Arc::clone(&seen);
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_start = Arc::clone(&stopped);
        app.find_tool = Box::new(|| ToolAnswer::Ready(Some(PathBuf::from("backprop.exe"))));
        app.ask_data = Box::new(|| None);
        ui.click(&mut app, "Browse");
        assert!(app.data_file.is_none());
        app.ask_data = Box::new(|| Some(PathBuf::from("notes.json")));
        ui.click(&mut app, "Browse");
        assert_eq!(
            app.data_file
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|file| file.to_str()),
            Some("notes.json")
        );
        app.start = Box::new(move |request| {
            let row = inspect(&request, &name);
            seen_start
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(row);
            Ok(Box::new(Scripted {
                lines: vec!["trainer ready".to_string()],
                running: true,
                stopped: Arc::clone(&stopped_start),
            }))
        });
        app.steps = "0".to_string();
        app.data_file = Some(PathBuf::from("notes.json"));
        ui.click(&mut app, "Train");
        assert_eq!(app.note, BAD_STEPS);
        app.steps.clear();
        app.data_file = None;
        ui.click(&mut app, "Train");
        assert_eq!(app.note, NEED_DATA);
        app.model = "small".to_string();
        app.steps = "12".to_string();
        app.data_file = Some(PathBuf::from("notes.json"));
        ui.click(&mut app, "Train");
        ui.click(&mut app, "Train");
        assert_eq!(app.note, ALREADY_RUNNING);
        ui.click(&mut app, "Eval");
        assert_eq!(app.note, ALREADY_RUNNING);
        ui.click(&mut app, "Export model");
        assert_eq!(app.note, ALREADY_RUNNING);
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        assert!(texts.iter().any(|text| text == "trainer ready"));
        output.drop_without_applying_deltas();
        let ok = {
            let rows = seen.lock().unwrap_or_else(|poison| poison.into_inner());
            rows.len() == 1
                && rows[0].verb == "train"
                && rows[0].data_ok
                && rows[0].output_ok
                && rows[0].cwd_ok
                && rows[0].has_model
                && rows[0].has_steps
        };
        assert!(ok);
        ui.click(&mut app, "Stop");
        assert!(stopped.load(Ordering::Relaxed));
        ui.show(&mut app, Vec::new()).drop_without_applying_deltas();
        assert!(app.session.is_none());
        assert!(app.note.starts_with("backprop exited"));
        assert!(app.history.is_some());
        ui.click(&mut app, "Stop");
        assert_eq!(app.note, NOTHING_RUNNING);
    }

    #[test]
    fn eval_and_export_use_the_selected_run() {
        let (folder, mut app, mut ui) = open_runs("eval");
        let name = folder_name(&folder);
        let seen = Arc::new(Mutex::new(Vec::<Seen>::new()));
        let seen_start = Arc::clone(&seen);
        let stopped = Arc::new(AtomicBool::new(false));
        app.find_tool = Box::new(|| ToolAnswer::Ready(Some(PathBuf::from("backprop.exe"))));
        app.start = Box::new(move |request| {
            let row = inspect(&request, &name);
            seen_start
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(row);
            Ok(Box::new(Scripted {
                lines: Vec::new(),
                running: false,
                stopped: Arc::clone(&stopped),
            }))
        });
        ui.click(&mut app, "Eval");
        ui.click(&mut app, "Export model");
        let ok = {
            let rows = seen.lock().unwrap_or_else(|poison| poison.into_inner());
            rows.len() == 2
                && rows[0].verb == "eval"
                && rows[0].run_ok
                && rows[0].output_ok
                && rows[0].cwd_ok
                && rows[1].verb == "export"
                && rows[1].checkpoint_ok
                && rows[1].output_ok
                && rows[1].cwd_ok
        };
        assert!(ok);
    }

    #[test]
    fn refused_commands_are_sentences() {
        let dir = scratch("refuse");
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(
            folder.join("run_history.json"),
            r#"[{"run_id":"-nope","checkpoint_path":"-ckpt","model_name":"Alpha"}]"#,
        )
        .unwrap();
        let mut app = RunForgeApp::open(dir.join("prefs"));
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        app.find_tool = Box::new(|| ToolAnswer::Ready(Some(PathBuf::from("backprop.exe"))));
        app.start = Box::new(|_| Err(START_FAILED));
        app.data_file = Some(PathBuf::from("notes.json"));
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        ui.click(&mut app, "Train");
        assert_eq!(app.note, START_FAILED);
        ui.click(&mut app, "Eval");
        assert_eq!(app.note, BAD_ARGUMENT);
        ui.click(&mut app, "Export model");
        assert_eq!(app.note, BAD_ARGUMENT);
    }

    #[test]
    fn an_empty_checkpoint_and_a_missing_folder_are_sentences() {
        let dir = scratch("empty-ckpt");
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(
            folder.join("run_history.json"),
            r#"[{"run_id":"plain","model_name":"Alpha"}]"#,
        )
        .unwrap();
        let mut app = RunForgeApp::open(dir.join("prefs"));
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        app.find_tool = Box::new(|| ToolAnswer::Ready(Some(PathBuf::from("backprop.exe"))));
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        ui.click(&mut app, "Export model");
        assert_eq!(app.note, NO_CHECKPOINT);
        app.selected = None;
        ui.click(&mut app, "Export model");
        assert_eq!(app.note, SELECT_RUN);
        ui.click(&mut app, "Eval");
        assert_eq!(app.note, SELECT_RUN);
        app.opened_file = None;
        ui.click(&mut app, "Train");
        assert_eq!(app.note, OPEN_FOLDER);
    }

    #[test]
    fn a_prefs_failure_on_exit_keeps_that_sentence() {
        let (folder, mut app, mut ui) = open_runs("exit-prefs");
        let name = folder_name(&folder);
        app.find_tool = Box::new(|| ToolAnswer::Ready(Some(PathBuf::from("backprop.exe"))));
        app.data_file = Some(PathBuf::from("notes.json"));
        app.start = Box::new(move |request| {
            let _ = inspect(&request, &name);
            Ok(Box::new(Scripted {
                lines: Vec::new(),
                running: false,
                stopped: Arc::new(AtomicBool::new(false)),
            }))
        });
        ui.click(&mut app, "Train");
        let blocker = scratch("exit-prefs-file").join("not-a-dir");
        std::fs::write(&blocker, b"x").unwrap();
        app.prefs_dir = blocker;
        ui.show(&mut app, Vec::new()).drop_without_applying_deltas();
        assert!(app.note.starts_with("could not save preferences"));
        assert!(!app.note.starts_with("backprop"));
    }

    #[test]
    fn a_running_session_requests_another_frame() {
        let dir = scratch("repaint");
        let mut app = RunForgeApp::open(dir);
        app.session = Some(Box::new(Scripted {
            lines: Vec::new(),
            running: true,
            stopped: Arc::new(AtomicBool::new(false)),
        }));
        let mut ui = Harness::new();
        let output = ui.show(&mut app, Vec::new());
        let soon = output
            .viewport_output
            .values()
            .any(|viewport| viewport.repaint_delay <= std::time::Duration::from_millis(250));
        assert!(soon);
        output.drop_without_applying_deltas();
    }

    #[test]
    fn a_failed_open_keeps_stop_and_the_log() {
        let (folder, mut app, mut ui) = open_runs("failed-open");
        app.session = Some(Box::new(Scripted {
            lines: vec!["still training".to_string()],
            running: true,
            stopped: Arc::new(AtomicBool::new(false)),
        }));
        ui.show(&mut app, Vec::new()).drop_without_applying_deltas();
        let Some(parent) = folder.parent() else {
            panic!("folder");
        };
        let empty = parent.join("no-history");
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir(&empty).unwrap();
        app.ask_folder = Box::new(move |_| Some(empty.clone()));
        ui.click(&mut app, "Open folder");
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        let kept = texts.iter().any(|text| text == "Stop")
            && texts.iter().any(|text| text == "still training");
        assert!(kept);
        assert!(app.history.is_some());
        assert!(app.opened_file.is_some());
        output.drop_without_applying_deltas();
        app.session = Some(Box::new(Scripted {
            lines: Vec::new(),
            running: false,
            stopped: Arc::new(AtomicBool::new(false)),
        }));
        let output = ui.show(&mut app, Vec::new());
        let texts = Harness::texts(&output);
        let reloaded = texts.iter().any(|text| text.contains("Beta"))
            && texts
                .iter()
                .all(|text| !text.contains("Open the folder where backpropagate wrote"));
        assert!(reloaded);
        output.drop_without_applying_deltas();
    }

    #[test]
    fn a_click_waits_without_calling_the_tool_missing() {
        let (folder, mut app, mut ui) = open_runs("walk-click");
        let name = folder_name(&folder);
        let pending = Arc::new(AtomicBool::new(true));
        let pending_find = Arc::clone(&pending);
        app.find_tool = Box::new(move || {
            if pending_find.load(Ordering::Relaxed) {
                ToolAnswer::Pending
            } else {
                ToolAnswer::Ready(Some(PathBuf::from("backprop.exe")))
            }
        });
        app.data_file = Some(PathBuf::from("notes.json"));
        let launched = Arc::new(AtomicBool::new(false));
        let launched_start = Arc::clone(&launched);
        let name_ok = Arc::new(AtomicBool::new(false));
        let name_ok_start = Arc::clone(&name_ok);
        app.start = Box::new(move |request| {
            let row = inspect(&request, &name);
            let program_ok =
                request.program.file_name().and_then(|file| file.to_str()) == Some("backprop.exe");
            name_ok_start.store(
                program_ok && row.verb == "train" && row.data_ok,
                Ordering::Relaxed,
            );
            launched_start.store(true, Ordering::Relaxed);
            Ok(Box::new(Scripted {
                lines: Vec::new(),
                running: true,
                stopped: Arc::new(AtomicBool::new(false)),
            }))
        });
        ui.click(&mut app, "Train");
        let waiting =
            app.note.is_empty() && app.session.is_none() && !launched.load(Ordering::Relaxed);
        assert!(waiting);
        assert!(app.note != MISSING_TOOL);
        pending.store(false, Ordering::Relaxed);
        ui.show(&mut app, Vec::new()).drop_without_applying_deltas();
        let started = launched.load(Ordering::Relaxed)
            && name_ok.load(Ordering::Relaxed)
            && app.session.is_some()
            && app.note != MISSING_TOOL;
        assert!(started);
    }

    fn channel(value: u8) -> f64 {
        let unit = f64::from(value) / 255.0;
        if unit <= 0.04045 {
            unit / 12.92
        } else {
            ((unit + 0.055) / 1.055).powf(2.4)
        }
    }

    fn luminance(color: egui::Color32) -> f64 {
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    fn contrast(fg: egui::Color32, bg: egui::Color32) -> f64 {
        let left = luminance(fg);
        let right = luminance(bg);
        let (lighter, darker) = if left >= right {
            (left, right)
        } else {
            (right, left)
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn long_history() -> String {
        r#"[
            {"run_id":"short","status":"completed","model_name":"Alpha","started_at":"2026-05-21T04:54:14","final_loss":0.42,"loss_history":[0.5,0.4]},
            {"run_id":"long","status":"completed","model_name":"qwen2.5-7b-instruct","started_at":"2026-05-21T04:54:14.646724","final_loss":1.234567,"loss_history":[1.2,0.8],"failure_reason":"boom"}
        ]"#
        .to_string()
    }

    fn open_long(name: &str) -> (RunForgeApp, Harness) {
        let dir = scratch(name);
        let folder = dir.join("runs");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("run_history.json"), long_history()).unwrap();
        let mut app = RunForgeApp::open(dir.join("prefs"));
        let chosen = folder.clone();
        app.ask_folder = Box::new(move |_| Some(chosen.clone()));
        let mut ui = Harness::new();
        ui.click(&mut app, "Open folder");
        (app, ui)
    }

    fn node_rects(output: &egui::FullOutput, needle: &str, exact: bool) -> Vec<egui::Rect> {
        let Some(update) = &output.platform_output.accesskit_update else {
            return Vec::new();
        };
        let mut rects = Vec::new();
        for (_, node) in &update.nodes {
            let label = node.label().unwrap_or("");
            let value = node.value().unwrap_or("");
            let hit = if exact {
                label == needle || value == needle
            } else {
                label.contains(needle) || value.contains(needle)
            };
            if !hit {
                continue;
            }
            let Some(bounds) = node.bounds() else {
                continue;
            };
            let width = (bounds.x1 - bounds.x0) as f32;
            let height = (bounds.y1 - bounds.y0) as f32;
            if width <= 0.0 || height <= 0.0 || width * height > 80_000.0 {
                continue;
            }
            rects.push(egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            ));
        }
        rects
    }

    fn click_pos(ui: &mut Harness, app: &mut RunForgeApp, pos: egui::Pos2) {
        ui.show(app, vec![Event::PointerMoved(pos)])
            .drop_without_applying_deltas();
        ui.show(
            app,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::default(),
            }],
        )
        .drop_without_applying_deltas();
        ui.show(
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

    fn solid(mode: &egui::epaint::ColorMode, color: egui::Color32) -> bool {
        matches!(mode, egui::epaint::ColorMode::Solid(solid) if *solid == color)
    }

    fn shape_has(shape: &egui::Shape, color: egui::Color32) -> bool {
        match shape {
            egui::Shape::Vec(inner) => inner.iter().any(|shape| shape_has(shape, color)),
            egui::Shape::Circle(circle) => circle.fill == color || circle.stroke.color == color,
            egui::Shape::Ellipse(ellipse) => ellipse.fill == color || ellipse.stroke.color == color,
            egui::Shape::LineSegment { stroke, .. } => stroke.color == color,
            egui::Shape::Path(path) => path.fill == color || solid(&path.stroke.color, color),
            egui::Shape::Rect(rect) => rect.fill == color || rect.stroke.color == color,
            egui::Shape::Text(text) => {
                text.fallback_color == color
                    || text.override_text_color == Some(color)
                    || text
                        .galley
                        .job
                        .sections
                        .iter()
                        .any(|section| section.format.color == color)
            }
            egui::Shape::Mesh(mesh) => mesh.vertices.iter().any(|vertex| vertex.color == color),
            egui::Shape::QuadraticBezier(curve) => {
                curve.fill == color || solid(&curve.stroke.color, color)
            }
            egui::Shape::CubicBezier(curve) => {
                curve.fill == color || solid(&curve.stroke.color, color)
            }
            egui::Shape::Noop | egui::Shape::Callback(_) => false,
        }
    }

    fn shape_uses(output: &egui::FullOutput, color: egui::Color32) -> bool {
        output
            .shapes
            .iter()
            .any(|clipped| shape_has(&clipped.shape, color))
    }

    fn text_uses(output: &egui::FullOutput, needle: &str, color: egui::Color32) -> bool {
        fn walk(shape: &egui::Shape, needle: &str, color: egui::Color32) -> bool {
            match shape {
                egui::Shape::Vec(inner) => inner.iter().any(|shape| walk(shape, needle, color)),
                egui::Shape::Text(text) => {
                    let colored = text.fallback_color == color
                        || text.override_text_color == Some(color)
                        || text
                            .galley
                            .job
                            .sections
                            .iter()
                            .any(|section| section.format.color == color);
                    colored && text.galley.text().contains(needle)
                }
                _ => false,
            }
        }
        output
            .shapes
            .iter()
            .any(|clipped| walk(&clipped.shape, needle, color))
    }

    #[test]
    fn theme_inks_clear_their_fills() {
        let dark = ink(true);
        let light = ink(false);
        let panel_dark = egui::Color32::from_gray(27);
        let panel_light = egui::Color32::from_gray(248);
        let plot_dark = egui::Color32::from_gray(10);
        let plot_light = egui::Color32::from_gray(255);
        let dark_ok = dark.note == egui::Color32::from_rgb(0xff, 0xd9, 0x3d)
            && dark.failure == egui::Color32::from_rgb(0xff, 0x8a, 0x80)
            && dark.curve == egui::Color32::from_rgb(0x4e, 0xcd, 0xc4)
            && dark.curve_b == egui::Color32::from_rgb(0xff, 0x6b, 0x6b)
            && contrast(dark.note, panel_dark) >= 4.5
            && contrast(dark.failure, panel_dark) >= 4.5
            && contrast(dark.curve, plot_dark) >= 3.0
            && contrast(dark.curve_b, plot_dark) >= 3.0;
        let light_ok = contrast(light.note, panel_light) >= 4.5
            && contrast(light.failure, panel_light) >= 4.5
            && contrast(light.curve, plot_light) >= 3.0
            && contrast(light.curve_b, plot_light) >= 3.0
            && light.note != dark.note
            && light.failure != dark.failure
            && light.curve != dark.curve
            && light.curve_b != dark.curve_b;
        assert!(dark_ok);
        assert!(light_ok);
    }

    #[test]
    fn the_window_paints_each_themes_inks() {
        let (mut app, mut ui) = open_long("inks");
        let output = ui.show(&mut app, Vec::new());
        let alpha = node_rects(&output, "Alpha", false);
        let compares = node_rects(&output, "Compare", true);
        let paired = alpha.first().and_then(|label| {
            compares
                .iter()
                .find(|compare| (compare.center().y - label.center().y).abs() < 8.0)
                .map(|compare| compare.center())
        });
        output.drop_without_applying_deltas();
        assert!(paired.is_some());
        click_pos(&mut ui, &mut app, paired.unwrap_or(egui::Pos2::ZERO));
        assert!(app.compare.is_some());
        app.note = "palette note".to_string();
        for dark in [true, false] {
            app.prefs
                .set_theme(if dark { Theme::Dark } else { Theme::Light });
            let colors = ink(dark);
            let output = ui.show(&mut app, Vec::new());
            let painted = text_uses(&output, "palette note", colors.note)
                && text_uses(&output, "boom", colors.failure)
                && shape_uses(&output, colors.curve)
                && shape_uses(&output, colors.curve_b);
            output.drop_without_applying_deltas();
            assert!(painted);
        }
    }

    #[test]
    fn a_long_run_keeps_compare_inside_the_list() {
        let (mut app, mut ui) = open_long("clip");
        let output = ui.show(&mut app, Vec::new());
        let labels = node_rects(&output, "qwen2.5-7b-instruct", false);
        let compares = node_rects(&output, "Compare", true);
        let row_ok = labels.iter().any(|label| {
            label.right() <= 460.0
                && compares.iter().any(|compare| {
                    (compare.center().y - label.center().y).abs() < 8.0
                        && label.right() <= compare.left() + 4.0
                        && compare.left() > label.left()
                        && compare.right() <= 460.0
                        && compare.width() >= 40.0
                })
        });
        let all_inside = !compares.is_empty()
            && compares.iter().all(|compare| {
                compare.left() >= 0.0 && compare.right() <= 460.0 && compare.width() >= 40.0
            });
        output.drop_without_applying_deltas();
        assert!(row_ok);
        assert!(all_inside);
    }
}
