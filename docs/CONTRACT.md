# RunForge 2 contract

Design of record. Written 2026-10-03. Slice 1, the reader, and slice 2, the launcher, are this repository. The package is still unsubmitted. The published Store app stays the 1.0.1 classifier build until a package above `1.0.1.0` is submitted.

The 1.0.1 classifier app stays in the prototypes archive. This repository does not carry it. New work follows this contract.

## Product

The sentence that called RunForge only a bench — list the runs, draw the stored loss, compare two entries, export the table — is replaced. Written 2026-10-05. RunForge is the instrument for a training record.

A sample stays a record: the epoch or the step, the loss, the learning rate, and every other field that was logged. A recipe stays a structure and is shown whole. An arm stays whole: a mean does not stand in for the levels, the coverage, or a control that never trained. The sidecar sits in the window. It reads the same records, cites the epoch or step range, and does not press Train. It calls a local model only. It does not call a cloud model. Marketplace submission waits until this instrument is the thing a person opens.

The history bench is still the view for a backpropagate folder. That view lists the runs, draws the stored loss, compares two entries, and exports the table. It is one record kind. It is not the ceiling.

Backpropagate remains the trainer: the Python library, the `backprop` CLI, and its own Store product `9MVXLZVL3TMT` (`mcp-tool-shop.backpropagate`). RunForge does not move into that repo and does not ship under that identity. The RunForge listing `9PHL1HX0CGMF` stays up. This update is how it changes.

A person who already installed RunForge from the Store will see version 2 replace the classifier trainer. The listing text and the update notes have to say that in the same submission. The notes say: version 2 opens a backpropagate run folder. The earlier app trained classifiers on a CSV.

ScalarScope already draws `loss_history` from the same file and keeps that curve off its inference page. RunForge does not compute ScalarScope's inference deltas. ScalarScope does not become the bench.

## Store identity

Copied from Partner Center for product `9PHL1HX0CGMF`. The pack fails if the package name, the publisher, or the publisher display name changes.

| Field | Value |
|---|---|
| Package/Identity/Name | `mcp-tool-shop.RunForge-Desktop` |
| Package/Identity/Publisher | `CN=5305D976-6952-4F00-9C21-3A5DB090359F` |
| PublisherDisplayName | `mcp-tool-shop` |
| Package family name | `mcp-tool-shop.RunForge-Desktop_yn6b8xqrexa5j` |
| Architecture | x64 |
| First rebuild version | `2.0.0.0` (display `2.0.0`) |

The package family name follows those three. Architecture and the version are not part of that set. The first rebuild is `2.0.0.0` (display `2.0.0`).

Partner Center already holds `RunForgeDesktop_1.0.0.0_x64.msixupload` and `RunForgeDesktop_1.0.1.0_x64_bundle.msixupload`. The next upload is strictly above `1.0.1.0`. The fourth version part stays `0`, which is the rule recorded in backpropagate's Store handoff on 2026-10-01 and the shape of both uploads already on this product.

The package is unsigned. Partner Center signs it. The unsigned file is not a double-click installer. The manifest asks for `runFullTrust` so the app can read a run folder the user picked anywhere on the machine. The justification is that sentence. The manifest does not ask for `internetClient`. The sidecar talks to a local model on the loopback address, and a cloud tag is not a model it may call. The package contains the Rust binary and its licences. It does not contain Python, PyTorch, bun, backpropagate, or a model.

The public title on the listing may read RunForge. The package name stays `RunForge-Desktop`.

The repository is `mcp-tool-shop-org/runforge`. The prototypes tree stays where it is, as the archive of the 1.0.1 app. This repository does not carry the MAUI app or the sklearn CLI.

## Slices

**Slice 1, the first package.** A reader. The user opens one folder. The app loads `run_history.json` and shows the bench. No training, no process spawn, no network.

**Slice 2, in this tree.** The package is still unsubmitted. A launcher. If `backprop` is already on `PATH`, the app can start three commands and follow the log: `train`, `eval`, `export`. Arguments are built by the app. Nothing is passed through a shell. If the command is missing, the buttons say so. The app does not download backpropagate, install it, or vendor it.

Slice 2's form has four fields: model name, data file, step count, and the open output folder. Every other training flag stays at backpropagate's default. Stop ends the process tree RunForge started. There is no separate backprop cancel protocol in this contract.

**The instrument, in this tree.** A series folder opens onto every sample, the shared recipe, and the sidecar. The history bench remains the view when `run_history.json` is the file. The sidecar is in this window. It is not a second product, and it does not press Train.

