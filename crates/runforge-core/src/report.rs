//! The comparison report. One string, two renderings.
//!
//! The window shows this text. Save writes this text. A number, a seed, a knob,
//! or a verdict in the body is filled from the board. The model does not write it.

use serde_json::Value;

use crate::series::{
    Board, Reading, Series, format_measure, read_board, recipe_keys, recipe_label, recipe_text,
};
use crate::weigh::{Neighborhood, Weighing, weigh};

/// Printed when a fresh orientation note crosses the fence.
pub const ORIENTATION_OMITTED: &str = "Assistant note omitted: it disagreed with the measurements. The measured lines stand on their own.";

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

/// The report for this board. `written_on` is the UTC day the file is written.
///
/// It is not a measurement date. The samples do not carry one.
pub fn comparison_report(board: &Board, written_on: Option<&str>) -> String {
    let reading = read_board(board);
    let weighing = weigh(board);
    let mut out = String::new();
    title(&mut out, board, written_on);
    what_this_is(&mut out, board, &reading);
    what_happened(&mut out, &reading, &weighing);
    lowest_point(&mut out, &weighing);
    argument(&mut out, board, &reading, &weighing);
    not_tested(&mut out, board);
    next_steps(&mut out, board, &weighing);
    cannot_tell(&mut out, board);
    sources(&mut out, board, &weighing);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
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
    match lowest_sentence(&weighing.neighborhoods) {
        Some(line) => format!("{ORIENTATION_OMITTED} {line} {status}"),
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
    line(out, "RunForge comparison report");
    let n = board.series.len();
    let subtitle = if n <= 1 {
        "One run.".to_string()
    } else if seed_only(board) {
        format!("{} runs, one recipe.", count_word(n, true))
    } else {
        format!("{} runs.", count_word(n, true))
    };
    line(out, &subtitle);
    if let Some(date) = written_on.map(str::trim).filter(|date| !date.is_empty()) {
        line(out, &format!("Report written {date} UTC."));
    }
}

fn what_this_is(out: &mut String, board: &Board, reading: &Reading) {
    section(out, "WHAT THIS IS");
    let n = board.series.len();
    if n <= 1 {
        line(
            out,
            "One training run. There is no second run, so this report does not name a winner.",
        );
    } else if seed_only(board) {
        let count = count_word(n, true);
        let mut text = format!(
            "{count} training runs with an identical recipe. The only thing that differs between them is the seed, the starting draw of randomness. {}",
            samples_clause(board)
        );
        if let Some(span) = span_clause(board) {
            text.push(' ');
            text.push_str(&span);
        }
        text.push_str(" Comparing identical recipes with different seeds is a standard way to see how much of a result is the recipe and how much is luck of the draw.");
        line(out, &text);
    } else if rank_moves(board) || alpha_moves(board) {
        line(out, &rank_intro(board));
    } else {
        let lever = lever_name(board);
        line(
            out,
            &format!(
                "{} training runs. The lever in this folder is {lever}. {}",
                count_word(n, true),
                samples_clause(board)
            ),
        );
    }
    if let Some(gaps) = gap_sentence(reading) {
        line(out, &gaps);
    }
}

fn what_happened(out: &mut String, reading: &Reading, weighing: &Weighing) {
    section(out, "WHAT HAPPENED");
    if let Some(text) = loss_span_sentence(reading) {
        line(out, &text);
    }
    if let Some(text) = climb_sentence(reading, &weighing.neighborhoods) {
        line(out, &text);
    }
    if weighing.neighborhoods.is_empty() {
        line(
            out,
            "No stored sample has a finite loss, so this report does not describe a curve.",
        );
    }
}

fn lowest_point(out: &mut String, weighing: &Weighing) {
    section(out, "THE SINGLE LOWEST POINT");
    if let Some(text) = lowest_sentence(&weighing.neighborhoods) {
        line(out, &text);
    } else {
        line(
            out,
            "No stored sample has a finite loss, so this report does not name a low.",
        );
    }
}

fn argument(out: &mut String, board: &Board, reading: &Reading, weighing: &Weighing) {
    let nears = &weighing.neighborhoods;
    if nears.len() >= 2 && (rank_moves(board) || alpha_moves(board)) {
        section(out, "THE LEVER UNDER TEST");
        line(out, &rank_detail(board));
        line(
            out,
            "[placeholder: a spread measure for the neighborhood window is not computed today]",
        );
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
            line(
                out,
                "This report does not credit the rank change. Run the same rank pair again across seeds before treating rank as a lever.",
            );
        }
    }
    if nears.len() < 2 {
        emit_lines(out, nears, &discussion_order(nears));
        return;
    }
    if weighing.abstain {
        abstain_argument(out, board, reading, weighing);
    } else {
        agree_argument(out, reading, weighing);
    }
    measured_note(out, board, weighing);
}

