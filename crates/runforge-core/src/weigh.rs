//! Weigh a board before the sidecar speaks.
//!
//! The arithmetic is measured from the samples and the recipe. A shared field
//! was not tested. An assumption is labeled, and it is not a result. The
//! reference is local. This module does not fetch a page or call a model.

use serde_json::Value;

use crate::series::{Board, Series, format_measure, recipe_label, recipe_text};

const HALF_EPOCH: f64 = 0.5;
const DECAYED: f64 = 0.05;

/// One knob the reference can speak about.
pub struct Card {
    pub key: &'static str,
    pub formula: &'static str,
    pub finding: &'static str,
    pub cite: &'static str,
    pub url: &'static str,
    /// The one kind of sentence this card may back in a report.
    pub supports: &'static str,
}

/// What the samples support, plus the reference cards for the knobs present.
pub struct Weighing {
    pub lines: Vec<String>,
    pub cards: Vec<&'static Card>,
    /// The lowest sample and the quieter half-epoch median are different series.
    pub abstain: bool,
    /// One neighborhood per series that has a finite low, in series order.
    pub neighborhoods: Vec<Neighborhood>,
}

/// The half-epoch around one series' lowest sample.
pub struct Neighborhood {
    pub name: String,
    pub low: f64,
    pub at: f64,
    pub count: usize,
    pub median: f64,
    /// The first and third quartiles of the same window: the middle half of its samples.
    pub q1: f64,
    pub q3: f64,
    pub next: Option<f64>,
    pub lr: Option<f64>,
    pub lr_max: Option<f64>,
}

/// How far apart the runs' window middles are, against the spread inside each window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Separation {
    /// The middles sit closer together than the narrowest middle half of any window.
    InsideNoise,
    /// Wider than the narrowest window's middle half, narrower than the widest.
    Partial,
    /// The middles sit farther apart than the widest middle half of any window.
    Apart,
}

/// The gap between the highest and lowest window middle, and the narrowest and widest middle half.
pub struct Spread {
    pub gap: f64,
    pub narrowest: f64,
    pub widest: f64,
    pub separation: Separation,
}

/// Measured over two or more neighborhoods. `None` for fewer.
pub fn spread(nears: &[Neighborhood]) -> Option<Spread> {
    if nears.len() < 2 {
        return None;
    }
    let medians = nears.iter().map(|near| near.median);
    let high = medians.clone().fold(f64::NEG_INFINITY, f64::max);
    let low = medians.fold(f64::INFINITY, f64::min);
    let widths = nears.iter().map(Neighborhood::middle_half);
    let narrowest = widths.clone().fold(f64::INFINITY, f64::min);
    let widest = widths.fold(f64::NEG_INFINITY, f64::max);
    let gap = high - low;
    if !gap.is_finite() || !narrowest.is_finite() || !widest.is_finite() {
        return None;
    }
    let separation = if gap < narrowest {
        Separation::InsideNoise
    } else if gap > widest {
        Separation::Apart
    } else {
        Separation::Partial
    };
    Some(Spread {
        gap,
        narrowest,
        widest,
        separation,
    })
}

impl Neighborhood {
    /// Width of the middle half of the window, third quartile minus first.
    pub fn middle_half(&self) -> f64 {
        self.q3 - self.q1
    }

    /// The next sample in the window is more than twice this low.
    pub fn lone_low(&self) -> bool {
        self.next
            .is_some_and(|next| self.low > 0.0 && next > self.low * 2.0)
    }

    /// Whether the learning rate on this low is under a twentieth of the series max.
    pub fn decayed(&self) -> Option<bool> {
        let (Some(lr), Some(max)) = (self.lr, self.lr_max) else {
            return None;
        };
        if !lr.is_finite() || !max.is_finite() || max <= 0.0 {
            return None;
        }
        Some(lr < max * DECAYED)
    }
}

