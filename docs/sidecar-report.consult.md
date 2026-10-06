# Consult brief — a report a person can read

> **Record.** Sent to Kimi K3 by hand on 2026-10-05. Its answer (a fixed report skeleton, the fenced model note, and the abstain paragraph) was built in commit 25f9404. The report was later rewritten to lead with its answer and to measure the seed spread; see `docs/sidecar-workbench.md`.

**Project:** RunForge. A Windows instrument for training runs. It draws every stored sample. It does not train.

**Your job.** Harden and polish the sidecar's weighing, and design the printed report. The report is the missing piece. A person who is not an engineer should be able to read it and say what happened, what was not tested, and what to do next. Change-nothing is an allowed ending.

**What we will check.** You write against the specimen below. Do not re-derive the numbers. If a sentence needs a number that is not in this brief, mark it as a placeholder. Do not fetch our files. Do not add a paper unless you give the authors, the year, the title, and the URL, and you say what sentence in the report that paper is allowed to support.

---

## What is already decided

These are closed. A recommendation that breaks one of them will not be used.

- A sample stays a record. The chart draws every point. It does not downsample. A gap is a gap. It is not drawn as zero.
- `training_summary.final_loss` is a marker beside the curve. It is not a point on the curve, and it is not a rank.
- The lowest sample stays visible even when it does not win. Hiding a deeper low because the line later climbs is the bug this instrument exists to stop.
- A field that is the same on every series was not tested. The report must not tell the reader to move it.
- An assumption is labeled. It is not a result.
- When the series with the lowest sample is not the series with the quietest neighborhood, the report abstains. It still names the lowest sample.
- The sidecar does not press Train. It does not call a cloud model. The app does not fetch the network. The reference catalog is already in the program.
- The chart remains the record. The report does not replace it. The report is the page you can hand to someone who will not stare at the chart.
- The report and the window must not disagree. If a model sentence contradicts a measured sentence, the measured sentence wins and the model sentence is dropped.

## Who the report is for

Someone who can look at a chart of loss over the run and understand that lower is better. They may not know what a seed, a median, an epoch, a cosine schedule, or LoRA means. They are deciding whether this run taught them anything, and whether to change a setting before the next one.

Write for that person. Keep the truth. Do not invent a winner to make the page feel finished.

---

## The specimen, measured 2026-10-05

Five runs. One shared recipe. The only thing that changes is the seed. Each run has 496 samples. Loss and learning rate are stored on the samples. The epoch axis runs from just after 0 to 8.

Shared recipe, so none of these is a result:

| Knob | Value |
|---|---|
| Method | bf16 LoRA |
| Learning rate | 0.00015 |
| Schedule | cosine |
| Warmup | 10 steps |
| Weight decay | 0 |
| Gradient clip | 1 |
| Batch | 1, accumulation 8, effective batch 8 |
| Epochs | 8 |
| Sequence length | 12288 |
| Prompt-loss weight | 0.1 |
| LoRA rank, alpha, dropout | 16, 32, 0.1 |
| Checkpoints | epochs 2, 4, and 8 |

LoRA scale alpha/r is 2. Rank and alpha did not move. The learning rate did not move. The effective batch did not move.

Warmup is 10 steps. A sample's epoch is not a step index, so the program does not turn 10 into a fraction of the run.

### What each series did

Neighborhood means every finite sample within half an epoch of that series' own lowest sample. Median is the middle of that window (the average of the two middle values when the count is even). "Next" is the next-lowest *other* sample in that window. A low is called a single sample only when that next value is more than twice the low. None of these five lows meets that test.

"Under a twentieth" means the learning rate on the lowest sample is under 5% of that series' own highest learning rate. The highest learning rate on every series here is 0.00015.

| Seed | First loss | Lowest sample | At epoch | Last sample | Neighborhood | Median | Next | Learning rate at the low |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| 13 | 12.1289 | 0.0176 | 7 | 0.0755 | 62 | 0.1165 | 0.0255 | 6.134e-6, under a twentieth |
| 42 | 10.6879 | 0.0251 | 7.4372 | 0.0481 | 62 | 0.1098 | 0.0341 | 2.022e-6, under a twentieth |
| 271 | 9.8087 | 0.0250 | 6.3077 | 0.0629 | 62 | 0.1280 | 0.0320 | 1.693e-5, not under a twentieth |
| 512 | 10.1197 | 0.0218 | 6.0324 | 0.0686 | 62 | 0.1235 | 0.0393 | 2.248e-5, not under a twentieth |
| 1024 | 9.1836 | 0.0347 | 7.5668 | 0.0471 | 58 | 0.0946 | 0.0471 | 1.225e-6, under a twentieth |

