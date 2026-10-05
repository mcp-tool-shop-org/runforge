<p align="center">
  <a href="README.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center"><img src="docs/bench-dark.png" alt="The RunForge window in the dark theme, open on a fixture folder" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# RunForge

RunForge is the Windows bench for a [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate) output folder. Open the folder that contains `run_history.json`, or the folder above an `output` directory. The window lists the runs, draws the stored loss, compares two rows, and exports the table or the curve.

Backpropagate is the trainer. This app does not contain the trainer, does not download a model, and does not ship PyTorch. When `backprop` is already on PATH, Train, Eval, and Export model start that command and follow its log. Arguments are built by the app. Nothing is passed through a shell. If `backprop` is missing, the buttons say so. RunForge does not download backpropagate, install it, or vendor it.

The curve is the stored `loss_history`, in file order, at most the samples the trainer kept. `final_loss` is a column. It is not appended to the line. A null sample is a gap, not a zero.

## Threat model

RunForge reads a folder you pick. It opens `run_history.json` in that folder, or `output/run_history.json` one level down, and it can export the table, the curve, or one entry. Preferences, the last folder and the theme, are written in the package LocalState when the app is packaged, and beside the executable when it is not. Train, Eval, and Export model start `backprop` only when you press the button and that program is already on PATH. The log is that program's output. Stop ends the process tree this window started.

Data it does not touch: the trainer, a model download, an install of backpropagate, a shell, a copy of the environment, or telemetry. There is no account.

Permissions stay on the folder you opened, the export path you pick, the data file you pick, and the preferences file. The app does not request a network capability.

How to report a vulnerability is in [SECURITY.md](SECURITY.md).

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

The design of record is [docs/CONTRACT.md](docs/CONTRACT.md). This repository supports the 2.0.0 source build. The published Store app stays the 1.0.1 classifier until a package above `1.0.1.0` is submitted.

Built by [MCP Tool Shop](https://mcp-tool-shop.github.io/).
