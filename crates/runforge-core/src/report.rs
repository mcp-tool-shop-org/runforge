//! The comparison report. One string, two renderings.
//!
//! The window shows this text. Save writes this text. A number, a seed, a knob,
//! or a verdict in the body is filled from the board and the ledger. The model
//! does not write it.
//!
//! The report leads with its conclusion. Each later part supports that
//! conclusion with the numbers it rests on, so a reader can stop after the first
//! paragraph. A part that has nothing to say for this board is not printed.

use serde_json::Value;

use crate::bench::{Hypothesis, LearnedTool};
use crate::ledger::{Ledger, Weighed};
use crate::series::{
    Board, Reading, Series, format_measure, read_board, recipe_keys, recipe_label, recipe_text,
};
use crate::weigh::{Neighborhood, Separation, Spread, Weighing, spread, weigh};

/// Printed when a fresh orientation note crosses the fence.
pub const ORIENTATION_OMITTED: &str = "Assistant note omitted: it carried a number or a verdict, which only the measurements may carry. The measured lines stand on their own.";

/// `unix_secs` is whole seconds since 1970-01-01 UTC.
///
/// Howard Hinnant's `civil_from_days`. Unix day 0 is 1970-01-01.
pub fn utc_date(unix_secs: u64) -> String {
    let (year, month, day) = civil_from_days((unix_secs / 86_400) as i64);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Date first, so a folder of reports sorts by the day they were written.
pub fn report_file_name(board: &Board, written_on: Option<&str>) -> String {
    let n = board.series.len();
    match written_on.map(str::trim).filter(|date| !date.is_empty()) {
        Some(date) => {
            let date: String = date
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
                .collect();
            format!("runforge-report-{date}-{n}-runs.txt")
        }
        None => format!("runforge-report-{n}-runs.txt"),
    }
}

/// The report for this board, with no earlier weighings.
pub fn comparison_report(board: &Board, written_on: Option<&str>) -> String {
    comparison_report_with(board, written_on, &Ledger::default())
}

/// The report with the workbench: hypotheses for this method and the learned tools.
pub fn comparison_report_full(
    board: &Board,
    written_on: Option<&str>,
    ledger: &Ledger,
    hypotheses: &[Hypothesis],
    tools: &[LearnedTool],
) -> String {
    let mut out = comparison_report_with(board, written_on, ledger);
    let bench = bench_section(board, hypotheses, tools);
    if bench.is_empty() {
        return out;
    }
    // The workbench goes before "What to do next", after the earlier weighings.
    match out.find("\nWhat to do next\n") {
        Some(at) => out.insert_str(at + 1, &bench),
        None => out.push_str(&bench),
    }
    out
}

fn bench_section(board: &Board, hypotheses: &[Hypothesis], tools: &[LearnedTool]) -> String {
    let key = crate::ledger::board_key(board);
    let method = crate::bench::board_method(board);
    let mine: Vec<&Hypothesis> = hypotheses.iter().filter(|h| h.method == method).collect();
    let mut out = String::new();
    if !mine.is_empty() {
        line(&mut out, "Hypotheses on the bench");
        line(
            &mut out,
            "Each was proposed with its test fixed: a knob, a formula, and a direction. The program sets the state.",
        );
        for hypothesis in mine {
            let here = hypothesis.evaluations.iter().find(|e| e.board == key);
            let text = match here {
                Some(evaluation) => format!(
                    "* {} Here: {}. {}",
                    hypothesis.statement(),
                    evaluation.state.word(),
                    evaluation.detail
                ),
                None => format!("* {} Not tested on these runs.", hypothesis.statement()),
            };
            line(&mut out, &text);
            let elsewhere = hypothesis
                .evaluations
                .iter()
                .filter(|e| e.board != key)
                .count();
            if elsewhere > 0 {
                let decided: Vec<String> = hypothesis
                    .evaluations
                    .iter()
                    .filter(|e| e.board != key)
                    .map(|e| format!("{} on {}", e.state.word(), e.date))
                    .collect();
                line(&mut out, &format!("  Elsewhere: {}.", decided.join("; ")));
            }
        }
        out.push('\n');
    }
    if !tools.is_empty() {
        line(&mut out, "Learned tools");
        for tool in tools {
            let status = if tool.kept() {
                "kept"
            } else {
                "provisional until used on a second folder"
            };
            line(
                &mut out,
                &format!(
                    "* {} = {}. {} ({status}).",
                    tool.name, tool.formula, tool.meaning
                ),
            );
        }
        out.push('\n');
    }
    out
}

/// The report for this board. `written_on` is the UTC day the file is written.
///
/// It is not a measurement date. The samples do not carry one. `ledger` is what
/// was weighed before; read it before this board is recorded.
pub fn comparison_report_with(board: &Board, written_on: Option<&str>, ledger: &Ledger) -> String {
    let reading = read_board(board);
    let weighing = weigh(board);
    let spread = spread(&weighing.neighborhoods);
    let mut out = String::new();
    title(&mut out, board, written_on);
    in_short(&mut out, board, &weighing, spread.as_ref());
    what_happened(&mut out, &reading, &weighing);
    the_runs(&mut out, board, &reading, &weighing);
    argument(&mut out, board, &reading, &weighing, spread.as_ref());
    changed_and_not(&mut out, board);
    earlier(&mut out, board, &weighing, ledger);
    next_steps(&mut out, board, &weighing, spread.as_ref());
    cannot_tell(&mut out, board);
    sources(&mut out, board, &weighing);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// A report line the window draws as a heading: short, not indented, no closing mark.
pub fn is_heading(line: &str) -> bool {
    let text = line.trim_end();
    !text.is_empty()
        && !text.starts_with(' ')
        && !text.starts_with("* ")
        && !text.starts_with(|ch: char| ch.is_ascii_digit())
        && text.chars().count() <= 48
        && !text.ends_with(['.', ':', ')', ',', ';'])
}

/// The omission line for a fresh note that crossed the fence, with the measured status filled in.
pub fn orientation_omission(board: &Board) -> String {
    let weighing = weigh(board);
    let status = if weighing.neighborhoods.len() < 2 {
        "There is no second run to weigh.".to_string()
    } else if weighing.abstain {
        "The weighing did not pick a winner.".to_string()
    } else {
        let names = names_at(
            &weighing.neighborhoods,
            &lowest_indexes(&weighing.neighborhoods),
        );
        format!(
            "The half-epoch median agrees with the lowest sample, {}.",
            english_list(&names)
        )
    };
    match deepest_clause(&weighing.neighborhoods) {
        Some(clause) => format!("{ORIENTATION_OMITTED} The deepest point is {clause}. {status}"),
        None => format!("{ORIENTATION_OMITTED} {status}"),
    }
}

/// An orientation paragraph may explain a loss curve or a seed.
///
/// It may not carry a digit, a knob, a verdict, or markdown the pane would strip.
pub fn orientation_allowed(text: &str) -> bool {
    if text.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }
    if text.contains("**") || text.lines().any(|line| line.trim_start().starts_with('#')) {
        return false;
    }
    let words = words_of(&text.to_lowercase());
    if words.windows(2).any(|pair| {
        (pair[0] == "learning" && pair[1] == "rate") || (pair[0] == "next" && pair[1] == "time")
    }) {
        return false;
    }
    const VERDICT: &[&str] = &[
        "best",
        "winner",
        "won",
        "better",
        "worse",
        "recommend",
        "should",
        "try",
    ];
    const KNOB: &[&str] = &[
        "warmup",
        "batch",
        "rank",
        "alpha",
        "dropout",
        "schedule",
        "clip",
        "decay",
        "checkpoint",
    ];
    !words
        .iter()
        .any(|word| VERDICT.contains(&word.as_str()) || KNOB.contains(&word.as_str()))
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

fn title(out: &mut String, board: &Board, written_on: Option<&str>) {
    line(out, "RunForge report");
    let n = board.series.len();
    let subtitle = if n <= 1 {
        "One run.".to_string()
    } else if seed_only(board) {
        format!(
            "{} runs of one recipe. Only the seed changed.",
            count_word(n, true)
        )
    } else if board.varying.is_empty() {
        format!("{} runs of one recipe.", count_word(n, true))
    } else {
        format!(
            "{} runs. What changed: {}.",
            count_word(n, true),
            lever_name(board)
        )
    };
    line(out, &subtitle);
    if let Some(date) = written_on.map(str::trim).filter(|date| !date.is_empty()) {
        line(out, &format!("Written {date} UTC."));
    }
}

fn in_short(out: &mut String, board: &Board, weighing: &Weighing, spread: Option<&Spread>) {
    section(out, "In short");
    let nears = &weighing.neighborhoods;
    if nears.is_empty() {
        line(
            out,
            "No stored sample has a finite loss, so there is nothing to weigh.",
        );
        return;
    }
    if nears.len() == 1 {
        let near = &nears[0];
        line(
            out,
            &format!(
                "There is no second run to compare it with, so this report names no winner. Its deepest point is {} at epoch {}, and the middle of the half epoch around it is {}.",
                format_measure(near.low),
                format_measure(near.at),
                format_measure(near.median)
            ),
        );
        return;
    }
    let low_at = lowest_indexes(nears);
    let quiet_at = quiet_indexes(nears);
    let low = &nears[low_at[0]];
    let quiet = &nears[quiet_at[0]];
    let low_names = english_list(&names_at(nears, &low_at));
    let quiet_names = english_list(&names_at(nears, &quiet_at));
    let mut text = if weighing.abstain {
        format!(
            "No run wins. {} has the deepest single point, {} at epoch {}. {} has the calmest stretch around its low: a middle of {}, against {} around {}'s low.",
            cap(&low_names),
            format_measure(low.low),
            format_measure(low.at),
            cap(&quiet_names),
            format_measure(quiet.median),
            format_measure(low.median),
            low_names
        )
    } else {
        format!(
            "{} leads on both counts: the deepest single point ({} at epoch {}) and the calmest stretch around its low (a middle of {}).",
            cap(&low_names),
            format_measure(low.low),
            format_measure(low.at),
            format_measure(low.median)
        )
    };
    if let Some(spread) = spread {
        text.push(' ');
        text.push_str(&spread_sentence(spread));
        if !weighing.abstain && spread.separation == Separation::InsideNoise {
            text.push_str(" Read the lead as a lean, not a result.");
        }
    }
    line(out, &text);
    let closing = if seed_only(board) {
        "Nothing in the recipe changed, so these runs say nothing about any setting.".to_string()
    } else if (rank_moves(board) || alpha_moves(board)) && seeds_differ(board) {
        "The runs differ in the seed as well, so no difference here belongs to one setting alone."
            .to_string()
    } else if !board.varying.is_empty() && seeds_differ(board) {
        format!(
            "The runs differ in {} and in the seed, so no difference here belongs to one setting alone.",
            lever_name(board)
        )
    } else if !board.varying.is_empty() {
        format!(
            "Only {} changed, on one seed, so this is a measured difference, not yet a result.",
            lever_name(board)
        )
    } else {
        String::new()
    };
    if !closing.is_empty() {
        line(out, &closing);
    }
}

/// The sentence that sets the gap between run middles against the noise inside one run.
fn spread_sentence(spread: &Spread) -> String {
    match spread.separation {
        Separation::InsideNoise => format!(
            "The runs' middles sit within {} of each other, less than the middle half of any one run's stretch (the narrowest spans {}). The difference between these runs is smaller than the noise inside a single run.",
            format_measure(spread.gap),
            format_measure(spread.narrowest)
        ),
        Separation::Partial => format!(
            "The runs' middles sit within {} of each other: wider than the calmest run's middle half ({}), narrower than the noisiest run's ({}). The runs only partly separate.",
            format_measure(spread.gap),
            format_measure(spread.narrowest),
            format_measure(spread.widest)
        ),
        Separation::Apart => format!(
            "The runs' middles are {} apart, wider than the middle half of any one run's stretch (the widest spans {}). The difference between these runs is larger than the noise inside a single run.",
            format_measure(spread.gap),
            format_measure(spread.widest)
        ),
    }
}

fn what_happened(out: &mut String, reading: &Reading, weighing: &Weighing) {
    let span = loss_span_sentence(reading);
    let climb = climb_sentence(reading, &weighing.neighborhoods);
    let gaps = gap_sentence(reading);
    if span.is_none() && climb.is_none() && gaps.is_none() {
        return;
    }
    section(out, "What happened");
    for text in [span, climb, gaps].into_iter().flatten() {
        line(out, &text);
    }
}

fn the_runs(out: &mut String, board: &Board, reading: &Reading, weighing: &Weighing) {
    let nears = &weighing.neighborhoods;
    if nears.is_empty() {
        return;
    }
    section(out, "The runs");
    let mut intro = samples_clause(board);
    if let Some(span) = span_clause(board) {
        intro.push(' ');
        intro.push_str(&span);
    }
    intro.push_str(
        " A run's stretch is every stored sample within half an epoch of its own deepest point.",
    );
    line(out, &intro);
    for index in discussion_order(nears) {
        let near = &nears[index];
        line(out, "");
        line(out, &cap(&near.name));
        let mut deepest = format!(
            "  Deepest point {} at epoch {}.",
            format_measure(near.low),
            format_measure(near.at)
        );
        if let (Some(lr), Some(max), Some(decayed)) = (near.lr, near.lr_max, near.decayed()) {
            let place = if decayed {
                "under a twentieth"
            } else {
                "still above a twentieth"
            };
            deepest.push_str(&format!(
                " Learning rate there {}, {place} of its peak of {}.",
                format_measure(lr),
                format_measure(max)
            ));
        }
        line(out, &deepest);
        let samples = if near.count == 1 { "sample" } else { "samples" };
        let mut stretch = format!(
            "  Stretch: {} {samples}, middle {}, middle half from {} to {}.",
            near.count,
            format_measure(near.median),
            format_measure(near.q1),
            format_measure(near.q3)
        );
        match near.next {
            Some(next) if near.lone_low() => stretch.push_str(&format!(
                " The deepest point is a lone sample: the next lowest is {}, more than twice as high.",
                format_measure(next)
            )),
            Some(next) => stretch.push_str(&format!(" Next lowest {}.", format_measure(next))),
            None => {}
        }
        line(out, &stretch);
        if let Some(series) = reading
            .series
            .iter()
            .find(|series| series.name == near.name)
        {
            let mut end = Vec::new();
            if let Some(last) = &series.last {
                end.push(format!("Last sample {}.", format_measure(last.loss)));
            }
            if let Some(final_loss) = series.summary_final {
                end.push(format!(
                    "training_summary.final_loss {}, a marker, not a point on the curve.",
                    format_measure(final_loss)
                ));
            }
            if !end.is_empty() {
                line(out, &format!("  {}", end.join(" ")));
            }
        }
    }
}

fn argument(
    out: &mut String,
    board: &Board,
    reading: &Reading,
    weighing: &Weighing,
    spread: Option<&Spread>,
) {
    let nears = &weighing.neighborhoods;
    if nears.len() >= 2 && (rank_moves(board) || alpha_moves(board)) {
        section(out, "The setting under test");
        line(out, &rank_detail(board));
        if seeds_differ(board) {
            let head = if board.series.len() == 2 {
                "The two runs differ in rank and in seed"
            } else {
                "These runs differ in rank and in seed"
            };
            line(
                out,
                &format!(
                    "{head}, so no measured difference here can be attributed to the rank alone."
                ),
            );
        } else {
            let measured = match spread.map(|spread| spread.separation) {
                Some(Separation::InsideNoise) => {
                    " The gap between the two middles is smaller than the noise inside either run."
                }
                Some(Separation::Apart) => {
                    " The gap between the two middles is larger than the noise inside either run, but it is one pair on one seed."
                }
                _ => "",
            };
            line(
                out,
                &format!(
                    "This report does not credit the rank change.{measured} Run the same rank pair again across seeds before treating rank as a lever."
                ),
            );
        }
    }
    if nears.len() < 2 {
        return;
    }
    let low_at = lowest_indexes(nears);
    let quiet_at = quiet_indexes(nears);
    let low = &nears[low_at[0]];
    let quiet = &nears[quiet_at[0]];
    let low_names = names_at(nears, &low_at);
    let quiet_names = names_at(nears, &quiet_at);
    if weighing.abstain {
        section(out, "Why no run wins");
        let company = match low.next {
            Some(next) if low.lone_low() => format!(
                "{}'s {} is a lone sample: the next lowest near it is {}, more than twice as high.",
                cap(&english_list(&low_names)),
                format_measure(low.low),
                format_measure(next)
            ),
            Some(next) => format!(
                "{}'s {} is not a lone spike: the next lowest near it is {}.",
                cap(&english_list(&low_names)),
                format_measure(low.low),
                format_measure(next)
            ),
            None => format!(
                "{}'s {} is the only sample in its stretch.",
                cap(&english_list(&low_names)),
                format_measure(low.low)
            ),
        };
        line(
            out,
            &format!(
                "{company} But its stretch is noisier: its middle is {}, against {} for {}.",
                format_measure(low.median),
                format_measure(quiet.median),
                english_list(&quiet_names)
            ),
        );
        line(
            out,
            &format!(
                "{} has the calmer stretch, but its own deepest point is {}, not the deepest. The deeper dip and the calmer stretch belong to different runs, and the report keeps both.",
                cap(&english_list(&quiet_names)),
                format_measure(quiet.low)
            ),
        );
    } else {
        section(out, &format!("Why {} leads", english_list(&low_names)));
        line(
            out,
            &format!(
                "The half-epoch median and the lowest sample are the same {}, {}.",
                if low_names.len() == 1 { "run" } else { "runs" },
                english_list(&low_names)
            ),
        );
        if !reading.quietest_end.is_empty() && !same_names(&reading.quietest_end, &low_names) {
            line(
                out,
                &format!(
                    "The lowest last sample is {}, from {}. That is not the same run as the lowest sample.",
                    last_values(reading, &reading.quietest_end),
                    english_list(&reading.quietest_end)
                ),
            );
        }
    }
    if let Some(text) = final_marker(reading, &low_names) {
        line(out, &text);
    }
    if let Some(text) = decay_prose(nears) {
        line(out, &text);
    }
    if let Some(text) = tangle_sentence(board, nears) {
        line(out, &text);
    }
}

fn changed_and_not(out: &mut String, board: &Board) {
    section(out, "What changed and what did not");
    if seed_only(board) {
        line(out, "Only the seed changed.");
    } else if board.series.len() <= 1 {
        line(
            out,
            "One run cannot show what a different setting would do.",
        );
    } else if !board.varying.is_empty() {
        let labels: Vec<String> = board
            .varying
            .iter()
            .map(|key| recipe_label(key).to_string())
            .collect();
        line(
            out,
            &format!(
                "Changed: {}. Everything below stayed the same.",
                english_list(&labels)
            ),
        );
    }
    if let Some(text) = inventory(board) {
        line(out, &text);
    }
    if let Some(rate) = shared_f64(board, "learning_rate")
        && !varies(board, "learning_rate")
    {
        line(
            out,
            &format!(
                "The learning rate, {}, was the same on every run, so these runs hold no evidence about it either way.",
                format_measure(rate)
            ),
        );
    }
    if !rank_moves(board)
        && !alpha_moves(board)
        && let (Some(alpha), Some(rank)) =
            (shared_f64(board, "lora_alpha"), shared_f64(board, "lora_r"))
        && rank != 0.0
    {
        line(
            out,
            &format!(
                "LoRA scale (alpha/r) is {} on every run (alpha {}, rank {}).",
                format_measure(alpha / rank),
                format_measure(alpha),
                format_measure(rank)
            ),
        );
    }
    if !varies(board, "warmup_steps")
        && let Some(steps) = shared_f64(board, "warmup_steps")
    {
        line(
            out,
            &format!(
                "Warmup is {} steps. The samples are placed by epoch, not by step, so this report does not place warmup on the epoch axis.",
                format_measure(steps)
            ),
        );
    }
    let assumptions = assumption_lines(board);
    if !assumptions.is_empty() {
        line(out, "Assumptions, labeled, not results:");
        for item in assumptions {
            line(out, &format!("* {item}"));
        }
    }
}

fn earlier(out: &mut String, board: &Board, weighing: &Weighing, ledger: &Ledger) {
    if ledger.is_empty() || weighing.neighborhoods.is_empty() {
        return;
    }
    section(out, "Earlier weighings");
    let here = crate::series::fingerprint(board);
    let nears = &weighing.neighborhoods;
    if let Some(before) = &ledger.same_runs {
        let then = before.deepest();
        let now = &nears[lowest_indexes(nears)[0]];
        match then {
            Some(then)
                if then.name == now.name
                    && then.low.to_bits() == now.low.to_bits()
                    && before.runs.len() == nears.len() =>
            {
                line(
                    out,
                    &format!(
                        "These runs were first weighed on {} and their deepest point has not moved since.",
                        before.date
                    ),
                );
            }
            Some(then) => line(
                out,
                &format!(
                    "When these runs were weighed on {}, the deepest point was {} ({}). Now it is {} ({}).",
                    before.date,
                    format_measure(then.low),
                    then.name,
                    format_measure(now.low),
                    now.name
                ),
            ),
            None => {}
        }
    }
    for item in &ledger.same_recipe {
        line(
            out,
            &format!(
                "On {}, other runs of this same recipe: {}",
                item.date,
                summary(item)
            ),
        );
    }
    if !ledger.same_recipe.is_empty()
        && let Some((low, high)) = middles(nears)
    {
        line(
            out,
            &format!(
                "Today's middles run from {} to {}.",
                format_measure(low),
                format_measure(high)
            ),
        );
    }
    for item in &ledger.same_method {
        let differs = recipe_difference(&item.fingerprint, &here);
        let what = if differs.is_empty() {
            String::new()
        } else {
            format!(" ({})", differs.join("; "))
        };
        line(
            out,
            &format!(
                "On {}, a different recipe with the same method{what}: {}",
                item.date,
                summary(item)
            ),
        );
    }
}

/// The shared fields where an earlier recipe differs from this one, as "LoRA rank 32 where this has 16".
fn recipe_difference(earlier: &str, here: &str) -> Vec<String> {
    let parse = |text: &str| match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(object)) => object,
        _ => serde_json::Map::new(),
    };
    let (earlier, here) = (parse(earlier), parse(here));
    let mut keys: Vec<&String> = earlier.keys().chain(here.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|key| key.as_str() != "varying")
        .filter_map(|key| {
            let then = earlier.get(key.as_str());
            let now = here.get(key.as_str());
            if then == now {
                return None;
            }
            let label = recipe_label(key);
            Some(match (then, now) {
                (Some(then), Some(now)) => format!(
                    "{label} {} where this has {}",
                    inventory_text(then),
                    inventory_text(now)
                ),
                (Some(then), None) => {
                    format!("{label} {} shared there, not here", inventory_text(then))
                }
                (None, _) => format!("{label} not shared there"),
            })
        })
        .collect()
}

