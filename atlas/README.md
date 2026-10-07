# runforge: how it works

Mapped at 2026-10-07 from commit c0c5042 by Atlas 1.24.0.

## What this is

A Windows instrument for fine-tuning runs: it draws every stored sample from a folder of run records, writes a report, and gives a local model a workbench. A Rust core library with an egui desktop app on top. (written by a person)

10 parts, mostly Rust (26 files), CSS (2), PowerShell (2), TypeScript (2), Astro (1) and JavaScript (1). Work enters through 3 doors; CI and runforge each reach 2 parts, and CI is followed because a pull request goes through it. It deploys a site to GitHub Pages. runforge is a command built from crates/runforge (nothing ships it).

## What changed since the last map

This is the first map.

## What comes in

1. **CI.** On a pull request; on a push to main. Runs crates/runforge-core/src/expr.rs, crates/runforge-core/src/parse.rs, crates/runforge-core/src/report.rs and 14 more; checks crates/runforge-core/src/lib.rs.
2. **Deploy site to GitHub Pages.** On a push to main touching 2 paths; or by hand. Runs site/astro.config.mjs and site/src/.
3. **runforge** (a command built from crates/runforge, which nothing ships). Runs crates/runforge/src/main.rs.

## What happens through CI

1. The workflow runs 6 files in runforge and 11 files in runforge-core; it checks crates/runforge-core/src/lib.rs in runforge-core.
2. It uploads coverage to Codecov.

## Who reads the results

CI writes nothing this map can see.

## The other doors

**Deploy site to GitHub Pages** runs site/astro.config.mjs and site/src/, and deploys the site.

**runforge** (a command built from crates/runforge, which nothing ships) runs crates/runforge/src/main.rs and reaches runforge-core.

## What breaks what

- **runforge-core** is imported by 1 part (runforge) and sits on the path of 2 doors.
- **runforge** is imported by no other part and sits on the path of 2 doors.

scripts holds only PowerShell files, which this map does not read, so what uses it cannot be seen.

## What tends to change together

- **crates/runforge-core/src/bench.rs** and **crates/runforge-core/tests/bench.rs** changed together in 5 of 6 commits, inside the runforge-core part.
- **crates/runforge-core/src/ledger.rs** and **crates/runforge-core/tests/bench.rs** changed together in 4 of 6 commits, inside the runforge-core part.
- **crates/runforge-core/src/lib.rs** and **crates/runforge-core/src/session.rs** changed together in 5 of 8 commits, inside the runforge-core part.
- **crates/runforge-core/src/bench.rs** and **crates/runforge-core/src/ledger.rs** changed together in 4 of 7 commits, inside the runforge-core part.
- **crates/runforge-core/src/lib.rs** and **crates/runforge/src/instrument.rs** changed together in 5 of 10 commits, and the runforge part imports the runforge-core part.

Confidence is low: fewer than 20 source files reach 10 revisions in the window.

Window: 180 days; a pair counts from 3 shared commits, since 1 source file reaches 10 revisions; the floor rises to 10 when 25 do.

## What no test touches

Every code part this map reads is touched by at least one test.

runforge is tested only by the unit tests in its own files.

scripts holds only PowerShell files, which this map does not read, so whether a test touches it cannot be seen.

## Written but never read

No place this map can see is written, so none goes unread.

## Helpers that look duplicated

No two parts export a helper that looks alike.

## Generated, never hand-edited

Nothing in this repository writes to a tracked place this map can see.

## Hand-authored

People write .cargo/, .github/, docs/, packaging/, the repository root, samples/ and site/; 2 writes with paths built at run time may land here.

## Where to start

crates/runforge/src/main.rs → crates/runforge-core/src/lib.rs → crates/runforge-core/src/bench.rs → crates/runforge-core/src/expr.rs → crates/runforge-core/src/series.rs → crates/runforge-core/src/export.rs → crates/runforge-core/src/history.rs

Read those in order to follow one run of runforge end to end. This path follows runforge (a command built from crates/runforge, which nothing ships) from its entry, since CI runs only tests, scripts that import no code here and checks.

## What this map cannot see

- 2 writes and 1 read use paths built at run time and are not named here.
- 4 writes and 5 reads go to a path their caller passes, not to this repository.
- Statistics confidence is low: fewer than 20 source files reach 10 revisions in the window.

Regenerate with `npx --yes @dogfood-lab/atlas map`.
