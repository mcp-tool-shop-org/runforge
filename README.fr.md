<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.md">English</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/mcp-tool-shop-org/runforge"><img src="https://codecov.io/gh/mcp-tool-shop-org/runforge/graph/badge.svg" alt="Coverage"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# RunForge

RunForge est un outil Windows permettant d’affiner les exécutions. Ouvrez un dossier contenant des exécutions, et il affichera chaque échantillon enregistré, rédigera un rapport qui commencera par sa conclusion, et fournira à un modèle local un espace de travail. Dans cet espace de travail, le modèle crée ses propres outils de formule, propose la fonction de chaque paramètre et collecte des données dans différents dossiers jusqu’à ce qu’une conclusion soit atteinte.

RunForge lit les enregistrements d’entraînement. Il ne réalise pas d’entraînement. Il ne télécharge pas de modèle, n’installe pas PyTorch et n’utilise pas de modèle cloud.

## Le rapport

Ouvrez un dossier contenant des fichiers `run-config*.json`. Ils peuvent se trouver dans le dossier lui-même ou dans un sous-dossier. RunForge affiche chaque échantillon fini, sans rééchantillonnage, et un écart reste un écart. `training_summary.final_loss` est un marqueur à côté de la courbe, jamais un point sur celle-ci, et le rapport ne classe jamais les résultats en fonction de ce marqueur.

Le rapport commence par : **En résumé** : si une exécution est gagnante, et pourquoi. Pour cinq séries d’une recette, le rapport pourrait ressembler à ceci :

> Aucune exécution n’est gagnante. La série 512 a le point unique le plus bas, 0,0674 à l’époque 3. La série 1024 a la période la plus stable autour de son point bas : une valeur médiane de 0,3474, contre 0,4041 autour du point bas de la série 512. Les valeurs médianes des séries se situent à moins de 0,1508 les unes des autres : plus large que la moitié de la période la plus stable (0,1133), plus étroite que celle de la série la plus instable (0,3238). Les séries ne sont que partiellement distinctes.

Après cela, le rapport présente :
- les exécutions
- pourquoi une exécution est gagnante ou pourquoi aucune ne l’est
- ce qui a changé et ce qui n’a pas changé
- les évaluations antérieures de la même recette
- ce qu’il faut faire ensuite
- ce que le rapport ne peut pas vous dire
- d’où provient chaque formule

Une section qui n’a rien à dire n’est pas affichée. Un paramètre qui était le même pour toutes les exécutions est répertorié comme non testé, et non comme un levier. Le panneau et l’option « Enregistrer le rapport » contiennent les mêmes informations.

## L’espace de travail

Appuyez sur **Poser une question**, et un modèle local analysera les exécutions à l’aide de cinq outils : mesurer une formule, comparer un paramètre qui a changé, conserver un nouvel outil, proposer une hypothèse et terminer. Le programme calcule chaque résultat et écrit chaque phrase contenant un nombre. Le modèle choisit ce qu’il doit examiner et le formule. Sa note de conclusion est étiquetée comme étant sa propre formulation, et non une mesure.

**Outils créés par le modèle.** Un outil est une formule dans un petit langage, évaluée une fois par exécution, par exemple :
- `last / low` : la distance parcourue par la courbe après son point bas
- `slope_between(end_epoch - 1, end_epoch)` : la pente de la dernière époque
- `knob('lora_r')` : une valeur de recette

Une formule ne peut pas lire un fichier, ouvrir le réseau ou exécuter du code. Un nouvel outil est conservé uniquement s’il fournit une valeur pour chaque exécution et n’est pas un doublon. Il reste provisoire jusqu’à ce qu’il soit utilisé dans un deuxième dossier. Les formules ultérieures peuvent l’utiliser par son nom, de sorte que la bibliothèque s’enrichit avec les données. Vous pouvez essayer une formule vous-même dans la zone de formule du panneau.

**Hypothèses sur les paramètres.** Une hypothèse nomme un paramètre, une formule et une direction, par exemple : « lorsque la valeur de LoRA augmente, `last / low` diminue ». Son test est fixé lorsqu’elle est proposée. Pour chaque dossier, le programme la marque comme étant l’une des suivantes :
- non testable : le paramètre n’a pas changé
- confondue : un autre paramètre a changé en même temps
- non concluante
- ou un résultat basé uniquement sur ces exécutions

Lorsque les exécutions ne peuvent pas valider une hypothèse, RunForge planifie l’ensemble minimal d’exécutions qui : un paramètre, deux réglages, trois séries ou plus pour chaque réglage. Il ne les lance jamais.

**Preuves dans différents dossiers.** Chaque dossier fournit une valeur e : une mesure de la preuve qui est exactement égale à 1 lorsque le paramètre n’a aucun effet, de sorte qu’elle puisse être multipliée entre les dossiers sans perdre de validité. Deux types de dossiers sont exclus :
- tout dossier contenant une exécution que RunForge a déjà vue lorsque l’hypothèse a été enregistrée
- une exécution déjà prise en compte