/// One earlier weighing in a sentence: how many runs, the verdict, the middles, the deepest point.
fn summary(item: &Weighed) -> String {
    let n = item.runs.len();
    let mut text = format!(
        "{} {}",
        count_word(n, false),
        if n == 1 { "run" } else { "runs" }
    );
    if n >= 2 {
        if item.abstain {
            text.push_str(", no winner");
        } else if let Some(lead) = item.deepest() {
            text.push_str(&format!(", {} led", lead.name));
        }
    }
    if let Some((low, high)) = item.middles() {
        if low.to_bits() == high.to_bits() {
            text.push_str(&format!(", middle {}", format_measure(low)));
        } else {
            text.push_str(&format!(
                ", middles from {} to {}",
                format_measure(low),
                format_measure(high)
            ));
        }
    }
    if let Some(deepest) = item.deepest() {
        text.push_str(&format!(
            ", deepest point {} ({})",
            format_measure(deepest.low),
            deepest.name
        ));
    }
    text.push('.');
    text
}

fn middles(nears: &[Neighborhood]) -> Option<(f64, f64)> {
    bounds(&nears.iter().map(|near| near.median).collect::<Vec<_>>())
}

fn next_steps(out: &mut String, board: &Board, weighing: &Weighing, spread: Option<&Spread>) {
    section(out, "What to do next");
    let mut steps = Vec::new();
    let n = board.series.len();
    let nears = &weighing.neighborhoods;
    let separation = spread.map(|spread| spread.separation);
    if n <= 1 {
        steps.push("Open a second run of the same recipe before changing a setting.".to_string());
    } else if seed_only(board) {
        if weighing.abstain {
            steps.push(
                "Changing nothing is a fair choice. No stored measurement here crowns a run, and no setting was tested.".to_string(),
            );
        } else {
            let lead = english_list(&names_at(nears, &lowest_indexes(nears)));
            steps.push(format!(
                "{} leads, but the recipe was not tested, so this is not a reason to change a setting.",
                cap(&lead)
            ));
        }
        steps.push(match separation {
            Some(Separation::InsideNoise) => "If you need a pick, run more seeds of this recipe and compare their middles. One more run cannot settle a difference smaller than the noise inside a single run.".to_string(),
            Some(Separation::Apart) => "The seeds separate on this recipe, so one run of it is a weak sample. Run several seeds of any recipe you want to compare with this one.".to_string(),
            _ => "If you need a pick, run more seeds of this recipe and compare their middles.".to_string(),
        });
    } else if rank_moves(board) || alpha_moves(board) {
        if seeds_differ(board) {
            steps.push(
                "Separate the seed from the rank before treating rank as a lever.".to_string(),
            );
        } else {
            steps.push(
                "Run the same rank pair again across seeds before treating rank as a lever."
                    .to_string(),
            );
        }
    } else {
        let labels: Vec<String> = board
            .varying
            .iter()
            .map(|key| recipe_label(key).to_string())
            .collect();
        steps.push(format!(
            "This folder changes {}. A difference here is not yet a reason to move one setting.",
            if labels.is_empty() {
                "more than the seed".to_string()
            } else {
                english_list(&labels)
            }
        ));
    }
    if n >= 2 {
        steps.push(
            "Compare runs by the middle of the stretch around the low, not by the single deepest point.".to_string(),
        );
    }
    push_checkpoint(&mut steps, board, nears);
    for (index, step) in steps.iter().enumerate() {
        line(out, &format!("{}. {step}", index + 1));
    }
}