fn abstain_argument(out: &mut String, board: &Board, reading: &Reading, weighing: &Weighing) {
    let nears = &weighing.neighborhoods;
    let low_at = lowest_indexes(nears);
    let quiet_at = quiet_indexes(nears);
    let low = &nears[low_at[0]];
    let quiet = &nears[quiet_at[0]];
    let low_names = names_at(nears, &low_at);
    let quiet_names = names_at(nears, &quiet_at);
    section(
        out,
        &format!("WHY {} IS NOT THE WINNER", low.name.to_uppercase()),
    );
    line(out, &company_prose(low, nears));
    line(
        out,
        &format!(
            "Across the {} stored samples within half an epoch of the low, the middle value is {}.",
            low.count,
            format_measure(low.median)
        ),
    );
    if let Some(text) = lr_prose(low, is_cosine(board)) {
        line(out, &text);
    }
    emit_lines(out, nears, &low_at);
    section(
        out,
        &format!("WHY {} IS NOT THE WINNER EITHER", quiet.name.to_uppercase()),
    );
    line(out, &quiet_intro(quiet, nears.len(), reading, &quiet_names));
    line(
        out,
        &format!(
            "{}'s own deepest point is {}.",
            english_list(&quiet_names),
            format_measure(quiet.low)
        ),
    );
    if let Some(text) = final_marker(reading, &low_names) {
        line(out, &text);
    }
    emit_lines(out, nears, &quiet_only(&low_at, &quiet_at));
    section(out, "SO: NO WINNER, AND THAT IS AN ANSWER");
    line(
        out,
        &abstain_paragraph(low, quiet, &low_names, &quiet_names, seed_only(board)),
    );
}

fn agree_argument(out: &mut String, reading: &Reading, weighing: &Weighing) {
    let nears = &weighing.neighborhoods;
    let low_at = lowest_indexes(nears);
    let low_names = names_at(nears, &low_at);
    section(out, "WHAT THE WEIGHING SAYS");
    let same = if low_names.len() == 1 {
        format!(
            "The half-epoch median and the lowest sample are the same run, {}.",
            low_names[0]
        )
    } else {
        format!(
            "The half-epoch median and the lowest sample are the same runs, {}.",
            english_list(&low_names)
        )
    };
    if same_names(&reading.quietest_end, &low_names) {
        line(
            out,
            &format!(
                "{same} The lowest last sample belongs to that run as well ({}).",
                last_values(reading, &reading.quietest_end)
            ),
        );
    } else {
        line(out, &same);
        if !reading.quietest_end.is_empty() {
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
    emit_lines(out, nears, &discussion_order(nears));
}

fn measured_note(out: &mut String, board: &Board, weighing: &Weighing) {
    let nears = &weighing.neighborhoods;
    let prose = decay_prose(nears);
    let tangle = tangle_sentence(board, nears);
    let rest = if weighing.abstain {
        rest_indexes(nears)
    } else {
        Vec::new()
    };
    if prose.is_none() && tangle.is_none() && rest.is_empty() {
        return;
    }
    section(out, "ONE MORE MEASURED NOTE");
    if let Some(text) = prose {
        line(out, &text);
    }
    if let Some(text) = tangle {
        line(out, &text);
    }
    emit_lines(out, nears, &rest);
}

fn not_tested(out: &mut String, board: &Board) {
    section(out, "WHAT WAS NOT TESTED");
    if seed_only(board) {
        line(out, "Everything except the seed.");
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
                "Under test: {}. The fields below did not change.",
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
                "Learning rate: {} on all {} runs. Since it never changed, these runs contain no evidence about it. Nothing here is a reason to raise or lower it, and this report declines to say anything about what a different rate would do.",
                format_measure(rate),
                count_word(board.series.len(), false)
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
                "LoRA scale (alpha/r): {} on all runs (alpha {}, rank {}). Listed for the record, not as a result.",
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
                "Warmup: {} steps. A stored sample's epoch is not a step index, so this report does not place warmup on the epoch axis.",
                format_measure(steps)
            ),
        );
    }
    let assumptions = assumption_lines(board);
    if !assumptions.is_empty() {
        line(out, "Assumptions — labeled, not results:");
        for item in assumptions {
            line(out, &format!("* {item}"));
        }
    }
}

