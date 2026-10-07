//! RunForge's measures: what a formula can read from one run's loss curve.
//!
//! The formula language itself (grammar, size caps, `knob`, arithmetic) is the
//! shared `workbench` crate's. This module names the loss measures and computes
//! them, once per run, over that run's stored samples.

use workbench::{Host, Measure};

use crate::series::{Series, recipe_keys, recipe_label};

const HALF_EPOCH: f64 = 0.5;

/// Every loss measure a formula may use. The workbench adds `knob` and arithmetic.
pub const LOSS_MEASURES: &[Measure] = &[
    Measure {
        name: "low",
        args: "",
        means: "the lowest stored loss",
    },
    Measure {
        name: "low_epoch",
        args: "",
        means: "the epoch of the lowest stored loss",
    },
    Measure {
        name: "median",
        args: "",
        means: "the median loss within half an epoch of the low",
    },
    Measure {
        name: "q1",
        args: "",
        means: "the first quartile of that window",
    },
    Measure {
        name: "q3",
        args: "",
        means: "the third quartile of that window",
    },
    Measure {
        name: "first",
        args: "",
        means: "the first stored loss",
    },
    Measure {
        name: "last",
        args: "",
        means: "the last stored loss",
    },
    Measure {
        name: "samples",
        args: "",
        means: "how many samples have a finite loss",
    },
    Measure {
        name: "peak_lr",
        args: "",
        means: "the highest stored learning rate",
    },
    Measure {
        name: "lr_at_low",
        args: "",
        means: "the learning rate on the lowest sample",
    },
    Measure {
        name: "end_epoch",
        args: "",
        means: "the last epoch with a finite loss",
    },
    Measure {
        name: "median_between",
        args: "a, b",
        means: "the median loss for epochs a to b",
    },
    Measure {
        name: "mean_between",
        args: "a, b",
        means: "the mean loss for epochs a to b",
    },
    Measure {
        name: "min_between",
        args: "a, b",
        means: "the lowest loss for epochs a to b",
    },
    Measure {
        name: "count_between",
        args: "a, b",
        means: "how many finite losses fall in epochs a to b",
    },
    Measure {
        name: "slope_between",
        args: "a, b",
        means: "least-squares slope of ln(loss) per epoch, epochs a to b",
    },
    Measure {
        name: "lr_between",
        args: "a, b",
        means: "the mean learning rate for epochs a to b",
    },
];

/// RunForge's runs, as the workbench measures them.
pub struct LossHost {
    series: Vec<Series>,
}

impl LossHost {
    pub fn new(series: Vec<Series>) -> Self {
        LossHost { series }
    }
}

impl Host for LossHost {
    fn program(&self) -> &str {
        "RunForge"
    }

    fn subject(&self) -> &str {
        "fine-tuning runs"
    }

    fn build_example(&self) -> &str {
        "For example, how far a curve climbs after its low (last / low), or how steep the last epoch is (slope_between(end_epoch - 1, end_epoch))."
    }

    fn measures(&self) -> &[Measure] {
        LOSS_MEASURES
    }

    fn measure(&self, run: usize, name: &str, args: &[f64]) -> Result<f64, String> {
        let view = RunView::new(&self.series[run]);
        match args {
            [] => view.measure(name),
            [a, b] => view.windowed(name, *a, *b),
            _ => Err(format!("{name} is not a measure this program knows.")),
        }
    }

    fn knob_label(&self, key: &str) -> String {
        recipe_label(key).to_string()
    }

    fn knob_keys<'a>(&self, knobs: &'a serde_json::Map<String, serde_json::Value>) -> Vec<&'a str> {
        recipe_keys(knobs)
    }
}

/// The finite samples of one run, sorted by epoch.
struct RunView<'a> {
    series: &'a Series,
    points: Vec<(f64, f64, Option<f64>)>,
}

impl<'a> RunView<'a> {
    fn new(series: &'a Series) -> Self {
        let mut points: Vec<(f64, f64, Option<f64>)> = series
            .samples
            .iter()
            .filter_map(|sample| {
                let x = sample.x.filter(|x| x.is_finite())?;
                let loss = sample.loss.filter(|loss| loss.is_finite())?;
                Some((x, loss, sample.lr.filter(|lr| lr.is_finite())))
            })
            .collect();
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        RunView { series, points }
    }