**Out of slice.** Embedding the trainer. A second sklearn trainer. Editing `run_history.json` in place. Merging two history files. Rendering eval generations. Walking the disk to find histories. ScalarScope's inference deltas. Changing the package identity. Submitting under backpropagate's Store product. Putting a cloud model or an API key in the package.

## What the app reads

The Open action takes a folder. One record kind wins. The files are not merged.

**History.** The file is `run_history.json` in that folder, or `output/run_history.json` one level down. The file in the opened folder wins when both exist. A `multi_run` history is a second file the user can open. This reader does not merge them. The parse rules below bind this file.

**Series.** When the history file is absent, RunForge reads `run-config*.json` in the opened folder and the same names one level down in child folders. It does not look further down. It does not search the disk. A file is a series when it is a JSON object and `saturation_log.loss_curve` is an array. Any other file is skipped and counted. One bad file does not hide the rest. A duplicate key inside one file refuses that file.

Each sample keeps its axis (`epoch`, otherwise `step`), `loss`, `lr`, and every other key on that object. A null or non-finite loss is a gap. A gap is not drawn as zero. The chart is every sample, in file order. The view is not resampled. `training_summary.final_loss`, when present, is a separate number on the series. It is not appended to the curve.

The recipe is the `hyperparameters` object. Keys whose parsed values agree across the series are shown once. A key that differs, and the seed, stay on the series they belong to. Target modules stay a set of marks.

**Named, and not drawn by this pane yet.** A style-run log is a step series of loss and learning rate, grouped by the lever named in the file. A caliper file is a second measurement on the same steps: geometry, uncertainty, and performance. That loss is not the train-log loss. The two series stay aligned on step and are not merged. An arm is a control kept whole: pass at one, pass at k, coverage beside the mean, and the per-level table. A healthy average does not hide a dead arm. Those records are part of the instrument. This pane does not read them yet.

The file is a JSON array. Backpropagate writes it from `RunHistoryManager` (`backpropagate/checkpoints.py`). `MAX_LOSS_HISTORY_POINTS` is 100. Entries are updated in place by `run_id`. `evaluate_run` merges `extra={"eval": ...}` onto the entry, so the on-disk key is `eval`.

`backprop runs --json` is a different shape. Its `schema_version` is `"1"`, and each row has `loss.final` and `loss.min` only. The curve is not in that payload. Slice 1 reads the file. It does not shell out to build the chart.

### Entry fields slice 1 understands

| Field | Use |
|---|---|
| `run_id` | Identity. A non-string or missing id skips that entry and increments a count the UI shows. |
| `status` | `running`, `completed`, or `failed`. Anything else is shown as the raw string. |
| `session_kind` | `single_run` or `multi_run`, or the raw string. |
| `model_name` | List and compare. |
| `dataset_info` | Shown on the selected run as Dataset. Not a run-list or list-CSV column. |
| `started_at`, else `timestamp` | List order. Newest first by that timestamp. An unparseable timestamp sorts last. |
| `completed_at` | Shown when present. |
| `duration_seconds` | Shown when it is a finite number. |
| `steps` | Shown as text when it is a string, number, or boolean. Empty is omitted. |
| `final_loss` | The list column. It is not appended to the curve. |
| `loss_history` | The curve, in stored order. |
| `hyperparameters` | Object. Compare shows keys whose parsed values differ. |
| `failure_reason` | Shown on a failed run. |
| `checkpoint_path` | Shown as text. Export model passes this path to an already-installed `backprop`. The app does not open the checkpoint itself. |
| `export_paths` | Shown as text. |
| `dataset_hash` | Shown when present. |
| `eval` | Summary only: `held_out_loss`, `perplexity`, `task_metrics`, `eval_n`, `n_prompts`. |
| `schema_version` | Informational. Missing is normal on older entries. A value other than `"1.0"` is a note, not a refusal. |

An entry `schema_version` of `"1.0"` is backpropagate's current entry version (`CURRENT_ENTRY_SCHEMA_VERSION`). The CLI payload version `"1"` is a different number and does not appear on the file.

Unknown keys are kept on the parsed value and round-trip through JSON export. They are not a reason to refuse the file. Backpropagate's own bump rule for `backprop runs` is that additive fields stay on schema `"1"`. A reader that rejected a new key would break on the next trainer release.

## Parse rules

