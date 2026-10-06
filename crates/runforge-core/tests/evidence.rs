//! The e-values must be valid, not just plausible. These tests check the
//! defining property exactly, then Ville's bound by simulation.

use runforge_core::{
    Direction, Evaluation, Hypothesis, LAMBDA, State, Verdict, evidence, permutation_e, record,
    threshold, verdicts,
};

/// xorshift64*: a fixed, dependency-free generator so the simulation is reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn relabelings(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn walk(n: usize, k: usize, start: usize, pick: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if pick.len() == k {
            out.push(pick.clone());
            return;
        }
        for index in start..n {
            pick.push(index);
            walk(n, k, index + 1, pick, out);
            pick.pop();
        }
    }
    let mut out = Vec::new();
    walk(n, k, 0, &mut Vec::new(), &mut out);
    out
}

/// The average of the e-value over every relabeling of one pooled set.
fn average_over_relabelings(pool: &[f64], k: usize, direction: Direction) -> f64 {
    let all = relabelings(pool.len(), k);
    let total: f64 = all
        .iter()
        .map(|chosen| {
            let high: Vec<f64> = chosen.iter().map(|i| pool[*i]).collect();
            let low: Vec<f64> = (0..pool.len())
                .filter(|i| !chosen.contains(i))
                .map(|i| pool[i])
                .collect();
            permutation_e(&low, &high, direction).unwrap()
        })
        .sum();
    total / all.len() as f64
}

#[test]
fn the_e_value_averages_exactly_one_under_exchangeability() {
    for (pool, k) in [
        (vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], 3),
        (vec![0.3, 0.3, 0.1, 0.5, 0.5, 0.2, 0.9], 3),
        (vec![1.0, 2.0], 1),
        (vec![0.2, 0.4, 0.1, 0.8, 0.6], 2),
    ] {
        for direction in [Direction::Lower, Direction::Higher] {
            let mean = average_over_relabelings(&pool, k, direction);
            assert!((mean - 1.0).abs() < 1e-12, "{pool:?} k={k}: mean {mean}");
        }
    }
}

#[test]
fn a_clean_separation_earns_evidence_and_a_reversal_spends_it() {
    let low = [0.3, 0.32, 0.31];
    let high = [0.1, 0.12, 0.11];
    let clean = permutation_e(&low, &high, Direction::Lower).unwrap();
    // The 20 relabelings of a 3-by-3 split give U = 0..9 pairs with these counts.
    let counts = [1.0, 1.0, 2.0, 3.0, 3.0, 3.0, 3.0, 2.0, 1.0, 1.0];
    let mean: f64 = counts
        .iter()
        .enumerate()
        .map(|(u, c)| c * (LAMBDA * u as f64 / 9.0).exp())
        .sum::<f64>()
        / 20.0;
    assert!((clean - LAMBDA.exp() / mean).abs() < 1e-9, "{clean}");
    assert!((clean - 9.532).abs() < 0.001);
    let reversed = permutation_e(&low, &high, Direction::Higher).unwrap();
    assert!(reversed < 0.01, "{reversed}");
    let single = permutation_e(&[0.3], &[0.1], Direction::Lower).unwrap();
    assert!((single - 2.0 * LAMBDA.exp() / (1.0 + LAMBDA.exp())).abs() < 1e-12);
    assert!(permutation_e(&[0.1; 11], &[0.2; 10], Direction::Lower).is_none());
}

/// Ville's inequality: under no effect, the running product of valid e-values
/// ever reaching 1/alpha has probability at most alpha.
#[test]
fn under_no_effect_the_product_rarely_reaches_twenty() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let sequences = 4000;
    let folders = 8;
    let sizes = [(3, 3), (1, 1), (2, 3), (3, 2), (4, 4)];
    let mut crossed = 0;
    for _ in 0..sequences {
        let mut product = 1.0;
        for folder in 0..folders {
            let (a, b) = sizes[folder % sizes.len()];
            let low: Vec<f64> = (0..a).map(|_| rng.next()).collect();
            let high: Vec<f64> = (0..b).map(|_| rng.next()).collect();
            product *= permutation_e(&low, &high, Direction::Lower).unwrap();
            if product >= 20.0 {
                crossed += 1;
                break;
            }
        }
    }
    let rate = crossed as f64 / sequences as f64;
    assert!(rate <= 0.05, "crossed in {rate} of sequences");
}