fn cannot_tell(out: &mut String, board: &Board) {
    section(out, "What this report cannot tell you");
    let mut parts = Vec::new();
    if !board.shared.is_empty() {
        parts.push("Whether any shared setting is right.".to_string());
    }
    let mut untouched = Vec::new();
    if shared_f64(board, "learning_rate").is_some() && !varies(board, "learning_rate") {
        untouched.push("learning rate");
    }
    if board.shared.contains_key("effective_batch") && !varies(board, "effective_batch") {
        untouched.push("batch");
    }
    if is_cosine(board) {
        untouched.push("schedule");
    }
    if !rank_moves(board) && !alpha_moves(board) && board.shared.contains_key("lora_r") {
        untouched.push("LoRA shape");
    }
    if !untouched.is_empty() {
        let items: Vec<String> = untouched.into_iter().map(str::to_string).collect();
        let because = if items.len() == 1 {
            "That did not change."
        } else {
            "None of those changed."
        };
        parts.push(format!(
            "Whether a different {} would do better. {because}",
            english_or(&items)
        ));
    }
    parts.push("What the saved checkpoint files hold. This report reads the stored samples, not the checkpoints.".to_string());
    parts.push(
        "Anything that needs the web or a cloud model. It uses only stored measurements and the reference catalog inside the program.".to_string(),
    );
    for part in parts {
        line(out, &format!("* {part}"));
    }
}

