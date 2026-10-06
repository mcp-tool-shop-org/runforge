//! The series window: every sample, the recipe board, the low row, the sidecar.

use eframe::egui::{self, Color32, RichText, Stroke};
use egui_plot::{Line, MarkerShape, Plot, PlotPoints, Points};
use runforge_core::{
    Board, Hypothesis, LearnedTool, Ledger, NOTE_LIMIT, Reading, Sample, SeriesRead, Step,
    Weighing, band_segments, comparison_report_full, epoch_floor, format_measure, is_heading,
    loss_segments, low_band, orientation_allowed, orientation_omission, recipe_keys, recipe_label,
    recipe_marks, recipe_text, spikes_above, utc_date, weigh, wording_problem,
};

pub enum InstrumentAction {
    None,
    Ask,
    SaveReport,
    TryFormula,
    Focus(String),
}

pub struct SidecarView<'a> {
    pub memory: &'a str,
    pub status: &'a str,
    pub answer: &'a str,
    pub can_ask: bool,
    /// Earlier weighings of these runs, this recipe, or this method.
    pub ledger: &'a Ledger,
    /// Hypotheses for this method, each with its latest test.
    pub hypotheses: &'a [Hypothesis],
    /// Learned tools.
    pub tools: &'a [LearnedTool],
    /// The calls of the last workbench session.
    pub trace: &'a [Step],
    /// The formula box and what it last gave.
    pub formula: &'a mut String,
    pub formula_lines: &'a [String],
    pub blocked: bool,
}

/// UTC day for the report stamp. The samples do not carry a measurement date.
pub(crate) fn report_date() -> Option<String> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(utc_date(secs))
}

struct Hover {
    name: String,
    x: f64,
    loss: Option<f64>,
    lr: Option<f64>,
    extra: String,
}

const LINK: &str = "series-epoch";

pub fn draw_instrument(
    ui: &mut egui::Ui,
    board: &Board,
    reading: &Reading,
    sidecar: &mut SidecarView<'_>,
    focus: Option<&str>,
) -> InstrumentAction {
    let dark = ui.visuals().dark_mode;
    let weighing = weigh(board);
    let report = comparison_report_full(
        board,
        report_date().as_deref(),
        sidecar.ledger,
        sidecar.hypotheses,
        sidecar.tools,
    );
    let omission = orientation_omission(board);
    let mut action = InstrumentAction::None;
    egui::Panel::right("sidecar")
        .exact_size(340.0)
        .show(ui, |ui| {
            action = draw_sidecar(ui, reading, &weighing, sidecar, &report, &omission);
        });
    egui::CentralPanel::default().show(ui, |ui| {
        if let Some(name) = draw_stage(ui, board, reading, dark, focus) {
            action = InstrumentAction::Focus(name);
        }
    });
    action
}

fn draw_stage(
    ui: &mut egui::Ui,
    board: &Board,
    reading: &Reading,
    dark: bool,
    focus: Option<&str>,
) -> Option<String> {
    let model = shared_model(board);
    ui.horizontal(|ui| {
        ui.heading(model.unwrap_or("Series"));
        ui.label(
            RichText::new(format!(
                "{} series  ·  {} samples",
                board.series.len(),
                board
                    .series
                    .iter()
                    .map(|series| series.samples.len())
                    .sum::<usize>()
            ))
            .weak(),
        );
    });
    ui.label(RichText::new(&reading.lever).strong());
    if let Some(finding) = reading.finding() {
        ui.label(RichText::new(finding).color(note_color(dark)));
    }
    if board.skipped > 0 {
        ui.label(format!(
            "{} files in this folder were not series",
            board.skipped
        ));
    }
    ui.add_space(6.0);
    draw_recipe(ui, board, dark);
    ui.add_space(8.0);
    let chosen = draw_legend(ui, board, dark, focus);
    let hover = hover_rows(board);
    let loss_height = (ui.available_height() * 0.34).clamp(200.0, 300.0);
    draw_loss(ui, board, reading, dark, loss_height, &hover, focus);
    ui.label("Every sample. Log loss. The diamond is the lowest sample. Scroll brushes the epoch range. Select a seed to read it through the others.");
    ui.add_space(4.0);
    draw_lr(ui, board, dark, &hover, focus);
    ui.label("Learning rate on the same epochs. Its vertical scale is its own.");
    ui.add_space(8.0);
    draw_band(ui, board, reading, dark, focus);
    chosen
}

