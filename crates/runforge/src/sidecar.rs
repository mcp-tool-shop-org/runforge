//! The in-window sidecar. The loop is the shared workbench's: it talks only to a
//! local Ollama on 127.0.0.1, refuses cloud tags, and names the model and its
//! digest. This module does not press Train.

pub use runforge_core::{BenchReply, start_bench};

#[cfg(all(test, feature = "live"))]
mod tests {
    use runforge_core::BenchReply;

    /// Live run against the local Ollama on a real folder. Opt-in, and not built in CI:
    /// RUNFORGE_LIVE_FOLDER=<series folder> cargo test -p runforge --features live live_workbench -- --nocapture
    #[test]
    fn live_workbench() {
        let folder = std::env::var("RUNFORGE_LIVE_FOLDER").expect("RUNFORGE_LIVE_FOLDER");
        let board = runforge_core::load_series_folder(std::path::Path::new(&folder)).unwrap();
        let bench = runforge_core::Workbench::new(
            runforge_core::bench_board(&board),
            Vec::new(),
            Vec::new(),
            "2026-10-06",
        );
        let started = std::time::Instant::now();
        match runforge_core::start_bench(bench).recv().unwrap() {
            BenchReply::Done {
                model,
                digest,
                bench,
                stopped,
            } => {
                println!(
                    "model {model} {digest:?}, {stopped} ({:.0} s)",
                    started.elapsed().as_secs_f64()
                );
                for step in &bench.steps {
                    println!("\n> {} {}\n{}", step.tool, step.args, step.result);
                }
                println!(
                    "\nlearned: {:?}",
                    bench
                        .learned
                        .iter()
                        .map(|t| (&t.name, &t.formula))
                        .collect::<Vec<_>>()
                );
                println!(
                    "proposed: {:?}",
                    bench
                        .proposed
                        .iter()
                        .map(|h| (h.statement_on(bench.board()), h.state().map(|s| s.word())))
                        .collect::<Vec<_>>()
                );
                println!("note: {:?} dropped: {}", bench.note, bench.note_dropped);
            }
            BenchReply::Absent(text) => println!("absent: {text}"),
        }
    }
}