fn sources(out: &mut String, board: &Board, weighing: &Weighing) {
    let mut keys = Vec::new();
    if seeds_differ(board) {
        keys.push("seed");
    }
    let cosine_used = is_cosine(board)
        && weighing
            .neighborhoods
            .iter()
            .any(|near| near.decayed() == Some(true));
    if cosine_used {
        keys.push("lr_scheduler");
    }
    if scale_discussed(board) {
        keys.push("lora_r");
    }
    if batch_rule(board) {
        keys.push("learning_rate");
    }
    if varies(board, "max_grad_norm") {
        keys.push("max_grad_norm");
    }
    if varies(board, "lora_dropout") {
        keys.push("lora_dropout");
    }
    if varies(board, "weight_decay") {
        keys.push("weight_decay");
    }
    let cited: Vec<_> = keys
        .into_iter()
        .filter_map(|key| weighing.cards.iter().find(|card| card.key == key))
        .collect();
    if cited.is_empty() {
        return;
    }
    section(out, "Where this comes from");
    for card in cited {
        let mut text = format!("* {} {} {}", card.supports, card.cite, card.url);
        if card.key == "lr_scheduler" {
            text.push_str(" This comparison does not measure warm restarts.");
        }
        if card.key == "learning_rate" {
            text.push_str(" It does not say what learning rate to use.");
        }
        line(out, &text);
    }
}

/// "0.0176, from seed 13 at epoch 7" for the deepest point, or the tied names.
fn deepest_clause(nears: &[Neighborhood]) -> Option<String> {
    let indexes = lowest_indexes(nears);
    let first = indexes.first().copied()?;
    let near = &nears[first];
    let names = names_at(nears, &indexes);
    let epoch = if indexes.len() == 1 {
        format!(" at epoch {}", format_measure(near.at))
    } else {
        String::new()
    };
    Some(format!(
        "{}, from {}{epoch}",
        format_measure(near.low),
        english_list(&names)
    ))
}

