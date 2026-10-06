//! A small formula language for tools the sidecar defines.
//!
//! A learned tool is a formula, not a program. It is parsed and evaluated here,
//! once per run, over that run's stored samples and recipe. It cannot read a
//! file, open the network, loop, or call anything outside this list. A formula
//! that fails to parse, or gives a non-finite value on a run, is refused.
//!
//! Grammar, lowest precedence first:
//!   sum     = product (("+" | "-") product)*
//!   product = unary (("*" | "/") unary)*
//!   unary   = "-" unary | power
//!   power   = atom ("^" unary)?
//!   atom    = number | name | name "(" args ")" | "(" sum ")"
//!   args    = (sum | 'text') ("," (sum | 'text'))*

use crate::series::Series;

const MAX_LEN: usize = 240;
const MAX_NODES: usize = 64;
const MAX_DEPTH: usize = 16;
const HALF_EPOCH: f64 = 0.5;

/// One run's view of a measure, documented for the model and the window.
pub struct Measure {
    pub name: &'static str,
    pub args: &'static str,
    pub means: &'static str,
}

/// Every name a formula may use. The model is shown this list, and nothing else exists.
pub const MEASURES: &[Measure] = &[
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
    Measure {
        name: "knob",
        args: "'name'",
        means: "a numeric recipe value, such as knob('lora_r')",
    },
    Measure {
        name: "abs",
        args: "x",
        means: "absolute value",
    },
    Measure {
        name: "sqrt",
        args: "x",
        means: "square root",
    },
    Measure {
        name: "ln",
        args: "x",
        means: "natural logarithm",
    },
    Measure {
        name: "exp",
        args: "x",
        means: "e to the power x",
    },
    Measure {
        name: "min",
        args: "x, y, ...",
        means: "the smallest argument",
    },
    Measure {
        name: "max",
        args: "x, y, ...",
        means: "the largest argument",
    },
];

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(f64),
    Text(String),
    Name(String),
    Call(String, Vec<Expr>),
    Neg(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
}

/// Parse a formula. The error is a plain sentence the window can show.
pub fn parse(text: &str) -> Result<Expr, String> {
    parse_open(text, &|_| None)
}

/// Parse a formula where `learned` gives the formula behind a learned tool's name.
///
/// A learned name is replaced by its own parsed formula, at most four levels deep,
/// and the whole result must still fit the size limits.
pub fn parse_open(text: &str, learned: &dyn Fn(&str) -> Option<String>) -> Result<Expr, String> {
    let expr = parse_raw(text)?;
    let expr = expand(expr, learned, 0)?;
    if size(&expr) > MAX_NODES * 4 {
        return Err("With its learned tools spelled out, the formula is too large.".to_string());
    }
    check_names(&expr)?;
    Ok(expr)
}

fn parse_raw(text: &str) -> Result<Expr, String> {
    if text.chars().count() > MAX_LEN {
        return Err(format!("A formula is at most {MAX_LEN} characters."));
    }
    let tokens = tokenize(text)?;
    let mut parser = Parser {
        tokens,
        at: 0,
        nodes: 0,
    };
    let expr = parser.sum(0)?;
    if parser.at != parser.tokens.len() {
        return Err("The formula has text after its end.".to_string());
    }
    Ok(expr)
}

fn expand(
    expr: Expr,
    learned: &dyn Fn(&str) -> Option<String>,
    depth: usize,
) -> Result<Expr, String> {
    Ok(match expr {
        Expr::Name(name) if arity(&name).is_none() => {
            let Some(formula) = learned(&name) else {
                return Err(format!("{name} is not a measure this program knows."));
            };
            if depth >= 4 {
                return Err("Learned tools nest at most four deep.".to_string());
            }
            expand(parse_raw(&formula)?, learned, depth + 1)?
        }
        Expr::Call(name, args) => Expr::Call(
            name,
            args.into_iter()
                .map(|arg| expand(arg, learned, depth))
                .collect::<Result<_, _>>()?,
        ),
        Expr::Neg(inner) => Expr::Neg(Box::new(expand(*inner, learned, depth)?)),
        Expr::Binary(op, left, right) => Expr::Binary(
            op,
            Box::new(expand(*left, learned, depth)?),
            Box::new(expand(*right, learned, depth)?),
        ),
        other => other,
    })
}

fn size(expr: &Expr) -> usize {
    match expr {
        Expr::Call(_, args) => 1 + args.iter().map(size).sum::<usize>(),
        Expr::Neg(inner) => 1 + size(inner),
        Expr::Binary(_, left, right) => 1 + size(left) + size(right),
        _ => 1,
    }
}

