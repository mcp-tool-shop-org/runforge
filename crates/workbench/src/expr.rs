//! A small formula language for tools the sidecar defines.
//!
//! A learned tool is a formula, not a program. It is parsed and evaluated here,
//! once per run, over the measures the host names and the run's knobs. It cannot
//! read a file, open the network, loop, or call anything outside that list. A
//! formula that fails to parse, or gives a non-finite value on a run, is refused.
//!
//! Grammar, lowest precedence first:
//!   sum     = product (("+" | "-") product)*
//!   product = unary (("*" | "/") unary)*
//!   unary   = "-" unary | power
//!   power   = atom ("^" unary)?
//!   atom    = number | name | name "(" args ")" | "(" sum ")"
//!   args    = (sum | 'text') ("," (sum | 'text'))*

use crate::board::Board;

const MAX_LEN: usize = 240;
const MAX_NODES: usize = 64;
const MAX_DEPTH: usize = 16;

/// One name a formula may use, documented for the model and the window.
///
/// `args` is how the arguments read in the prompt: empty for none, `a, b` for two,
/// `x, y, ...` for two to eight, `'name'` for one quoted name.
pub struct Measure {
    pub name: &'static str,
    pub args: &'static str,
    pub means: &'static str,
}

impl Measure {
    fn arity(&self) -> (usize, usize) {
        if self.args.trim().is_empty() {
            (0, 0)
        } else if self.args.contains("...") {
            (2, 8)
        } else {
            let n = self.args.split(',').count();
            (n, n)
        }
    }
}

