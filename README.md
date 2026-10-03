<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

# RunForge

RunForge is the Windows bench for a [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate) output folder. Open the folder that contains `run_history.json`, or the folder above an `output` directory. The window lists the runs, draws the stored loss, compares two rows, and exports the table or the curve.

Backpropagate is the trainer. This app does not train, does not download a model, and does not ship PyTorch. A later slice may launch a `backprop` command that is already installed. That slice is not in this tree.

The curve is the stored `loss_history`, in file order, at most the samples the trainer kept. `final_loss` is a column. It is not appended to the line. A null sample is a gap, not a zero.

## Build

Rust 1.98.1, edition 2024. The toolchain file pins it.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

CI runs the coverage command and uploads `lcov.info`. Codecov fails the status when line coverage is under 90%. The file dialog is not opened by the tests.

## Store

The published listing is product `9PHL1HX0CGMF`, package `mcp-tool-shop.RunForge-Desktop`. Version 2 replaces the earlier classifier app, and the listing text has to say so in the same submission. This repository does not contain that package yet. The package identity does not change when the package is built.

The design of record is [docs/CONTRACT.md](docs/CONTRACT.md).
