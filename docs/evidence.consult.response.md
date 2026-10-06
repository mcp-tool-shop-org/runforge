# Review of the evidence pipeline: response and changes

**Reviewer:** Kimi K3, run by hand by the Director on 2026-10-06, against `docs/evidence.consult.md`. Under the standing rule, a review that statistical verdicts depend on is done by hand, not by a cloud call from the app or the agent.

**Verdict:** the architecture is sound. The per-folder construction, the product rule, Ville's bound and the dependence handling in e-BH hold up as stated. The reviewer reproduced our numbers by exact enumeration: 9.5323, 0.00320, at most 1.9993, and an average of exactly 1 with ties and unequal groups. It also confirmed the four citations. It asked for four fixes, and all four are in.

| # | Finding | Fix in RunForge |
|---|---|---|
| 1 | Excluding only "the folder the hypothesis was proposed on" leaves out folders that were seen earlier and could equally have shaped the claim. | A hypothesis records every run RunForge knew at registration: the open folder, plus every run in the ledger. A folder holding any of them never counts. |
| 2 | The supported/refuted label was an uncontrolled direction call. | e-BH now runs over the 2K directional e-values, for and against, so both labels are discoveries at the same false discovery rate. |
| 3 | Re-running e-BH as the bench grows chooses the reporting time, which voids the batch guarantee. | Verdicts are issued only at checkpoints, one every five new folders, a schedule fixed in advance. Each checkpoint is final. Between checkpoints the report shows evidence only. |
| 4 | λ = 8 has negative growth for small folders even under real effects. | λ comes from a table of group sizes, fixed at design time, maximizing the expected log e-value under a one-standard-deviation shift. Our own simulation agreed with the reviewer's table, for example 1v1 at −1.2 with λ = 8. |

**Noted, not changed:** validity rests on settings being assigned to runs independently of anything else that moves the loss. Assigning settings at random would turn that assumption into a guarantee. It is stated in the design note.

**Not taken:** online FDR (e-LORD, e-SAFFRON) for verdicts that stay live as hypotheses arrive. That is a bigger change, and checkpoints answer the problem.