fn draw_recipe(ui: &mut egui::Ui, board: &Board, dark: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for key in recipe_keys(&board.shared) {
            if key == "target_modules" {
                continue;
            }
            let Some(value) = board.shared.get(key) else {
                continue;
            };
            chip(ui, recipe_label(key), &recipe_text(value));
        }
    });
    if let Some(modules) = board.shared.get("target_modules").and_then(recipe_marks) {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("target modules").small().weak());
            let stroke = series_color(0, dark);
            for module in modules {
                egui::Frame::new()
                    .stroke(Stroke::new(1.0, stroke))
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.label(module);
                    });
            }
        });
    }
    if board.varying.is_empty() {
        return;
    }
    ui.add_space(6.0);
    ui.label("These recipe fields are not shared");
    for key in &board.varying {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(recipe_label(key)).small().weak());
            for series in &board.series {
                let value = series
                    .recipe
                    .get(key)
                    .map(recipe_text)
                    .unwrap_or_else(|| "absent".to_string());
                chip(ui, &series.name, &value);
            }
        });
    }
}

fn chip(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::new()
        .fill(ui.visuals().widgets.inactive.bg_fill)
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(label).small().weak());
                ui.label(RichText::new(value).strong());
            });
        });
}

fn draw_legend(
    ui: &mut egui::Ui,
    board: &Board,
    dark: bool,
    focus: Option<&str>,
) -> Option<String> {
    let mut chosen = None;
    ui.horizontal_wrapped(|ui| {
        for (index, series) in board.series.iter().enumerate() {
            let color = series_color(index, dark);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 3.0, color);
            let on = focus == Some(series.name.as_str());
            let text = RichText::new(&series.name).color(color).strong();
            if ui
                .add(egui::Button::selectable(on, text).frame(false))
                .clicked()
            {
                chosen = Some(series.name.clone());
            }
        }
    });
    chosen
}

fn draw_loss(
    ui: &mut egui::Ui,
    board: &Board,
    reading: &Reading,
    dark: bool,
    height: f32,
    hover: &[Hover],
    focus: Option<&str>,
) {
    let axis = ui.id().with(LINK);
    Plot::new("series-loss")
        .height(height)
        .x_axis_label("epoch")
        .y_axis_label("loss")
        .link_axis(axis, egui::Vec2b::new(true, false))
        .link_cursor(axis, egui::Vec2b::new(true, false))
        .y_axis_formatter(|mark, _| format_measure(10f64.powf(mark.value)))
        .label_formatter({
            let hover = hover_owned(hover);
            move |position| Some(format_hover(&hover, position))
        })
        .show(ui, |plot| {
            for (index, series) in board.series.iter().enumerate() {
                let (color, width) = ink_for(series_color(index, dark), &series.name, focus);
                draw_lines(plot, &series.name, color, width, &series.samples, true);
                if let Some(mark) = reading
                    .series
                    .iter()
                    .find(|item| item.name == series.name)
                    .and_then(|item| item.low.as_ref())
                    && mark.loss > 0.0
                {
                    plot.points(
                        Points::new("", PlotPoints::new(vec![[mark.x, mark.loss.log10()]]))
                            .color(color)
                            .radius(5.0)
                            .shape(MarkerShape::Diamond),
                    );
                }
            }
        });
}

fn draw_lr(ui: &mut egui::Ui, board: &Board, dark: bool, hover: &[Hover], focus: Option<&str>) {
    let axis = ui.id().with(LINK);
    Plot::new("series-lr")
        .height(120.0)
        .x_axis_label("epoch")
        .y_axis_label("learning rate")
        .link_axis(axis, egui::Vec2b::new(true, false))
        .link_cursor(axis, egui::Vec2b::new(true, false))
        .label_formatter({
            let hover = hover_owned(hover);
            move |position| Some(format_hover(&hover, position))
        })
        .show(ui, |plot| {
            for (index, series) in board.series.iter().enumerate() {
                let (color, width) = ink_for(series_color(index, dark), &series.name, focus);
                draw_lr_lines(plot, &series.name, color, width, &series.samples);
            }
        });
}

