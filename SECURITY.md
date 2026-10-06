# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 2.0.0 (this tree) | Yes. Windows source build. The Store package on product `9PHL1HX0CGMF` is still the 1.0.1 classifier until a package above `1.0.1.0` is submitted. |
| 1.0.1 (Store) | The published classifier. This repository does not contain that app. |

## Reporting a vulnerability

Report privately through [GitHub Security Advisories](https://github.com/mcp-tool-shop-org/runforge/security/advisories/new).

Do not open a public issue for a security report.

| Action | Target |
|--------|--------|
| Acknowledge the report | 48 hours |
| Assess severity | 7 days |
| Release a fix | 30 days |

## Threat model

RunForge is a Windows bench for a folder you pick. It is not the trainer.

| Boundary | What happens |
|----------|----------------|
| Run history | The app reads `run_history.json` in the folder you open, or `output/run_history.json` one level down. It does not walk the disk and it does not merge two files. |
| A series folder | `run-config*.json` in the opened folder and in its immediate children. One bad file is skipped. A duplicate key refuses that file. The series files are not rewritten. |
| Preferences | The last folder and the theme. A packaged run writes them in that package's LocalState. An unpackaged run writes them beside the executable. If the executable path cannot be read, they fall back to the process temporary directory. A sidecar note, when one is kept, stays with those preferences, in `sidecar-memory.json`. That file also keeps the measured weighing of each opened series: run names (the seed, or the file name when a run has no seed), losses, epochs, and a recipe fingerprint, at most 40 entries. It also keeps the workbench: learned formula tools (at most 50) and hypotheses with their test results (at most 60). It holds no folder path. A learned tool is a formula evaluated by the program; it cannot read a file, open the network, or run code. |
| Train, Eval, Export model | Those buttons start an already-installed `backprop` with arguments the app builds. Nothing is passed through a shell. The child does not receive a copy of the environment. Stop ends the process tree this window started. They are on the history bench. The sidecar does not press them. |
| The log | The window shows that program's output. It is not sent anywhere. |
| The local model | Ask connects to `127.0.0.1` port `11434` and uses only a local model that Ollama reports can call tools. A cloud-tagged name is not chosen. The session sends run names, the recipe values and the program's tool results; it does not send the folder path. The model can only call the five workbench tools, each validated by the program. At most six requests and ten calls per session. If nothing answers, the measured report still shows. |
| Network | The manifest does not request `internetClient`. The app does not download a model, does not install backpropagate, and does not send telemetry. |

## What RunForge does not do

- It does not contain Python, PyTorch, bun, or backpropagate.
- It does not edit `run_history.json`.
- It does not collect telemetry or open an account.
- It does not start `backprop` except from Train, Eval, or Export model.
- It does not call a cloud model, and it does not fetch the report's reference list.
