<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.md">English</a>
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

RunForge é um programa para Windows usado para registrar dados de treinamento. Abra uma pasta. Uma pasta de séries exibe cada amostra armazenada, a receita compartilhada e um relatório em texto simples. Uma pasta de retropropagação abre o histórico: a lista de execuções, a perda armazenada, uma comparação de duas linhas e a exportação.

A imagem acima é o histórico. Uma pasta de séries é uma tela diferente.

A retropropagação é o treinador. Este aplicativo não contém o treinador, não baixa um modelo e não inclui o PyTorch. Quando `backprop` já estiver no PATH, os comandos "Treinar", "Avaliar" e "Exportar modelo" no histórico iniciarão esse comando e seguirão seu log. Os argumentos são criados pelo aplicativo. Nada é passado por meio de um shell. Se `backprop` estiver faltando, os botões indicarão isso. RunForge não baixa, instala ou inclui a retropropagação.

## Uma pasta de séries

O arquivo é `run-config*.json` na pasta que você abre, e os mesmos nomes estão um nível abaixo, em uma pasta filha. RunForge não procura mais a fundo e não pesquisa o disco. Um arquivo inválido é ignorado e contabilizado. Uma chave duplicada rejeita apenas esse arquivo.

Cada amostra permanece como um registro: a época ou o passo, a perda, a taxa de aprendizado e todos os outros campos que foram registrados. Cada amostra finita é exibida. A visualização não é reamostrada. Uma perda nula ou não finita é uma lacuna, não um zero. `training_summary.final_loss`, quando o arquivo o tiver, é um marcador ao lado da curva. Não é anexado à linha, e o relatório não é classificado por ele.

O relatório secundário imprime um relatório com base nessas medições. O painel e "Salvar relatório" são as mesmas palavras. "Salvar" usa a mesma caixa de diálogo da exportação do histórico. A opção "Perguntar" pode adicionar uma breve nota sobre como ler a página. Essa nota pode não conter um dígito, nomear uma configuração ou nomear um resultado. Uma nota que ultrapassa o limite é descartada, e o painel indica isso. Se nenhum modelo local responder, o relatório ainda será gerado.

O modelo, quando solicitado, é um Ollama local na porta `127.0.0.1` e `11434`. Um nome marcado como "nuvem" não é escolhido. O relatório secundário não pressiona o botão "Treinar". A pergunta não inclui o caminho da pasta. Uma nota mantida permanece com as preferências. Não é reescrita nos arquivos de séries.

## O histórico

Abra a pasta que contém `run_history.json` ou a pasta acima de um diretório `output`. O arquivo na pasta aberta é usado quando ambos existem. A janela lista as execuções, exibe a perda armazenada, compara duas linhas e exporta a tabela ou a curva.

A curva é o `loss_history` armazenado, em ordem de arquivo, no máximo o número de amostras que o treinador manteve. `final_loss` é uma coluna. Não é anexado à linha. Uma amostra nula é uma lacuna, não um zero.

Os comandos "Treinar", "Avaliar" e "Exportar modelo" permanecem nesta tela. Eles não estão na tela de séries.

## Modelo de ameaças

RunForge lê uma pasta que você seleciona. Uma pasta de histórico é aberta `run_history.json` lá, ou `output/run_history.json` um nível abaixo. Uma pasta de séries é aberta `run-config*.json` nessa pasta e em seus filhos imediatos. O aplicativo não percorre o restante do disco e não mescla os dois tipos de registro. "Exportar" e "Salvar relatório" gravam em um caminho que você seleciona. As preferências, a última pasta e o tema são gravados no pacote LocalState quando o aplicativo é empacotado e ao lado do arquivo executável quando não é.

Os comandos "Treinar", "Avaliar" e "Exportar modelo" iniciam `backprop` somente quando você pressiona o botão no histórico e esse programa já está no PATH. O log é a saída desse programa. "Parar" encerra a árvore de processos que esta janela iniciou. O relatório secundário não pressiona esses botões.

O relatório secundário pode se conectar a um servidor de modelo no endereço de loopback. O manifesto do pacote não solicita `internetClient`. Não há telemetria e não há conta. A lista de referência do relatório está dentro do programa. Não é obtida externamente.

Dados que não são alterados: o treinador, um download de modelo, uma instalação de retropropagação, um shell, uma cópia do ambiente, um modelo em nuvem ou uma reescrita nos arquivos de séries ou `run_history.json`.

As permissões permanecem na pasta que você abriu, no caminho de exportação que você seleciona, no arquivo de dados que você seleciona, no arquivo de preferências e na porta de loopback `11434` quando você solicita o modelo local.

Como relatar uma vulnerabilidade está em [SECURITY.md](SECURITY.md).

## Compilação

Rust 1.98.1, edição 2024. O arquivo da cadeia de ferramentas fixa essa versão.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

O CI executa o comando de cobertura e carrega `lcov.info`. O Codecov falha no status quando a cobertura de linha estiver abaixo de 90%. A caixa de diálogo de arquivo não é aberta pelos testes.

## Loja

A listagem publicada é o produto `9PHL1HX0CGMF`, o pacote `mcp-tool-shop.RunForge-Desktop`. A versão 2 substitui o aplicativo de classificação anterior, e o texto da listagem deve indicar isso na mesma submissão. Este repositório ainda não contém esse pacote. A identidade do pacote não muda quando o pacote é compilado.

O projeto de referência é [docs/CONTRACT.md](docs/CONTRACT.md). Este repositório oferece suporte à compilação da fonte 2.0.0. O aplicativo da Loja publicado permanece na versão 1.0.1 até que um pacote acima de `1.0.1.0` seja enviado.

Criado por [MCP Tool Shop](https://mcp-tool-shop.github.io/).