fn next_steps(out: &mut String, board: &Board, weighing: &Weighing) {
    section(out, "WHAT TO DO NEXT");
    let mut steps = Vec::new();
    let n = board.series.len();
    let nears = &weighing.neighborhoods;
    if n <= 1 {
        steps.push("Open a second run of the same recipe before changing a setting.".to_string());
    } else if seed_only(board) && weighing.abstain {
        let low = names_at(nears, &lowest_indexes(nears));
        let quiet = names_at(nears, &quiet_indexes(nears));
        steps.push(
            "Changing nothing is a legitimate outcome. The recipe produced stable runs whose differences are mostly seed texture, and no stored measurement here crowns a winner.".to_string(),
        );
        steps.push(format!(
            "If you want a winner, run more seeds of this identical recipe. If {}'s steadiness repeats across more draws, that becomes evidence. If deep one-point dips like {}'s keep appearing and vanishing, that is evidence too. [placeholder: what an additional seed costs on this rig is not in this report]",
            english_list(&quiet),
            english_list(&low)
        ));
        steps.push(
            "Whatever you run next, compare runs by the middle of the half epoch around the low, not by the single lowest point. The single point is where these curves are noisiest. The body is where they can actually be told apart.".to_string(),
        );
    } else if seed_only(board) {
        steps.push(
            "The half-epoch median agrees with the lowest sample. That agreement is the result of this weighing. The shared recipe was still not tested, so this comparison is not a reason to change a setting.".to_string(),
        );
        steps.push(
            "Compare later runs by the middle of the half epoch around the low, not by the single lowest point.".to_string(),
        );
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
        steps.push(
            "Compare runs by the middle of the half epoch around the low, not by the single lowest point.".to_string(),
        );
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
        steps.push(
            "Compare runs by the middle of the half epoch around the low, not by the single lowest point.".to_string(),
        );
    }
    push_checkpoint(&mut steps, board, nears);
    for (index, step) in steps.iter().enumerate() {
        line(out, &format!("{}. {step}", index + 1));
    }
}

fn cannot_tell(out: &mut String, board: &Board) {
    section(out, "WHAT THIS REPORT CANNOT TELL YOU");
    let mut parts = Vec::new();
    if !board.shared.is_empty() {
        parts.push("whether any shared setting is right".to_string());
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
            "because that did not change"
        } else {
            "because none of those changed"
        };
        parts.push(format!(
            "whether a different {} would do better, {because}",
            english_or(&items)
        ));
    }
    parts.push(
        "which saved checkpoint file to keep [placeholder: checkpoint contents are not in this report]".to_string(),
    );
    parts.push(
        "anything that needs the web or a cloud model. This page uses only stored measurements and the reference catalog inside the program".to_string(),
    );
    line(
        out,
        &format!("This report cannot tell you {}.", english_list(&parts)),
    );
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
    let mut cited = Vec::new();
    for key in keys {
        if let Some(card) = weighing.cards.iter().find(|card| card.key == key) {
            cited.push((key, format!("* {} — {}", card.cite, card.url)));
        }
    }
    if cited.is_empty() {
        return;
    }
    section(out, "WHERE THIS COMES FROM");
    for (key, text) in cited {
        line(out, &text);
        if key == "lr_scheduler" {
            line(
                out,
                "The cosine card is the decay inside the run. This comparison does not measure warm restarts.",
            );
        }
        if key == "learning_rate" {
            line(
                out,
                "That card is cited to name a rule that was not applied. It does not say what learning rate to use.",
            );
        }
    }
}

