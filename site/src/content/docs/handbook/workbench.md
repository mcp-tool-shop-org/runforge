---
title: The workbench
description: How the local model builds formula tools, proposes hypotheses about knobs, and how evidence turns into a verdict across folders.
sidebar:
  order: 3
---

The workbench is where RunForge stops describing the runs and starts asking what the knobs do. A local model drives it through five tools. The program does every computation and writes every number. The model chooses what to look at, builds formulas when the built-in measures aren't enough, and states hypotheses.

## Starting a session

Open a series folder and press **Ask**. RunForge needs a local Ollama on `127.0.0.1:11434` with a model that Ollama reports can call tools. It tries these first, in order:
- `qwen3:14b`
- `qwen2.5:14b`
- `qwen3:8b`
- `qwen2.5:7b`
- `llama3.1:8b`
- `hermes3:8b`

After those, it tries any other local model that reports tool support. Cloud-tagged names are skipped.

```bash
ollama pull qwen3:14b
```

A session runs in four phases, and each phase offers only a few tools:

| Phase | Rounds | Tools offered | What the model is asked to do |
| --- | --- | --- | --- |
| Look | 1–2 | measure, compare_knob, learn_tool, finish | Measure the runs |
| Build | 3 | learn_tool, measure | Build at least one formula the listed measures don't capture |
| Propose | 4–5 | propose_hypothesis, learn_tool, measure, finish | State hypotheses from what it measured |
| Close | 6 | finish | End with a note in words |

A session is capped at six requests and ten tool calls. On a 14B model it usually takes a few seconds. The pane's Workbench section shows every call and the program's answer under "Last session".

## The five tools

| Tool | What the program does |
| --- | --- |
| `measure(formula)` | Evaluates the formula on every run. On a folder where only the seed changed, it also reports the spread across runs as seed noise. |
| `compare_knob(knob, formula)` | Compares the runs at the lowest and highest setting of a knob that changed. It reports the medians, the gap, the widest seed spread inside one setting, the pair counts, and an exact one-sided rank test each way. It refuses a knob that didn't change. |
| `learn_tool(name, formula, meaning)` | Keeps a new formula as a named tool, after the checks below. |
| `propose_hypothesis(knob, formula, knob_change, formula_moves, why)` | Records a hypothesis, tests it on these runs, and, when the runs cannot settle it, plans the runs that would. |
| `finish(note)` | Ends the session. The note may not carry a digit, markdown, or a verdict word. |

## Tools the model builds

A learned tool is a formula, not a program. It is evaluated once per run over that run's stored samples, and it can use earlier tools by name:

```text
post_low_recovery = last / low
late_slope        = slope_between(end_epoch - 1, end_epoch)
settle_ratio      = median_between(end_epoch - 1, end_epoch) / low
```

Before a tool is kept, the program checks that:
- the name is new
- the formula parses within the size limits
- it gives a finite value on every run of the open folder
- it is not a duplicate of an existing tool, either by formula or by the values it gives on these runs

A new tool is provisional. It becomes kept once it is used on a second folder. When the library passes 50 tools, the least-used provisional tool is trimmed first.

You can try any formula yourself: type it into the formula box and press **Run**.

## Hypotheses

A hypothesis is a claim about one knob and one formula, such as "when LoRA rank goes up, `post_low_recovery` goes lower". The model gives it in its own framing ("raise the knob, the formula goes down"), and the program turns that into the direction, so the reason and the claim cannot disagree. The knob, the formula and the direction are fixed when the hypothesis is registered. A later look at the data cannot change what is being tested.

On each folder, the program marks the hypothesis one of:

| On these runs | Meaning |
| --- | --- |
| not testable here | The knob had the same value on every run, or the formula has no value. |
| confounded | Another knob changed along with it. |
| inconclusive | One run per setting, a gap inside the seed spread, fewer than three runs per setting, or the rank test does not reach one in twenty. |
| passes its test on these runs alone | The exact rank test passes at 0.05, after Holm's adjustment across the hypotheses tested on these runs. |
| goes the other way on these runs alone | The same test passes in the opposite direction. |

When a hypothesis is not settled, the session proposes a run plan: one knob, two settings, at least three seeds each, everything else as in the open folder. When a seed spread exists, the seed count comes from it (Colas et al. 2018). RunForge never starts the runs. It tells you what to run.

## Evidence across folders

One small folder rarely settles a knob, so evidence is gathered as e-values. Each folder gives one for each direction. If the knob does nothing, it averages exactly 1, so values from separate folders can be multiplied and the result stays valid.

```text
When LoRA rank goes up, low goes lower.
Evidence so far: 20.38 for, 0.006835 against, from two folders.
```

Which folders count:
- Only folders of **new runs**. A folder holding any run RunForge had seen when the hypothesis was registered does not count, because that data may have shaped the claim.
- Each run counts **once**. A run's identity is its seed and its first samples, so a run that kept training is still the same run.
- Folders are taken **oldest first**, and a folder opened again keeps its place, so which folders count never depends on their results.

How much evidence one folder gives depends on its size:

| Runs per setting | A clean separation gives |
| --- | --- |
| 1 against 1 | at most 1.46 |
| 3 against 3 | 4.51 |
| 5 against 5 | more |

A messy folder gives less than 1 and costs evidence.

## Checkpoints and verdicts

A verdict, supported or refuted, is issued only at a checkpoint. A checkpoint comes every five new folders. That schedule is fixed in advance, so the moment of reporting is never chosen.

At each checkpoint, e-BH runs at a 5% false discovery rate over both directions of every hypothesis on the bench. A discovery in the declared direction is supported. A discovery in the opposite direction is refuted. Both are held to the same rate.

For a single hypothesis, one direction needs 40. That is about three clean folders of three runs per setting. A bench with more hypotheses needs more per hypothesis.

Between checkpoints the report shows the evidence so far and what one direction needs, never a verdict.

The guarantee assumes each knob setting was assigned to runs independently of anything else that moves the loss. Assigning settings at random makes that true by construction.

## What the model cannot do

- **It cannot print a number.** Every sentence with a number is the program's.
- **It cannot change the data.** It cannot read a file, open the network, or run code.
- **It cannot start training.**
- **It cannot crown a run.** Its closing note is shown as its words, not a measurement, and is dropped if it carries a digit or a verdict word.

The model can be wrong in words. A live session once wrote that the learning rate varies with the seed; it doesn't, the lows fall at different epochs. The label exists for that reason.