fn final_winners(reading: &Reading) -> Option<(Vec<String>, f64, f64)> {
    let known: Vec<(&str, f64)> = reading
        .series
        .iter()
        .filter_map(|series| {
            series
                .summary_final
                .map(|value| (series.name.as_str(), value))
        })
        .collect();
    let min = known
        .iter()
        .map(|(_, value)| *value)
        .min_by(|left, right| left.total_cmp(right))?;
    let max = known
        .iter()
        .map(|(_, value)| *value)
        .max_by(|left, right| left.total_cmp(right))?;
    let winners = known
        .iter()
        .filter(|(_, value)| value.to_bits() == min.to_bits())
        .map(|(name, _)| (*name).to_string())
        .collect();
    Some((winners, min, max))
}

fn final_marker(reading: &Reading, low_names: &[String]) -> Option<String> {
    let (winners, min, max) = final_winners(reading)?;
    let hides = winners.iter().all(|name| !low_names.contains(name));
    let points = stored_points(reading);
    if hides {
        let low_loss = reading
            .series
            .iter()
            .find(|series| low_names.iter().any(|name| name == &series.name))
            .and_then(|series| series.low.as_ref())
            .map(|mark| format_measure(mark.loss))
            .unwrap_or_else(|| "the deeper low".to_string());
        let cluster = if min.to_bits() == max.to_bits() {
            String::new()
        } else {
            format!(
                ", the lowest of {} to {}",
                format_measure(min),
                format_measure(max)
            )
        };
        Some(format!(
            "Ranked by training_summary.final_loss ({} for {}{cluster}), {} would be crowned and {}'s {} would disappear from the page, because final_loss is not one of the {points}.",
            format_measure(min),
            english_list(&winners),
            english_list(&winners),
            english_list(low_names),
            low_loss
        ))
    } else {
        Some(format!(
            "training_summary.final_loss is a marker printed beside the curve, not one of the {points}. The lowest marker is {} for {}.",
            format_measure(min),
            english_list(&winners)
        ))
    }
}

fn decay_prose(nears: &[Neighborhood]) -> Option<String> {
    let mut under = Vec::new();
    let mut above = Vec::new();
    for near in nears {
        match near.decayed() {
            Some(true) => under.push(near.name.clone()),
            Some(false) => above.push(near.name.clone()),
            None => {}
        }
    }
    if under.is_empty() && above.is_empty() {
        return None;
    }
    if above.is_empty() && under.len() == nears.len() {
        return Some(
            "Every deepest point came after the learning rate had decayed under a twentieth of its peak.".to_string(),
        );
    }
    if under.is_empty() {
        return Some(
            "No deepest point came after the learning rate had decayed under a twentieth of its peak."
                .to_string(),
        );
    }
    let under_verb = if under.len() == 1 { "was" } else { "were" };
    let above_verb = if above.is_empty() {
        String::new()
    } else {
        format!(
            " {} reached {} while the rate was still above that.",
            cap(&english_list(&above)),
            if above.len() == 1 {
                "its deepest point"
            } else {
                "their deepest points"
            }
        )
    };
    Some(format!(
        "The deepest points of {} {under_verb} reached after the learning rate had decayed under a twentieth of its peak.{above_verb}",
        english_list(&under)
    ))
}

fn tangle_sentence(board: &Board, nears: &[Neighborhood]) -> Option<String> {
    let decayed = nears.iter().any(|near| near.decayed() == Some(true));
    if !is_cosine(board) || !decayed {
        return None;
    }
    let span = match shared_f64(board, "epochs") {
        Some(epochs) => format!(" for the same {} epochs", format_measure(epochs)),
        None => String::new(),
    };
    Some(format!(
        "Every run used the same cosine schedule{span}, so a late low and the schedule running out happen together here, and this comparison cannot tell them apart."
    ))
}

fn assumption_lines(board: &Board) -> Vec<String> {
    let mut lines = Vec::new();
    let rate_shared =
        shared_f64(board, "learning_rate").is_some() && !varies(board, "learning_rate");
    if rank_moves(board) && !alpha_moves(board) && rate_shared {
        if let Some(text) = rank_scale_assumption(board) {
            lines.push(text);
        }
    } else if alpha_moves(board) && !rank_moves(board) && rate_shared {
        lines.push(
            "The catalog states a formula (scale = alpha/r). It does not state how an alpha change changes loss. This report assumes the scale change acts like a learning-rate change on the adapter. That is not measured here.".to_string(),
        );
    } else if scale_discussed(board) && !rank_moves(board) && !alpha_moves(board) && rate_shared {
        lines.push(
            "Alpha can stand in for the learning rate only when the initialization is scaled. Neither alpha nor rank moved here, so nothing rests on this.".to_string(),
        );
    }
    if batch_rule(board) {
        let batch = shared_f64(board, "effective_batch")
            .map(format_measure)
            .unwrap_or_else(|| "the same value".to_string());
        lines.push(format!(
            "The batch rule (multiply the batch by k, multiply the learning rate by k) was not applied: the effective batch stayed at {batch} on every run."
        ));
    }
    lines
}

fn rank_scale_assumption(board: &Board) -> Option<String> {
    let alpha = shared_alpha(board)?;
    let mut scales = Vec::new();
    for series in &board.series {
        let Some(rank) = series_f64(series, "lora_r")
            .or_else(|| shared_f64(board, "lora_r"))
            .filter(|rank| *rank != 0.0)
        else {
            continue;
        };
        let text = format_measure(alpha / rank);
        if !scales.iter().any(|item: &String| item == &text) {
            scales.push(text);
        }
    }
    let change = match scales.as_slice() {
        [from, to] => format!("from {from} to {to}"),
        [only] => format!("at {only}"),
        _ => format!("across {}", english_list(&scales)),
    };
    Some(format!(
        "The catalog states a formula (scale = alpha/r). It does not state how a rank change changes loss. With alpha unchanged, changing rank moves alpha/r {change}, and this report assumes that acts like a learning-rate change on the adapter. That is not measured here."
    ))
}

fn rank_detail(board: &Board) -> String {
    let mut parts = Vec::new();
    for series in &board.series {
        let rank = series_f64(series, "lora_r").or_else(|| shared_f64(board, "lora_r"));
        let alpha = series_f64(series, "lora_alpha").or_else(|| shared_f64(board, "lora_alpha"));
        match (rank, alpha) {
            (Some(rank), Some(alpha)) if rank != 0.0 => parts.push(format!(
                "{} rank {} (scale {})",
                series.name,
                format_measure(rank),
                format_measure(alpha / rank)
            )),
            (Some(rank), _) => parts.push(format!(
                "{} rank {} (no alpha in its recipe, so no scale)",
                series.name,
                format_measure(rank)
            )),
            (None, Some(alpha)) => {
                parts.push(format!("{} alpha {}", series.name, format_measure(alpha)))
            }
            _ => parts.push(format!("{} has no LoRA rank in the recipe", series.name)),
        }
    }
    format!("Under test: {}.", parts.join("; "))
}

fn inventory(board: &Board) -> Option<String> {
    let mut parts = Vec::new();
    for key in recipe_keys(&board.shared) {
        let Some(value) = board.shared.get(key) else {
            continue;
        };
        parts.push(format!("{} {}", recipe_label(key), inventory_text(value)));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("Shared by every run: {}.", parts.join(", ")))
}

fn inventory_text(value: &Value) -> String {
    match value {
        Value::Array(items) if !items.is_empty() => {
            items.iter().map(recipe_text).collect::<Vec<_>>().join(", ")
        }
        other => recipe_text(other),
    }
}

