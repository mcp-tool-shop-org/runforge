# Sample runs

Two folders of real fine-tuning runs to open in RunForge. Each holds five seeds of a bf16 LoRA fine-tune of Qwen2.5-7B-Instruct on one A100: the same recipe, so the spread between seeds is seed noise.

- `arc-b2`: five runs, four epochs each.
- `arc-v1`: five runs, eight epochs each, split across `podA` and `podB` to show that RunForge reads one level down.

Open either folder with **Open series**.

The files are the trainer's own records with one block removed: `inputs`, which named the training data's paths and fingerprints. RunForge does not read it.

`runforge-samples.zip` holds both folders, for downloading in one file.