These come from the studio rust-knowledge base, rustc 1.98.1, edition 2024, ledger verified. The base does not cover eframe, MSIX, or `std::process`. Those decisions are named below and are not recipes from that base.

1. **Input fails as `Result`, never as a panic.** Recipe: use `expect` only for an invariant the compiler cannot see; data from a file is input. A bad file is a sentence on screen and an empty bench.
2. **The library returns `thiserror`. The binary may use `anyhow`.** Recipe: thiserror 2 for a library error enum, anyhow only in the application. `runforge-core` has one error enum. The GUI crate is the binary.
3. **A `Result` is not discarded.** Recipe: `Result` is `must_use`. CI denies `unused_must_use`.
4. **Parse with `float_roundtrip`.** Recipe: serde_json 1.0.151 feature `float_roundtrip`, so an f64 that survives a JSON round trip keeps its bits. The dependency is pinned in `Cargo.lock` from a resolution on rustc 1.98.1. The feature is on.
5. **Do not put `deny_unknown_fields` on a backpropagate entry.** The same recipe refuses unknown fields on an si-rpg-engine world file, because an extra key there is a different law. A backpropagate entry is the opposite: the trainer adds keys. RunForge keeps them. `deny_unknown_fields` also cannot be combined with `flatten`, which is how those keys would be retained. This is a deliberate split from that recipe, recorded here so a later reader does not "correct" it.
6. **Compare numbers by parsed bits, not by source text.** The same recipe: JSON text is not a stable artefact. Hyperparameter compare and any hash RunForge writes use the parsed value. A hash of the file's bytes is only a label of that file, never a claim that two encodings are different runs.
7. **NaN and infinity.** serde_json writes them as `null`, and a JSON `null` in a loss sample is a gap in the line. A non-finite number is the same gap. A gap is not drawn as zero. `final_loss` null means the column is empty.
8. **`f64` is not `Ord`.** Recipe: rank with `total_cmp`. The best-loss pick uses `total_cmp`, and a missing or non-finite `final_loss` sorts after every finite loss. The curve itself is not sorted. Sample order is the file order.
9. **Do not append `final_loss` to `loss_history`.** ScalarScope already draws the stored series alone. RunForge draws the same series. A file with more than 100 points is drawn as stored. RunForge does not resample.
10. **Duplicate `run_id`.** Both rows stay, distinguished by file index. The UI says the file has duplicate ids. Compare addresses rows by index.

A file that is not a JSON array fails as one refusal. A duplicate key inside an object refuses the whole file too. serde_json would keep the last value. This reader does not, because the last `run_id` would hide the first. After a successful parse, a bad entry is skipped and counted. One bad entry does not hide the rest.

## Series screen

Shown when the opened folder is a series folder. The history bench is not this screen.

- Every series on one epoch axis. Loss is on a log scale so the fall from the first sample to the low sample stays readable. Hover shows the stored triple: epoch, loss, learning rate. Scroll brushes an epoch range. The learning-rate strip shares that range and keeps its own vertical scale.
- The lowest sample on each series is marked. The last sample is not a substitute for that mark. Selecting a series draws it through the others. When the deepest low and the lowest last sample are different series, that sentence is on the stage. A second row draws the stretch around the lows on one linear scale. Every sample under the ceiling is drawn there, and a bold line follows the lowest sample in each epoch, so a climb after the low is visible in the stored units. A spike above that ceiling is a gap on that row and stays on the main chart.
- The shared recipe is a board: schedule, warmup, weight decay, gradient clip, batch, accumulation, effective batch, epochs, sequence length, LoRA rank, alpha, dropout, checkpoints, and the target modules. A key the file adds is still shown.
- The sidecar prints one plain-text report from the measurements. The window shows that text, and Save report writes the same string through the history bench's save dialog. The report does not use markdown headings. Ask may add one orientation paragraph with no digit, no setting, and no verdict. A fresh note that crosses that line is dropped and the pane says so. A stored note that crosses it is not shown. The source path is not part of the question. A kept note stays with the recipe fingerprint in the preferences directory. It is not written back into the series files. Opening a folder does not overwrite a richer note.
- The report weighs the samples before it asks. The neighborhood is half an epoch around each low, and those lines stay in the report. It also states the learning rate on that sample and LoRA alpha/r when the recipe has them. It cites only the local reference cards that a printed sentence uses. The full reference list in the column stays closed. It does not fetch the network. An assumption is labeled and is not a result. It abstains when the lowest sample and the quieter neighborhood are different series. The lowest sample stays on the page either way.
- Train, Eval, and Export model stay on the history bench. They are not on this screen.

