# The sidecar workbench

The sidecar is a workbench. A local model investigates the open runs by calling tools. It builds new formula tools when the ones it has don't answer its question, and it proposes hypotheses about what each knob does. The program computes every number, sets every verdict, and writes every sentence that carries a number. The model chooses what to look at and puts its reasoning into words. Its words are fenced.

This document records the design, the evidence behind it, and what is deferred. Code: `crates/runforge-core/src/{expr,bench,session}.rs` and `crates/runforge/src/sidecar.rs`.

## Standards compliance

Scored 0 to 3 against the six workflow standards.

| Standard | Score | Evidence |
|---|---|---|
| PIN_PER_STEP | 2 | Each session pins the model choice (preference order, plus Ollama's `tools` capability), temperature 0.2, `think` off, the phase prompts and the tool schemas, all in code. Each `Step` records the tool, its arguments and the program's answer. What it lacks for a 3: the model name and digest are not saved with the trace. |
| ANDON_AUTHORITY | 3 | Every call is validated before it runs: tool name, knob enum, formula grammar and size, finiteness on every run, duplicate checks, and the wording fence. A refusal goes back to the model as a sentence and nothing is kept. The loop stops at the round and call caps. Tested in `session.rs`, `bench.rs` and `sidecar.rs`, including a fake-Ollama end-to-end test. |
| NAMED_COMPENSATORS | 2 | The session's only writes are to `sidecar-memory.json` beside the preferences: learned tools (`tools`) and hypotheses (`hypotheses`). The table below gives the undo for each. Nothing is published, trained or fetched. |
| DECOMPOSE_BY_SECRETS | 3 | What changes together stays together. The formula language (`expr`) knows nothing about tools or models. The statistics and verdicts (`bench`) know nothing about chat. The session (`session`) knows nothing about HTTP. The HTTP loop (`sidecar`) knows nothing about statistics. Each has its own tests. |
| UNCERTAINTY_GATED_HUMANS | 3 | A verdict across folders (supported or refuted) needs the combined e-value to pass e-BH at a 5% false discovery rate across the whole bench. The folder a hypothesis was proposed on never counts. Everything else is open, with the evidence so far and the threshold printed, plus a run plan. Validity is tested exactly (the e-value averages 1 over every relabeling) and by simulation (Ville's bound under no effect). The pane labels the model's note as its words, not a measurement. |
| EXTERNAL_VERIFIER | n/a | No specialized claims. The statistics are textbook (Mann-Whitney, Holm, Vargha-Delaney, Colas power), and their values are pinned in tests. |

### Compensators

| Action | Undo | State after undo | Owner |
|---|---|---|---|
| A learned tool is kept | Remove its entry from `tools` in `sidecar-memory.json`, or delete the file. | The tool is gone. Hypotheses that name it are refused on their next test, because the formula no longer parses. | The person running the app |
| A hypothesis is recorded or retested | Remove its entry from `hypotheses`. | The hypothesis and its evidence are gone. Nothing else depends on it. | The person running the app |

## Research grounding

Four questions decided the design. Sources were located by search on 2026-10-06, and each was read at the abstract or summary level.

**1. Should the model compute or call?** Call.
- Offloading the computation to an interpreter beat larger models doing it in context. Gao et al. 2022, PAL, arXiv:2211.10435.
- Separating computation from reasoning gained about 12% on tabular and financial question sets. Chen et al. 2022, Program of Thoughts, arXiv:2211.12588.
- Chain-of-thought explanations can misstate the real cause of an answer. Turpin et al. 2023, arXiv:2305.04388. Our own runs matched this: the 14B dropped facts and swapped reasons.

*Implication:* the model never prints a number. The program writes every sentence that has one, and the model's note sits under a label saying it is not a measurement.

**2. How should a small model use tools?** Few tools, a short loop, and a way to abstain.
- Showing fewer tools per turn raised function-calling success on edge hardware. Paramanayakam et al. 2024, "Less is More", arXiv:2411.15399.
- Multi-turn use is where function calling still fails. Patil et al., BFCL, ICML 2025, https://openreview.net/forum?id=2GmDdhBdDk.
- Format constraints hurt reasoning. Tam et al. 2024, arXiv:2408.02442.

*Implication:* each phase offers two to four tools, and the last phase offers only `finish`. The schema constrains only the call arguments (enums for knobs and directions), never the reasoning. There are at most six rounds and ten calls.

**3. How should a tool library grow?** Make a tool once, check it before keeping it, and trim the library.
- A tool made once and reused later recovers most of the gain. Cai et al. 2023, LATM, arXiv:2305.17126.
- A tool should be admitted only after correctness and diversity filters. Yuan et al. 2023, CRAFT, arXiv:2309.17428.
- Growing the library through use, then trimming it, kept it small. Wang et al. 2024, TroVE, arXiv:2401.12869.
- A compute-matched re-evaluation shrank TroVE's gain to about one point. Sesterhenn et al. 2025, arXiv:2507.22069.
- Unstructured piles of tools hurt retrieval. ToolLibGen 2025, arXiv:2510.07768.
- Model-written code is safe only inside a restricted interpreter. Debenedetti et al. 2025, CaMeL, arXiv:2503.18813.

*Implication:* a learned tool is a formula in a small language, not code. It has no I/O and no loops, and it has size and depth caps. It is kept only if it parses, gives a finite value on every run, and is not a duplicate of an existing tool by formula or by values. It stays provisional until it is used on a second folder, and the least-used provisional tool is trimmed first once the library passes 50. These papers studied full code, so whether the formula restriction keeps their gains is our inference, not their result.

**4. When is a hypothesis about a knob settled?** Only by a test declared before looking.
- Sequential falsification keeps Type-I error controlled. Huang et al. 2025, POPPER, arXiv:2502.09858.
- Post-hoc metric choice is a hidden pitfall of automated science. Luo et al. 2025, arXiv:2509.08713.
- Agents fail most ML experiments. Huang et al. 2023, MLAgentBench, arXiv:2310.03302. Beel et al. 2025, arXiv:2502.14297.
- Explanations raise acceptance whether the AI is right or wrong. Bansal et al. 2021, arXiv:2006.14779.
- Seed variance is large and must be estimated, not assumed. Bouthillier et al. 2021, arXiv:2103.03098. Henderson et al. 2018, arXiv:1709.06560.

*Implication:* a hypothesis fixes its knob, formula and direction when it is proposed. On each folder alone, the program sets its state:
- **Not testable**: the knob did not vary.
- **Confounded**: another knob moved with it.
- **Inconclusive**: one run per setting, a gap inside the seed spread, fewer than three runs per setting, or the test not passed.
- **Passes** or **goes the other way**: the exact one-sided Mann-Whitney test passes at 0.05 on that folder (Mann and Whitney 1947), after Holm's adjustment across the hypotheses tested on those runs (Holm 1979).

With three runs per setting, 1/20 is the smallest p the exact test can give, so one small folder rarely settles anything. The verdict that counts is the one across folders. It is in "Evidence across folders" below. Opening a folder retests the stored hypotheses for its method. A run plan has one knob, two settings and at least three seeds each. Its seed count comes from the pilot sigma when a seed spread exists (Colas et al. 2018, arXiv:1806.08295).

The formulas that do not hold up with two to five runs are left out: fANOVA (Hutter et al. 2014), almost stochastic dominance (Dror et al. 2019), learning-curve extrapolation (Domhan et al. 2015), and the gradient noise scale (McCandlish et al. 2018, which needs per-example gradients we don't store).

## What a live session did

The runs were with qwen3:14b on the studio's own fine-tune folders (2026-10-06). After the phases were added:
- It measured the runs and built `post_low_recovery = last / low`.
- Its hypotheses about shared knobs came back "not testable here", each with a six-run plan.
- It finished with a note.

Before the phases, it measured one thing per round until the rounds ran out.

The program refused each of these, and the model saw the reason:
- A hypothesis about "seed" as a knob.
- A note over the length limit.
- A reason that contradicted its direction. Hypotheses are now asked as raise or lower the knob, formula up or down.

A note can still contain a wrong claim in words. That is why the pane labels it.

## Evidence across folders

A single folder rarely settles a knob. With three runs per setting, the exact test's smallest p is 1/20. So evidence is gathered as e-values, which can be multiplied across folders without losing validity. An outside review checked this section (Kimi K3, run by hand on 2026-10-06; `docs/evidence.consult.md` and `docs/evidence.consult.response.md`). Its four fixes are in.

- **Per folder: a permutation e-value.**
  - Let S be the share of (low-setting run, high-setting run) pairs that move in the declared direction, ties counting half.
  - The e-value is exp(λS) divided by its average over every relabeling of the pooled runs into groups of the same sizes. If the knob does nothing, the runs are exchangeable and the e-value averages exactly 1 (Koning 2023, arXiv:2310.01153).
  - λ comes from a table of group sizes, fixed at design time. Each entry maximizes the expected log e-value under a one-standard-deviation shift, by simulation:
    - 1 against 1: λ = 1
    - 2 against 2: λ = 3
    - 3 against 3: λ = 4
    - 5 against 5: λ = 8
  - A single fixed λ = 8 lost evidence on average for folders up to 2 against 3, even under a real effect.
  - With the table, a clean three-against-three separation gives 4.51, and one run against one gives at most 1.46.
  - Validity needs runs assigned to a setting independently of anything else that moves the loss. "Everything else equal" carries that assumption; assigning settings at random would guarantee it.
- **Across folders.** The e-values are multiplied, oldest folder first. A product of e-values from new, independent runs stays valid even when the decision to run another folder depended on earlier ones (Grünwald, de Heide and Koolen 2024, "Safe testing", JRSS-B 86(5); Ramdas, Grünwald, Vovk and Shafer 2023, arXiv:2210.01948).
  - A folder holding any run RunForge knew when the hypothesis was registered is left out. Any of those runs may have shaped the claim, not only the folder it was proposed on. Runs seen outside RunForge are beyond what the program can know.
  - A folder that shares a run with one already counted is left out. A run's identity is its seed plus a hash of its first 32 samples, so a run that kept training is the same run.
  - A folder tested again keeps its place, so which folders count never depends on their results.
- **The verdict.** e-BH at a 5% false discovery rate over the 2K directional e-values of the K hypotheses on the bench: each hypothesis's evidence for and evidence against (Wang and Ramdas 2022, JRSS-B 84(3)).
  - A discovered "for" is supported; a discovered "against" is refuted. Both are directional discoveries held to the same rate, under any dependence.
  - One direction alone needs 2K/0.05: 40 for a bench of one hypothesis, about three clean three-against-three folders.
- **Checkpoints.** e-BH is a batch procedure. Re-running it as the bench grows would let the reporting moment be chosen, so verdicts are issued only at checkpoints, one every five new folders, a schedule fixed in advance.
  - A checkpoint judges every hypothesis registered by then, and its verdicts are final for that checkpoint.
  - Between checkpoints the report shows the evidence so far, not a verdict.
  - Each folder still shows its own exact-test result, labeled "on these runs alone".

Tests pin the exact average of 1 over every relabeling, including with ties and unequal groups. Two simulations check the rest. Under no effect, the product crosses 20 in at most 5% of 4,000 eight-folder sequences. Under a real shift, four folders usually pass 40.

## Deferred

- **A block-bootstrap interval for a run's window median** (Efron 1979). Adjacent samples are correlated, so it needs a block length.
- **Tool retrieval by description, and merging near-duplicates into parameterized tools** (CRAFT, ToolLibGen). These matter once a library holds dozens of tools.
- **Recording the model's name and digest with each trace**, which would raise PIN_PER_STEP to 3.
