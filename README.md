<p align="center">
  <a href="README.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/mcp-tool-shop-org/runforge"><img src="https://codecov.io/gh/mcp-tool-shop-org/runforge/graph/badge.svg" alt="Coverage"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# RunForge

RunForge is a Windows instrument for fine-tuning runs. Open a folder of runs and it draws every stored sample, writes a report that leads with its answer, and gives a local model a workbench. On the workbench the model builds its own formula tools, proposes what each knob does, and gathers evidence across folders until a verdict is earned.

RunForge reads training records. It does not train. It does not download a model, ship PyTorch, or call a cloud model.

## The report

Open a folder of `run-config*.json` files. They can sit in the folder itself or one level down. RunForge draws every finite sample, with no resampling, and a gap stays a gap. `training_summary.final_loss` is a marker beside the curve, never a point on it, and the report never ranks by it.

The report opens with **In short**: whether a run wins, and why. For five seeds of one recipe that reads like this:

> No run wins. Seed 512 has the deepest single point, 0.0674 at epoch 3. Seed 1024 has the calmest stretch around its low: a middle of 0.3474, against 0.4041 around seed 512's low. The runs' middles sit within 0.1508 of each other: wider than the calmest run's middle half (0.1133), narrower than the noisiest run's (0.3238). The runs only partly separate.

After that, the report sets out:
- the runs
- why one wins or none does
- what changed and what did not
- earlier weighings of the same recipe
- what to do next
- what the report cannot tell you
- where each formula comes from

A section with nothing to say is not printed. A setting that was the same on every run is listed as untested, never as a lever. The pane and Save report hold the same words.

## The workbench

Press **Ask** and a local model investigates the runs through five tools: measure a formula, compare a knob that changed, keep a new tool, propose a hypothesis, and finish. The program computes every result and writes every sentence that carries a number. The model chooses what to look at and puts it into words. Its closing note is labeled as its words, not a measurement.

**Tools the model builds.** A tool is a formula in a small language evaluated once per run, for example:
- `last / low`: how far the curve climbs after its low
- `slope_between(end_epoch - 1, end_epoch)`: how steep the last epoch is
- `knob('lora_r')`: a recipe value

A formula cannot read a file, open the network, or run code. A new tool is kept only if it gives a value on every run and isn't a duplicate. It stays provisional until it's used on a second folder. Later formulas can use it by name, so the library grows with the data. You can try a formula yourself in the pane's formula box.

**Hypotheses about knobs.** A hypothesis names a knob, a formula and a direction, for example "when LoRA rank goes up, `last / low` goes lower". Its test is fixed when it is proposed. On each folder the program marks it one of:
- not testable: the knob did not change
- confounded: another knob changed with it
- inconclusive
- or a result on those runs alone

When the runs cannot settle a hypothesis, RunForge plans the smallest set of runs that would: one knob, two settings, three seeds or more each. It never starts them.

**Evidence across folders.** Each folder gives an e-value: a measure of evidence that averages exactly 1 when the knob does nothing, so it can be multiplied across folders without losing validity. Two kinds of folder are left out:
- any folder holding a run RunForge had seen when the hypothesis was registered
- a run already counted

Verdicts, supported or refuted, are issued only at checkpoints, one every five new folders. Each checkpoint applies e-BH at a 5% false discovery rate over both directions of every hypothesis on the bench. A single hypothesis needs about three clean folders of three runs per setting. The method, its sources, and an outside review are in [docs/sidecar-workbench.md](docs/sidecar-workbench.md) and [docs/evidence.consult.response.md](docs/evidence.consult.response.md).

The model is a local Ollama on `127.0.0.1:11434`. RunForge uses only a model that Ollama reports can call tools, and skips cloud-tagged names. A session is capped at six requests and ten tool calls. Without a local model the report still stands, and so does the formula box.

## The history bench

A folder with a backpropagate `run_history.json` opens the history bench instead, or the folder above an `output` directory. The bench lists the runs, draws the stored `loss_history` in file order, compares two rows, and exports the table or the curve.

When `backprop` is already on PATH, Train, Eval, and Export model on this bench start that command and follow its log. The app builds the arguments, and nothing passes through a shell. If `backprop` is missing, the buttons say so. RunForge does not download, install, or vendor backpropagate.

<p align="center"><img src="docs/bench-dark.png" alt="The history bench in the dark theme, open on a fixture folder" width="720"></p>

## Threat model

**What it reads.** A folder you pick:
- a history folder: `run_history.json` there, or `output/run_history.json` one level down
- a series folder: `run-config*.json` there and in its immediate children

It does not walk the rest of the disk, and it never writes back into those files. Export and Save report write to a path you pick.

**What it keeps.** Preferences (the last folder and the theme) and `sidecar-memory.json` sit together: in the package's LocalState when the app is packaged, and beside the executable when it is not. The memory file holds no folder path. It holds:
- the model's notes
- each measured weighing
- the learned formula tools (at most 50)
- the hypotheses with their test results (at most 60)
- the checkpoints

**What reaches the local model.** Run names, recipe values, and the program's own tool results go to loopback port `11434`. The folder path does not. The model can call only the five workbench tools, and the program validates every call.

**What it starts.** Train, Eval, and Export model start `backprop` only when you press the button on the history bench and the program is already on PATH. Stop ends the process tree this window started. The sidecar never presses those buttons.

**What it never touches.** The package manifest does not request `internetClient`. There is no telemetry and no account, and the report's reference list is built into the program. The app has no cloud model, no shell, no copy of the environment, no trainer, and no model download.

How to report a vulnerability is in [SECURITY.md](SECURITY.md).

## Build

Rust 1.98.1, edition 2024. The toolchain file pins it.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

Coverage gates:
- CI fails under 90% line coverage.
- Codecov holds both the project and each pull request's new lines to 90%.

The tests run the model loop against a fake Ollama on loopback. A session against a real local model is opt-in:

```bash
RUNFORGE_LIVE_FOLDER=<series folder> cargo test -p runforge --features live live_workbench -- --nocapture
```

## Store

The published listing is product `9PHL1HX0CGMF`, package `mcp-tool-shop.RunForge-Desktop`. Version 2 replaces the earlier classifier app, and the listing text has to say so in the same submission. The published Store app stays the 1.0.1 classifier until a package above `1.0.1.0` is submitted. The design of record is [docs/CONTRACT.md](docs/CONTRACT.md).

Built by [MCP Tool Shop](https://mcp-tool-shop.github.io/).