#[test]
fn with_a_real_effect_the_product_gets_there() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let sequences = 1000;
    let mut crossed = 0;
    for _ in 0..sequences {
        let mut product = 1.0;
        for _ in 0..4 {
            let low: Vec<f64> = (0..3).map(|_| 0.3 + 0.1 * rng.next()).collect();
            let high: Vec<f64> = (0..3).map(|_| 0.22 + 0.1 * rng.next()).collect();
            product *= permutation_e(&low, &high, Direction::Lower).unwrap();
        }
        if product >= 40.0 {
            crossed += 1;
        }
    }
    assert!(
        crossed as f64 / sequences as f64 > 0.5,
        "{crossed} of {sequences}"
    );
}

fn evaluation(board: &str, date: &str, e_for: f64, e_against: f64, runs: &[&str]) -> Evaluation {
    Evaluation {
        date: date.into(),
        board: board.into(),
        state: State::Inconclusive,
        detail: String::new(),
        e_for: Some(e_for),
        e_against: Some(e_against),
        runs: runs.iter().map(|run| run.to_string()).collect(),
    }
}

fn hypothesis(id: &str, proposed_on: &str) -> Hypothesis {
    Hypothesis {
        id: id.into(),
        method: "bf16 LoRA".into(),
        knob: "lora_r".into(),
        formula: "low".into(),
        direction: Direction::Lower,
        why: String::new(),
        proposed: "2026-10-01".into(),
        proposed_on: proposed_on.into(),
        evaluations: Vec::new(),
    }
}

#[test]
fn the_proposal_folder_and_reused_runs_are_not_counted_and_order_is_fixed() {
    let mut h = hypothesis("h1", "A");
    record(
        &mut h,
        evaluation("A", "2026-10-01", 9.0, 0.1, &["a1", "a2"]),
    );
    record(
        &mut h,
        evaluation("B", "2026-10-02", 3.0, 0.5, &["b1", "b2"]),
    );
    record(
        &mut h,
        evaluation("C", "2026-10-03", 5.0, 0.2, &["b2", "c1"]),
    );
    record(
        &mut h,
        evaluation("D", "2026-10-04", 2.0, 0.6, &["d1", "d2"]),
    );
    let gathered = evidence(&h);
    assert_eq!(gathered.counted, vec!["2026-10-02", "2026-10-04"]);
    assert!((gathered.e_for - 6.0).abs() < 1e-12);
    assert!((gathered.e_against - 0.3).abs() < 1e-12);
    assert_eq!(gathered.left_out[0].1, "proposed on these runs");
    assert_eq!(
        gathered.left_out[1].1,
        "shares runs with a folder already counted"
    );
    // B tested again keeps its place: C still loses to B, whatever B now says.
    record(
        &mut h,
        evaluation("B", "2026-10-05", 0.5, 2.0, &["b1", "b2"]),
    );
    let again = evidence(&h);
    assert_eq!(again.counted.len(), 2);
    assert!((again.e_for - 1.0).abs() < 1e-12);
}

#[test]
fn e_bh_discovers_only_what_clears_its_share_and_reads_the_direction() {
    let mut strong = hypothesis("h1", "");
    record(&mut strong, evaluation("A", "d1", 9.5, 0.01, &["a"]));
    record(&mut strong, evaluation("B", "d2", 9.5, 0.01, &["b"]));
    let mut reversed = hypothesis("h2", "");
    record(&mut reversed, evaluation("A", "d1", 0.01, 9.5, &["a"]));
    record(&mut reversed, evaluation("B", "d2", 0.01, 9.5, &["b"]));
    let mut weak = hypothesis("h3", "");
    record(&mut weak, evaluation("A", "d1", 2.0, 0.5, &["a"]));
    let bench = vec![strong, reversed, weak];
    assert_eq!(threshold(bench.len()), 60.0);
    let results = verdicts(&bench);
    // e_any is 45.1 for the first two: the 2nd largest needs 3 / (0.05 * 2) = 30.
    assert_eq!(results[0].1, Verdict::Supported);
    assert_eq!(results[1].1, Verdict::Refuted);
    assert_eq!(results[2].1, Verdict::Open);
    let alone = verdicts(&bench[2..]);
    assert_eq!(alone[0].1, Verdict::Open);
}
