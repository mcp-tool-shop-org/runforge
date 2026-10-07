---
title: Architecture
description: The library parses a history or a series folder. The window draws it, writes the report, and, on request, starts backprop.
sidebar:
  order: 5
---

Three crates. The libraries never open a window. The binary never decides that an unknown key is illegal.

## `runforge-core`

The library owns the file. It finds `run_history.json` at the open folder or one level down under `output`, parses it, and returns rows the window can draw. Errors use `thiserror`. A bad file is a `Result`, not a panic.

`serde_json` is built with `float_roundtrip`, so a finite loss that survives a JSON round trip keeps its bits. Numbers are compared by those bits. `f64` ordering uses `total_cmp` where a rank is required. The curve itself is not sorted. Sample order is the file order.

A duplicate key inside a JSON object refuses the whole file. The parser that would keep the last value would also hide the first `run_id`. After a file parses, a single bad entry is skipped and counted.

`final_loss` is stored beside `loss_history`. The library does not append it. A null or non-finite sample stays a gap.

Unknown keys on an entry are retained. That is the opposite of a world file that must refuse an extra key. The trainer adds fields. Export of one entry writes them back out.

A series folder is the other reader. It loads `run-config*.json` from the opened folder and from immediate child folders. A recipe that nests LoRA settings under `lora`, or uses `lr` and `schedule`, is read as the usual knobs. Each sample keeps its axis, loss, learning rate, and every other logged field.

The rest of the library is the instrument and the workbench. None of it calls a model or opens the network.

| Module | What it owns |
| --- | --- |
| `weigh` | The window around each low: median, quartiles, the spread between runs, and the reference cards. |
| `report` | The report text: the string the window shows and the string Save report writes. |
| `ledger` | Each measured weighing, keyed by the runs' identities, for the report's earlier weighings. |
| `expr` | The loss measures a formula can read from one run: the low, the window around it, and measures over a span of epochs. |
| `bench` | RunForge's side of the workbench: it hands the open series to the `workbench` crate as runs with knobs, and keeps RunForge's memory file. |

## `workbench`

The workbench itself is a crate of its own, shared with ScalarScope. It knows nothing about losses or latencies. A host names its measures and computes them, and hands over its runs with their knobs.

| Module | What it owns |
| --- | --- |
| `expr` | The formula language: a parser with size and depth caps, `knob` and arithmetic, over the host's measures. |
| `bench` | The statistics, learned tools, hypotheses, per-folder tests, e-values, evidence across folders, e-BH, and checkpoints. |
| `session` | One workbench session: the tool schemas per phase, the prompts, and what each tool call does. |
| `ollama` | The loop against a local Ollama on 127.0.0.1. It refuses cloud tags and names the model and its digest. |

Each module knows only what it needs. `expr` knows nothing about tools, `bench` knows nothing about chat, `session` knows nothing about HTTP, and `ollama` knows nothing about statistics.

## `runforge`

The binary is the window: `eframe` 0.36, `egui_plot` 0.37, `rfd` 0.15. `anyhow` is allowed here and not in the library. The toolchain is Rust 1.98.1, edition 2024. `Cargo.lock` is committed, and CI builds with `--locked`.

A history folder draws the list, the chart, compare, and export. It does not recompute the curve. The caption stays "The chart is the stored samples, in file order."

A series folder draws every sample, the shared recipe, the low row, the report, and the workbench pane. The report is generated again from the measurements. It is not a paraphrase stored beside them.

Ask runs a session on a background thread in `sidecar`:
- It talks only to a local Ollama on `127.0.0.1:11434`, through `/api/chat` with tools.
- It picks a model that `/api/show` says can call tools, and drops cloud-tagged names.
- It sends each call to `session` and returns the program's answer to the model.
- When the session ends, the app keeps the learned tools and hypotheses in `sidecar-memory.json`, beside the weighings and the checkpoints.

Opening a folder does three things: it retests the stored hypotheses for its method, counts a new folder toward the next checkpoint, and records the weighing.

The launcher is a separate path, used only by Train, Eval, and Export model. It walks `PATH` for an absolute `backprop.exe`, `backprop.com`, or extensionless `backprop`, and it skips `.cmd` and `.bat`. The walk does not block the window. A click waits for the last finished answer instead of reporting the tool missing early.

The child is started with an argument list. There is no shell and no copied environment. On Windows the child is attached to a hidden pseudoconsole so a progress line can appear while the process is still running. A carriage return updates the open line. The next reprint replaces it. The log keeps the latest 400 lines. While a command is live, the window requests a frame about every 200 milliseconds.

Stop kills the process tree this window started. The job is created with kill-on-close before the process is resumed. If that assignment fails, the function returns the start-failed sentence and does not resume the thread.

## What stays out

The package does not vendor Python, PyTorch, bun, or backpropagate. The string `backprop` in the binary is the program name the launcher looks up. It is not a copy of the trainer.

Preferences are the last folder and the theme. Packaged, they live in LocalState. Unpackaged, they live beside the executable, or in the process temporary directory when that path cannot be read. Running the debug build in place rewrites the preferences file next to that executable. The checks do not do that.

Line coverage in CI fails under 90%. The operating-system file dialog and the packaged-process success path stay uncalled.