fn draw_band(ui: &mut egui::Ui, board: &Board, reading: &Reading, dark: bool, focus: Option<&str>) {
    let Some((start, end, top)) = low_band(board) else {
        return;
    };
    ui.label(format!(
        "Around the lows, from epoch {}. Linear loss, one scale. The bold line is the lowest sample in each epoch. The diamond is that low. The circle is the last sample. A spike above {} stays on the main chart.",
        format_measure(start),
        format_measure(top)
    ));
    let count = board.series.len().max(1);
    ui.columns(count, |columns| {
        for (column, series) in columns.iter_mut().zip(&board.series) {
            let index = board
                .series
                .iter()
                .position(|item| item.file_name == series.file_name)
                .unwrap_or(0);
            let base = series_color(index, dark);
            let focused = focus == Some(series.name.as_str());
            let dim = focus.is_some() && !focused;
            let ink = if dim { base.gamma_multiply(0.28) } else { base };
            let raw = if dim {
                base.gamma_multiply(0.10)
            } else if focused {
                base.gamma_multiply(0.55)
            } else {
                base.gamma_multiply(0.34)
            };
            let read = reading.series.iter().find(|item| item.name == series.name);
            egui::Frame::new()
                .fill(column.visuals().extreme_bg_color)
                .corner_radius(8)
                .inner_margin(egui::Margin::same(8))
                .show(column, |ui| {
                    ui.label(RichText::new(&series.name).color(ink).strong());
                    if let Some(read) = read {
                        ui.label(band_caption(read));
                        if read.climbed {
                            ui.label(RichText::new("climbs after the low").color(note_color(dark)));
                        }
                    }
                    let spikes = spikes_above(&series.samples, start, top);
                    if spikes > 0 {
                        ui.label(
                            RichText::new(format!("{spikes} spikes stay on the main chart"))
                                .small()
                                .weak(),
                        );
                    }
                    let id = format!("late-{}", series.file_name);
                    let low = read.and_then(|item| item.low.clone());
                    let last = read.and_then(|item| item.last.clone());
                    Plot::new(id)
                        .height(124.0)
                        .allow_zoom(egui::Vec2b::FALSE)
                        .allow_drag(egui::Vec2b::FALSE)
                        .allow_scroll(egui::Vec2b::FALSE)
                        .y_axis_formatter(|mark, _| format_measure(mark.value))
                        .default_x_bounds(start, end)
                        .default_y_bounds(0.0, top)
                        .show(ui, |plot| {
                            for segment in band_segments(&series.samples, start, top) {
                                plot.line(
                                    Line::new("", PlotPoints::new(segment))
                                        .color(raw)
                                        .width(1.0),
                                );
                            }
                            for segment in epoch_floor(&series.samples, start) {
                                plot.line(
                                    Line::new("", PlotPoints::new(segment))
                                        .color(ink)
                                        .width(if dim { 1.8 } else { 2.6 }),
                                );
                            }
                            if let Some(mark) = last.filter(|mark| {
                                mark.loss > 0.0
                                    && mark.loss <= top
                                    && mark.x + f64::EPSILON >= start
                            }) {
                                plot.points(
                                    Points::new("", PlotPoints::new(vec![[mark.x, mark.loss]]))
                                        .color(ink)
                                        .radius(3.5)
                                        .shape(MarkerShape::Circle),
                                );
                            }
                            if let Some(mark) =
                                low.filter(|mark| mark.loss > 0.0 && mark.x + f64::EPSILON >= start)
                            {
                                plot.points(
                                    Points::new("", PlotPoints::new(vec![[mark.x, mark.loss]]))
                                        .color(ink)
                                        .radius(4.5)
                                        .shape(MarkerShape::Diamond),
                                );
                            }
                        });
                });
        }
    });
}

fn band_caption(read: &SeriesRead) -> String {
    let mut line = String::new();
    if let Some(low) = &read.low {
        line.push_str(&format!(
            "low {} at {}",
            format_measure(low.loss),
            format_measure(low.x)
        ));
    }
    if let Some(last) = &read.last {
        if !line.is_empty() {
            line.push_str("  ·  ");
        }
        line.push_str(&format!("last {}", format_measure(last.loss)));
    }
    line
}