fn push_checkpoint(steps: &mut Vec<String>, board: &Board, nears: &[Neighborhood]) {
    let Some(epochs) = checkpoints(board) else {
        return;
    };
    if nears.is_empty() || !checkpoint_misses_lows(&epochs, nears) {
        return;
    }
    let (min_at, max_at) =
        bounds(&nears.iter().map(|near| near.at).collect::<Vec<_>>()).expect("nears");
    let when = if (min_at - max_at).abs() < 1e-9 {
        format!("at epoch {}", format_measure(min_at))
    } else {
        format!(
            "between epochs {} and {}",
            format_measure(min_at),
            format_measure(max_at)
        )
    };
    let list = english_list(
        &epochs
            .iter()
            .copied()
            .map(format_measure)
            .collect::<Vec<_>>(),
    );
    steps.push(format!(
        "A record-keeping note, not a recipe change: every deepest point fell {when}, and checkpoints were saved at epochs {list}, so no saved checkpoint holds the weights from those moments. Save one in that band next time if you want to keep them. Nothing here says that changes any loss."
    ));
}

/// Capitalize the first letter, for a run name that starts a sentence.
fn cap(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn loss_span_sentence(reading: &Reading) -> Option<String> {
    let firsts: Vec<f64> = reading
        .series
        .iter()
        .filter_map(|series| series.first.as_ref().map(|mark| mark.loss))
        .collect();
    let lasts: Vec<f64> = reading
        .series
        .iter()
        .filter_map(|series| series.last.as_ref().map(|mark| mark.loss))
        .collect();
    let (start_lo, start_hi) = bounds(&firsts)?;
    let (end_lo, end_hi) = bounds(&lasts)?;
    let start = if start_lo.to_bits() == start_hi.to_bits() {
        format!("Loss started at {}", format_measure(start_lo))
    } else {
        format!(
            "Loss started between {} and {}",
            format_measure(start_lo),
            format_measure(start_hi)
        )
    };
    let end = if end_lo.to_bits() == end_hi.to_bits() {
        format!("ended at {}", format_measure(end_lo))
    } else {
        format!(
            "ended between {} and {}",
            format_measure(end_lo),
            format_measure(end_hi)
        )
    };
    Some(format!("{start} and {end}."))
}

fn climb_sentence(reading: &Reading, nears: &[Neighborhood]) -> Option<String> {
    if nears.is_empty() {
        return None;
    }
    let exceptions: Vec<String> = reading
        .series
        .iter()
        .filter(|series| series.low.is_some() && !series.climbed)
        .map(|series| series.name.clone())
        .collect();
    let min_at = nears
        .iter()
        .map(|near| near.at)
        .min_by(|left, right| left.total_cmp(right))?;
    let max_at = nears
        .iter()
        .map(|near| near.at)
        .max_by(|left, right| left.total_cmp(right))?;
    let when = if (min_at - max_at).abs() < 1e-9 {
        format!("at epoch {}", format_measure(min_at))
    } else {
        format!(
            "between epochs {} and {}",
            format_measure(min_at),
            format_measure(max_at)
        )
    };
    if exceptions.is_empty() {
        Some(format!(
            "Every run reached its lowest stored point {when}, then climbed a little by the end."
        ))
    } else {
        Some(format!(
            "The lowest stored points fell {when}. {} did not climb after the low.",
            cap(&english_list(&exceptions))
        ))
    }
}

fn samples_clause(board: &Board) -> String {
    let counts: Vec<usize> = board
        .series
        .iter()
        .map(|series| series.samples.len())
        .collect();
    let min = counts.iter().copied().min().unwrap_or(0);
    let max = counts.iter().copied().max().unwrap_or(0);
    if min == max {
        format!(
            "Each run left {min} stored measurements of loss (the score being watched; lower is better) and learning rate."
        )
    } else {
        format!(
            "The runs left between {min} and {max} stored measurements of loss (the score being watched; lower is better) and learning rate."
        )
    }
}

fn span_clause(board: &Board) -> Option<String> {
    let (min_x, max_x) = axis_bounds(board)?;
    let from = if min_x > 0.0 && min_x < 0.5 {
        "from just after epoch 0".to_string()
    } else {
        format!("from epoch {}", format_measure(min_x))
    };
    Some(format!(
        "They are spread {from} to epoch {}.",
        format_measure(max_x)
    ))
}

fn gap_sentence(reading: &Reading) -> Option<String> {
    let gaps: usize = reading.series.iter().map(|series| series.gaps).sum();
    let unplaced: usize = reading.series.iter().map(|series| series.unplaced).sum();
    if gaps == 0 && unplaced == 0 {
        return None;
    }
    let mut text = String::new();
    if gaps > 0 {
        text.push_str("Some stored records have no finite loss. The chart leaves those as gaps.");
    }
    if unplaced > 0 {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str("Some records have no epoch or step, so they are not placed on the axis.");
    }
    Some(text)
}

fn last_values(reading: &Reading, names: &[String]) -> String {
    let mut values = Vec::new();
    for name in names {
        let Some(series) = reading.series.iter().find(|series| &series.name == name) else {
            continue;
        };
        let Some(last) = &series.last else {
            continue;
        };
        let text = format_measure(last.loss);
        if !values.contains(&text) {
            values.push(text);
        }
    }
    english_list(&values)
}

fn stored_points(reading: &Reading) -> String {
    let counts: Vec<usize> = reading.series.iter().map(|series| series.samples).collect();
    let min = counts.iter().copied().min().unwrap_or(0);
    let max = counts.iter().copied().max().unwrap_or(0);
    if min == max {
        format!("{min} stored points")
    } else {
        "stored points".to_string()
    }
}

fn same_names(left: &[String], right: &[String]) -> bool {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort();
    right.sort();
    left == right
}

fn checkpoints(board: &Board) -> Option<Vec<f64>> {
    if varies(board, "checkpoint_epochs") {
        return None;
    }
    let value = board.shared.get("checkpoint_epochs")?;
    let epochs = match value {
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_f64)
            .filter(|epoch| epoch.is_finite())
            .collect::<Vec<_>>(),
        Value::Number(number) => number.as_f64().into_iter().collect(),
        _ => Vec::new(),
    };
    if epochs.is_empty() {
        None
    } else {
        Some(epochs)
    }
}

fn checkpoint_misses_lows(epochs: &[f64], nears: &[Neighborhood]) -> bool {
    let Some(min_at) = nears
        .iter()
        .map(|near| near.at)
        .min_by(|left, right| left.total_cmp(right))
    else {
        return false;
    };
    let Some(max_at) = nears
        .iter()
        .map(|near| near.at)
        .max_by(|left, right| left.total_cmp(right))
    else {
        return false;
    };
    !epochs
        .iter()
        .any(|epoch| *epoch >= min_at - 1e-9 && *epoch <= max_at + 1e-9)
}

fn axis_bounds(board: &Board) -> Option<(f64, f64)> {
    let mut values = Vec::new();
    for series in &board.series {
        for sample in &series.samples {
            if let Some(x) = sample.x.filter(|x| x.is_finite()) {
                values.push(x);
            }
        }
    }
    bounds(&values)
}

fn bounds(values: &[f64]) -> Option<(f64, f64)> {
    let min = values
        .iter()
        .copied()
        .min_by(|left, right| left.total_cmp(right))?;
    let max = values
        .iter()
        .copied()
        .max_by(|left, right| left.total_cmp(right))?;
    Some((min, max))
}

fn discussion_order(nears: &[Neighborhood]) -> Vec<usize> {
    let mut order = lowest_indexes(nears);
    for index in quiet_indexes(nears) {
        if !order.contains(&index) {
            order.push(index);
        }
    }
    for index in 0..nears.len() {
        if !order.contains(&index) {
            order.push(index);
        }
    }
    order
}