fn lowest_sentence(nears: &[Neighborhood]) -> Option<String> {
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
        "The lowest loss any run recorded is {}, from {}{epoch}. That point happened, so it stays on the chart and in this report, whether or not it wins anything.",
        format_measure(near.low),
        english_list(&names)
    ))
}

fn company_prose(near: &Neighborhood, nears: &[Neighborhood]) -> String {
    let mut text = match near.next {
        Some(next) if near.lone_low() => format!(
            "The low is a single sample: the next-lowest point nearby is {}, more than twice this low.",
            format_measure(next)
        ),
        Some(next) => format!(
            "The next-lowest point nearby is {}, within twice its value.",
            format_measure(next)
        ),
        None => "The window around that low holds only that point.".to_string(),
    };
    let all_have_next = nears.iter().all(|item| item.next.is_some());
    if all_have_next && !nears.iter().any(Neighborhood::lone_low) {
        text.push_str(" None of the lows fails that company test.");
    }
    text
}

fn lr_prose(near: &Neighborhood, cosine: bool) -> Option<String> {
    let flag = near.decayed()?;
    let place = if flag {
        "under a twentieth"
    } else {
        "still above a twentieth"
    };
    let mut text = format!(
        "When {} reached {} its learning rate was {}, {place} of its peak of {}.",
        near.name,
        format_measure(near.low),
        format_measure(near.lr?),
        format_measure(near.lr_max?)
    );
    let far = near.low > 0.0 && near.median > near.low * 2.0;
    if flag && cosine && far {
        text.push_str(" The run was at the tail of its cosine schedule. A point this far from the middle of its neighborhood, reached while the schedule is idling, is texture, not a place the run settled.");
    }
    Some(text)
}

fn quiet_intro(
    quiet: &Neighborhood,
    n: usize,
    reading: &Reading,
    quiet_names: &[String],
) -> String {
    let mut text = format!(
        "{} has the calmest neighborhood of the {}: a middle value of {} across {} samples",
        english_list(quiet_names),
        count_word(n, false),
        format_measure(quiet.median),
        quiet.count
    );
    if same_names(&reading.quietest_end, quiet_names) {
        text.push_str(&format!(
            ", and the lowest last sample ({})",
            last_values(reading, quiet_names)
        ));
    }
    text.push('.');
    if !same_names(&reading.quietest_end, quiet_names) && !reading.quietest_end.is_empty() {
        text.push_str(&format!(
            " The lowest last sample is {}, from {}. That is a different run from the calmest neighborhood.",
            last_values(reading, &reading.quietest_end),
            english_list(&reading.quietest_end)
        ));
    }
    text
}

fn abstain_paragraph(
    low: &Neighborhood,
    quiet: &Neighborhood,
    low_names: &[String],
    quiet_names: &[String],
    seed_only: bool,
) -> String {
    let low_names = english_list(low_names);
    let quiet_names = english_list(quiet_names);
    let mut text = format!(
        "This comparison has no winner, and that is the result of weighing, not a failure to weigh. The deepest single point measured anywhere in these runs is {low_loss}, from {low_names} at epoch {low_at} — and it stays on this page and on the chart whether or not it wins. The calmest stretch around a low belongs to a different run, {quiet_names}: the middle of the {quiet_n} samples around its low is {quiet_median}, where {low_names}'s neighborhood sits at {low_median}. Those medians are not the same, and this report will not pretend they are. But the two claims point at different runs, and the body of these curves is exactly where these runs are hardest to tell apart, so no stored measurement in this comparison settles it. To be plain about what that means: this is not \"the runs are all the same\" — {quiet_median} and {low_median} differ. And it is not \"{low_names} is the best run\" — the stretch around its low is not the calmest one measured. Both facts are kept: the deeper dip and the calmer neighborhood.",
        low_loss = format_measure(low.low),
        low_at = format_measure(low.at),
        quiet_n = quiet.count,
        quiet_median = format_measure(quiet.median),
        low_median = format_measure(low.median),
    );
    if seed_only {
        text.push_str(" Your next step is more evidence at this same recipe, not a change to it, and changing nothing on the strength of that answer is a legitimate choice, because nothing about the recipe was tested.");
    } else {
        text.push_str(" The recipe also changes in this comparison, so this page does not treat the difference as a test of one setting.");
    }
    text
}