const CATALOG: &[Card] = &[
    Card {
        key: "seed",
        supports: "Why runs of one recipe with different seeds are compared.",
        formula: "Same recipe, different seed.",
        finding: "Weight initialization and data order each move fine-tuning results by as much as a hyperparameter change. A single extreme sample is a weak summary of that spread.",
        cite: "Dodge, Ilharco, Schwartz, Farhadi, Hajishirzi, and Smith, 2020, Fine-Tuning Pretrained Language Models.",
        url: "https://arxiv.org/abs/2002.06305",
    },
    Card {
        key: "lora_r",
        supports: "LoRA scale is alpha divided by rank.",
        formula: "LoRA scale = alpha / r.",
        finding: "The low-rank update is scaled by alpha/r. With Adam, tuning alpha is roughly tuning the learning rate when the initialization is scaled. Rank and alpha were a pair, not two independent results.",
        cite: "Hu, Shen, Wallis, Allen-Zhu, Li, Wang, Wang, and Chen, 2021, LoRA.",
        url: "https://arxiv.org/abs/2106.09685",
    },
    Card {
        key: "learning_rate",
        supports: "The batch rule, named only where it was not applied.",
        formula: "When the batch is multiplied by k, multiply the learning rate by k.",
        finding: "The linear scaling rule kept ImageNet accuracy as the minibatch grew, with a warmup. It applies when the batch changes. It is not applied when the batch is shared.",
        cite: "Goyal, Dollár, Girshick, Noordhuis, Wesolowski, Kyrola, Tulloch, Jia, and He, 2017, Accurate, Large Minibatch SGD.",
        url: "https://arxiv.org/abs/1706.02677",
    },
    Card {
        key: "lr_scheduler",
        supports: "A cosine schedule lowers the learning rate inside the run.",
        formula: "Cosine annealing decays the learning rate inside the run.",
        finding: "A cosine schedule spends the late epochs at a small learning rate. A low that appears there is not evidence about the learning rate at the start of the run.",
        cite: "Loshchilov and Hutter, 2017, SGDR.",
        url: "https://arxiv.org/abs/1608.03983",
    },
    Card {
        key: "weight_decay",
        supports: "Decoupled weight decay, for Adam-style optimizers.",
        formula: "Decoupled weight decay is applied to the weights, not inside the adaptive step.",
        finding: "For Adam, L2 on the gradient is not weight decay. The decay strength is its own knob, and it was easy to confuse with the learning rate before it was decoupled.",
        cite: "Loshchilov and Hutter, 2019, Decoupled Weight Decay Regularization.",
        url: "https://arxiv.org/abs/1711.05101",
    },
    Card {
        key: "max_grad_norm",
        supports: "Clipping by the gradient norm.",
        formula: "Clip the gradient when its norm exceeds the threshold.",
        finding: "Global-norm clipping limits a step without rescaling every small gradient. The threshold was not a result unless the runs used different thresholds.",
        cite: "Pascanu, Mikolov, and Bengio, 2013, On the difficulty of training recurrent neural networks.",
        url: "https://arxiv.org/abs/1211.5063",
    },
    Card {
        key: "lora_dropout",
        supports: "Dropout inside the low-rank update.",
        formula: "Dropout inside the low-rank update.",
        finding: "Dropout limits co-adaptation by dropping units during training. On a LoRA update it regularizes the small matrices. It was not measured unless dropout changed.",
        cite: "Srivastava, Hinton, Krizhevsky, Sutskever, and Salakhutdinov, 2014, Dropout.",
        url: "https://www.jmlr.org/papers/v15/srivastava14a.html",
    },
];

const NAMED: &[&str] = &[
    "learning_rate",
    "lr_scheduler",
    "warmup_steps",
    "weight_decay",
    "max_grad_norm",
    "per_device_batch",
    "grad_accum",
    "effective_batch",
    "epochs",
    "max_seq_len",
    "prompt_loss_weight",
    "lora_r",
    "lora_alpha",
    "lora_dropout",
];