    fn missing(&self, what: &str) -> String {
        format!("{} has no {what}.", self.series.name)
    }

    fn lowest(&self) -> Result<(f64, f64, Option<f64>), String> {
        self.points
            .iter()
            .copied()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .ok_or_else(|| self.missing("finite loss"))
    }

    fn window(&self) -> Result<Vec<f64>, String> {
        let (at, _, _) = self.lowest()?;
        let mut values: Vec<f64> = self
            .points
            .iter()
            .filter(|point| (point.0 - at).abs() <= HALF_EPOCH)
            .map(|point| point.1)
            .collect();
        values.sort_by(f64::total_cmp);
        Ok(values)
    }

    fn between(&self, a: f64, b: f64) -> Vec<(f64, f64, Option<f64>)> {
        let (from, to) = if a <= b { (a, b) } else { (b, a) };
        self.points
            .iter()
            .copied()
            .filter(|point| point.0 >= from && point.0 <= to)
            .collect()
    }

    fn measure(&self, name: &str) -> Result<f64, String> {
        match name {
            "low" => Ok(self.lowest()?.1),
            "low_epoch" => Ok(self.lowest()?.0),
            "median" => {
                workbench::quantile(&self.window()?, 0.5).ok_or_else(|| self.missing("window"))
            }
            "q1" => {
                workbench::quantile(&self.window()?, 0.25).ok_or_else(|| self.missing("window"))
            }
            "q3" => {
                workbench::quantile(&self.window()?, 0.75).ok_or_else(|| self.missing("window"))
            }
            "first" => self
                .points
                .first()
                .map(|p| p.1)
                .ok_or_else(|| self.missing("finite loss")),
            "last" => self
                .points
                .last()
                .map(|p| p.1)
                .ok_or_else(|| self.missing("finite loss")),
            "end_epoch" => self
                .points
                .last()
                .map(|p| p.0)
                .ok_or_else(|| self.missing("finite loss")),
            "samples" => Ok(self.points.len() as f64),
            "peak_lr" => self
                .series
                .samples
                .iter()
                .filter_map(|sample| sample.lr.filter(|lr| lr.is_finite()))
                .reduce(f64::max)
                .ok_or_else(|| self.missing("learning rate")),
            "lr_at_low" => self
                .lowest()?
                .2
                .ok_or_else(|| self.missing("learning rate at its low")),
            other => Err(format!("{other} is not a measure this program knows.")),
        }
    }

    fn windowed(&self, name: &str, a: f64, b: f64) -> Result<f64, String> {
        let points = self.between(a, b);
        let empty = || {
            format!(
                "{} has no finite loss between epochs {a} and {b}.",
                self.series.name
            )
        };
        let mut losses: Vec<f64> = points.iter().map(|point| point.1).collect();
        losses.sort_by(f64::total_cmp);
        match name {
            "count_between" => Ok(points.len() as f64),
            "median_between" => workbench::quantile(&losses, 0.5).ok_or_else(empty),
            "min_between" => losses.first().copied().ok_or_else(empty),
            "mean_between" => {
                if losses.is_empty() {
                    Err(empty())
                } else {
                    Ok(losses.iter().sum::<f64>() / losses.len() as f64)
                }
            }
            "lr_between" => {
                let rates: Vec<f64> = points.iter().filter_map(|point| point.2).collect();
                if rates.is_empty() {
                    Err(format!(
                        "{} has no learning rate between epochs {a} and {b}.",
                        self.series.name
                    ))
                } else {
                    Ok(rates.iter().sum::<f64>() / rates.len() as f64)
                }
            }
            "slope_between" => {
                let pairs: Vec<(f64, f64)> = points
                    .iter()
                    .filter(|point| point.1 > 0.0)
                    .map(|point| (point.0, point.1.ln()))
                    .collect();
                least_squares_slope(&pairs).ok_or_else(|| {
                    format!(
                        "{} needs two positive losses at different epochs between {a} and {b}.",
                        self.series.name
                    )
                })
            }
            other => Err(format!("{other} is not a measure this program knows.")),
        }
    }
}

