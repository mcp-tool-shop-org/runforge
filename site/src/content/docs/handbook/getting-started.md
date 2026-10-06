---
title: Getting Started
description: Build the 2.0.0 source tree and open a series folder or a backpropagate output folder.
sidebar:
  order: 1
---

RunForge 2.0.0 is a Windows source build. It is not an npm package, and the copy on the Store today is still the older classifier app.

## What you need

Rust 1.98.1, edition 2024. The repository pins that compiler in `rust-toolchain.toml`.

A series folder that already contains `run-config` files, or a backpropagate folder that already contains `run_history.json`. RunForge does not create a training run by itself.

`backprop` on `PATH` is optional. Both screens read and draw without it. Train, Eval, and Export model, on the history bench only, need the program. The app will not download it. A local Ollama on `127.0.0.1` port `11434` is optional. The series report is written without it. Ask uses it, and a cloud-tagged name is not chosen.

## Build

From the repository:

```bash
cargo test --locked --workspace
cargo run -p runforge --locked
```

CI also runs coverage and fails the status when line coverage is under 90%:

```bash
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
```

The file dialog is not opened by the tests. A local check of format, clippy, the tests, and a release build is `scripts/verify.ps1`.

The window title is `RunForge 2.0.0`. The first size is 1480 by 960. It opens in the dark theme.

## First open

Choose one folder. RunForge does not search the rest of the disk, and it does not merge the two record kinds.

A history file wins when it is present. That file is `run_history.json` in the folder, or `output/run_history.json` one level down. Direct wins. A second history file is not merged in. A file that is not a JSON array is refused as a whole. A duplicate key inside an object refuses the whole history file too.

When there is no history file, RunForge reads `run-config*.json` in the folder and the same names one level down in a child folder. One bad series file is skipped and counted. A duplicate key refuses that file only. If neither record is there, the window says `no run_history.json in this folder, and no run-config series in this folder`.

## The Store listing

Product `9PHL1HX0CGMF`, package `mcp-tool-shop.RunForge-Desktop`, is the published listing. Version 2 replaces the classifier app, and the listing text has to say so in the same submission. That package has not been submitted. Installing from the Store today still installs the 1.0.1 classifier.

The source build in this repository is the instrument described here. The history bench is the screen a backpropagate folder opens.