fn draw_sidecar(
    ui: &mut egui::Ui,
    reading: &Reading,
    weighing: &Weighing,
    sidecar: &mut SidecarView<'_>,
    report: &str,
    omission: &str,
) -> InstrumentAction {
    let mut action = InstrumentAction::None;
    let dark = ui.visuals().dark_mode;
    ui.heading("Sidecar");
    ui.label(
        RichText::new("Local model only. It does not press Train.")
            .small()
            .weak(),
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(sidecar.can_ask, egui::Button::new("Ask the local model"))
            .clicked()
        {
            action = InstrumentAction::Ask;
        }
        if ui.button("Save report").clicked() {
            action = InstrumentAction::SaveReport;
        }
    });
    if !sidecar.status.is_empty() && sidecar.answer.is_empty() && !sidecar.blocked {
        ui.add_space(6.0);
        ui.label(RichText::new(sidecar.status).weak());
    }
    egui::ScrollArea::vertical()
        .id_salt("sidecar-scroll")
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(finding) = reading.finding() {
                ui.add_space(8.0);
                ui.label(RichText::new(finding).color(note_color(dark)).strong());
            }
            ui.add_space(8.0);
            draw_report(ui, report);
            if sidecar.blocked {
                ui.add_space(8.0);
                model_note(ui, "", omission);
            }
            if !sidecar.answer.is_empty() && wording_problem(sidecar.answer, NOTE_LIMIT).is_none() {
                ui.add_space(8.0);
                model_note(
                    ui,
                    "The model's note (its words, not a measurement)",
                    sidecar.answer,
                );
            } else if let Some(remembered) = remembered_note(sidecar.memory, reading)
                && (orientation_allowed(&remembered)
                    || wording_problem(&remembered, NOTE_LIMIT).is_none())
            {
                ui.add_space(8.0);
                model_note(ui, "Remembered", &remembered);
            }
            ui.add_space(8.0);
            if draw_bench(ui, sidecar) {
                action = InstrumentAction::TryFormula;
            }
            if !weighing.cards.is_empty() {
                ui.add_space(8.0);
                egui::CollapsingHeader::new("Reference")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for card in &weighing.cards {
                            ui.add_space(6.0);
                            ui.label(RichText::new(card.formula).strong());
                            ui.add(egui::Label::new(card.finding).wrap());
                            ui.label(RichText::new(card.cite).small().weak());
                            ui.label(RichText::new(card.url).small().weak());
                        }
                    });
            }
        });
    action
}

/// The workbench: a formula box, the last session's calls, and the learned tools.
/// Returns true when Run was pressed.
fn draw_bench(ui: &mut egui::Ui, sidecar: &mut SidecarView<'_>) -> bool {
    let mut run = false;
    egui::CollapsingHeader::new("Workbench")
        .default_open(true)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Try a formula, such as last / low.")
                    .small()
                    .weak(),
            );
            ui.horizontal(|ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(sidecar.formula)
                        .desired_width(ui.available_width() - 48.0)
                        .hint_text("formula"),
                );
                let entered =
                    field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if ui.button("Run").clicked() || entered {
                    run = true;
                }
            });
            for line in sidecar.formula_lines {
                ui.add(egui::Label::new(RichText::new(line).small()).wrap());
            }
            if !sidecar.trace.is_empty() {
                ui.add_space(6.0);
                egui::CollapsingHeader::new(format!("Last session: {} calls", sidecar.trace.len()))
                    .default_open(false)
                    .show(ui, |ui| {
                        for step in sidecar.trace {
                            ui.label(
                                RichText::new(format!("{} {}", step.tool, step.args))
                                    .small()
                                    .strong(),
                            );
                            ui.add(egui::Label::new(RichText::new(&step.result).small()).wrap());
                            ui.add_space(4.0);
                        }
                    });
            }
            if !sidecar.tools.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("Learned tools").strong());
                for tool in sidecar.tools {
                    let status = if tool.kept() { "kept" } else { "provisional" };
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!(
                                "{} = {}. {} ({status}, used {} times)",
                                tool.name, tool.formula, tool.meaning, tool.uses
                            ))
                            .small(),
                        )
                        .wrap(),
                    );
                }
            }
        });
    run
}

fn draw_report(ui: &mut egui::Ui, report: &str) {
    for line in report.lines() {
        if line.is_empty() {
            ui.add_space(8.0);
            continue;
        }
        if let Some(rest) = line.strip_prefix("  ") {
            ui.add(egui::Label::new(RichText::new(rest).small()).wrap());
            continue;
        }
        if is_heading(line) {
            ui.label(RichText::new(line).strong());
            continue;
        }
        ui.add(egui::Label::new(line).wrap());
    }
}

fn plain_note(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_start_matches('#').trim_start().replace("**", ""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn model_note(ui: &mut egui::Ui, title: &str, body: &str) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(8)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            if !title.is_empty() {
                ui.label(RichText::new(title).strong());
            }
            ui.add(egui::Label::new(plain_note(body)).wrap());
        });
}

