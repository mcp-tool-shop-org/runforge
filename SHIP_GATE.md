# Ship Gate

> No repo is "done" until every applicable line is checked.
> Copy this into your repo root. Check items off per-release.

**Tags:** `[all]` `[desktop]` — Rust eframe bench. Shipcheck's detector does not see this shape (it looks for Tauri, Electron, or .NET), so the desktop lines are filled by hand.

---

## A. Security Baseline

- [x] `[all]` SECURITY.md exists (report email, supported versions, response timeline) — executed by `npx @mcptoolshop/shipcheck security-docs` (A1: present + reporting contact, not an empty stub) (2026-10-05, Gate K passed; contact is the GitHub Security Advisories URL)
- [x] `[all]` README includes threat model paragraph (data touched, data NOT touched, permissions required) — executed by `npx @mcptoolshop/shipcheck security-docs` (A2: trust/threat-model section present + non-empty; *quality* is not machine-checkable) (2026-10-05, Gate K passed)
- [ ] `[all]` SKIP: `shipcheck secrets` found no publishable npm package, so it did not scan a tarball. This repository is not an npm package. The identity scan of the tracked tree is a separate gate and was CLEAN on 2026-10-05.
- [x] `[all]` No telemetry by default — state it explicitly even if obvious (2026-10-05, README and SECURITY.md; the crate graph has no telemetry client)

### Default safety posture

- [ ] `[cli|mcp|desktop]` SKIP: Train, Eval, Export model, and Stop run only from those buttons. There is no `--allow-*` flag surface.
- [x] `[cli|mcp|desktop]` File operations constrained to known directories (2026-10-05, history is the opened folder or `output` one level down; export and the data file are paths the user picks; the app does not walk the disk)
- [ ] `[mcp]` SKIP: not an MCP server
- [ ] `[mcp]` SKIP: not an MCP server

## B. Error Handling

- [ ] `[all]` SKIP: the window shows fixed sentences and `HistoryError` display text. The type does not carry `hint`, `cause`, or `retryable`.
- [ ] `[cli]` SKIP: not a CLI
- [ ] `[cli]` SKIP: not a CLI
- [ ] `[mcp]` SKIP: not an MCP server
- [ ] `[mcp]` SKIP: not an MCP server
- [x] `[desktop]` Errors shown as user-friendly messages — no raw exceptions in UI (2026-10-05, notes are display strings and the fixed launch sentences)
- [ ] `[vscode]` SKIP: not a VS Code extension

## C. Operator Docs

- [x] `[all]` README is current: what it does, install, usage, supported platforms + runtime versions (2026-10-05, Windows bench, Rust 1.98.1)
- [x] `[all]` CHANGELOG.md (Keep a Changelog format) (2026-10-05, 2.0.0 is the crate version and is not a git tag)
- [x] `[all]` LICENSE file present and repo states support status (2026-10-05, MIT; SECURITY.md supports the 2.0.0 source build; the Store listing is still the 1.0.1 classifier)
- [ ] `[cli]` SKIP: not a CLI
- [ ] `[cli|mcp|desktop]` SKIP: there is no silent / normal / verbose / debug switch. The window shows the child output as produced.
- [ ] `[mcp]` SKIP: not an MCP server
- [ ] `[complex]` SKIP: not a daemon. The product guide is the Starlight handbook, not a root ops runbook.

## D. Shipping Hygiene

- [x] `[all]` `verify` script exists (test + build + smoke in one command) (2026-10-05, `scripts/verify.ps1`: fmt, clippy, 87 tests, release build)
- [ ] `[all]` SKIP: `shipcheck manifest` skipped because there is no npm or Python manifest. The crate version is 2.0.0. There is no release tag, and this treatment does not create one.
- [x] `[all]` Dependency scanning runs in CI (ecosystem-appropriate) — executed by `npx @mcptoolshop/shipcheck ci` (D3: a recognized scanner is *configured* in CI, or dependabot is present) (2026-10-05, Gate L passed on `cargo audit`. Local cargo-audit 0.22.2 exited 0. `cargo deny check` stays in the licenses job.)
- [ ] `[all]` SKIP: `shipcheck deps` only reads npm lockfiles and skipped. `cargo deny check` already runs in the licenses job. The ignored advisory is RUSTSEC-2026-0192, unmaintained ttf-parser, with no safe upgrade.
- [ ] `[all]` SKIP: no Dependabot update bot. The org rule is not to add dependabot.yml unless asked. The template marks the update bot as optional. Vulnerability scanning stays in CI.
- [ ] `[npm]` SKIP: not published to npm. `shipcheck ci` provenance check skipped because there is no npm-publish workflow.
- [ ] `[npm]` SKIP: no publishable package. `shipcheck pack` has nothing to pack.
- [ ] `[npm]` `engines.node` set · `[pypi]` `python_requires` set — SKIP: not published to npm or PyPI. `shipcheck manifest` engines check skipped.
- [ ] `[npm]` SKIP: not an npm package. `Cargo.lock` is committed and CI builds with `--locked`.
- [ ] `[vsix]` SKIP: not a VS Code extension
- [x] `[desktop]` Installer/package builds and runs on stated platforms (2026-10-05, unsigned `release/RunForge_2.0.0.0_x64.msix`, package `mcp-tool-shop.RunForge-Desktop` version `2.0.0.0` x64, identity scan CLEAN, window title `RunForge 2.0.0` stayed up. Not submitted.)

## E. Identity (soft gate — does not block ship)

- [x] `[all]` Logo in README header (2026-10-05, brand wordmark, width 720, and the dark-window screenshot)
- [x] `[all]` Translations (polyglot-mcp, 8 languages) (2026-10-05, local TranslateGemma 27B, 7/7 plus the English source, nav bar injected, Japanese reads as Japanese)
- [x] `[org]` Landing page (@mcptoolshop/site-theme) (2026-10-05, site-theme 2.2.0, base `/runforge`, cyan accent, build wrote `dist/index.html`, `dist/handbook/index.html`, and `dist/pagefind/pagefind.js`. The Pages URL follows the push.)
- [x] `[all]` GitHub repo metadata: description, homepage, topics (2026-10-05, description and homepage set, topics rust, windows, egui, machine-learning, desktop)

---

## Gate Rules

**Hard gate (A–D):** Must pass before any version is tagged or published.
If a section doesn't apply, mark `SKIP:` with justification — don't leave it unchecked.

**Soft gate (E):** Should be done. Product ships without it, but isn't "whole."

**Executed vs attested.** `shipcheck audit` only *counts these checkboxes* — it does not read your repo, so a box can be green while the fact is false. The lines that say **"executed by `npx @mcptoolshop/shipcheck <gate>`"** are backed by a command that reads the real artifact and exits 1 on the real defect. Run those gates (they are wired into shipcheck's own `verify`); don't just tick their boxes. Executed today: **A1/A2** (`security-docs`), **A3** (`secrets`), **D2/D6/D7** (`manifest`), **D3-config + OIDC/provenance** (`ci`), **real vulnerabilities + alerting** (`deps`), **D5** (`pack`), plus front-door (`front-door`) and dogfood freshness (`dogfood`). Every other line is still an attestation you are vouching for. Note the two dependency layers: `ci` proves a scanner is *configured*; `deps` proves there are *no known vulnerabilities* — a repo can pass the first while failing the second.

**Checking off:**
```
- [x] `[all]` SECURITY.md exists (2026-02-27)
```

**Skipping:**
```
- [ ] `[pypi]` SKIP: not a Python project
```