/// The formula written back in one canonical spelling, so two spellings of one tool compare equal.
pub fn canonical(expr: &Expr) -> String {
    match expr {
        Expr::Number(value) => format_number(*value),
        Expr::Text(text) => format!("'{text}'"),
        Expr::Name(name) => name.clone(),
        Expr::Call(name, args) => format!(
            "{name}({})",
            args.iter().map(canonical).collect::<Vec<_>>().join(", ")
        ),
        Expr::Neg(inner) => format!("-({})", canonical(inner)),
        Expr::Binary(op, left, right) => {
            format!("({} {op} {})", canonical(left), canonical(right))
        }
    }
}

/// Evaluate on one run. A non-finite value is an error, not a result.
pub fn eval(expr: &Expr, series: &Series) -> Result<f64, String> {
    let view = RunView::new(series);
    let value = view.eval(expr)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!(
            "The formula is not a finite number on {}.",
            series.name
        ))
    }
}

fn format_number(value: f64) -> String {
    format!("{value}")
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Text(String),
    Name(String),
    Op(char),
}

fn tokenize(text: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit()
            || (ch == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '-' || chars[j] == '+') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    i = j;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let literal: String = chars[start..i].iter().collect();
            let value = literal
                .parse::<f64>()
                .map_err(|_| format!("{literal} is not a number."))?;
            tokens.push(Token::Number(value));
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token::Name(chars[start..i].iter().collect()));
        } else if ch == '\'' {
            let start = i + 1;
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                if !(chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    return Err("A quoted name holds letters, digits and _ only.".to_string());
                }
                i += 1;
            }
            if i >= chars.len() {
                return Err("A quoted name is not closed.".to_string());
            }
            tokens.push(Token::Text(chars[start..i].iter().collect()));
            i += 1;
        } else if "+-*/^(),".contains(ch) {
            tokens.push(Token::Op(ch));
            i += 1;
        } else {
            return Err(format!("'{ch}' is not part of the formula language."));
        }
    }
    if tokens.is_empty() {
        return Err("The formula is empty.".to_string());
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    nodes: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn eat(&mut self, op: char) -> bool {
        if self.peek() == Some(&Token::Op(op)) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn node(&mut self, depth: usize) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(format!("A formula has at most {MAX_NODES} parts."));
        }
        if depth > MAX_DEPTH {
            return Err(format!("A formula nests at most {MAX_DEPTH} deep."));
        }
        Ok(())
    }

    fn sum(&mut self, depth: usize) -> Result<Expr, String> {
        self.node(depth)?;
        let mut left = self.product(depth + 1)?;
        loop {
            let op = if self.eat('+') {
                '+'
            } else if self.eat('-') {
                '-'
            } else {
                return Ok(left);
            };
            let right = self.product(depth + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn product(&mut self, depth: usize) -> Result<Expr, String> {
        self.node(depth)?;
        let mut left = self.unary(depth + 1)?;
        loop {
            let op = if self.eat('*') {
                '*'
            } else if self.eat('/') {
                '/'
            } else {
                return Ok(left);
            };
            let right = self.unary(depth + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn power(&mut self, depth: usize) -> Result<Expr, String> {
        self.node(depth)?;
        let base = self.atom(depth + 1)?;
        if self.eat('^') {
            let exponent = self.unary(depth + 1)?;
            return Ok(Expr::Binary('^', Box::new(base), Box::new(exponent)));
        }
        Ok(base)
    }

    fn unary(&mut self, depth: usize) -> Result<Expr, String> {
        self.node(depth)?;
        if self.eat('-') {
            return Ok(Expr::Neg(Box::new(self.unary(depth + 1)?)));
        }
        self.power(depth + 1)
    }

    fn atom(&mut self, depth: usize) -> Result<Expr, String> {
        self.node(depth)?;
        match self.peek().cloned() {
            Some(Token::Number(value)) => {
                self.at += 1;
                Ok(Expr::Number(value))
            }
            Some(Token::Text(text)) => {
                self.at += 1;
                Ok(Expr::Text(text))
            }
            Some(Token::Name(name)) => {
                self.at += 1;
                if !self.eat('(') {
                    return Ok(Expr::Name(name));
                }
                let mut args = Vec::new();
                if !self.eat(')') {
                    loop {
                        args.push(self.sum(depth + 1)?);
                        if self.eat(')') {
                            break;
                        }
                        if !self.eat(',') {
                            return Err(format!("{name}( needs a comma or a closing parenthesis."));
                        }
                    }
                }
                Ok(Expr::Call(name, args))
            }
            Some(Token::Op('(')) => {
                self.at += 1;
                let inner = self.sum(depth + 1)?;
                if !self.eat(')') {
                    return Err("A parenthesis is not closed.".to_string());
                }
                Ok(inner)
            }
            _ => Err("The formula expected a number, a name, or a parenthesis.".to_string()),
        }
    }
}

fn arity(name: &str) -> Option<(usize, usize)> {
    Some(match name {
        "low" | "low_epoch" | "median" | "q1" | "q3" | "first" | "last" | "samples" | "peak_lr"
        | "lr_at_low" | "end_epoch" => (0, 0),
        "median_between" | "mean_between" | "min_between" | "count_between" | "slope_between"
        | "lr_between" => (2, 2),
        "knob" | "abs" | "sqrt" | "ln" | "exp" => (1, 1),
        "min" | "max" => (2, 8),
        _ => return None,
    })
}

fn check_names(expr: &Expr) -> Result<(), String> {
    match expr {
        Expr::Number(_) => Ok(()),
        Expr::Text(_) => Err("A quoted name only goes inside knob('...').".to_string()),
        Expr::Name(name) => match arity(name) {
            Some((0, 0)) => Ok(()),
            Some(_) => Err(format!("{name} needs arguments in parentheses.")),
            None => Err(format!("{name} is not a measure this program knows.")),
        },
        Expr::Call(name, args) => {
            let Some((least, most)) = arity(name) else {
                return Err(format!("{name} is not a measure this program knows."));
            };
            if args.len() < least || args.len() > most {
                return Err(format!("{name} takes {}.", count_args(least, most)));
            }
            if name == "knob" {
                return match &args[0] {
                    Expr::Text(_) => Ok(()),
                    _ => {
                        Err("knob takes a quoted recipe name, such as knob('lora_r').".to_string())
                    }
                };
            }
            args.iter().try_for_each(check_names)
        }
        Expr::Neg(inner) => check_names(inner),
        Expr::Binary(_, left, right) => {
            check_names(left)?;
            check_names(right)
        }
    }
}

fn count_args(least: usize, most: usize) -> String {
    match (least, most) {
        (0, 0) => "no arguments".to_string(),
        (1, 1) => "one argument".to_string(),
        (2, 2) => "two arguments".to_string(),
        _ => format!("{least} to {most} arguments"),
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

    fn eval(&self, expr: &Expr) -> Result<f64, String> {
        match expr {
            Expr::Number(value) => Ok(*value),
            Expr::Text(_) => Err("A quoted name only goes inside knob('...').".to_string()),
            Expr::Neg(inner) => Ok(-self.eval(inner)?),
            Expr::Binary(op, left, right) => {
                let (left, right) = (self.eval(left)?, self.eval(right)?);
                Ok(match op {
                    '+' => left + right,
                    '-' => left - right,
                    '*' => left * right,
                    '/' => left / right,
                    _ => left.powf(right),
                })
            }
            Expr::Name(name) => self.measure(name),
            Expr::Call(name, args) => self.call(name, args),
        }
    }

    fn measure(&self, name: &str) -> Result<f64, String> {
        match name {
            "low" => Ok(self.lowest()?.1),
            "low_epoch" => Ok(self.lowest()?.0),
            "median" => quantile(&self.window()?, 0.5).ok_or_else(|| self.missing("window")),
            "q1" => quantile(&self.window()?, 0.25).ok_or_else(|| self.missing("window")),
            "q3" => quantile(&self.window()?, 0.75).ok_or_else(|| self.missing("window")),
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

    fn call(&self, name: &str, args: &[Expr]) -> Result<f64, String> {
        if name == "knob" {
            let Some(Expr::Text(key)) = args.first() else {
                return Err("knob takes a quoted recipe name.".to_string());
            };
            return self
                .series
                .recipe
                .get(key)
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| {
                    format!("{} has no numeric {key} in its recipe.", self.series.name)
                });
        }
        let values = args
            .iter()
            .map(|arg| self.eval(arg))
            .collect::<Result<Vec<f64>, String>>()?;
        match name {
            "abs" => Ok(values[0].abs()),
            "sqrt" => Ok(values[0].sqrt()),
            "ln" => Ok(values[0].ln()),
            "exp" => Ok(values[0].exp()),
            "min" => Ok(values.iter().copied().fold(f64::INFINITY, f64::min)),
            "max" => Ok(values.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
            _ => self.windowed(name, values[0], values[1]),
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
            "median_between" => quantile(&losses, 0.5).ok_or_else(empty),
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

/// Linear interpolation between the closest ranks of sorted `values`.
pub(crate) fn quantile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = p * (sorted.len() - 1) as f64;
    let below = rank.floor() as usize;
    let above = rank.ceil() as usize;
    Some(sorted[below] + (sorted[above] - sorted[below]) * (rank - below as f64))
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
    use crate::series::Sample;
    use serde_json::{Map, Value};

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
}