fn final_marker(reading: &Reading, low_names: &[String]) -> Option<String> {
    let known: Vec<(&str, f64)> = reading
        .series
        .iter()
        .filter_map(|series| {
            series
                .summary_final
                .map(|value| (series.name.as_str(), value))
        })
        .collect();
    if known.is_empty() {
        return None;
    }
    let min = known
        .iter()
        .map(|(_, value)| *value)
        .min_by(|left, right| left.total_cmp(right))
        .expect("known finals");
    let max = known
        .iter()
        .map(|(_, value)| *value)
        .max_by(|left, right| left.total_cmp(right))
        .expect("known finals");
    let winners: Vec<String> = known
        .iter()
        .filter(|(_, value)| value.to_bits() == min.to_bits())
        .map(|(name, _)| (*name).to_string())
        .collect();
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
                ", best of a {}–{} cluster",
                format_measure(min),
                format_measure(max)
            )
        };
        Some(format!(
            "Ranked by training_summary.final_loss — {} for {}{cluster} — {} would be crowned, and {}'s {} would disappear from the page, because final_loss is not one of the {points}.",
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
            "Every lowest sample was reached when the learning rate had already decayed under a twentieth of its peak.".to_string(),
        );
    }
    if under.is_empty() {
        return Some(
            "None of these lows was reached under a twentieth of that run's peak learning rate."
                .to_string(),
        );
    }
    if above.is_empty() {
        return Some(format!(
            "{} lows ({}) were reached when the learning rate had already decayed under a twentieth of its peak.",
            under.len(),
            english_list(&under)
        ));
    }
    Some(format!(
        "{} of the {} lows ({}) were reached when the learning rate had already decayed under a twentieth of its peak. The other {} ({}) reached theirs while the rate was still above that.",
        count_word(under.len(), true),
        count_word(nears.len(), false),
        english_list(&under),
        count_word(above.len(), false),
        english_list(&above)
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
        "Because every run used the same cosine schedule{span}, \"low late\" and \"the schedule ran out\" are tangled together in this data, and this comparison cannot untangle them."
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
            "Assumption, labeled, not a result: the catalog states a formula (scale = alpha/r). It does not state how an alpha change changes loss. That scale change is assumed to act like a learning-rate change on the adapter. It is not measured here.".to_string(),
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
            "The batch rule (when the batch is multiplied by k, multiply the learning rate by k) was not applied: the effective batch stayed at {batch} in every run. Note this so no one later believes the learning rate was chosen by that rule."
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
        "Assumption, labeled, not a result: the catalog states a formula (scale = alpha/r). It does not state how a rank change changes loss. With alpha unchanged, changing rank changes alpha/r {change}, and this report assumes that scale change acts the way a learning-rate change on the adapter would. That is assumed here, not measured."
    ))
}

fn rank_intro(board: &Board) -> String {
    let lever = if rank_moves(board) && alpha_moves(board) {
        "LoRA rank and LoRA alpha"
    } else if rank_moves(board) {
        "LoRA rank"
    } else {
        "LoRA alpha"
    };
    format!("The lever under test is {lever}. {}", samples_clause(board))
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
                "{} rank {} (scale [placeholder: alpha for this run is not in the recipe])",
                series.name,
                format_measure(rank)
            )),
            (None, Some(alpha)) => {
                parts.push(format!("{} alpha {}", series.name, format_measure(alpha)))
            }
            _ => parts.push(format!("{} has no LoRA rank in the recipe", series.name)),
        }
    }
    format!(
        "Under test: {}. Everything else that is shared is listed with what was not tested.",
        parts.join("; ")
    )
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
    Some(format!(
        "On all {} runs, identically: {}.",
        count_word(board.series.len(), false),
        parts.join(", ")
    ))
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
    let min_at = nears
        .iter()
        .map(|near| near.at)
        .min_by(|left, right| left.total_cmp(right))
        .expect("nears");
    let max_at = nears
        .iter()
        .map(|near| near.at)
        .max_by(|left, right| left.total_cmp(right))
        .expect("nears");
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
        "Record-keeping note, not a recipe change: every low in these runs occurred {when}, and checkpoints were saved at epochs {list}, so no saved checkpoint sits at the moments this report discusses. If keeping such a moment matters to you, save one nearer that band next time. This report has no evidence that doing so changes any loss."
    ));
}

