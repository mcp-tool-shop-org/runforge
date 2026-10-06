# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

The README, the landing page, and the handbook describe the instrument, the report and the workbench; the handbook gains a workbench page and a reference of hypothesis states, verdicts and limits. Coverage is 93.8% of lines, with each pull request's new lines also held to 90%. No package has been submitted to Partner Center.

A formula's limits count what a reader sees: 64 parts (numbers, names, calls, operators) and 16 levels of parentheses, calls and signs. Before, they counted parser steps, so a formula with five functions was refused.

The window is an instrument. A folder of run-config series draws every sample, the shared recipe, and a local sidecar. The row around the lows draws those samples on one linear scale, with a line through the lowest sample of each epoch. A spike above that row stays on the main chart. A backpropagate `run_history.json` still opens the history bench. The sidecar does not press Train and does not call a cloud model. It prints the weighing as one plain-text report, and Save report writes those same words. A local note may add one orientation paragraph only when that paragraph has no digit, no setting, and no verdict. The report weighs each low against the samples around it, cites the local reference cards it actually uses, and labels an assumption so it is not a result. Opening a folder does not replace a stored answer.

The report leads with its answer. "In short" says whether a run wins and why, then the runs, the argument, what changed and what did not, and what to do next, each part printed only when it has something to say. Each run's stretch around its deepest point now has a middle half (first to third quartile), so the report can say whether the gap between the runs' middles is smaller than the noise inside a single run, larger, or in between. No placeholder text is printed, and no interpretation the samples do not measure.

The memory file keeps the weighing as well as the notes: each run's deepest point, its middle and middle half, and its last sample, with the day it was first weighed. A later report sets the open runs beside earlier runs of the same recipe and other recipes with the same method, and names the settings where those recipes differ. The pane's list of earlier model notes is gone. Each reference card names the one kind of sentence it may back, and the report's source list prints that with the citation.

The sidecar is a workbench. Ask runs a session in which a local model that can call tools investigates the runs: it measures with formulas, compares a knob that changed, builds new formula tools, and proposes hypotheses about what a knob does. The program computes every result and writes every sentence with a number. A learned tool is a formula in a small language (no files, no network, no loops), kept only if it has a value on every run and is not a duplicate, and provisional until it is used on a second folder. A hypothesis fixes its knob, formula and direction when proposed; the program marks it not testable, confounded, inconclusive, supported or refuted, using an exact rank test and Holm's adjustment, and plans the smallest set of runs that would settle it. Opening a folder retests the stored hypotheses for its method. The pane has a formula box, the last session's calls and the learned tools; the report lists the hypotheses and the learned tools. Recipes that nest LoRA settings under `lora` or use `lr` and `schedule` are read as the usual knobs, and two runs with one seed are named apart by file. The design and its sources are in `docs/sidecar-workbench.md`.

Evidence for a hypothesis now gathers across folders. Each folder gives a permutation e-value, with λ from a table of group sizes fixed in advance, multiplied across folders whose runs are new; a folder holding any run RunForge knew when the hypothesis was registered does not count, and a run already counted is not counted again. Verdicts are issued only at checkpoints, one every five new folders, by e-BH at a 5% false discovery rate over both directions of every hypothesis, so supported and refuted are both error-controlled. An outside review (Kimi K3, by hand) checked the method; its four fixes are in, recorded in `docs/evidence.consult.response.md`. The report prints each hypothesis's latest checkpoint verdict, its evidence so far for and against, and how many folders counted; a folder's own test is labeled "on these runs alone". Runs are identified by their seed and first samples, so two experiments that reuse a recipe and seed names are no longer mistaken for the same runs in the report's earlier weighings.

## [2.0.0]

The Windows bench reads a backpropagate `run_history.json`. It lists the runs, draws the stored loss, compares two rows, and exports the table. When `backprop` is already installed, Train, Eval, and Export model start that command, and Stop ends the process tree. The app does not contain the trainer.

Line coverage is gated at 90% in Codecov, for the project and for the patch.

The published Store app is still the 1.0.1 classifier build until a package above `1.0.1.0` is submitted on product `9PHL1HX0CGMF`.