struct Near {
    name: String,
    low: f64,
    at: f64,
    count: usize,
    median: f64,
    q1: f64,
    q3: f64,
    next: Option<f64>,
    lr: Option<f64>,
    lr_max: Option<f64>,
}

/// Measure the board. The lowest sample stays in the lines even when the rank abstains.
pub fn weigh(board: &Board) -> Weighing {
    let nears: Vec<Near> = board.series.iter().filter_map(near_low).collect();
    let mut lines = Vec::new();
    for near in &nears {
        lines.push(near_line(near));
    }
    let abstain = rank_line(&nears, &mut lines);
    lines.extend(decay_lines(&nears));
    if let Some(line) = scale_line(board) {
        lines.push(line);
    }
    if let Some(line) = warmup_line(board) {
        lines.push(line);
    }
    if let Some(line) = shared_line(board) {
        lines.push(line);
    }
    lines.extend(assumptions(board));
    let neighborhoods = nears.iter().map(publish).collect();
    Weighing {
        lines,
        cards: cards_for(board),
        abstain,
        neighborhoods,
    }
}

fn publish(near: &Near) -> Neighborhood {
    Neighborhood {
        name: near.name.clone(),
        low: near.low,
        at: near.at,
        count: near.count,
        median: near.median,
        q1: near.q1,
        q3: near.q3,
        next: near.next,
        lr: near.lr,
        lr_max: near.lr_max,
    }
}

fn cards_for(board: &Board) -> Vec<&'static Card> {
    CATALOG
        .iter()
        .filter(|card| applies(board, card.key))
        .collect()
}

fn applies(board: &Board, key: &str) -> bool {
    match key {
        "seed" => {
            board
                .series
                .iter()
                .filter(|series| series.seed.is_some())
                .count()
                > 1
        }
        "lora_r" => has(board, "lora_r") || has(board, "lora_alpha"),
        "learning_rate" => has(board, "learning_rate") || has(board, "effective_batch"),
        other => has(board, other),
    }
}

fn has(board: &Board, key: &str) -> bool {
    board.shared.contains_key(key) || board.varying.iter().any(|item| item == key)
}

fn near_low(series: &Series) -> Option<Near> {
    let mut best: Option<(usize, f64, f64)> = None;
    for (index, sample) in series.samples.iter().enumerate() {
        let (Some(x), Some(loss)) = (sample.x, sample.loss) else {
            continue;
        };
        if !x.is_finite() || !loss.is_finite() {
            continue;
        }
        let replace = best.is_none_or(|(_, _, low)| loss.total_cmp(&low).is_lt());
        if replace {
            best = Some((index, x, loss));
        }
    }
    let (low_index, at, low) = best?;
    let mut losses = Vec::new();
    let mut others = Vec::new();
    for (index, sample) in series.samples.iter().enumerate() {
        let (Some(x), Some(loss)) = (sample.x, sample.loss) else {
            continue;
        };
        if !x.is_finite() || !loss.is_finite() || (x - at).abs() > HALF_EPOCH {
            continue;
        }
        losses.push(loss);
        if index != low_index {
            others.push(loss);
        }
    }
    let median = median(&mut losses)?;
    let q1 = quantile(&losses, 0.25)?;
    let q3 = quantile(&losses, 0.75)?;
    let next = others
        .iter()
        .copied()
        .min_by(|left, right| left.total_cmp(right));
    let lr = series.samples.get(low_index).and_then(|sample| sample.lr);
    let lr_max = series
        .samples
        .iter()
        .filter_map(|sample| sample.lr)
        .filter(|value| value.is_finite())
        .max_by(|left, right| left.total_cmp(right));
    Some(Near {
        name: series.name.clone(),
        low,
        at,
        count: losses.len(),
        median,
        q1,
        q3,
        next,
        lr,
        lr_max,
    })
}

