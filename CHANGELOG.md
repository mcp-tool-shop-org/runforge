# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

The README, the landing page, and the handbook describe the instrument. No package has been submitted to Partner Center.

The window is an instrument. A folder of run-config series draws every sample, the shared recipe, and a local sidecar. The row around the lows draws those samples on one linear scale, with a line through the lowest sample of each epoch. A spike above that row stays on the main chart. A backpropagate `run_history.json` still opens the history bench. The sidecar does not press Train and does not call a cloud model. It prints the weighing as one plain-text report, and Save report writes those same words. A local note may add one orientation paragraph only when that paragraph has no digit, no setting, and no verdict. The report weighs each low against the samples around it, cites the local reference cards it actually uses, and labels an assumption so it is not a result. Opening a folder does not replace a stored answer.

## [2.0.0]

The Windows bench reads a backpropagate `run_history.json`. It lists the runs, draws the stored loss, compares two rows, and exports the table. When `backprop` is already installed, Train, Eval, and Export model start that command, and Stop ends the process tree. The app does not contain the trainer.

Line coverage is gated at 90% in Codecov, for the project and for the patch.

The published Store app is still the 1.0.1 classifier build until a package above `1.0.1.0` is submitted on product `9PHL1HX0CGMF`.