Every series climbs after its low. The lowest sample is seed 13. The lowest last sample is seed 1024. The quietest neighborhood is also seed 1024. Those are different series, so the weighing abstains.

`training_summary.final_loss` is about 0.69 to 0.72 on these runs (seed 1024 is 0.6911, seed 13 is 0.7123). It is not one of the 496 samples. A ranking by it crowns seed 1024 and hides seed 13's 0.0176.

The late window is noisy. Seed 13's extreme low sits next to a sample of 0.0255, and the middle of the surrounding half-epoch is 0.1165. The body of the curve, not the single lowest point, is where the seeds are hard to tell apart.

## What the sidecar does today

The window draws all five curves on a log-loss chart, a linked learning-rate strip, and a row around the lows. That row is linear. A bold line follows the lowest sample in each epoch, so the climb is visible. Spikes above the row's ceiling stay on the main chart and are gaps on the row.

The sidecar is a column in the same window. Top to bottom, today:

1. The measured finding, in gold: lowest sample is seed 13, lowest last sample is seed 1024, a last-sample ranking would hide the deeper low.
2. The lever: the seed. The recipe is shared.
3. The weighed lines: one neighborhood line per series, the abstain sentence, one learning-rate line per series, LoRA scale, warmup left uncomputed, one "shared, so not a result" line, and two labeled assumptions.
4. The local model's note, when one exists.
5. A reference list, open by default, under that note. Seven cards. Each card is a formula, a finding, a citation, and a URL.
6. Earlier notes from other recipes that used the same method, when any exist.
7. The `final_loss` markers, named as not on the curve.
8. A short line per series.

There is no report export on this screen. The history bench can save a CSV or a JSON entry. This screen cannot.

The reference cards, and only these, are the catalog:

| Knob | Formula the program states | Source |
|---|---|---|
| Seed | Same recipe, different seed. | Dodge, Ilharco, Schwartz, Farhadi, Hajishirzi, and Smith, 2020, Fine-Tuning Pretrained Language Models. https://arxiv.org/abs/2002.06305 |
| LoRA rank and alpha | Scale = alpha / r. | Hu, Shen, Wallis, Allen-Zhu, Li, Wang, Wang, and Chen, 2021, LoRA. https://arxiv.org/abs/2106.09685 |
| Learning rate and batch | When the batch is multiplied by k, multiply the learning rate by k. Applied only when the batch changes. | Goyal, Dollár, Girshick, Noordhuis, Wesolowski, Kyrola, Tulloch, Jia, and He, 2017, Accurate, Large Minibatch SGD. https://arxiv.org/abs/1706.02677 |
| Schedule | Cosine annealing decays the learning rate inside the run. | Loshchilov and Hutter, 2017, SGDR. https://arxiv.org/abs/1608.03983 |
| Weight decay | Decoupled weight decay is on the weights, not inside the adaptive step. For Adam, L2 on the gradient is not weight decay. | Loshchilov and Hutter, 2019, Decoupled Weight Decay Regularization. https://arxiv.org/abs/1711.05101 |
| Gradient clip | Clip when the norm exceeds the threshold. | Pascanu, Mikolov, and Bengio, 2013, On the difficulty of training recurrent neural networks. https://arxiv.org/abs/1211.5063 |
| LoRA dropout | Dropout inside the low-rank update. | Srivastava, Hinton, Krizhevsky, Sutskever, and Salakhutdinov, 2014, Dropout. https://www.jmlr.org/papers/v15/srivastava14a.html |

No prompt-loss paper is in the catalog. Do not invent one.

The two assumptions, printed only because those knobs are shared:

- Alpha stands in for the learning rate only when the initialization is scaled, and neither knob moved.
- The linear scaling rule would move the learning rate with the batch. The batch did not change, so the rule is not applied.

## What the local model did with this specimen

The model is local only. It prefers a 14B instruct model. Temperature 0.2. It is asked after the weighing is in the question. Three tries:

