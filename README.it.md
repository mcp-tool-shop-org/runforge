<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.md">English</a> | <a href="README.pt-BR.md">Português (BR)</a>
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

RunForge è uno strumento per Windows progettato per ottimizzare i risultati di un processo. Apre una cartella contenente i risultati, analizza ogni campione memorizzato, genera un report che inizia con una sintesi e fornisce a un modello locale un ambiente di lavoro. In questo ambiente, il modello crea i propri strumenti di analisi, suggerisce la funzione di ogni parametro e raccoglie dati da diverse cartelle fino a quando non si raggiunge una conclusione.

RunForge legge i dati di addestramento. Non esegue l'addestramento. Non scarica un modello, non include PyTorch e non utilizza un modello basato su cloud.

## Il report

Apri una cartella contenente i file `run-config*.json`. Questi possono essere direttamente nella cartella o in una sottocartella. RunForge analizza ogni campione disponibile, senza effettuare alcun ricampionamento, e mantiene eventuali lacune. `training_summary.final_loss` è un indicatore accanto alla curva, non un punto sulla curva, e il report non effettua classifiche basate su questo indicatore.

Il report inizia con la sezione **In sintesi**: indica se un processo ha avuto successo e perché. Per cinque iterazioni di una stessa configurazione, il report potrebbe essere simile a questo:

> Nessun processo ha avuto successo. L'iterazione 512 presenta il punto più basso, con un valore di 0,0674 all'epoca 3. L'iterazione 1024 presenta il periodo più stabile intorno al suo valore minimo: un valore medio di 0,3474, rispetto a 0,4041 per l'iterazione 512. I valori medi delle iterazioni si discostano di non più di 0,1508 l'uno dall'altro: un valore inferiore alla metà del periodo più stabile (0,1133) e superiore a quello del periodo più variabile (0,3238). Le iterazioni sono solo parzialmente distinte.

Successivamente, il report presenta:
- le iterazioni
- il motivo per cui una ha avuto successo o nessuna
- cosa è cambiato e cosa è rimasto invariato
- analisi precedenti della stessa configurazione
- cosa fare successivamente
- cosa il report non può indicare
- da dove proviene ogni formula

Una sezione senza informazioni utili non viene visualizzata. Un'impostazione che è rimasta la stessa in tutte le iterazioni viene elencata come non testata, non come un parametro. La finestra e il pulsante "Salva report" contengono lo stesso testo.

## L'ambiente di lavoro

Premi **Chiedi** e un modello locale analizza le iterazioni utilizzando cinque strumenti: misura una formula, confronta un parametro che è cambiato, conserva un nuovo strumento, propone un'ipotesi e conclude. Il programma calcola ogni risultato e scrive ogni frase che contiene un numero. Il modello sceglie cosa analizzare e lo esprime a parole. La nota finale è etichettata come "osservazioni del modello", non come una misurazione.

**Strumenti creati dal modello.** Uno strumento è una formula in un linguaggio semplice, valutata una volta per ogni iterazione, ad esempio:
- `last / low`: quanto la curva sale dopo il suo punto più basso
- `slope_between(end_epoch - 1, end_epoch)`: quanto è ripida l'ultima epoca
- `knob('lora_r')`: un valore di configurazione

Una formula non può leggere un file, aprire la rete o eseguire codice. Un nuovo strumento viene conservato solo se fornisce un valore per ogni iterazione e non è un duplicato. Rimane provvisorio fino a quando non viene utilizzato in una seconda cartella. Le formule successive possono utilizzarlo per nome, in modo che la libreria cresca con i dati. È possibile provare una formula nell'apposita casella nella finestra.

**Ipotesi sui parametri.** Un'ipotesi indica un parametro, una formula e una direzione, ad esempio "quando il valore di LoRA aumenta, `last / low` diminuisce". Il test è definito quando l'ipotesi viene proposta. Per ogni cartella, il programma indica se l'ipotesi è:
- non testabile: il parametro non è cambiato
- confusa: un altro parametro è cambiato insieme ad esso
- inconcludente
- o fornisce un risultato basato solo su quelle iterazioni

Quando le iterazioni non riescono a confermare un'ipotesi, RunForge pianifica il set minimo di iterazioni che consentirebbe di: un parametro, due impostazioni, tre o più iterazioni per ciascuna impostazione. Non le avvia automaticamente.

**Evidenza tra le cartelle.** Ogni cartella fornisce un valore e: una misura dell'evidenza che è esattamente 1 quando il parametro non ha alcun effetto, in modo che possa essere moltiplicata tra le cartelle senza perdere validità. Due tipi di cartella vengono esclusi:
- qualsiasi cartella contenente un'iterazione che RunForge ha già analizzato quando l'ipotesi è stata registrata
- un'iterazione già conteggiata