fn emit_lines(out: &mut String, nears: &[Neighborhood], indexes: &[usize]) {
    for index in indexes {
        let near = &nears[*index];
        let samples = if near.count == 1 { "sample" } else { "samples" };
        let mut text = format!(
            "  {}: low {} at {}. Half an epoch holds {} {}, median {}.",
            near.name,
            format_measure(near.low),
            format_measure(near.at),
            near.count,
            samples,
            format_measure(near.median)
        );
        if let Some(next) = near.next {
            text.push_str(&format!(" Next lowest {}.", format_measure(next)));
            if near.lone_low() {
                text.push_str(" The low is a single sample.");
            }
        }
        line(out, &text);
        if let Some(text) = lr_line(near) {
            line(out, &text);
        }
    }
}

fn lr_line(near: &Neighborhood) -> Option<String> {
    let flag = near.decayed()?;
    let place = if flag {
        "under a twentieth"
    } else {
        "not under a twentieth"
    };
    Some(format!(
        "  Learning rate at the lowest sample, {}: {} ({place} of max {}).",
        near.name,
        format_measure(near.lr?),
        format_measure(near.lr_max?)
    ))
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
            english_list(&exceptions)
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

fn rest_indexes(nears: &[Neighborhood]) -> Vec<usize> {
    let used = discussion_order(nears);
    let low = lowest_indexes(nears);
    let quiet = quiet_indexes(nears);
    used.into_iter()
        .filter(|index| !low.contains(index) && !quiet.contains(index))
        .collect()
}

fn quiet_only(low: &[usize], quiet: &[usize]) -> Vec<usize> {
    quiet
        .iter()
        .copied()
        .filter(|index| !low.contains(index))
        .collect()
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
    fn an_abstaining_seed_report_keeps_both_facts_and_cites_only_the_used_cards() {
        let board = abstain_board();
        assert_eq!(
            report_file_name(&board, Some("2026-10-05")),
            "runforge-report-2026-10-05-3-runs.txt"
        );
        let report = comparison_report(&board, Some("2026-10-05"));
        assert!(report.contains("Report written 2026-10-05 UTC."));
        assert!(!report.contains("Measured"));
        assert!(report.contains("This comparison has no winner"));
        assert!(report.contains("the runs are all the same"));
        assert!(report.contains("is the best run"));
        assert!(report.contains("nothing about the recipe was tested"));
        assert!(report.contains("seed 13"));
        assert!(report.contains("seed 1024"));
        assert!(report.contains("0.02"));
        assert!(report.contains("would disappear"));
        assert!(
            report.contains("still above a twentieth") || report.contains("not under a twentieth")
        );
        assert!(report.contains("LoRA scale (alpha/r): 2"));
        assert!(report.contains("no evidence about it"));
        assert!(report.contains("does not place warmup on the epoch axis"));
        assert!(report.contains("was not applied"));
        assert!(report.contains("no saved checkpoint"));
        assert!(report.contains("Half an epoch"));
        assert!(report.contains("2002.06305"));
        assert!(report.contains("1608.03983"));
        assert!(report.contains("2106.09685"));
        assert!(report.contains("1706.02677"));
        assert!(!report.contains("1211.5063"));
        assert!(!report.contains("srivastava14a"));
        assert!(!report.contains("1711.05101"));
        assert!(!report.contains("**"));
        assert!(report.lines().all(|line| !line.starts_with('#')));
        assert!(!report.contains("move the learning rate"));
        assert!(!report.contains('\\'));
        let omission = orientation_omission(&board);
        assert!(omission.contains("did not pick a winner"));
        assert!(omission.contains("0.02"));
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
        assert!(report.contains("scale 2"));
        assert!(report.contains("scale 1"));
        assert!(report.contains("from 2 to 1"));
        assert!(
            report.contains("a spread measure for the neighborhood window is not computed today")
        );
        assert!(!report.contains("nothing about the recipe was tested"));
        assert!(!report.contains("Listed for the record, not as a result"));
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
        assert!(!report.contains("differ in rank and in seed"));
        assert!(!report.contains("nothing about the recipe was tested"));
    }
}