/// The model half of a stored note. The reading above it is already on the pane.
fn remembered_note(memory: &str, reading: &Reading) -> Option<String> {
    let prefix = format!("{}\n\n", reading.lines().join("\n"));
    let rest = memory.strip_prefix(&prefix)?.trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

fn ink_for(color: Color32, name: &str, focus: Option<&str>) -> (Color32, f32) {
    match focus {
        Some(focus) if focus != name => (color.gamma_multiply(0.18), 1.1),
        _ => (color, 2.4),
    }
}

fn draw_lines(
    plot: &mut egui_plot::PlotUi<'_>,
    name: &str,
    color: Color32,
    width: f32,
    samples: &[Sample],
    log_scale: bool,
) {
    for (index, segment) in loss_segments(samples, log_scale).into_iter().enumerate() {
        let label = if index == 0 { name } else { "" };
        plot.line(
            Line::new(label, PlotPoints::new(segment))
                .color(color)
                .width(width),
        );
    }
}

fn draw_lr_lines(
    plot: &mut egui_plot::PlotUi<'_>,
    name: &str,
    color: Color32,
    width: f32,
    samples: &[Sample],
) {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for sample in samples {
        match (sample.x, sample.lr) {
            (Some(x), Some(lr)) => current.push([x, lr]),
            _ => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    for (index, segment) in segments.into_iter().enumerate() {
        let label = if index == 0 { name } else { "" };
        plot.line(
            Line::new(label, PlotPoints::new(segment))
                .color(color)
                .width(width),
        );
    }
}

fn hover_rows(board: &Board) -> Vec<Hover> {
    let mut rows = Vec::new();
    for series in &board.series {
        for sample in &series.samples {
            let Some(x) = sample.x else { continue };
            let extra = sample
                .extra
                .iter()
                .map(|(key, value)| format!("{key} {}", recipe_text(value)))
                .collect::<Vec<_>>()
                .join("\n");
            rows.push(Hover {
                name: series.name.clone(),
                x,
                loss: sample.loss,
                lr: sample.lr,
                extra,
            });
        }
    }
    rows
}

fn hover_owned(rows: &[Hover]) -> Vec<Hover> {
    rows.iter()
        .map(|row| Hover {
            name: row.name.clone(),
            x: row.x,
            loss: row.loss,
            lr: row.lr,
            extra: row.extra.clone(),
        })
        .collect()
}

fn format_hover(rows: &[Hover], position: &egui_plot::HoverPosition<'_>) -> String {
    let (name, x) = match position {
        egui_plot::HoverPosition::NearDataPoint {
            plot_name,
            position,
            ..
        } => (plot_name.to_string(), position.x),
        egui_plot::HoverPosition::Elsewhere { position } => (String::new(), position.x),
    };
    let nearest = rows
        .iter()
        .filter(|row| name.is_empty() || row.name == name)
        .min_by(|left, right| {
            (left.x - x)
                .abs()
                .total_cmp(&(right.x - x).abs())
                .then_with(|| left.name.cmp(&right.name))
        });
    let Some(row) = nearest else {
        return format!("epoch {}", format_measure(x));
    };
    let loss = row
        .loss
        .map(format_measure)
        .unwrap_or_else(|| "—".to_string());
    let lr = row
        .lr
        .map(format_measure)
        .unwrap_or_else(|| "—".to_string());
    let mut text = format!(
        "{}\nepoch {}\nloss {}\nlr {}",
        row.name,
        format_measure(row.x),
        loss,
        lr
    );
    if !row.extra.is_empty() {
        text.push('\n');
        text.push_str(&row.extra);
    }
    text
}

fn shared_model(board: &Board) -> Option<&str> {
    let first = board.series.first()?.model.as_str();
    if !first.is_empty() && board.series.iter().all(|series| series.model == first) {
        Some(first)
    } else {
        None
    }
}

fn series_color(index: usize, dark: bool) -> Color32 {
    const DARK: [Color32; 8] = [
        Color32::from_rgb(0x5e, 0xe0, 0xd0),
        Color32::from_rgb(0xf0, 0xc0, 0x5a),
        Color32::from_rgb(0xff, 0x7a, 0x59),
        Color32::from_rgb(0xb7, 0x94, 0xf6),
        Color32::from_rgb(0x7d, 0xb7, 0xff),
        Color32::from_rgb(0xf4, 0x8f, 0xb1),
        Color32::from_rgb(0x86, 0xef, 0xac),
        Color32::from_rgb(0xfd, 0xba, 0x74),
    ];
    const LIGHT: [Color32; 8] = [
        Color32::from_rgb(0x0b, 0x6e, 0x68),
        Color32::from_rgb(0x8a, 0x5a, 0x00),
        Color32::from_rgb(0xb4, 0x2c, 0x12),
        Color32::from_rgb(0x5b, 0x3d, 0xb0),
        Color32::from_rgb(0x1d, 0x4e, 0x89),
        Color32::from_rgb(0x9d, 0x17, 0x4d),
        Color32::from_rgb(0x16, 0x65, 0x34),
        Color32::from_rgb(0x9a, 0x34, 0x12),
    ];
    let palette = if dark { &DARK } else { &LIGHT };
    palette[index % palette.len()]
}

fn note_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0xff, 0xd9, 0x3d)
    } else {
        Color32::from_rgb(0x6b, 0x44, 0x00)
    }
}