/// The names every host has: a run's knob, and arithmetic.
pub const BUILTINS: &[Measure] = &[
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

/// Every name a formula over this host may use: the host's measures, then the built-ins.
/// A host measure that reuses a built-in's name is left out, so a host cannot shadow one.
pub fn catalogue(host: &[Measure]) -> Vec<&Measure> {
    host.iter()
        .filter(|measure| !BUILTINS.iter().any(|builtin| builtin.name == measure.name))
        .chain(BUILTINS)
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(f64),
    Text(String),
    Name(String),
    Call(String, Vec<Expr>),
    Neg(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
}

/// Parse a formula over the host's `measures`. The error is a plain sentence the window can show.
pub fn parse(text: &str, measures: &[Measure]) -> Result<Expr, String> {
    parse_open(text, measures, &|_| None)
}

/// Parse a formula where `learned` gives the formula behind a learned tool's name.
///
/// A learned name is replaced by its own parsed formula, at most four levels deep,
/// and the whole result must still fit the size limits.
pub fn parse_open(
    text: &str,
    measures: &[Measure],
    learned: &dyn Fn(&str) -> Option<String>,
) -> Result<Expr, String> {
    let names = catalogue(measures);
    let expr = parse_raw(text)?;
    let expr = expand(expr, &names, learned, 0)?;
    if size(&expr) > MAX_NODES * 4 {
        return Err("With its learned tools spelled out, the formula is too large.".to_string());
    }
    check_names(&expr, &names)?;
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
    names: &[&Measure],
    learned: &dyn Fn(&str) -> Option<String>,
    depth: usize,
) -> Result<Expr, String> {
    Ok(match expr {
        Expr::Name(name) if arity(&name, names).is_none() => {
            let Some(formula) = learned(&name) else {
                return Err(format!("{name} is not a measure this program knows."));
            };
            if depth >= 4 {
                return Err("Learned tools nest at most four deep.".to_string());
            }
            expand(parse_raw(&formula)?, names, learned, depth + 1)?
        }
        Expr::Call(name, args) => Expr::Call(
            name,
            args.into_iter()
                .map(|arg| expand(arg, names, learned, depth))
                .collect::<Result<_, _>>()?,
        ),
        Expr::Neg(inner) => Expr::Neg(Box::new(expand(*inner, names, learned, depth)?)),
        Expr::Binary(op, left, right) => Expr::Binary(
            op,
            Box::new(expand(*left, names, learned, depth)?),
            Box::new(expand(*right, names, learned, depth)?),
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

/// Evaluate on run `run` of the board. A non-finite value is an error, not a result.
pub fn eval(expr: &Expr, board: &Board, run: usize) -> Result<f64, String> {
    let value = eval_inner(expr, board, run)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!(
            "The formula is not a finite number on {}.",
            board.runs[run].name
        ))
    }
}

fn eval_inner(expr: &Expr, board: &Board, run: usize) -> Result<f64, String> {
    match expr {
        Expr::Number(value) => Ok(*value),
        Expr::Text(_) => Err("A quoted name only goes inside knob('...').".to_string()),
        Expr::Neg(inner) => Ok(-eval_inner(inner, board, run)?),
        Expr::Binary(op, left, right) => {
            let (left, right) = (
                eval_inner(left, board, run)?,
                eval_inner(right, board, run)?,
            );
            Ok(match op {
                '+' => left + right,
                '-' => left - right,
                '*' => left * right,
                '/' => left / right,
                _ => left.powf(right),
            })
        }
        Expr::Name(name) => board.host.measure(run, name, &[]),
        Expr::Call(name, args) => {
            if name == "knob" {
                let Some(Expr::Text(key)) = args.first() else {
                    return Err("knob takes a quoted recipe name.".to_string());
                };
                let this = &board.runs[run];
                return this
                    .knobs
                    .get(key)
                    .and_then(serde_json::Value::as_f64)
                    .ok_or_else(|| format!("{} has no numeric {key} in its recipe.", this.name));
            }
            let values = args
                .iter()
                .map(|arg| eval_inner(arg, board, run))
                .collect::<Result<Vec<f64>, String>>()?;
            match name.as_str() {
                "abs" => Ok(values[0].abs()),
                "sqrt" => Ok(values[0].sqrt()),
                "ln" => Ok(values[0].ln()),
                "exp" => Ok(values[0].exp()),
                "min" => Ok(values.iter().copied().fold(f64::INFINITY, f64::min)),
                "max" => Ok(values.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
                _ => board.host.measure(run, name, &values),
            }
        }
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

    /// Count one part: a number, a name, a call, or an operator.
    fn part(&mut self) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(format!("A formula has at most {MAX_NODES} parts."));
        }
        Ok(())
    }

    /// `depth` counts what a reader sees nest: parentheses, call arguments, and signs.
    fn deep(&self, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err(format!("A formula nests at most {MAX_DEPTH} deep."));
        }
        Ok(())
    }

    fn sum(&mut self, depth: usize) -> Result<Expr, String> {
        self.deep(depth)?;
        let mut left = self.product(depth)?;
        loop {
            let op = if self.eat('+') {
                '+'
            } else if self.eat('-') {
                '-'
            } else {
                return Ok(left);
            };
            self.part()?;
            let right = self.product(depth)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn product(&mut self, depth: usize) -> Result<Expr, String> {
        let mut left = self.unary(depth)?;
        loop {
            let op = if self.eat('*') {
                '*'
            } else if self.eat('/') {
                '/'
            } else {
                return Ok(left);
            };
            self.part()?;
            let right = self.unary(depth)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn power(&mut self, depth: usize) -> Result<Expr, String> {
        let base = self.atom(depth)?;
        if self.eat('^') {
            self.part()?;
            let exponent = self.unary(depth + 1)?;
            return Ok(Expr::Binary('^', Box::new(base), Box::new(exponent)));
        }
        Ok(base)
    }

    fn unary(&mut self, depth: usize) -> Result<Expr, String> {
        self.deep(depth)?;
        if self.eat('-') {
            self.part()?;
            return Ok(Expr::Neg(Box::new(self.unary(depth + 1)?)));
        }
        self.power(depth)
    }

    fn atom(&mut self, depth: usize) -> Result<Expr, String> {
        self.deep(depth)?;
        self.part()?;
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

fn arity(name: &str, names: &[&Measure]) -> Option<(usize, usize)> {
    names
        .iter()
        .find(|measure| measure.name == name)
        .map(|measure| measure.arity())
}

fn check_names(expr: &Expr, names: &[&Measure]) -> Result<(), String> {
    match expr {
        Expr::Number(_) => Ok(()),
        Expr::Text(_) => Err("A quoted name only goes inside knob('...').".to_string()),
        Expr::Name(name) => match arity(name, names) {
            Some((0, 0)) => Ok(()),
            Some(_) => Err(format!("{name} needs arguments in parentheses.")),
            None => Err(format!("{name} is not a measure this program knows.")),
        },
        Expr::Call(name, args) => {
            let Some((least, most)) = arity(name, names) else {
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
            args.iter().try_for_each(|arg| check_names(arg, names))
        }
        Expr::Neg(inner) => check_names(inner, names),
        Expr::Binary(_, left, right) => {
            check_names(left, names)?;
            check_names(right, names)
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

/// Linear interpolation between the closest ranks of sorted `values`.
pub fn quantile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = p * (sorted.len() - 1) as f64;
    let below = rank.floor() as usize;
    let above = rank.ceil() as usize;
    Some(sorted[below] + (sorted[above] - sorted[below]) * (rank - below as f64))
}