fn lowest_indexes(nears: &[Neighborhood]) -> Vec<usize> {
    let Some(best) = nears
        .iter()
        .map(|near| near.low)
        .min_by(|left, right| left.total_cmp(right))
    else {
        return Vec::new();
    };
    nears
        .iter()
        .enumerate()
        .filter(|(_, near)| near.low.to_bits() == best.to_bits())
        .map(|(index, _)| index)
        .collect()
}

fn quiet_indexes(nears: &[Neighborhood]) -> Vec<usize> {
    let Some(best) = nears
        .iter()
        .map(|near| near.median)
        .min_by(|left, right| left.total_cmp(right))
    else {
        return Vec::new();
    };
    nears
        .iter()
        .enumerate()
        .filter(|(_, near)| near.median.to_bits() == best.to_bits())
        .map(|(index, _)| index)
        .collect()
}

fn names_at(nears: &[Neighborhood], indexes: &[usize]) -> Vec<String> {
    indexes
        .iter()
        .map(|index| nears[*index].name.clone())
        .collect()
}

fn seed_only(board: &Board) -> bool {
    seeds_differ(board) && board.varying.is_empty()
}

fn seeds_differ(board: &Board) -> bool {
    let mut seen: Vec<Option<i64>> = board.series.iter().map(|series| series.seed).collect();
    seen.sort();
    seen.dedup();
    seen.len() > 1
}

fn lever_name(board: &Board) -> String {
    if seeds_differ(board) && board.varying.is_empty() {
        "the seed".to_string()
    } else if board.varying.len() == 1 {
        recipe_label(&board.varying[0]).to_string()
    } else if board.varying.is_empty() {
        "nothing in the recipe or the seed".to_string()
    } else {
        let labels: Vec<String> = board
            .varying
            .iter()
            .map(|key| recipe_label(key).to_string())
            .collect();
        english_list(&labels)
    }
}

fn varies(board: &Board, key: &str) -> bool {
    board.varying.iter().any(|item| item == key)
}

fn rank_moves(board: &Board) -> bool {
    varies(board, "lora_r")
}

fn alpha_moves(board: &Board) -> bool {
    varies(board, "lora_alpha")
}

fn scale_discussed(board: &Board) -> bool {
    rank_moves(board)
        || alpha_moves(board)
        || (shared_f64(board, "lora_alpha").is_some()
            && shared_f64(board, "lora_r").is_some_and(|rank| rank != 0.0))
}

fn batch_rule(board: &Board) -> bool {
    board.shared.contains_key("effective_batch")
        && shared_f64(board, "learning_rate").is_some()
        && !varies(board, "effective_batch")
        && !varies(board, "learning_rate")
}

fn is_cosine(board: &Board) -> bool {
    !varies(board, "lr_scheduler")
        && board
            .shared
            .get("lr_scheduler")
            .and_then(Value::as_str)
            .is_some_and(|text| text.to_ascii_lowercase().contains("cosine"))
}

fn shared_f64(board: &Board, key: &str) -> Option<f64> {
    board
        .shared
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn series_f64(series: &Series, key: &str) -> Option<f64> {
    series
        .recipe
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn shared_alpha(board: &Board) -> Option<f64> {
    if let Some(alpha) = shared_f64(board, "lora_alpha") {
        return Some(alpha);
    }
    let mut values = Vec::new();
    for series in &board.series {
        values.push(series_f64(series, "lora_alpha")?);
    }
    let first = *values.first()?;
    if values
        .iter()
        .all(|value| value.to_bits() == first.to_bits())
    {
        Some(first)
    } else {
        None
    }
}

fn count_word(n: usize, cap: bool) -> String {
    let word = match n {
        0 => "no",
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        10 => "ten",
        _ => return n.to_string(),
    };
    if !cap {
        return word.to_string();
    }
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn english_or(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} or {second}"),
        _ => {
            let last = items.len() - 1;
            format!("{}, or {}", items[..last].join(", "), items[last])
        }
    }
}

fn english_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let last = items.len() - 1;
            format!("{}, and {}", items[..last].join(", "), items[last])
        }
    }
}

fn words_of(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            word.push(ch);
        } else if !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

fn line(out: &mut String, text: &str) {
    out.push_str(text);
    out.push('\n');
}

