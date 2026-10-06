# Privacy policy

**RunForge** (Microsoft Store product 9PHL1HX0CGMF). Effective 2026-10-06.

RunForge collects no personal information. Everything it does stays on your computer.

## What RunForge reads

The folder you open, and nothing else on the disk:
- a series folder: `run-config*.json` files there and one level down
- a history folder: `run_history.json`

It never writes back into those files.

## What RunForge keeps on your computer

Two files sit together: in the app's local storage when it is installed from the Store, and beside the program otherwise. They are never uploaded anywhere.
- **The preferences file:** the last folder you opened, and the theme.
- **`sidecar-memory.json`:** what the workbench needs between sessions:
  - each run's measurements: run names, losses, epochs, and recipe values
  - the formula tools the workbench has learned
  - the hypotheses with their test results
  - the checkpoints
  - the local model's notes

  It holds no folder path.

Uninstalling the app removes its local storage. You can also delete either file at any time.

## What leaves the app

**Nothing leaves your computer.** RunForge has no telemetry, no analytics, no account, and no advertising. The package does not request internet access.

**The local model.** When you press **Ask**, RunForge connects to a model server running on your own computer (Ollama at `127.0.0.1:11434`), and only if you have installed one. It sends run names, recipe values, and the program's own measurements, but not folder paths. It never uses a cloud model.

**Files you save.** Save report and Export write files only where you choose.

**History-bench commands.** When you press Train, Eval, or Export model, RunForge starts the `backprop` program already installed on your computer. That program is separate software with its own behavior.

## Changes and contact

Changes to this policy are recorded in this file's history at https://github.com/mcp-tool-shop-org/runforge/blob/main/PRIVACY.md.

Questions go to https://github.com/mcp-tool-shop-org/runforge/issues. Security reports go privately through [GitHub Security Advisories](https://github.com/mcp-tool-shop-org/runforge/security/advisories/new).
