---
title: Getting Started
description: Build the 2.0.0 source tree and open a backpropagate output folder.
sidebar:
  order: 1
---

RunForge 2.0.0 is a Windows source build. It is not an npm package, and the copy on the Store today is still the older classifier app.

## What you need

Rust 1.98.1, edition 2024. The repository pins that compiler in `rust-toolchain.toml`.

A folder from backpropagate that already contains `run_history.json`, or a parent folder whose `output` directory contains that file. RunForge does not create a training run by itself.

`backprop` on `PATH` is optional. The bench reads and draws without it. Train, Eval, and Export model need the program. The app will not download it.

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

The window title is `RunForge 2.0.0`. The first size is 1200 by 780. It opens in the dark theme.

## First open

Choose the folder where backpropagate wrote the history. The file is `run_history.json` in that folder, or `output/run_history.json` one level down. There is no search of the rest of the disk, and a second history file is not merged in.

If the file is missing, the window says `no run_history.json in this folder` and the bench stays empty. A file that is not a JSON array is refused as a whole. A duplicate key inside an object refuses the whole file too.

## The Store listing

Product `9PHL1HX0CGMF`, package `mcp-tool-shop.RunForge-Desktop`, is the published listing. Version 2 replaces the classifier app, and the listing text has to say so in the same submission. That package has not been submitted. Installing from the Store today still installs the 1.0.1 classifier.

The source build in this repository is the bench described here.