fn section(out: &mut String, title: &str) {
    if !out.is_empty() && !out.ends_with("\n\n") {
        out.push('\n');
    }
    line(out, title);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::series::Sample;
    use serde_json::Map;

    fn point(x: f64, loss: f64, lr: f64) -> Sample {
        Sample {
            x: Some(x),
            loss: Some(loss),
            lr: Some(lr),
            extra: Map::new(),
        }
    }

    fn run(
        name: &str,
        seed: i64,
        samples: Vec<Sample>,
        recipe: Map<String, Value>,
        final_loss: f64,
    ) -> Series {
        let mut summary = Map::new();
        summary.insert("final_loss".to_string(), Value::from(final_loss));
        Series {
            name: name.to_string(),
            seed: Some(seed),
            model: String::new(),
            file_name: format!("{name}.json"),
            samples,
            recipe,
            summary,
        }
    }

    fn shared_recipe() -> Map<String, Value> {
        let mut recipe = Map::new();
        recipe.insert("method".to_string(), Value::from("bf16 LoRA"));
        recipe.insert("learning_rate".to_string(), Value::from(0.00015));
        recipe.insert("lr_scheduler".to_string(), Value::from("cosine"));
        recipe.insert("warmup_steps".to_string(), Value::from(10));
        recipe.insert("weight_decay".to_string(), Value::from(0.0));
        recipe.insert("max_grad_norm".to_string(), Value::from(1.0));
        recipe.insert("effective_batch".to_string(), Value::from(8));
        recipe.insert("epochs".to_string(), Value::from(8));
        recipe.insert("lora_r".to_string(), Value::from(16));
        recipe.insert("lora_alpha".to_string(), Value::from(32));
        recipe.insert("lora_dropout".to_string(), Value::from(0.1));
        recipe.insert(
            "checkpoint_epochs".to_string(),
            Value::Array(vec![Value::from(2), Value::from(4), Value::from(8)]),
        );
        recipe
    }

    fn abstain_board() -> Board {
        let recipe = shared_recipe();
        let low = run(
            "seed 13",
            13,
            vec![
                point(0.0, 12.0, 0.00015),
                point(6.7, 0.10, 0.00002),
                point(6.8, 0.03, 0.00001),
                point(7.0, 0.02, 0.000001),
                point(7.3, 0.12, 0.000001),
                point(8.0, 0.08, 0.0000001),
            ],
            recipe.clone(),
            0.7123,
        );
        let quiet = run(
            "seed 1024",
            1024,
            vec![
                point(0.0, 9.0, 0.00015),
                point(7.2, 0.05, 0.000001),
                point(7.5, 0.04, 0.000001),
                point(7.8, 0.06, 0.000001),
                point(8.0, 0.047, 0.0000001),
            ],
            recipe.clone(),
            0.6911,
        );
        let early = run(
            "seed 271",
            271,
            vec![
                point(0.0, 10.0, 0.00015),
                point(6.0, 0.055, 0.00002),
                point(6.3, 0.03, 0.00002),
                point(6.5, 0.12, 0.00002),
                point(6.6, 0.14, 0.00002),
                point(8.0, 0.07, 0.000001),
            ],
            recipe.clone(),
            0.72,
        );
        Board {
            series: vec![low, quiet, early],
            skipped: 0,
            shared: recipe,
            varying: Vec::new(),
        }
    }

    #[test]
    fn utc_days_start_at_the_unix_epoch() {
        assert_eq!(utc_date(0), "1970-01-01");
        assert_eq!(utc_date(86_400), "1970-01-02");
    }

    #[test]
    fn the_fence_keeps_digits_knobs_and_verdicts_out() {
        assert!(orientation_allowed(
            "A seed is the starting draw of randomness."
        ));
        assert!(orientation_allowed(""));
        assert!(orientation_allowed("I wonder what a loss curve is."));
        assert!(!orientation_allowed("Seed 13 won"));
        assert!(!orientation_allowed(
            "You should try a higher learning rate"
        ));
        assert!(!orientation_allowed("Pick a winner next time"));
        assert!(!orientation_allowed("# How to read\nA curve falls."));
        assert!(!orientation_allowed("Use **this** curve."));
    }

    #[test]
    fn an_abstaining_seed_report_leads_with_the_answer_and_keeps_both_facts() {
        let board = abstain_board();
        assert_eq!(
            report_file_name(&board, Some("2026-10-05")),
            "runforge-report-2026-10-05-3-runs.txt"
        );
        let report = comparison_report(&board, Some("2026-10-05"));
        assert!(report.contains("Written 2026-10-05 UTC."));
        let short = report.find("In short").unwrap();
        assert!(short < report.find("The runs").unwrap());
        assert!(report[short..].starts_with(
            "In short\nNo run wins. Seed 13 has the deepest single point, 0.02 at epoch 7."
        ));
        assert!(report.contains("Seed 1024 has the calmest stretch"));
        assert!(report.contains("seed 1024 would be crowned"));
        assert!(report.contains("Seed 271 reached its deepest point"));
        assert!(report.contains("these runs say nothing about any setting"));
        assert!(report.contains("would disappear"));
        assert!(report.contains("still above a twentieth"));
        assert!(report.contains("middle half from"));
        assert!(report.contains("LoRA scale (alpha/r) is 2"));
        assert!(report.contains("no evidence about it"));
        assert!(report.contains("does not place warmup on the epoch axis"));
        assert!(report.contains("was not applied"));
        assert!(report.contains("no saved checkpoint"));
        assert!(report.contains("2002.06305"));
        assert!(report.contains("1608.03983"));
        assert!(report.contains("2106.09685"));
        assert!(report.contains("1706.02677"));
        assert!(!report.contains("1211.5063"));
        assert!(!report.contains("srivastava14a"));
        assert!(!report.contains("1711.05101"));
        assert!(!report.contains("placeholder"));
        assert!(!report.contains("texture"));
        assert!(!report.contains("Earlier weighings"));
        assert!(!report.contains("**"));
        assert!(report.lines().all(|line| !line.starts_with('#')));
        assert!(!report.contains("move the learning rate"));
        assert!(!report.contains('\\'));
        let omission = orientation_omission(&board);
        assert!(omission.contains("did not pick a winner"));
        assert!(omission.contains("0.02"));
    }

    #[test]
    fn headings_are_short_unpunctuated_lines() {
        let report = comparison_report(&abstain_board(), Some("2026-10-05"));
        let headings: Vec<&str> = report.lines().filter(|line| is_heading(line)).collect();
        assert_eq!(
            headings,
            vec![
                "RunForge report",
                "In short",
                "What happened",
                "The runs",
                "Seed 13",
                "Seed 1024",
                "Seed 271",
                "Why no run wins",
                "What changed and what did not",
                "What to do next",
                "What this report cannot tell you",
                "Where this comes from",
            ]
        );
    }

    #[test]
    fn the_spread_sentence_follows_the_measured_separation() {
        let inside = Spread {
            gap: 0.01,
            narrowest: 0.05,
            widest: 0.08,
            separation: Separation::InsideNoise,
        };
        assert!(spread_sentence(&inside).contains("smaller than the noise inside a single run"));
        let apart = Spread {
            gap: 0.2,
            narrowest: 0.05,
            widest: 0.08,
            separation: Separation::Apart,
        };
        assert!(spread_sentence(&apart).contains("larger than the noise inside a single run"));
        let partial = Spread {
            gap: 0.06,
            narrowest: 0.05,
            widest: 0.08,
            separation: Separation::Partial,
        };
        assert!(spread_sentence(&partial).contains("only partly separate"));
    }

    #[test]
    fn earlier_weighings_set_this_board_beside_the_ledger() {
        let board = abstain_board();
        let mut earlier = crate::ledger::weighed_now(&board, "2026-09-01");
        for run in &mut earlier.runs {
            run.name = format!("{} (old)", run.name);
            run.median += 0.5;
        }
        let ledger = Ledger {
            same_runs: Some(crate::ledger::weighed_now(&board, "2026-10-01")),
            same_recipe: vec![earlier],
            same_method: Vec::new(),
        };
        let report = comparison_report_with(&board, Some("2026-10-05"), &ledger);
        assert!(report.contains("Earlier weighings"));
        assert!(report.contains("first weighed on 2026-10-01"));
        assert!(
            report.contains("On 2026-09-01, other runs of this same recipe: three runs, no winner")
        );
        assert!(report.contains("Today's middles run from"));
        assert!(
            report.find("Earlier weighings").unwrap() < report.find("What to do next").unwrap()
        );
    }

    #[test]
    fn a_rank_change_across_seeds_is_a_confound_not_a_result() {
        let mut shared = shared_recipe();
        shared.remove("lora_r");
        let mut rank16 = shared.clone();
        rank16.insert("lora_r".to_string(), Value::from(16));
        let mut rank32 = shared.clone();
        rank32.insert("lora_r".to_string(), Value::from(32));
        let left = run(
            "seed 13",
            13,
            vec![point(7.0, 0.02, 0.000001), point(8.0, 0.05, 0.0)],
            rank16,
            0.7,
        );
        let right = run(
            "seed 1024",
            1024,
            vec![point(7.0, 0.04, 0.000001), point(8.0, 0.06, 0.0)],
            rank32,
            0.7,
        );
        let board = Board {
            series: vec![left, right],
            skipped: 0,
            shared,
            varying: vec!["lora_r".to_string()],
        };
        let report = comparison_report(&board, None);
        assert!(report.contains("The two runs differ in rank and in seed"));
        assert!(report.contains("attributed to the rank alone"));
        assert!(report.contains("differ in the seed as well"));
        assert!(report.contains("scale 2"));
        assert!(report.contains("scale 1"));
        assert!(report.contains("from 2 to 1"));
        assert!(!report.contains("placeholder"));
        assert!(!report.contains("say nothing about any setting"));
        assert!(!report.contains("LoRA scale (alpha/r) is"));
        assert!(!report.contains("**"));
    }

    #[test]
    fn the_same_seed_does_not_credit_a_rank_change() {
        let mut shared = shared_recipe();
        shared.remove("lora_r");
        let mut rank16 = shared.clone();
        rank16.insert("lora_r".to_string(), Value::from(16));
        let mut rank32 = shared.clone();
        rank32.insert("lora_r".to_string(), Value::from(32));
        let left = run("rank 16", 13, vec![point(7.0, 0.02, 0.000001)], rank16, 0.7);
        let right = run("rank 32", 13, vec![point(7.0, 0.04, 0.000001)], rank32, 0.7);
        let board = Board {
            series: vec![left, right],
            skipped: 0,
            shared,
            varying: vec!["lora_r".to_string()],
        };
        let report = comparison_report(&board, None);
        assert!(report.contains("does not credit the rank change"));
        assert!(report.contains("on one seed"));
        assert!(!report.contains("differ in rank and in seed"));
        assert!(!report.contains("say nothing about any setting"));
    }
}