Le conclusioni, confermate o confutate, vengono emesse solo in corrispondenza di punti di controllo, uno ogni cinque nuove cartelle. Ogni punto di controllo applica il metodo e-BH con un tasso di falsi positivi del 5% per entrambe le direzioni di ogni ipotesi nell'ambiente di lavoro. Un'ipotesi singola richiede circa tre cartelle pulite, ciascuna contenente tre iterazioni per impostazione. Il metodo, le sue fonti e una revisione esterna sono disponibili in [docs/sidecar-workbench.md](docs/sidecar-workbench.md) e [docs/evidence.consult.response.md](docs/evidence.consult.response.md).

Il modello è un'istanza locale di Ollama su `127.0.0.1:11434`. RunForge utilizza solo un modello che Ollama indica come in grado di utilizzare gli strumenti e ignora i nomi contrassegnati come basati su cloud. Una sessione è limitata a sei richieste e dieci chiamate agli strumenti. In assenza di un modello locale, il report rimane valido, così come la casella delle formule.

## L'ambiente di lavoro storico

Una cartella contenente un file di backpropagation `run_history.json` apre l'ambiente di lavoro storico, oppure la cartella sopra una cartella `output`. L'ambiente di lavoro elenca le iterazioni, visualizza i dati `loss_history` memorizzati in ordine di file, confronta due righe ed esporta la tabella o la curva.

Quando `backprop` è già presente in PATH, i pulsanti "Esegui addestramento", "Esegui valutazione" ed "Esporta modello" avviano quel comando e ne seguono i log. L'app crea gli argomenti e nulla passa attraverso una shell. Se `backprop` è mancante, i pulsanti lo indicano. RunForge non scarica, installa o include backpropagation.

<p align="center"><img src="docs/bench-dark.png" alt="The history bench in the dark theme, open on a fixture folder" width="720"></p>

## Modello di minaccia

**Cosa legge.** Una cartella che si seleziona:
- una cartella storica: `run_history.json` presente, o `output/run_history.json` in una sottocartella
- una cartella di serie: `run-config*.json` presente e nelle sue sottocartelle immediate

Non esamina il resto del disco e non scrive mai di nuovo in tali file. "Esporta" e "Salva report" scrivono in un percorso che si seleziona.

**Cosa conserva.** Le preferenze (l'ultima cartella e il tema) e `sidecar-memory.json` sono memorizzate insieme: nel pacchetto LocalState quando l'app è impacchettata e accanto all'eseguibile quando non lo è. Il file di memoria non contiene alcun percorso di cartella. Contiene:
- le note del modello
- ogni misurazione effettuata
- gli strumenti di analisi appresi (massimo 50)
- le ipotesi con i relativi risultati (massimo 60)
- i punti di controllo

**Cosa viene trasmesso al modello locale.** I nomi delle esecuzioni, i valori delle ricette e i risultati degli strumenti del programma vengono inviati alla porta di loopback `11434`. Il percorso della cartella non viene trasmesso. Il modello può utilizzare solo i cinque strumenti del workbench e il programma convalida ogni chiamata.

**Da dove inizia.** L'avvio dei modelli di addestramento, valutazione ed esportazione `backprop` avviene solo quando si preme il pulsante sulla console e il programma è già presente nel PATH. L'arresto termina l'albero dei processi avviato da questa finestra. Il processo secondario non preme mai questi pulsanti.

**Cosa non viene mai toccato.** Il file manifest del pacchetto non richiede `internetClient`. Non sono presenti dati di telemetria né account e l'elenco di riferimento del report è integrato nel programma. L'app non dispone di un modello cloud, di una shell, di una copia dell'ambiente, di un trainer né di un download del modello.

Le istruzioni su come segnalare una vulnerabilità sono disponibili in [SECURITY.md](SECURITY.md).

## Compilazione

Rust 1.98.1, edizione 2024. Il file della toolchain lo specifica.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

Soglie di copertura:
- Il CI fallisce se la copertura delle righe è inferiore al 90%.
- Codecov mantiene sia il progetto che le nuove righe di ogni pull request al 90%.

I test eseguono il ciclo del modello su un'istanza fittizia di Ollama sulla porta di loopback. Una sessione su un modello locale reale è facoltativa:

```bash
RUNFORGE_LIVE_FOLDER=<series folder> cargo test -p runforge --features live live_workbench -- --nocapture
```

## Archiviazione

L'inserzione pubblicata è il prodotto `9PHL1HX0CGMF`, il pacchetto `mcp-tool-shop.RunForge-Desktop`. La versione 2 sostituisce l'app di classificazione precedente e il testo dell'inserzione deve indicarlo nella stessa richiesta. L'app Store pubblicata rimane la versione 1.0.1 fino a quando non viene inviato un pacchetto superiore a `1.0.1.0`. Il progetto di riferimento è disponibile all'indirizzo [docs/CONTRACT.md](docs/CONTRACT.md).

Creato da [MCP Tool Shop](https://mcp-tool-shop.github.io/).