fn near_line(near: &Near) -> String {
    let samples = if near.count == 1 { "sample" } else { "samples" };
    let mut line = format!(
        "{}: low {} at {}. Half an epoch holds {} {}, median {}.",
        near.name,
        format_measure(near.low),
        format_measure(near.at),
        near.count,
        samples,
        format_measure(near.median)
    );
    if let Some(next) = near.next {
        line.push_str(&format!(" Next lowest {}.", format_measure(next)));
        if near.low > 0.0 && next > near.low * 2.0 {
            line.push_str(" The low is a single sample.");
        }
    }
    line
}

fn rank_line(nears: &[Near], lines: &mut Vec<String>) -> bool {
    if nears.len() < 2 {
        return false;
    }
    let by_low = nears
        .iter()
        .min_by(|left, right| left.low.total_cmp(&right.low))
        .expect("two series");
    let best_median = nears
        .iter()
        .map(|near| near.median)
        .min_by(|left, right| left.total_cmp(right))
        .expect("two series");
    if by_low.median.to_bits() == best_median.to_bits() {
        lines.push(format!(
            "The half-epoch median agrees with the lowest sample, {}.",
            by_low.name
        ));
        false
    } else {
        let quieter: Vec<&str> = nears
            .iter()
            .filter(|near| near.median.to_bits() == best_median.to_bits())
            .map(|near| near.name.as_str())
            .collect();
        lines.push(format!(
            "The lowest sample is {}. The lowest half-epoch median is {}. Those are different series. Abstain on crowning a series.",
            by_low.name,
            quieter.join(", ")
        ));
        true
    }
}

fn decay_lines(nears: &[Near]) -> Vec<String> {
    let known: Vec<&Near> = nears
        .iter()
        .filter(|near| near.lr.is_some() && near.lr_max.is_some_and(|max| max > 0.0))
        .collect();
    if known.is_empty() {
        return Vec::new();
    }
    let decayed = known.iter().all(|near| {
        let lr = near.lr.expect("filtered");
        let max = near.lr_max.expect("filtered");
        lr < max * DECAYED
    });
    if decayed && known.len() == nears.len() {
        return vec![
            "Every lowest sample sits under a twentieth of that series' highest learning rate. The lows are in the decay.".to_string(),
        ];
    }
    known
        .iter()
        .map(|near| {
            let lr = near.lr.expect("filtered");
            let max = near.lr_max.expect("filtered");
            let place = if lr < max * DECAYED {
                "under a twentieth"
            } else {
                "not under a twentieth"
            };
            format!(
                "Learning rate at the lowest sample, {}: {} ({place} of max {}).",
                near.name,
                format_measure(lr),
                format_measure(max)
            )
        })
        .collect()
}

fn scale_line(board: &Board) -> Option<String> {
    if board
        .varying
        .iter()
        .any(|key| key == "lora_r" || key == "lora_alpha")
    {
        return Some("LoRA rank or alpha changes, so alpha/r is not one number.".to_string());
    }
    let alpha = shared_number(board, "lora_alpha")?;
    let rank = shared_number(board, "lora_r")?;
    if rank == 0.0 {
        return None;
    }
    Some(format!(
        "LoRA scale alpha/r is {}.",
        format_measure(alpha / rank)
    ))
}

fn warmup_line(board: &Board) -> Option<String> {
    if board.varying.iter().any(|key| key == "warmup_steps") {
        return None;
    }
    let steps = shared_number(board, "warmup_steps")?;
    Some(format!(
        "Warmup is {} steps. The samples are not a step index, so the warmup fraction is not computed.",
        format_measure(steps)
    ))
}

fn shared_line(board: &Board) -> Option<String> {
    let mut parts = Vec::new();
    for key in NAMED {
        if board.varying.iter().any(|item| item == key) {
            continue;
        }
        let Some(value) = board.shared.get(*key) else {
            continue;
        };
        parts.push(format!("{} {}", recipe_label(key), recipe_text(value)));
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!("Shared, so not a result: {}.", parts.join(", ")))
    }
}

