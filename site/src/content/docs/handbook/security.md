---
title: Security
description: The folder you pick, the process the window starts, and the things the package does not contain.
sidebar:
  order: 5
---

RunForge reads a folder you pick. It is not the trainer, and it does not phone home.

Report a vulnerability privately through [GitHub Security Advisories](https://github.com/mcp-tool-shop-org/runforge/security/advisories/new). Do not open a public issue for a security report. The supported source build is 2.0.0 in this repository. The 1.0.1 classifier on the Store is a different app and is not in this tree.

## Boundaries

| Boundary | What happens |
| --- | --- |
| Run history | `run_history.json` in the folder you open, or `output/run_history.json` one level down. No disk walk. No merge. |
| Export and the data file | Paths you pick. The history file is not rewritten. |
| Preferences | Last folder and theme. Packaged LocalState, otherwise beside the executable, otherwise the process temporary directory. |
| Train, Eval, Export model | An already-installed `backprop`, arguments built by the app, no shell, no copy of the environment. |
| Stop | The process tree this window started. If kill-on-close cannot be assigned, the process is not resumed. |
| The log | The child program's output, shown in the window. It is not sent anywhere. |
| Network | No model download, no install of backpropagate, no telemetry, no account. |

The manifest for the unsigned package asks only for `runFullTrust`, so a packaged run can read a folder you pick anywhere on the machine. It does not ask for `internetClient`.

## What the package does not contain

Python, PyTorch, bun, and backpropagate are not in the package. The binary may contain the name `backprop` because that is the program it looks up. That is not a vendored trainer.

Upstream licence texts that carry a contact address are left out of the package. The NOTICE keeps the SPDX identifier for those terms.

## What a bad file can do

A hostile `run_history.json` can make the window show text from that file: model names, failure reasons, log-shaped fields. It cannot make the app execute that text. Arguments that begin with `-`, contain a null, or are empty are refused before a process starts.

The app does not follow a path inside the JSON to read a second history. Checkpoint text is shown, and Export model passes that string only when you press the button and `backprop` is already installed.