Les conclusions, confirmées ou réfutées, ne sont émises qu’aux points de contrôle, soit une fois tous les cinq nouveaux dossiers. Chaque point de contrôle applique e-BH avec un taux de faux positifs de 5 % pour les deux directions de chaque hypothèse dans l’espace de travail. Une seule hypothèse nécessite environ trois dossiers propres contenant trois exécutions par réglage. La méthode, ses sources et une évaluation externe sont disponibles dans [docs/sidecar-workbench.md](docs/sidecar-workbench.md) et [docs/evidence.consult.response.md](docs/evidence.consult.response.md).

Le modèle est un Ollama local sur `127.0.0.1:11434`. RunForge n’utilise qu’un modèle qu’Ollama indique comme étant capable d’utiliser des outils, et ignore les noms marqués comme étant des modèles cloud. Une session est limitée à six requêtes et dix appels d’outils. Sans modèle local, le rapport reste valable, tout comme la zone de formule.

## L’espace de travail historique

Un dossier contenant un fichier de rétropropagation `run_history.json` ouvre l’espace de travail historique, ou le dossier au-dessus d’un dossier `output`. L’espace de travail répertorie les exécutions, affiche les `loss_history` enregistrés dans l’ordre des fichiers, compare deux lignes et exporte le tableau ou la courbe.

Lorsque `backprop` est déjà dans PATH, les boutons « Entraîner », « Évaluer » et « Exporter le modèle » dans cet espace de travail lancent cette commande et suivent son journal. L’application crée les arguments, et rien ne passe par une ligne de commande. Si `backprop` est manquant, les boutons l’indiquent. RunForge ne télécharge, n’installe ni ne fournit de logiciel de rétropropagation.

<p align="center"><img src="docs/bench-dark.png" alt="The history bench in the dark theme, open on a fixture folder" width="720"></p>

## Modèle de menace

**Ce qu’il lit.** Un dossier que vous choisissez :
- un dossier historique : `run_history.json` à l’intérieur, ou `output/run_history.json` un niveau plus bas
- un dossier de séries : `run-config*.json` à l’intérieur et dans ses sous-dossiers immédiats

Il ne parcourt pas le reste du disque, et il n’écrit jamais dans ces fichiers. Les options « Exporter » et « Enregistrer le rapport » écrivent dans un chemin que vous choisissez.

**Ce qu’il conserve.** Les préférences (le dernier dossier et le thème) et `sidecar-memory.json` sont stockés ensemble : dans le LocalState du package lorsque l’application est empaquetée, et à côté de l’exécutable lorsqu’elle ne l’est pas. Le fichier mémoire ne contient aucun chemin de dossier. Il contient :
- les notes du modèle
- chaque évaluation mesurée
- les outils de formule appris (au maximum 50)
- les hypothèses avec leurs résultats de test (au maximum 60)
- les points de contrôle

**Ce qui est transmis au modèle local.** Les noms des exécutions, les valeurs des recettes et les résultats des outils du programme sont envoyés au port de boucle `11434`. Le chemin du dossier, lui, ne l’est pas. Le modèle ne peut appeler que les cinq outils de l’atelier, et le programme valide chaque appel.

**Ce qui déclenche le processus.** Le démarrage des modèles d’entraînement, d’évaluation et d’exportation `backprop` ne se fait que lorsque vous appuyez sur le bouton de l’historique et que le programme est déjà dans le PATH. L’arrêt met fin à l’arborescence des processus lancée par cette fenêtre. Le processus secondaire n’appuie jamais sur ces boutons.

**Ce qui n’est jamais modifié.** Le manifeste du package ne demande pas `internetClient`. Il n’y a pas de télémétrie ni de compte, et la liste de références du rapport est intégrée au programme. L’application ne dispose pas de modèle cloud, d’interface en ligne de commande, de copie de l’environnement, d’outil d’entraînement ni de téléchargement de modèle.

La procédure à suivre pour signaler une vulnérabilité est décrite dans [SECURITY.md](SECURITY.md).

## Compilation

Rust 1.98.1, édition 2024. Le fichier de la chaîne d’outils le spécifie.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

Seuils de couverture :
- Les tests CI échouent si la couverture des lignes est inférieure à 90 %.
- Codecov maintient une couverture de 90 % pour le projet et pour les nouvelles lignes de chaque demande de tirage.

Les tests exécutent le modèle en boucle sur une version simulée d’Ollama en mode boucle. Une session avec un modèle local réel est facultative :

```bash
RUNFORGE_LIVE_FOLDER=<series folder> cargo test -p runforge --features live live_workbench -- --nocapture
```

## Magasin

La liste publiée correspond au produit `9PHL1HX0CGMF`, au package `mcp-tool-shop.RunForge-Desktop`. La version 2 remplace l’application de classification précédente, et le texte de la liste doit l’indiquer dans la même soumission. L’application du magasin publiée reste la version 1.0.1 de l’application de classification jusqu’à ce qu’un package supérieur à `1.0.1.0` soit soumis. La conception de référence est disponible dans [docs/CONTRACT.md](docs/CONTRACT.md).

Créé par [MCP Tool Shop](https://mcp-tool-shop.github.io/).
