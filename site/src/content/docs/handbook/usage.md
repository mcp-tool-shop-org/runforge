---
title: Usage
description: How a series folder, the report, the history bench, and the three launch buttons behave.
sidebar:
  order: 2
---

One window. No account. The screen follows the folder you opened. A history file opens the bench below. Otherwise a series folder opens the instrument in this first section.

## A series folder

Every series is drawn on one epoch axis. Loss is on a log scale. Hover shows the stored epoch, loss, and learning rate. Scroll brushes an epoch range, and the learning-rate strip shares that range with its own vertical scale. The lowest sample on each series is marked. The last sample is not a substitute for that mark. Selecting a series draws it through the others.

A second row draws the stretch around the lows on one linear scale. Every sample under the ceiling is drawn there. A bold line follows the lowest sample in each epoch, so a climb after the low stays visible in the stored units. A spike above that ceiling is a gap on that row and stays on the main chart.

The shared recipe is shown whole: the keys whose values agree across the series, once, and a key that differs stays on the series it belongs to. A key the file adds is still shown.

`training_summary.final_loss` is printed as a marker beside the curve. It is not a sample, and it is not how the report ranks.

## The report

The sidecar prints one plain-text report from the measurements. The pane shows that text. Save report writes the same string, through the same dialog the history bench uses for CSV and JSON. The report does not use markdown headings.

The report keeps the lowest stored loss on the page even when that point does not win. It weighs the half epoch around each low, and it states the learning rate on that sample. When the deepest point and the calmest neighborhood belong to different series, the report says there is no winner. That is a result of weighing. It is not a missing page.

Shared settings are listed as untested. An assumption is labeled, and it is not a result. The report cites only the reference cards a printed sentence uses. The longer list in the column stays closed. Nothing on this page is fetched.

Ask may add one short paragraph on how to read the page. That paragraph may not contain a digit, name a setting, or name a verdict. A fresh note that crosses the line is dropped, and the pane says the note was omitted. A stored note that crosses the line is not shown. Opening a folder does not replace a richer note. The sidecar does not press Train.

## Before a folder is open

The window asks you to open the folder where backpropagate wrote `run_history.json`. Train, Eval, and Export model answer `Open a folder first.` until you do.

## The list

Each row shows status, model, final loss, and the started time. Newest first, by `started_at` or else `timestamp`. A time that cannot be parsed sorts last.

Status is `running`, `completed`, or `failed`, or the raw string when it is something else. A missing or non-string `run_id` skips that entry. The window shows the skipped count when it is not zero. Two rows can share a `run_id`. Both stay, and the window says the file has duplicate ids.

`dataset_info` is the Dataset line on the selected run. It is not a column in the list. `steps` is shown as text when it is a string, a number, or a boolean. Empty is omitted. `final_loss` is the list column. A null there leaves the cell empty.

## The chart

The caption is fixed: "The chart is the stored samples, in file order."

The line is `loss_history` against the stored-sample index. The trainer usually keeps at most 100 points. A longer series is still drawn as stored. RunForge does not resample it and does not append `final_loss`.

A null sample, or a non-finite number, is a gap. The gap is not drawn as zero. A run with an empty `loss_history` stays in the list, and the chart says there is no stored loss.

## Compare

Pick two rows. Both series are drawn on one chart, by sample index. Compare lists hyperparameters whose parsed values differ, and it shows eval summaries when the entry has them. Rows are addressed by their place in the file, so a repeated `run_id` does not hide one of them.

## Export

Three exports, each to a path you pick:

- The list, as CSV.
- The selected curve, as CSV: index, loss, and an empty cell for a gap.
- The selected entry, as JSON. Keys the trainer added are kept.

RunForge does not edit `run_history.json`.

## Train, Eval, and Export model

These appear once a folder is open. The form has four fields: model name, data file, step count, and the open output folder. Every other training flag stays at backpropagate's default.

The buttons build an argument list and start `backprop`. Nothing is passed through a shell. `.cmd` and `.bat` are not chosen. The child is not given a copy of the environment. The log is that program's output, capped in the window, and it stays on screen while the command runs.

Blank model and blank steps are left off the Train command. Steps, when you type them, must be a non-empty run of ASCII digits with at least one digit that is not zero.

Eval uses the selected run's id. Export model uses that run's `checkpoint_path`. The app does not open the checkpoint itself.

Stop ends the process tree this window started. It does not send a separate cancel protocol into backpropagate. If kill-on-close cannot be assigned, the start is refused before the process runs, and the window says `backprop could not be started.`

## Preferences

The last folder and the theme are the only preferences. A packaged run writes them in that package's LocalState. An unpackaged run writes them beside the executable. If the executable path cannot be read, they fall back to the process temporary directory. The history file is never stored there.
