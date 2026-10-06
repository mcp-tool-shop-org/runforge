<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.md">English</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center"><img src="docs/bench-dark.png" alt="The history bench in the dark theme, open on a fixture folder" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# RunForge

RunForge est un outil Windows pour gérer les données d’entraînement. Ouvrez un dossier. Un dossier de séries affiche tous les exemples enregistrés, la recette partagée et un rapport en texte brut. Un dossier de rétropropagation ouvre l’historique : la liste des exécutions, la perte enregistrée, une comparaison de deux lignes et l’exportation.

L’image ci-dessus représente l’historique. Un dossier de séries est un écran différent.

La rétropropagation est le programme d’entraînement. Cette application ne contient pas le programme d’entraînement, ne télécharge pas de modèle et n’inclut pas PyTorch. Lorsque `backprop` est déjà présent dans le PATH, les boutons « Entraîner », « Évaluer » et « Exporter le modèle » sur l’historique lancent cette commande et suivent son journal. Les arguments sont générés par l’application. Rien n’est transmis via un shell. Si `backprop` est manquant, les boutons l’indiquent. RunForge ne télécharge pas, n’installe pas et n’intègre pas la rétropropagation.

## Un dossier de séries

Le fichier est `run-config*.json` dans le dossier que vous ouvrez, et les mêmes noms se trouvent un niveau plus bas dans un dossier enfant. RunForge ne recherche pas plus loin et n’explore pas le disque. Un fichier incorrect est ignoré et comptabilisé. Une clé en double refuse ce fichier uniquement.

Chaque exemple reste une donnée : l’époque ou l’étape, la perte, le taux d’apprentissage et tous les autres champs qui ont été enregistrés. Chaque exemple fini est affiché. La vue n’est pas rééchantillonnée. Une perte nulle ou non finie est un trou, et non un zéro. `training_summary.final_loss`, lorsque le fichier le contient, est un marqueur à côté de la courbe. Il n’est pas ajouté à la ligne, et le rapport n’est pas classé en fonction de celui-ci.

Le fichier annexe imprime un rapport à partir de ces mesures. Le panneau et l’option « Enregistrer le rapport » sont les mêmes. L’option « Enregistrer » utilise la même boîte de dialogue que l’exportation de l’historique. L’option « Aide » peut ajouter une brève note sur la façon de lire la page. Cette note ne peut pas contenir de chiffre, mentionner un paramètre ou indiquer un résultat. Une note qui dépasse les limites est supprimée, et le panneau l’indique. Si aucun modèle local n’est disponible, le rapport reste valide.

Le modèle, lorsque vous le demandez, est un Ollama local sur le port `127.0.0.1`, `11434`. Un nom étiqueté comme « cloud » n’est pas sélectionné. Le fichier annexe ne lance pas l’entraînement. La question n’inclut pas le chemin du dossier. Une note conservée est stockée dans les préférences. Elle n’est pas réécrite dans les fichiers de séries.

## L’historique

Ouvrez le dossier qui contient `run_history.json`, ou le dossier situé au-dessus d’un dossier `output`. Le fichier dans le dossier ouvert est utilisé lorsque les deux existent. La fenêtre affiche les exécutions, affiche la perte enregistrée, compare deux lignes et exporte le tableau ou la courbe.

La courbe représente les valeurs `loss_history` enregistrées, dans l’ordre des fichiers, et au maximum le nombre d’échantillons conservés par le programme d’entraînement. `final_loss` est une colonne. Elle n’est pas ajoutée à la ligne. Un échantillon nul est un espace, et non un zéro.

Les options « Entraîner », « Évaluer » et « Exporter le modèle » restent sur cet écran. Elles ne sont pas sur l’écran des séries.

## Modèle de menace

RunForge lit un dossier que vous sélectionnez. Un dossier d’historique s’ouvre `run_history.json`, ou `output/run_history.json` un niveau plus bas. Un dossier de séries s’ouvre `run-config*.json` dans ce dossier et dans ses dossiers enfants immédiats. L’application n’explore pas le reste du disque et ne fusionne pas les deux types d’enregistrements. Les options « Exporter » et « Enregistrer le rapport » écrivent dans un chemin que vous sélectionnez. Les préférences, le dernier dossier et le thème sont écrits dans le package LocalState lorsque l’application est empaquetée, et à côté de l’exécutable lorsqu’elle ne l’est pas.

Les options « Entraîner », « Évaluer » et « Exporter le modèle » lancent `backprop` uniquement lorsque vous appuyez sur le bouton de l’historique et que ce programme est déjà présent dans le PATH. Le journal est la sortie de ce programme. L’option « Arrêter » met fin à l’arborescence des processus lancée par cette fenêtre. Le fichier annexe n’appuie pas sur ces boutons.

Le fichier annexe peut se connecter à un serveur de modèle sur l’adresse de bouclage. Le manifeste du package ne demande pas `internetClient`. Il n’y a pas de télémétrie et pas de compte. La liste de référence du rapport se trouve à l’intérieur du programme. Elle n’est pas récupérée.

Données auxquelles il n’accède pas : le programme d’entraînement, le téléchargement d’un modèle, l’installation de la rétropropagation, un shell, une copie de l’environnement, un modèle cloud ou une réécriture dans les fichiers de séries ou `run_history.json`.

Les autorisations restent sur le dossier que vous avez ouvert, le chemin d’exportation que vous sélectionnez, le fichier de données que vous sélectionnez, le fichier de préférences et le port de bouclage `11434` lorsque vous demandez le modèle local.

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
