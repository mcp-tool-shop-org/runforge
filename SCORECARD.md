# Scorecard

**Repo:** mcp-tool-shop-org/runforge
**Date:** 2026-10-05
**Type tags:** `[all]` `[desktop]`

`npx @mcptoolshop/shipcheck audit` on this date, after the boxes below were marked from the work in the tree:

Checked 16, Unchecked 0, Skipped 23, pass rate 100%. The command exited 0.

The audit counts checkboxes. It does not open the repository. The counts below are those boxes, with the gate-rules sample lines left in the skipped and checked totals the audit already includes.

## Pre-Remediation Assessment

| Category | What the boxes say | Notes |
|----------|--------------------|-------|
| A. Security | 4 checked, 4 skipped | SECURITY.md uses the GitHub Security Advisories URL. Threat model is in the README. No telemetry client. The npm secrets scan does not apply. Train, Eval, Export model, and Stop are buttons, so there is no `--allow-*` flag. |
| B. Error Handling | 1 checked, 6 skipped | The window shows fixed sentences and `HistoryError` display text. It does not carry hint, cause, and retryable. Not a CLI, MCP server, or VS Code extension. |
| C. Operator Docs | 3 checked, 4 skipped | README, CHANGELOG, and LICENSE. No log-level switch. The handbook is the product guide. |
| D. Shipping Hygiene | 3 checked, 8 skipped | `scripts/verify.ps1` passed fmt, clippy, 87 tests, and the release build. `cargo audit` is in CI and cargo-audit 0.22.2 exited 0 locally. The unsigned `2.0.0.0` package was built and not submitted. No Dependabot file: the org rule leaves that bot off unless asked, and the template marks it optional. Not an npm or PyPI package, so provenance, pack, engines, and the npm lockfile line are skipped. `Cargo.lock` is committed. |
| E. Identity (soft) | 4 checked | Wordmark at width 720, seven translations plus English, the site-theme build, and GitHub description, homepage, and topics. |
| **Overall** | **16 checked, 0 unchecked, 23 skipped** | Hard gates A–D have no open box. |

## Key Gaps

1. The Pages URL is not live until the workflow on this commit deploys. The landing build already writes `dist/index.html`, `dist/handbook/index.html`, and `dist/pagefind/pagefind.js`.
2. The Store listing is still the 1.0.1 classifier. The unsigned package is local and is not submitted.
3. There is no npm publish, git tag, or GitHub release. This is a Windows bench, not an npm package.

## Remediation Priority

| Priority | Item | Where it stands |
|----------|------|-----------------|
| 1 | Identity scan, then push the site, handbook, and translations | This commit |
| 2 | Confirm the Pages site and the handbook URL | After the workflow |
| 3 | Operator submits the unsigned 2.0.0.0 package | Not this treatment |

## Post-Remediation

The audit above is the post-remediation count. Nothing in A–D was left unchecked. Skips are the surfaces this repository does not have.
