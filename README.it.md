<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.md">English</a> | <a href="README.pt-BR.md">Português (BR)</a>
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

RunForge è uno strumento per Windows utilizzato per registrare i dati di un addestramento. Apri una cartella. Una cartella di serie visualizza ogni campione memorizzato, la ricetta condivisa e un report in formato testo semplice. Una cartella di backpropagation apre la cronologia: l'elenco delle esecuzioni, la perdita memorizzata, un confronto tra due righe ed esportazione.

L'immagine qui sopra mostra la cronologia. Una cartella di serie è una schermata diversa.

Backpropagation è il trainer. Questa applicazione non contiene il trainer, non scarica un modello e non include PyTorch. Quando `backprop` è già presente in PATH, i comandi "Train", "Eval" ed "Export model" sulla cronologia avviano tale comando e ne seguono i log. Gli argomenti sono generati dall'applicazione. Nulla viene passato tramite una shell. Se `backprop` è mancante, i pulsanti lo indicano. RunForge non scarica, installa o include backpropagation.

## Una cartella di serie

Il file è `run-config*.json` nella cartella che si apre, e gli stessi nomi si trovano un livello più in basso in una cartella secondaria. RunForge non cerca ulteriormente e non esegue una ricerca sul disco. Un file non valido viene saltato e conteggiato. Una chiave duplicata rifiuta solo quel file.

Ogni campione rimane una registrazione: l'epoca o il passo, la perdita, il tasso di apprendimento e tutti gli altri campi che sono stati registrati. Ogni campione finito viene visualizzato. La visualizzazione non viene ricampionata. Una perdita nulla o non finita è un'interruzione, non uno zero. `training_summary.final_loss`, quando presente nel file, è un marcatore accanto alla curva. Non viene aggiunto alla riga e il report non viene ordinato in base a esso.

Il pannello laterale stampa un report basato su tali misurazioni. Il pannello e "Salva report" sono le stesse parole. "Salva" utilizza la stessa finestra di dialogo dell'esportazione della cronologia. L'opzione "Chiedi" può aggiungere una breve nota su come leggere la pagina. Tale nota potrebbe non contenere una cifra, nominare un'impostazione o nominare un risultato. Una nota che supera i limiti viene eliminata e il pannello lo indica. Se nessun modello locale risponde, il report rimane comunque valido.

Il modello, quando richiesto, è un Ollama locale sulla porta `127.0.0.1` `11434`. Un nome contrassegnato come "cloud" non viene scelto. Il pannello laterale non preme "Train". La domanda non include il percorso della cartella. Una nota salvata rimane nelle preferenze. Non viene riscritta nei file di serie.

## La cronologia

Apri la cartella che contiene `run_history.json` o la cartella sopra una directory `output`. Il file nella cartella aperta viene utilizzato quando entrambi esistono. La finestra elenca le esecuzioni, visualizza la perdita memorizzata, confronta due righe ed esporta la tabella o la curva.

La curva è la `loss_history` memorizzata, in ordine di file, e contiene al massimo il numero di campioni che il programma di addestramento ha conservato. `final_loss` è una colonna. Non viene aggiunta alla riga. Un campione nullo è uno spazio vuoto, non uno zero.

"Train", "Eval" ed "Export model" rimangono su questa schermata. Non si trovano nella schermata di serie.

## Modello di minaccia

RunForge legge una cartella che si sceglie. Una cartella di cronologia si apre `run_history.json` lì, o `output/run_history.json` un livello più in basso. Una cartella di serie si apre `run-config*.json` in quella cartella e nelle sue cartelle secondarie immediate. L'applicazione non esamina il resto del disco e non unisce i due tipi di record. "Esporta" e "Salva report" scrivono in un percorso che si sceglie. Le preferenze, l'ultima cartella e il tema vengono scritti nel pacchetto LocalState quando l'applicazione è impacchettata, e accanto all'eseguibile quando non lo è.

"Train", "Eval" ed "Export model" avviano `backprop` solo quando si preme il pulsante sulla cronologia e tale programma è già presente in PATH. Il log è l'output di quel programma. "Stop" termina l'albero dei processi avviato da questa finestra. Il pannello laterale non preme tali pulsanti.

Il pannello laterale può connettersi a un server di modelli sull'indirizzo di loopback. Il manifesto del pacchetto non richiede `internetClient`. Non c'è telemetria e non c'è un account. L'elenco di riferimento del report è all'interno del programma. Non viene recuperato.

Dati che non vengono toccati: il trainer, il download di un modello, l'installazione di backpropagation, una shell, una copia dell'ambiente, un modello cloud o una riscrittura nei file di serie o `run_history.json`.

I permessi rimangono sulla cartella che si è aperta, sul percorso di esportazione che si sceglie, sul file di dati che si sceglie, sul file delle preferenze e sulla porta di loopback `11434` quando si richiede il modello locale.

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
