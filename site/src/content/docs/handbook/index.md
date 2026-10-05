---
title: RunForge Handbook
description: What the Windows bench is, what it reads, and what it refuses to contain.
sidebar:
  order: 0
---

RunForge is the Windows bench for a [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate) output folder. Open one folder. The window lists the runs, draws the stored loss, compares two rows, and can export the table.

Backpropagate is the trainer. This app is the place you look at what that trainer already wrote. The two products stay separate, including on the Microsoft Store.

## What you can do from here

- [Build it and open a folder](./getting-started/)
- [Read the window](./usage/)
- [The commands and the sentences](./reference/)
- [How the two crates split the work](./architecture/)
- [What the app is allowed to touch](./security/)

The [landing page](/runforge/) is the short version of the same product. The design of record is `docs/CONTRACT.md` in the repository.

## What this app is not

It does not contain the trainer. It does not download a model. It does not ship Python, PyTorch, or bun. When Train, Eval, or Export model is pressed, the window starts `backprop` only if that program is already on `PATH`. If it is missing, the button says so. RunForge does not install it.

The curve is the stored `loss_history`, in file order. `final_loss` is a column in the list. It is not another point on the line. A null sample is a gap, not a zero.

The published Store listing, product `9PHL1HX0CGMF`, is still the 1.0.1 classifier app until a package above `1.0.1.0` is submitted. This handbook describes the 2.0.0 source build in this repository.