fn assumptions(board: &Board) -> Vec<String> {
    let mut lines = Vec::new();
    let scale_shared = shared_number(board, "lora_alpha").is_some()
        && shared_number(board, "lora_r").is_some()
        && !board
            .varying
            .iter()
            .any(|key| key == "lora_r" || key == "lora_alpha");
    let rate_shared = shared_number(board, "learning_rate").is_some()
        && !board.varying.iter().any(|key| key == "learning_rate");
    if scale_shared && rate_shared {
        lines.push(
            "Assumption, not a result: alpha stands in for the learning rate only when the initialization is scaled, and neither knob moved.".to_string(),
        );
    }
    let batch_shared = board.shared.contains_key("effective_batch")
        && !board
            .varying
            .iter()
            .any(|key| key == "effective_batch" || key == "learning_rate");
    if batch_shared && rate_shared {
        lines.push(
            "Assumption, not a result: the linear scaling rule would move the learning rate with the batch. The batch did not change, so the rule is not applied.".to_string(),
        );
    }
    lines
}

fn shared_number(board: &Board, key: &str) -> Option<f64> {
    board
        .shared
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

/// Linear interpolation between the closest ranks of sorted `values`.
fn quantile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = p * (sorted.len() - 1) as f64;
    let below = rank.floor() as usize;
    let above = rank.ceil() as usize;
    let fraction = rank - below as f64;
    Some(sorted[below] + (sorted[above] - sorted[below]) * fraction)
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.total_cmp(right));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some((values[mid - 1] + values[mid]) / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::series::Sample;
    use serde_json::Map;

    fn sample(x: f64, loss: f64, lr: f64) -> Sample {
        Sample {
            x: Some(x),
            loss: Some(loss),
            lr: Some(lr),
            extra: Map::new(),
        }
    }

    fn series(name: &str, seed: i64, samples: Vec<Sample>) -> Series {
        Series {
            name: name.to_string(),
            seed: Some(seed),
            model: String::new(),
            file_name: format!("{name}.json"),
            samples,
            recipe: Map::new(),
            summary: Map::new(),
        }
    }

    #[test]
    fn a_lone_spike_does_not_carry_the_rank() {
        let mut recipe = Map::new();
        recipe.insert("lora_r".to_string(), Value::from(16));
        recipe.insert("lora_alpha".to_string(), Value::from(32));
        recipe.insert("learning_rate".to_string(), Value::from(0.00015));
        recipe.insert("effective_batch".to_string(), Value::from(8));
        recipe.insert("warmup_steps".to_string(), Value::from(10));
        let spike = series(
            "seed 13",
            13,
            vec![
                sample(6.6, 0.20, 0.0001),
                sample(7.0, 0.01, 0.000001),
                sample(7.4, 0.19, 0.000001),
            ],
        );
        let steady = series(
            "seed 1024",
            1024,
            vec![
                sample(6.6, 0.06, 0.0001),
                sample(7.0, 0.05, 0.000001),
                sample(7.4, 0.055, 0.000001),
            ],
        );
        let board = Board {
            series: vec![spike, steady],
            skipped: 0,
            shared: recipe,
            varying: Vec::new(),
        };
        let weighing = weigh(&board);
        let text = weighing.lines.join("\n");
        assert!(weighing.abstain);
        assert!(text.contains("seed 13"));
        assert!(text.contains("The low is a single sample"));
        assert!(text.contains("Abstain on crowning a series"));
        assert!(text.contains("lowest half-epoch median is seed 1024"));
        assert!(text.contains("alpha/r is 2"));
        assert!(text.contains("The lows are in the decay"));
        assert!(text.contains("warmup fraction is not computed"));
        assert!(text.contains("not a result"));
        assert!(text.contains("Assumption, not a result"));
        assert!(weighing.cards.iter().any(|card| card.key == "seed"));
        assert!(
            weighing
                .cards
                .iter()
                .any(|card| card.url.contains("2106.09685"))
        );
    }
}
