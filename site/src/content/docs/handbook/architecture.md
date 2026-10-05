---
title: Architecture
description: The library parses the history. The window draws it and, on request, starts backprop.
sidebar:
  order: 4
---

Two crates. The library never opens a window. The binary never decides that an unknown key is illegal.

## `runforge-core`

The library owns the file. It finds `run_history.json` at the open folder or one level down under `output`, parses it, and returns rows the window can draw. Errors use `thiserror`. A bad file is a `Result`, not a panic.

`serde_json` is built with `float_roundtrip`, so a finite loss that survives a JSON round trip keeps its bits. Numbers are compared by those bits. `f64` ordering uses `total_cmp` where a rank is required. The curve itself is not sorted. Sample order is the file order.

A duplicate key inside a JSON object refuses the whole file. The parser that would keep the last value would also hide the first `run_id`. After a file parses, a single bad entry is skipped and counted.

`final_loss` is stored beside `loss_history`. The library does not append it. A null or non-finite sample stays a gap.

Unknown keys on an entry are retained. That is the opposite of a world file that must refuse an extra key. The trainer adds fields. Export of one entry writes them back out.

## `runforge`

The binary is the window: `eframe` 0.36, `egui_plot` 0.37, `rfd` 0.15. `anyhow` is allowed here and not in the library. The toolchain is Rust 1.98.1, edition 2024. `Cargo.lock` is committed, and CI builds with `--locked`.

The window draws the list, the chart, compare, and export. It does not recompute the curve. The caption stays "The chart is the stored samples, in file order."

The launcher is a separate path, used only by Train, Eval, and Export model. It walks `PATH` for an absolute `backprop.exe`, `backprop.com`, or extensionless `backprop`, and it skips `.cmd` and `.bat`. The walk does not block the window. A click waits for the last finished answer instead of reporting the tool missing early.

The child is started with an argument list. There is no shell and no copied environment. On Windows the child is attached to a hidden pseudoconsole so a progress line can appear while the process is still running. A carriage return updates the open line. The next reprint replaces it. The log keeps the latest 400 lines. While a command is live, the window requests a frame about every 200 milliseconds.

Stop kills the process tree this window started. The job is created with kill-on-close before the process is resumed. If that assignment fails, the function returns the start-failed sentence and does not resume the thread.

## What stays out

The package does not vendor Python, PyTorch, bun, or backpropagate. The string `backprop` in the binary is the program name the launcher looks up. It is not a copy of the trainer.

Preferences are the last folder and the theme. Packaged, they live in LocalState. Unpackaged, they live beside the executable, or in the process temporary directory when that path cannot be read. Running the debug build in place rewrites the preferences file next to that executable. The checks do not do that.

Line coverage in CI fails under 90%. The operating-system file dialog and the packaged-process success path stay uncalled.
