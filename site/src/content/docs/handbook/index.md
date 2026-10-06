---
title: RunForge Handbook
description: A Windows instrument for fine-tuning runs, a report that leads with its answer, and a workbench where a local model builds tools and tests what each knob does.
sidebar:
  order: 0
---

RunForge is a Windows instrument for fine-tuning runs. Open a folder of runs and it does three things:
- It draws every stored sample.
- It writes a report that starts with the answer: does a run win, and why.
- It gives a local model a workbench. There the model builds its own formula tools, proposes what each knob does, and gathers evidence across folders until a checkpoint can call a verdict.

A [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate) folder opens a different screen, the history bench. It shows the run list, the stored loss, a comparison of two rows, and export.

RunForge reads training records. It does not train, download a model, ship PyTorch, or call a cloud model.

## What you can do from here

- [Build it and open a folder](./getting-started/)
- [Read the window and the report](./usage/)
- [Work with the workbench: tools, hypotheses, evidence](./workbench/)
- [The formula language, the states, and the limits](./reference/)
- [How the crates split the work](./architecture/)
- [What the app is allowed to touch](./security/)

The [landing page](/runforge/) is the short version. The design of the workbench, its sources, and the outside review of its statistics are in `docs/sidecar-workbench.md` and `docs/evidence.consult.response.md` in the repository.

## The rules it keeps

- **Every sample stays a record.** Every finite sample is drawn, a gap is a gap, and `final_loss` is a marker beside the curve, never a rank.
- **The program writes every number.** The model chooses what to compute. The program computes it and writes the sentence. The model's own words are fenced and labeled.
- **A knob that never changed was never tested.** RunForge says so, and plans the runs that would test it.
- **A verdict is earned, not announced.** Evidence comes from new runs only. It is multiplied across folders and judged at a fixed checkpoint, at a 5% false discovery rate.

The published Store listing, product `9PHL1HX0CGMF`, is still the 1.0.1 classifier app until a package above `1.0.1.0` is submitted. This handbook describes the 2.0.0 source build in this repository.
