<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.md">English</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center"><img src="docs/bench-dark.png" alt="The RunForge window in the dark theme, open on a fixture folder" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# RunForge

RunForge est l’outil Windows qui permet d’analyser le dossier de résultats d’un [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate). Ouvrez le dossier qui contient `run_history.json`, ou le dossier situé au-dessus d’un dossier `output`. La fenêtre affiche les exécutions, trace la courbe de perte enregistrée, compare deux lignes et exporte le tableau ou la courbe.

Backpropagate est le programme d’entraînement. Cette application ne contient pas le programme d’entraînement, ne télécharge pas de modèle et n’inclut pas PyTorch. Lorsque `backprop` est déjà présent dans le PATH, les boutons « Entraîner », « Évaluer » et « Exporter le modèle » lancent cette commande et suivent son journal. Les arguments sont générés par l’application. Rien n’est transmis via un interpréteur de commandes. Si `backprop` est manquant, les boutons l’indiquent. RunForge ne télécharge pas, n’installe pas et n’intègre pas Backpropagate.

La courbe représente les valeurs `loss_history` enregistrées, dans l’ordre des fichiers, et au maximum le nombre d’échantillons conservés par le programme d’entraînement. `final_loss` est une colonne. Elle n’est pas ajoutée à la ligne. Un échantillon nul est un espace, et non un zéro.

## Modèle de menace

RunForge lit un dossier que vous sélectionnez. Il ouvre `run_history.json` dans ce dossier, ou `output/run_history.json` un niveau plus bas, et il peut exporter le tableau, la courbe ou une entrée. Les préférences, le dernier dossier et le thème sont écrits dans le dossier LocalState du package lorsque l’application est empaquetée, et à côté du fichier exécutable lorsqu’elle ne l’est pas. Les boutons « Entraîner », « Évaluer » et « Exporter le modèle » lancent `backprop` uniquement lorsque vous appuyez sur le bouton et que ce programme est déjà présent dans le PATH. Le journal correspond à la sortie de ce programme. L’option « Arrêter » met fin à l’arborescence des processus lancée par cette fenêtre.

Données auxquelles il n’accède pas : le programme d’entraînement, le téléchargement d’un modèle, l’installation de Backpropagate, un interpréteur de commandes, une copie de l’environnement ou la télémétrie. Il n’y a pas de compte.

Les autorisations restent définies sur le dossier que vous avez ouvert, le chemin d’exportation que vous sélectionnez, le fichier de données que vous choisissez et le fichier de préférences. L’application ne demande pas d’accès au réseau.

La procédure à suivre pour signaler une vulnérabilité est décrite dans [SECURITY.md](SECURITY.md).

## Compilation

Rust 1.98.1, édition 2024. Le fichier de la chaîne d’outils fixe cette version.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

L’exécution continue effectue la commande de couverture et télécharge `lcov.info`. Codecov signale une erreur si la couverture des lignes est inférieure à 90 %. La boîte de dialogue de fichier n’est pas ouverte par les tests.

## Publication

La publication correspond au produit `9PHL1HX0CGMF`, au package `mcp-tool-shop.RunForge-Desktop`. La version 2 remplace l’application de classification précédente, et le texte de la publication doit l’indiquer dans la même soumission. Ce dépôt ne contient pas encore ce package. L’identité du package ne change pas lorsque le package est compilé.

La conception de référence est [docs/CONTRACT.md](docs/CONTRACT.md). Ce dépôt prend en charge la compilation de la source 2.0.0. L’application Store publiée reste la version 1.0.1 jusqu’à ce qu’un package supérieur à `1.0.1.0` soit soumis.

Créé par [MCP Tool Shop](https://mcp-tool-shop.github.io/).
