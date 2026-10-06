# Consult brief: does RunForge's evidence across folders hold up?

**Ask:** check the statistics below. Is each step valid as stated? Say yes or no, with the reason. If a step fails, give the smallest fix. Do not redesign the product.

## Setting

A hypothesis says: "when knob K goes up, measure M goes down" (or up). It is fixed before the folders below are opened.

A folder holds a few fine-tuning runs: typically 1 to 5 per setting of K, across 2 settings, with everything else equal. Each run gives one number, M.

## Step 1: per-folder e-value

- **Setup.** Take the runs at the lowest setting of K (group L) and at the highest (group H). Let S be the share of the |L|·|H| pairs (l, h) with h below l, for "goes down", ties counting 1/2.
- **Definition.** e = exp(λS_obs) / ( (1/N) Σ_g exp(λS_g) ). The sum runs over all N ways to choose which |H| of the pooled values form group H. λ = 8 is fixed in advance, and folders above 20 runs are skipped.
- **Claim.** If K has no effect and the runs are exchangeable between settings, then E[e] = 1. The reason: the observed labeling is uniform over the N relabelings given the pooled values. (Cf. Koning 2023, arXiv:2310.01153.)
- **Tests we ran.** The average over every relabeling equals 1 to 1e-12, including with ties and with unequal group sizes. Values: a clean 3-vs-3 separation gives 9.53, a 3-vs-3 reversal gives 0.0032, and 1 vs 1 gives at most 2.

## Step 2: across folders

- **Rule.** Multiply the per-folder e-values, oldest folder first, counting a folder only if all three hold:
  - (a) It is not the folder the hypothesis was proposed on.
  - (b) It shares no run with a folder already counted. A run's identity is its seed plus a hash of its first 32 loss samples.
  - (c) It gave a test, meaning K varied and nothing else did.
- **Fixed order.** A folder tested again keeps its original place.
- **Claim.** Folders hold new, independent runs, and the decision to open another folder may depend on earlier results. So the product is a test supermartingale, and P(product ever ≥ 1/α) ≤ α by Ville's inequality (Grünwald, de Heide and Koolen 2024, "Safe testing"; Ramdas et al. 2023, arXiv:2210.01948).
- **Simulation.** Under no effect, with 4,000 sequences of 8 folders of mixed sizes, the product crossed 20 in at most 5%.

## Step 3: two directions and many hypotheses

- **Two directions.** For each hypothesis there are two products, E_for (declared direction) and E_against (opposite direction, same data). We use E = (E_for + E_against)/2 as the e-value for "K has no effect on M".
- **Many hypotheses.** e-BH runs at q = 0.05 across all K hypotheses on the bench (Wang and Ramdas 2022, JRSS-B 84(3)). Sort E from largest, and discover the top k*, where k* = max{k : E_(k) ≥ K/(q·k)}.
- **Direction call.** A discovered hypothesis is "supported" if E_for ≥ E_against, and "refuted" otherwise.
- **Claim.** e-BH controls the FDR for the null "no effect" under any dependence. The direction call is not separately error-controlled.

## Questions

1. Is Step 1's E[e] = 1 argument right when the pooled values contain ties, and when the groups differ in size?
2. Does excluding the proposal folder, and folders that share runs, keep the product valid? Is there a selection effect we missed, for example in the order folders are opened, or because a hypothesis can be proposed after seeing several folders?
3. Is averaging E_for and E_against, then running e-BH, valid for the null "no effect"? How much does the uncontrolled direction call matter here? Should "refuted" need its own threshold?
4. Hypotheses on the bench share folders, so their e-values are dependent. Does e-BH still apply as used, given that K grows as hypotheses are added over time?
5. Is λ = 8 a sensible fixed choice for groups of 1 to 5? Would a predictable plug-in (λ chosen from earlier folders only) be worth it?