1. Before the weighing existed, it ranked the runs by `final_loss` and told us to move the shared learning rate. The question was tightened. That answer was rejected.
2. With the weighing in the question, it abstained, then gave the reason as "lowest sample versus lowest last sample." That is the gold finding, not the weighing. It skipped the decay split and the LoRA scale. A short token cap cut a later try off mid-sentence.
3. After the question said the abstain reason is the half-epoch median, it abstained for that reason and did not move a knob. It still did not restate which lows sit in the decay, and it did not state that alpha/r is 2. It rewrote the headline as a memo with markdown headings. The pane strips `**` and leading `#` on display. The stored note still has them.

The weighed lines on the pane are the reliable layer. The model note is a paraphrase that keeps the headline and drops the lines a person most needs.

---

## The gap

The column tells the truth and it does not read as a report. A person has to assemble the conclusion from a gold sentence, five neighborhood lines, five learning-rate lines, a scale, a warmup refusal, a shared-knob list, two assumptions, and a model memo that repeats the headline. The papers sit below that, so the first screen does not show them.

We want one report. It draws a conclusion in plain words. It stays faithful to the table above. It can be saved, and the same words can be shown in the window.

---

## Questions

Answer these. Skip any you cannot ground. Do not widen into a new product.

**Q1. Write the report for this specimen.** One to two pages, in the voice you recommend. A person who has trained a model once, and a person who has not, should both be able to say: what happened, why seed 13 is not the winner, why seed 1024 is not the winner either, which settings were not tested, and what to do next. Use only the numbers in the table. The 0.0176 sample stays in the report. Change-nothing is allowed. Do not say "move the learning rate."

**Q2. Who writes which sentence?** Split the report into sentences the program fills from the weighing, and sentences a model may write. Our evidence is that a local 14B model drops the non-obvious lines and sometimes swaps the abstain reason. We have not decided that no model can write prose. We have decided that every number in the report must match the weighing, and that a contradiction is dropped. If you still want a model paragraph, show the fence: what it is forbidden to contradict, and what the program prints instead when the model crosses the fence.

**Q3. What do we say when the honest answer is abstain?** Give the words. "Abstain" has been read as "the tool failed" and as "the runs are the same." Neither is what this table says. The neighborhood medians are not the same. The lowest point and the quietest neighborhood belong to different seeds. The reader still needs a next step.

**Q4. How do we talk about a setting nobody varied?** The failure we already hit was a recommendation to move the learning rate when every series used 0.00015. Show the sentence you would print for the shared learning rate, the shared LoRA scale, and the batch rule that was not applied. The reader should understand why the report refuses, without feeling lectured.

**Q5. Where do the papers go?** A non-engineer should not have to read an abstract to trust the page. An engineer should be able to find the paper that backs a sentence. Say what stays in the report body, what moves to a short "where this comes from" note, and which of the seven cards you would leave out of this specimen's report because the knob was not in play. If you think we are using one of the seven papers past what it supports, say so and name the paper.

**Q6. What is the printed thing?** We can save a file, the way the history bench already saves an export, and we can show the same text in the window. Say whether those two must be the same words. Say what you would title the file. Do not add a network fetch. Do not add a cloud model. Do not propose a database engine. The catalog above is the reference.

**Q7. What would you change in the column, and what would you leave?** The chart, the low row, and the gold finding already work. The column is 340 pixels wide. The weighed lines and the model memo fill it. The papers start below the fold. The model memo repeats the finding. Name only the changes that make the column and the report one argument instead of two essays. For each change, point at the specimen fact that makes it necessary.

**Q8. A second specimen, so the rules are not overfit to five seeds.** Two runs. Same recipe except LoRA rank 16 versus 32. You do not have the losses. Use placeholders and mark them. Write the two or three sentences that change when the rank actually differs, and the sentence that must still be labeled an assumption. Then say what the report does if those two neighborhoods disagree by less than the noise inside either run. Do not invent a measured margin.

---

## Return this, in this order

1. The specimen report, ready to read.
2. A sentence inventory. For each sentence: an id, who writes it (program or model), when it appears, and the pattern with placeholders.
3. The fence, if any model text remains. Include the fallback sentence the program prints when the model contradicts a measured line.
4. The abstain paragraph you want used whenever the lowest sample and the quietest neighborhood disagree.
5. Pane polish, as a short list. Each item tied to a fact in "What the sidecar does today" or "What the local model did."
6. What you would not change.
7. Any paper above that you would narrow or drop, with the reason. New papers only with author, year, title, URL, and the sentence they support.

Do not send code. Do not send a redesign of the chart. Do not send a plan to fetch the web from the app.
