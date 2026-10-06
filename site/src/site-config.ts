import type { SiteConfig } from '@mcptoolshop/site-theme';

export const config: SiteConfig = {
  title: 'RunForge',
  description:
    'Windows instrument for a training record. A series folder draws every sample and a plain-text report. A backpropagate folder opens the history bench. It does not contain the trainer.',
  logoBadge: 'RF',
  brandName: 'RunForge',
  repoUrl: 'https://github.com/mcp-tool-shop-org/runforge',
  footerText:
    'MIT Licensed — built by <a href="https://mcp-tool-shop.github.io/" style="color:var(--color-muted);text-decoration:underline">MCP Tool Shop</a>',

  hero: {
    badge: 'Windows · source build 2.0.0',
    headline: 'Read the run, then weigh it',
    headlineAccent: 'not the trainer.',
    description:
      'Open one folder. A series folder draws every stored sample, the shared recipe, and a report written from those measurements. A backpropagate folder opens the history bench. Train starts <code>backprop</code> only when that program is already on PATH, and only from that bench. This app does not download a model and does not ship PyTorch.',
    primaryCta: { href: '#commands', label: 'See the commands' },
    secondaryCta: { href: 'handbook/', label: 'Read the Handbook' },
    previews: [
      {
        label: 'Open',
        code: 'run_history.json\nor output/run_history.json',
      },
      {
        label: 'Train',
        code: 'backprop train --data FILE [--model NAME] [--steps N] --output DIR',
      },
      {
        label: 'Eval and export',
        code: 'backprop eval RUN_ID --output DIR\nbackprop export CHECKPOINT --output DIR',
      },
    ],
  },

  sections: [
    {
      kind: 'features',
      id: 'series',
      title: 'A series folder',
      subtitle: 'Every sample stays. The report is the same words in the pane and in the file.',
      features: [
        {
          title: 'The samples, unflattened',
          desc: 'run-config files in the folder you open, and one level down. Every finite sample is drawn. A gap stays a gap. final_loss is a marker beside the curve, not a point on it.',
        },
        {
          title: 'One report',
          desc: 'The sidecar prints the weighing as plain text. Save report writes that string. The lowest sample stays on the page. When the deepest point and the calmest neighborhood disagree, the report says there is no winner.',
        },
        {
          title: 'A local note, fenced',
          desc: 'Ask talks to a local model on the loopback address. It does not press Train and it does not call a cloud model. A note with a digit, a setting, or a verdict is dropped, and the pane says so.',
        },
      ],
    },
    {
      kind: 'features',
      id: 'bench',
      title: 'The history bench',
      subtitle: 'One backpropagate folder. The history file in it. The curve the trainer already stored.',
      features: [
        {
          title: 'The file you opened',
          desc: 'Open reads run_history.json in that folder, or output/run_history.json one level down. It does not walk the disk and it does not merge a second file.',
        },
        {
          title: 'Stored loss, in file order',
          desc: 'The chart is loss_history as stored. final_loss is a column. It is not appended to the line. A null sample is a gap, not a zero.',
        },
        {
          title: 'Compare, export, and launch',
          desc: 'Compare draws two rows and lists hyperparameters that differ. Export keeps keys the trainer added. Train, Eval, and Export model start an already-installed backprop. Nothing goes through a shell. Stop ends the process tree this window started.',
        },
      ],
    },
    {
      kind: 'data-table',
      id: 'boundary',
      title: 'Where this app stops',
      subtitle:
        'The published Store listing is still the 1.0.1 classifier until a later package is submitted.',
      columns: ['', 'RunForge', 'Not in this app'],
      rows: [
        ['History', 'The folder you pick', 'A search of the disk'],
        ['Series', 'run-config files, one level down', 'A resampled curve, or a rank by final_loss'],
        ['Report', 'The measurements, saved as the same words', 'A cloud model, or a fetched paper'],
        ['Curve', 'Stored loss_history', 'final_loss appended as a point'],
        ['Train, Eval, Export model', 'backprop already on PATH, on the history bench', 'A copy of the trainer, Python, or PyTorch'],
        ['Stop', 'The process tree this window started', 'A cancel protocol inside backpropagate'],
        ['Store', 'Same product, source build 2.0.0', 'A submitted 2.0 package'],
      ],
    },
    {
      kind: 'code-cards',
      id: 'commands',
      title: 'Commands the buttons build',
      subtitle: 'Blank model and blank steps are left off. Steps, when present, are a positive whole number.',
      cards: [
        {
          title: 'Train',
          code: 'backprop train --data notes.json --model small --steps 12 --output out',
        },
        {
          title: 'Train, defaults kept',
          code: 'backprop train --data notes.json --output out',
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
      title: 'What the window says',
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