fn least_squares_slope(pairs: &[(f64, f64)]) -> Option<f64> {
    if pairs.len() < 2 {
        return None;
    }
    let n = pairs.len() as f64;
    let mean_x = pairs.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_y = pairs.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = pairs.iter().map(|p| (p.0 - mean_x).powi(2)).sum();
    if sxx <= 0.0 {
        return None;
    }
    let sxy: f64 = pairs.iter().map(|p| (p.0 - mean_x) * (p.1 - mean_y)).sum();
    Some(sxy / sxx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::series::{Board, Sample};
    use serde_json::{Map, Value};
    use workbench::Expr;

    fn parse(text: &str) -> Result<Expr, String> {
        workbench::parse(text, LOSS_MEASURES)
    }

    fn parse_open(text: &str, learned: &dyn Fn(&str) -> Option<String>) -> Result<Expr, String> {
        workbench::parse_open(text, LOSS_MEASURES, learned)
    }

    fn canonical(expr: &Expr) -> String {
        workbench::canonical(expr)
    }

    /// One run through the workbench, as RunForge hands it over.
    fn eval(expr: &Expr, series: &Series) -> Result<f64, String> {
        let board = Board {
            series: vec![series.clone()],
            skipped: 0,
            shared: Map::new(),
            varying: Vec::new(),
        };
        workbench::eval(expr, &crate::bench::bench_board(&board), 0)
    }

    fn series() -> Series {
        let mut recipe = Map::new();
        recipe.insert("lora_r".to_string(), Value::from(16));
        let samples = [
            (0.0, 8.0, 0.0001),
            (1.0, 4.0, 0.0001),
            (2.0, 2.0, 0.00005),
            (2.4, 1.0, 0.00001),
            (3.0, 1.5, 0.0),
        ]
        .iter()
        .map(|(x, loss, lr)| Sample {
            x: Some(*x),
            loss: Some(*loss),
            lr: Some(*lr),
            extra: Map::new(),
        })
        .collect();
        Series {
            name: "seed 1".to_string(),
            seed: Some(1),
            model: String::new(),
            file_name: "a.json".to_string(),
            samples,
            recipe,
            summary: Map::new(),
        }
    }

    fn value(text: &str) -> f64 {
        eval(&parse(text).unwrap(), &series()).unwrap()
    }

    #[test]
    fn measures_and_arithmetic_evaluate_per_run() {
        assert_eq!(value("low"), 1.0);
        assert_eq!(value("low_epoch"), 2.4);
        assert_eq!(value("last / first"), 1.5 / 8.0);
        assert_eq!(value("knob('lora_r') * 2"), 32.0);
        assert_eq!(value("count_between(0, 1)"), 2.0);
        assert_eq!(value("max(low, last, 1.2)"), 1.5);
        assert_eq!(value("-2 ^ 2"), -4.0);
        assert_eq!(value("2 ^ -1"), 0.5);
        assert_eq!(value("1 - 2 - 3"), -4.0);
        let slope = value("slope_between(0, 2)");
        assert!((slope - (2.0_f64.ln() - 8.0_f64.ln()) / 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_language_refuses_what_it_does_not_know() {
        for bad in [
            "",
            "low +",
            "std::fs::read('x')",
            "system('rm')",
            "median_between(1)",
            "knob(lora_r)",
            "'lora_r'",
            "low; last",
            "eval(low)",
            "low $ last",
        ] {
            assert!(parse(bad).is_err(), "{bad} should be refused");
        }
        assert!(parse(&"(".repeat(30)).is_err());
        assert!(parse(&"low + ".repeat(60)).is_err());
    }

    #[test]
    fn a_non_finite_value_is_refused_and_spellings_share_a_canonical_form() {
        assert!(eval(&parse("ln(0 * low)").unwrap(), &series()).is_err());
        assert!(eval(&parse("knob('lora_alpha')").unwrap(), &series()).is_err());
        assert!(eval(&parse("median_between(9, 10)").unwrap(), &series()).is_err());
        assert_eq!(
            canonical(&parse("low/first").unwrap()),
            canonical(&parse(" low / first ").unwrap())
        );
    }

    #[test]
    fn every_measure_evaluates_on_a_run() {
        // Samples: (0, 8), (1, 4), (2, 2), (2.4, 1), (3, 1.5); learning rates 1e-4 .. 0.
        let near = |text: &str, want: f64| {
            let got = value(text);
            assert!((got - want).abs() < 1e-9, "{text}: {got} != {want}");
        };
        near("median", 1.5);
        near("q1", 1.25);
        near("q3", 1.75);
        near("first", 8.0);
        near("last", 1.5);
        near("samples", 5.0);
        near("end_epoch", 3.0);
        near("peak_lr", 0.0001);
        near("lr_at_low", 0.00001);
        near("median_between(0, 1)", 6.0);
        near("mean_between(1, 0)", 6.0);
        near("min_between(2, 3)", 1.0);
        near("lr_between(0, 1)", 0.0001);
        near(
            "abs(-2) + sqrt(4) + ln(exp(1)) + min(3, 1) + max(1, 2, 5)",
            2.0 + 2.0 + 1.0 + 1.0 + 5.0,
        );
        near("1e-3 * 1000 + 2.5E+1", 26.0);
    }

    #[test]
    fn each_refusal_says_what_is_wrong() {
        let refused = |text: &str, says: &str| {
            let error = parse(text).unwrap_err();
            assert!(error.contains(says), "{text}: {error}");
        };
        refused("low )", "text after its end");
        refused("knob('lora-r')", "letters, digits and _ only");
        refused("knob('lora_r", "not closed");
        refused("(low + 1", "parenthesis is not closed");
        refused("max(low last)", "needs a comma");
        refused("median_between", "needs arguments in parentheses");
        refused("low(1)", "takes no arguments");
        refused("abs()", "takes one argument");
        refused("max(1)", "takes 2 to 8 arguments");
        refused("knob(1)", "quoted recipe name");
        refused(&vec!["1"; 70].join("+"), "at most 64 parts");
        refused(
            &format!("{}1{}", "(".repeat(20), ")".repeat(20)),
            "nests at most",
        );
        let run = series();
        let error = |text: &str| eval(&parse(text).unwrap(), &run).unwrap_err();
        assert!(error("mean_between(7, 9)").contains("no finite loss between"));
        assert!(error("slope_between(2.4, 2.4)").contains("two positive losses"));
        assert!(error("min_between(7, 9)").contains("no finite loss"));
        let mut bare = series();
        for sample in &mut bare.samples {
            sample.lr = None;
        }
        assert!(
            eval(&parse("lr_between(0, 3)").unwrap(), &bare)
                .unwrap_err()
                .contains("no learning rate")
        );
        assert!(
            eval(&parse("lr_at_low").unwrap(), &bare)
                .unwrap_err()
                .contains("learning rate at its low")
        );
        assert!(
            eval(&parse("peak_lr").unwrap(), &bare)
                .unwrap_err()
                .contains("no learning rate")
        );
        bare.samples.clear();
        for text in ["low", "median", "first", "last", "end_epoch"] {
            assert!(eval(&parse(text).unwrap(), &bare).is_err(), "{text}");
        }
    }

    #[test]
    fn learned_names_expand_within_limits() {
        let library = |name: &str| match name {
            "a" => Some("b + 1".to_string()),
            "b" => Some("c + 1".to_string()),
            "c" => Some("d + 1".to_string()),
            "d" => Some("e + 1".to_string()),
            "e" => Some("f + 1".to_string()),
            "f" => Some("low".to_string()),
            "wide" => Some(vec!["low"; 30].join("+")),
            _ => None,
        };
        assert!(parse_open("c * 2", &library).is_ok());
        assert!(parse_open("a", &library).unwrap_err().contains("four deep"));
        assert!(
            parse_open("wide + wide + wide + wide + wide", &library)
                .unwrap_err()
                .contains("too large")
        );
        assert!(
            parse_open("nothing", &library)
                .unwrap_err()
                .contains("not a measure")
        );
        assert_eq!(
            canonical(&parse("-knob('lora_r')").unwrap()),
            "-(knob('lora_r'))"
        );
    }
}
