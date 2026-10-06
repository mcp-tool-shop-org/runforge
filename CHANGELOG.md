# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

The README, the landing page, and the handbook describe the instrument. No package has been submitted to Partner Center.

The window is an instrument. A folder of run-config series draws every sample, the shared recipe, and a local sidecar. The row around the lows draws those samples on one linear scale, with a line through the lowest sample of each epoch. A spike above that row stays on the main chart. A backpropagate `run_history.json` still opens the history bench. The sidecar does not press Train and does not call a cloud model. It prints the weighing as one plain-text report, and Save report writes those same words. A local note may add one orientation paragraph only when that paragraph has no digit, no setting, and no verdict. The report weighs each low against the samples around it, cites the local reference cards it actually uses, and labels an assumption so it is not a result. Opening a folder does not replace a stored answer.

The report leads with its answer. "In short" says whether a run wins and why, then the runs, the argument, what changed and what did not, and what to do next, each part printed only when it has something to say. Each run's stretch around its deepest point now has a middle half (first to third quartile), so the report can say whether the gap between the runs' middles is smaller than the noise inside a single run, larger, or in between. No placeholder text is printed, and no interpretation the samples do not measure.

The memory file keeps the weighing as well as the notes: each run's deepest point, its middle and middle half, and its last sample, with the day it was first weighed. A later report sets the open runs beside earlier runs of the same recipe and other recipes with the same method, and names the settings where those recipes differ. The pane's list of earlier model notes is gone. Each reference card names the one kind of sentence it may back, and the report's source list prints that with the citation.

## [2.0.0]

The Windows bench reads a backpropagate `run_history.json`. It lists the runs, draws the stored loss, compares two rows, and exports the table. When `backprop` is already installed, Train, Eval, and Export model start that command, and Stop ends the process tree. The app does not contain the trainer.

Line coverage is gated at 90% in Codecov, for the project and for the patch.

The published Store app is still the 1.0.1 classifier build until a package above `1.0.1.0` is submitted on product `9PHL1HX0CGMF`.