## Screen

One window. A history folder uses the bench below. A series folder uses the series screen above.

- No folder yet: one sentence that names `run_history.json` and a series folder.
- List: status, model, final loss, started time. Newest first.
- Chart: the selected run's `loss_history` against stored-sample index. The caption says the chart is the stored samples, in file order. The trainer usually keeps at most 100. A longer series is still drawn as stored.
- A run with an empty `loss_history` stays in the list. The chart says there is no stored loss.
- Compare: two rows, both series on one chart, by sample index. Hyperparameters that differ. Eval summaries when present.
- The skipped-entry count, when it is not zero.
- Export of the list as CSV, of the selected curve as CSV (index, loss, empty cell for a gap), and of the selected entry as JSON with unknown keys preserved.
- Once a folder is open: model name, data file, step count, and the open output folder. Train, Eval, and Export model start an already-installed `backprop`. The log follows that command. Stop ends the process tree. The app does not download, install, or vendor backpropagate.
- The version string `2.0.0`.

No account, no telemetry. Preferences (last folder, theme) go to the packaged app's LocalState. An unpackaged run keeps them beside the executable and does not pretend to read LocalState. Run history is only read from the folder the user opened.

## Rust crate

Measured on this machine on 2026-10-03: `rustc 1.98.1 (48a229cea 2026-09-01)`.

| Choice | Pin |
|---|---|
| Toolchain file | `1.98.1` |
| `edition` | `2024` |
| `rust-version` | `1.98.1` |
| Resolver | edition 2024 default (`"3"`), so the lock prefers MSRV-compatible versions |
| `workbench` | library shared with ScalarScope, `serde_json` 1 with `float_roundtrip`, nothing else |
| `runforge-core` | library, `thiserror` 2, `serde` 1 with `derive`, `serde_json` 1 with `float_roundtrip` |
| `runforge` | binary, `anyhow` 1, `eframe` 0.36, `egui_plot` 0.37, `rfd` 0.15 |

The GUI pins match the ScalarScope review as read on 2026-10-03 (`eframe` 0.36, `egui_plot` 0.37, `rfd` 0.15). ScalarScope itself is edition 2021. RunForge is a new crate and takes edition 2024 because that is the edition the knowledge base was checked on. A bump of any pin is a line in the changelog, not a silent lockfile drift.

`cargo deny` allow list, from the knowledge base's licence recipes for a shipped MIT/Apache product: `MIT`, `Apache-2.0`, `Unlicense`, `BSD-1-Clause`, `0BSD`, `Zlib`, confidence threshold 0.95. MPL-2.0 is not on that list. Measured against the eframe 0.36 graph on 2026-10-03, these crates needed a per-crate exception. The global list was not widened. `arrayref` is BSD-2-Clause. `tiny-skia` and `tiny-skia-path` are BSD-3-Clause. `clipboard-win` and `error-code` are BSL-1.0. `libloading` is ISC. The ICU crates and `unicode-ident` are Unicode-3.0. `epaint_default_fonts` adds OFL-1.1 and the Ubuntu Font Licence 1.0 on top of MIT OR Apache-2.0. `ttf-parser` is unmaintained (RUSTSEC-2026-0192), pulled in by winit for Linux window decorations, with no safe upgrade. That ignore is in `deny.toml`. Apache-2.0 crates ship the licence text, and a NOTICE file when the crate has one. Collecting those files is part of packaging, not this slice.

`Cargo.lock` is committed. CI builds with `--locked`.

The fixtures cover: a failed entry with an empty `loss_history`, a completed entry whose curve is drawn without appending `final_loss`, an entry with `eval`, an unknown key that survives export, a null inside `loss_history` that becomes a gap, a missing `run_id` that is skipped, a file that is not an array, a duplicate key that refuses the file, and a best-loss pick that puts a non-finite loss last. The window is exercised headlessly. Codecov enforces 90% line coverage on the project and on the patch. The operating-system file dialog and the packaged-process success path stay uncalled. A screenshot of the window is not the check.

## Production gate

The dogfood swarm runs on this repository after slice 1 is in it and the coverage gate is green. It does not run on this file. Phase 10 is full treatment, including the identity scan, and it ends with an unsigned `2.0.0.0` package whose identity matches the table above. Submission to Partner Center stays with the operator. The series instrument does not move that submission forward by itself.

The swarm is not a substitute for the fixture list. A run that never loads a `run_history.json` has not seen the product.
