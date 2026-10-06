import type { SiteConfig } from '@mcptoolshop/site-theme';

export const config: SiteConfig = {
  title: 'RunForge',
  description:
    'Windows instrument for fine-tuning runs. It draws every stored sample, writes a report that leads with its answer, and gives a local model a workbench to build formula tools and test what each knob does, with evidence gathered across folders.',
  logoBadge: 'RF',
  brandName: 'RunForge',
  repoUrl: 'https://github.com/mcp-tool-shop-org/runforge',
  footerText:
    'MIT Licensed — built by <a href="https://mcp-tool-shop.github.io/" style="color:var(--color-muted);text-decoration:underline">MCP Tool Shop</a>',

  hero: {
    badge: 'Windows · local model only',
    headline: 'Find out what your knobs',
    headlineAccent: 'actually do.',
    description:
      'Open a folder of fine-tuning runs. RunForge draws every stored sample and writes a report that starts with the answer. Then a local model works the data through tools the program computes: it builds new formulas, proposes what a knob does, and the evidence gathers across folders until a checkpoint can call it. No cloud model, no download, no trainer inside.',
    primaryCta: { href: '#workbench', label: 'See the workbench' },
    secondaryCta: { href: 'handbook/', label: 'Read the Handbook' },
    previews: [
      {
        label: 'In short',
        code: 'No run wins. Seed 512 has the deepest single point,\n0.0674 at epoch 3. Seed 1024 has the calmest stretch\naround its low. The runs only partly separate.',
      },
      {
        label: 'A tool the model built',
        code: 'post_low_recovery = last / low\n# how far the curve climbs after its low',
      },
      {
        label: 'A hypothesis',
        code: 'When LoRA rank goes up, low goes lower.\nEvidence so far: 20.38 for, 0.006835 against,\nfrom two folders. Next checkpoint in 3 folders.',
      },
    ],
  },

  sections: [
    {
      kind: 'features',
      id: 'report',
      title: 'The report',
      subtitle: 'It leads with its answer. Every number in it is measured, and the pane and the saved file are the same words.',
      features: [
        {
          title: 'In short, first',
          desc: 'Whether a run wins and why, in two or three sentences, then the runs, the argument, what changed and what did not, and what to do next. A part with nothing to say is not printed.',
        },
        {
          title: 'Seed noise, measured',
          desc: 'Each run gets a stretch around its low with a middle and a middle half. The report says whether the gap between runs is smaller than the noise inside one run, or larger.',
        },
        {
          title: 'Honest about the recipe',
          desc: 'A setting that was the same on every run is listed as untested, never as a lever. final_loss is a marker beside the curve, never a rank. The deepest point stays on the page even when it does not win.',
        },
      ],
    },
    {
      kind: 'features',
      id: 'workbench',
      title: 'The workbench',
      subtitle: 'A local model investigates through tools. The program computes every result and writes every number.',
      features: [
        {
          title: 'Tools it builds',
          desc: 'A tool is a formula over the stored samples, such as last / low or slope_between(end_epoch - 1, end_epoch). It is kept only if it has a value on every run and is not a duplicate, and later formulas can use it by name. It cannot read a file or run code.',
        },
        {
          title: 'Hypotheses with a fixed test',
          desc: 'A hypothesis names a knob, a formula and a direction when it is proposed. The program marks it not testable, confounded, or inconclusive, and plans the smallest set of runs that would settle it. RunForge never starts them.',
        },
        {
          title: 'Evidence across folders',
          desc: 'Each folder gives an e-value, multiplied across folders of new runs. Verdicts come only at checkpoints, every five new folders, under e-BH at a 5% false discovery rate. The method had an outside review.',
        },
      ],
    },
    {
      kind: 'data-table',
      id: 'verdict',
      title: 'Verdicts',
      subtitle: 'How a verdict is earned: what decides a hypothesis, and what never does.',
      columns: ['', 'Counts', 'Does not count'],
      rows: [
        ['Folder', 'New runs, opened after the hypothesis was registered', 'Any run RunForge had already seen'],
        ['Knob', 'One knob changed, everything else equal', 'A knob that was the same on every run, or one that changed with another'],
        ['Evidence', 'An e-value per folder, multiplied across folders', 'The model\'s words'],
        ['Verdict', 'A checkpoint every five new folders, e-BH at 5% over both directions', 'A look at the evidence between checkpoints'],
        ['Next runs', 'One knob, two settings, three seeds or more each', 'A Train button pressed by the sidecar'],
      ],
    },
    {
      kind: 'code-cards',
      id: 'formulas',
      title: 'Formulas',
      subtitle: 'Numbers, + - * / ^, parentheses, learned tool names, and the measures RunForge computes per run.',
      cards: [
        {
          title: 'Around the low',
          code: 'low\nlow_epoch\nmedian   # within half an epoch of the low\nq3 - q1  # its middle half',
        },
        {
          title: 'A window of epochs',
          code: 'median_between(2, 4)\nslope_between(end_epoch - 1, end_epoch)\nlr_between(0, 1)',
        },
        {
          title: 'Recipe and arithmetic',
          code: "knob('lora_r')\nlast / low\nmax(low, 0.01) * 2",
        },
      ],
    },
    {
      kind: 'features',
      id: 'bench',
      title: 'History bench',
      subtitle: 'A backpropagate folder opens its own screen: the stored runs, compared and exported.',
      features: [
        {
          title: 'The file you opened',
          desc: 'run_history.json in that folder, or output/run_history.json one level down. It does not walk the disk and it does not merge a second file.',
        },
        {
          title: 'Stored loss, in file order',
          desc: 'The chart is loss_history as stored. final_loss is a column, not a point on the line. A null sample is a gap, not a zero.',
        },
        {
          title: 'Compare, export, and launch',
          desc: 'Compare two rows and see which settings differ. Train, Eval, and Export model start a backprop that is already installed. Nothing goes through a shell. Stop ends the process tree this window started.',
        },
      ],
    },
    {
      kind: 'code-cards',
      id: 'commands',
      title: 'Commands',
      subtitle: 'What the history bench buttons build. Blank model and blank steps are left off. Steps, when present, are a positive whole number.',
      cards: [
        {
          title: 'Train',
          code: 'backprop train --data notes.json --model small --steps 12 --output out',
        },
        {
          title: 'Eval',
          code: 'backprop eval newer --output out',
        },
        {
          title: 'Export model',
          code: 'backprop export ckpt --output out',
        },
      ],
    },
    {
      kind: 'api',
      id: 'sentences',
      title: 'Sentences',
      subtitle: 'These lines are fixed. The log under them is the child program output.',
      apis: [
        {
          signature: 'Open a folder first.',
          description: 'Train, Eval, or Export model before a folder is open.',
        },
        {
          signature: 'Choose a data file.',
          description: 'Train with an empty data path.',
        },
        {
          signature: 'Steps must be a positive whole number.',
          description: 'A steps field that is not empty and not a positive whole number.',
        },
        {
          signature: 'backprop is not on PATH. RunForge does not install it.',
          description:
            'The lookup finished and did not find backprop.exe, backprop.com, or an extensionless backprop.',
        },
        {
          signature: 'Select a run.',
          description: 'Eval with no selected run.',
        },
        {
          signature: 'The selected run has no checkpoint.',
          description: 'Export model when that field is empty.',
        },
        {
          signature: 'A command is already running.',
          description: 'A second start while the first is live.',
        },
        {
          signature: 'No command is running.',
          description: 'Stop when nothing was started.',
        },
        {
          signature: 'backprop could not be started.',
          description:
            'The process could not be started, including when kill-on-close could not be assigned.',
        },
        {
          signature: 'That value cannot be passed as an argument.',
          description: 'A path or name that is empty, contains a null, or begins with a dash.',
        },
      ],
    },
  ],
};
