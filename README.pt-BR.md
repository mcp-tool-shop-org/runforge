<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.md">English</a>
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

RunForge é um instrumento para Windows, utilizado para ajustar finamente as execuções. Abra uma pasta de execuções e ele exibirá cada amostra armazenada, criará um relatório que começará com sua conclusão e fornecerá um ambiente de trabalho para um modelo local. No ambiente de trabalho, o modelo cria suas próprias ferramentas de fórmula, propõe a função de cada parâmetro e coleta evidências em várias pastas até que se chegue a uma conclusão.

RunForge lê os registros de treinamento. Ele não realiza o treinamento. Ele não baixa um modelo, não instala o PyTorch nem acessa um modelo na nuvem.

## O relatório

Abra uma pasta de arquivos `run-config*.json`. Eles podem estar na própria pasta ou em um nível abaixo. RunForge exibe cada amostra finita, sem reamostragem, e uma lacuna permanece uma lacuna. `training_summary.final_loss` é um marcador ao lado da curva, nunca um ponto nela, e o relatório nunca classifica com base nele.

O relatório começa com **Em resumo**: se uma execução é bem-sucedida e por quê. Para cinco conjuntos de dados de uma mesma configuração, o relatório pode apresentar algo como:

> Nenhuma execução é bem-sucedida. O conjunto de dados 512 tem o ponto único mais profundo, 0,0674 na época 3. O conjunto de dados 1024 tem o período mais estável em torno de seu valor mínimo: uma média de 0,3474, em comparação com 0,4041 em torno do valor mínimo do conjunto de dados 512. As médias das execuções estão a uma distância de 0,1508 umas das outras: maior do que a metade do período mais estável (0,1133), menor do que o período mais instável (0,3238). As execuções se separam apenas parcialmente.

Depois disso, o relatório detalha:
- as execuções
- por que uma é bem-sucedida ou nenhuma o é
- o que mudou e o que não mudou
- avaliações anteriores da mesma configuração
- o que fazer a seguir
- o que o relatório não pode informar
- de onde vem cada fórmula

Uma seção sem informações relevantes não será impressa. Uma configuração que foi a mesma em todas as execuções será listada como não testada, nunca como um parâmetro. O painel e o botão "Salvar relatório" contêm as mesmas informações.

## O ambiente de trabalho

Pressione **Perguntar** e um modelo local investigará as execuções por meio de cinco ferramentas: medir uma fórmula, comparar um parâmetro que mudou, manter uma nova ferramenta, propor uma hipótese e finalizar. O programa calcula cada resultado e escreve cada frase que contém um número. O modelo escolhe o que analisar e expressa isso em palavras. Sua nota final é rotulada como suas palavras, não como uma medição.

**Ferramentas que o modelo cria.** Uma ferramenta é uma fórmula em uma linguagem simples, avaliada uma vez por execução, por exemplo:
- `last / low`: quão longe a curva sobe após seu ponto mais baixo
- `slope_between(end_epoch - 1, end_epoch)`: quão íngreme é a última época
- `knob('lora_r')`: um valor de configuração

Uma fórmula não pode ler um arquivo, abrir a rede ou executar código. Uma nova ferramenta é mantida apenas se fornecer um valor em cada execução e não for uma duplicata. Ela permanece provisória até ser usada em uma segunda pasta. Fórmulas posteriores podem usá-la pelo nome, para que a biblioteca cresça com os dados. Você pode testar uma fórmula no painel, na caixa de fórmula.

**Hipóteses sobre parâmetros.** Uma hipótese nomeia um parâmetro, uma fórmula e uma direção, por exemplo, "quando a classificação LoRA aumenta, `last / low` diminui". Seu teste é fixo quando é proposto. Em cada pasta, o programa marca como:
- não testável: o parâmetro não mudou
- confuso: outro parâmetro mudou junto com ele
- inconclusivo
- ou um resultado apenas nessas execuções

Quando as execuções não conseguem validar uma hipótese, RunForge planeja o menor conjunto de execuções que: um parâmetro, duas configurações, três conjuntos de dados ou mais cada. Ele nunca as inicia.

**Evidências em várias pastas.** Cada pasta fornece um valor e: uma medida de evidência que tem uma média exatamente de 1 quando o parâmetro não tem efeito, para que possa ser multiplicada em várias pastas sem perder validade. Dois tipos de pasta são excluídos:
- qualquer pasta que contenha uma execução que o RunForge já tenha visto quando a hipótese foi registrada
- uma execução já contada

