---
title: Reference
description: Argument lists, the fixed sentences, and the history fields the bench understands.
sidebar:
  order: 3
---

The buttons do not accept extra flags. There is no `--allow-*` switch and no silent, normal, verbose, or debug mode. The log is the child program's output as produced.

## Argument lists

Train, with a model and a step count:

```text
backprop train --data notes.json --model small --steps 12 --output out
```

Train, when model and steps are blank. Those flags are omitted. Backpropagate keeps its own defaults:

```text
backprop train --data notes.json --output out
```

Eval and Export model:

```text
backprop eval newer --output out
backprop export ckpt --output out
```

`FILE`, the model name, the run id, the checkpoint, and `DIR` are passed as themselves. A value that is empty, contains a null, or begins with `-` is refused with `That value cannot be passed as an argument.`

## Sentences

| The window says | When |
| --- | --- |
| `Open a folder first.` | Train, Eval, or Export model before a folder is open. |
| `Choose a data file.` | Train with an empty data path. |
| `Steps must be a positive whole number.` | Steps is not empty, and it is not a positive whole number. |
| `backprop is not on PATH. RunForge does not install it.` | The lookup found no `backprop.exe`, `backprop.com`, or extensionless `backprop`. |
| `Select a run.` | Eval with no selected run. |
| `The selected run has no checkpoint.` | Export model when `checkpoint_path` is empty. |
| `A command is already running.` | A second start while the first is live. |
| `No command is running.` | Stop when nothing was started. |
| `backprop could not be started.` | The process could not be started, including a missing kill-on-close assignment. |
| `That value cannot be passed as an argument.` | A path or name that cannot be passed as one argument. |
| `no run_history.json in this folder` | Open found neither location. |
| `run history must be a JSON array` | The file parsed, and the top value is not an array. |
| `run history is not valid JSON` | The file is not JSON. A duplicate key is this refusal too. |
| `could not read run_history.json` | The file was found and could not be read. |

A bad entry after a successful parse is skipped and counted. One bad entry does not hide the rest.

## Fields

| Field | On screen |
| --- | --- |
| `run_id` | Identity. A non-string or missing id skips the entry. |
| `status` | `running`, `completed`, `failed`, or the raw string. |
| `session_kind` | `single_run`, `multi_run`, or the raw string. |
| `model_name` | List and compare. |
| `dataset_info` | Dataset line on the selected run. Not a list column. |
| `started_at`, else `timestamp` | List order. Newest first. Unparseable sorts last. |
| `completed_at` | Shown when present. |
| `duration_seconds` | Shown when it is a finite number. |
| `steps` | Text, when it is a string, number, or boolean. Empty is omitted. |
| `final_loss` | The list column. Not a point on the curve. |
| `loss_history` | The curve, in stored order. |
| `hyperparameters` | Compare shows keys whose parsed values differ. |
| `failure_reason` | Shown on a failed run. |
| `checkpoint_path` | Shown as text. Export model passes it to `backprop`. |
| `export_paths` | Shown as text. |
| `dataset_hash` | Shown when present. |
| `eval` | Summary only: `held_out_loss`, `perplexity`, `task_metrics`, `eval_n`, `n_prompts`. |
| `schema_version` | Informational. Missing is normal. A value other than `"1.0"` is a note, not a refusal. |

Unknown keys are kept. They survive the JSON export. `deny_unknown_fields` is not applied, because the trainer adds keys.

`backprop runs --json` is a different shape. It has final and minimum loss and no curve. This bench reads the file. It does not shell out to build the chart.

## Where the file is

1. `run_history.json` in the folder you opened.
2. Otherwise `output/run_history.json` one level down.

Direct wins. No disk walk. No merge with `multi_run` history. That second file can be opened on its own.
