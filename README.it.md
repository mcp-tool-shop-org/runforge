<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.md">English</a> | <a href="README.pt-BR.md">Português (BR)</a>
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

RunForge è l'interfaccia di Windows per una cartella di output di [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate). Aprire la cartella che contiene `run_history.json`, oppure la cartella immediatamente superiore a una cartella `output`. La finestra elenca le esecuzioni, visualizza la perdita memorizzata, confronta due righe ed esporta la tabella o la curva.

Backpropagate è il programma di addestramento. Questa applicazione non contiene il programma di addestramento, non scarica un modello e non include PyTorch. Quando `backprop` è già presente in PATH, i comandi Addestra, Valuta ed Esporta modello avviano tale comando e ne seguono il log. Gli argomenti sono generati dall'applicazione. Nessun dato viene passato tramite una shell. Se `backprop` non è presente, i pulsanti lo indicano. RunForge non scarica, installa o include backpropagate.

La curva è la `loss_history` memorizzata, in ordine di file, e contiene al massimo il numero di campioni che il programma di addestramento ha conservato. `final_loss` è una colonna. Non viene aggiunta alla riga. Un campione nullo è uno spazio vuoto, non uno zero.

## Modello di minaccia

RunForge legge una cartella selezionata dall'utente. Apre `run_history.json` in quella cartella, oppure `output/run_history.json` un livello più in profondità, e può esportare la tabella, la curva o una singola voce. Le preferenze, l'ultima cartella e il tema vengono salvati nel pacchetto LocalState quando l'applicazione è impacchettata, e accanto all'eseguibile quando non lo è. I comandi Addestra, Valuta ed Esporta modello avviano `backprop` solo quando si preme il pulsante e tale programma è già presente in PATH. Il log è l'output di quel programma. Stop termina l'albero dei processi avviato da questa finestra.

Dati che non vengono toccati: il programma di addestramento, il download di un modello, l'installazione di backpropagate, una shell, una copia dell'ambiente o la telemetria. Non è richiesto alcun account.

Le autorizzazioni rimangono sulla cartella aperta, sul percorso di esportazione selezionato, sul file di dati selezionato e sul file delle preferenze. L'applicazione non richiede alcuna funzionalità di rete.

Le istruzioni su come segnalare una vulnerabilità sono disponibili in [SECURITY.md](SECURITY.md).

## Compilazione

Rust 1.98.1, edizione 2024. Il file della toolchain lo specifica.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

Il sistema di integrazione continua esegue il comando di copertura e carica `lcov.info`. Codecov fa fallire lo stato quando la copertura delle righe è inferiore al 90%. La finestra di dialogo dei file non viene aperta dai test.

## Archiviazione

L'inserzione pubblicata è il prodotto `9PHL1HX0CGMF`, il pacchetto `mcp-tool-shop.RunForge-Desktop`. La versione 2 sostituisce la precedente applicazione di classificazione e il testo dell'inserzione deve indicarlo nella stessa versione. Questo repository non contiene ancora tale pacchetto. L'identità del pacchetto non cambia quando il pacchetto viene compilato.

La specifica di progettazione è disponibile in [docs/CONTRACT.md](docs/CONTRACT.md). Questo repository supporta la compilazione della versione 2.0.0. L'applicazione Store pubblicata rimane la versione 1.0.1 fino a quando non viene inviata una versione superiore a `1.0.1.0`.

Realizzato da [MCP Tool Shop](https://mcp-tool-shop.github.io/).