Conclusões, confirmadas ou refutadas, são emitidas apenas em pontos de verificação, um a cada cinco novas pastas. Cada ponto de verificação aplica o e-BH em uma taxa de descoberta falsa de 5% em ambas as direções de cada hipótese no ambiente de trabalho. Uma única hipótese precisa de cerca de três pastas limpas de três execuções por configuração. O método, suas fontes e uma revisão externa estão em [docs/sidecar-workbench.md](docs/sidecar-workbench.md) e [docs/evidence.consult.response.md](docs/evidence.consult.response.md).

O modelo é um Ollama local em `127.0.0.1:11434`. RunForge usa apenas um modelo que o Ollama relata que pode chamar ferramentas e ignora nomes marcados como da nuvem. Uma sessão é limitada a seis solicitações e dez chamadas de ferramentas. Sem um modelo local, o relatório ainda é válido, assim como a caixa de fórmula.

## O ambiente de histórico

Uma pasta com um backpropagate `run_history.json` abre o ambiente de histórico, ou a pasta acima de um diretório `output`. O ambiente lista as execuções, exibe os `loss_history` armazenados em ordem de arquivo, compara duas linhas e exporta a tabela ou a curva.

Quando `backprop` já está no PATH, os botões "Treinar", "Avaliar" e "Exportar modelo" iniciam esse comando e acompanham seu log. O aplicativo cria os argumentos e nada passa por um shell. Se `backprop` estiver faltando, os botões indicam isso. RunForge não baixa, instala ou fornece o backpropagate.

<p align="center"><img src="docs/bench-dark.png" alt="The history bench in the dark theme, open on a fixture folder" width="720"></p>

## Modelo de ameaças

**O que ele lê.** Uma pasta que você seleciona:
- uma pasta de histórico: `run_history.json` lá, ou `output/run_history.json` um nível abaixo
- uma pasta de série: `run-config*.json` lá e em seus filhos imediatos

Ele não percorre o restante do disco e nunca grava de volta nesses arquivos. Exportar e Salvar relatório gravam em um caminho que você seleciona.

**O que ele mantém.** Preferências (a última pasta e o tema) e `sidecar-memory.json` ficam juntos: no LocalState do pacote quando o aplicativo é empacotado e ao lado do executável quando não é. O arquivo de memória não contém nenhum caminho de pasta. Ele contém:
- as notas do modelo
- cada avaliação medida
- as ferramentas de fórmula aprendidas (no máximo 50)
- as hipóteses com seus resultados de teste (no máximo 60)
- os pontos de verificação

**O que é enviado para o modelo local.** Os nomes das execuções, os valores da receita e os resultados das ferramentas do programa são enviados para a porta de loopback `11434`. O caminho da pasta não é enviado. O modelo pode chamar apenas as cinco ferramentas da estação de trabalho, e o programa valida cada chamada.

**O que é iniciado.** O início do treinamento, da avaliação e da exportação do modelo `backprop` ocorre apenas quando você pressiona o botão na estação de trabalho e o programa já está no PATH. A função "Parar" encerra a árvore de processos iniciada por esta janela. O programa auxiliar nunca pressiona esses botões.

**O que nunca é acessado.** O manifesto do pacote não solicita `internetClient`. Não há telemetria nem conta, e a lista de referências do relatório é incorporada ao programa. O aplicativo não possui um modelo na nuvem, nem um shell, nem uma cópia do ambiente, nem um treinador, nem um download de modelo.

Como relatar uma vulnerabilidade está em [SECURITY.md](SECURITY.md).

## Compilação

Rust 1.98.1, edição 2024. O arquivo da cadeia de ferramentas fixa esta versão.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

Limites de cobertura:
- O CI falha se a cobertura de linhas for inferior a 90%.
- O Codecov mantém tanto o projeto quanto as novas linhas de cada solicitação de pull em 90%.

Os testes executam o loop do modelo em relação a um Ollama simulado na porta de loopback. Uma sessão em relação a um modelo local real é opcional:

```bash
RUNFORGE_LIVE_FOLDER=<series folder> cargo test -p runforge --features live live_workbench -- --nocapture
```

## Loja

O anúncio publicado é o produto `9PHL1HX0CGMF`, pacote `mcp-tool-shop.RunForge-Desktop`. A versão 2 substitui o aplicativo de classificação anterior, e o texto do anúncio deve indicar isso na mesma submissão. O aplicativo da Loja publicado permanece na versão 1.0.1 até que um pacote acima de `1.0.1.0` seja submetido. O projeto de referência está em [docs/CONTRACT.md](docs/CONTRACT.md).

Criado por [MCP Tool Shop](https://mcp-tool-shop.github.io/).
