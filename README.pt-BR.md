<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.md">English</a>
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

RunForge é a ferramenta do Windows para uma pasta de saída do [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate). Abra a pasta que contém `run_history.json` ou a pasta acima de um diretório `output`. A janela lista as execuções, desenha a perda armazenada, compara duas linhas e exporta a tabela ou a curva.

Backpropagate é o treinador. Este aplicativo não contém o treinador, não baixa um modelo e não inclui o PyTorch. Quando `backprop` já estiver no PATH, os comandos Treinar, Avaliar e Exportar modelo iniciam esse comando e acompanham seu log. Os argumentos são criados pelo aplicativo. Nada é passado por meio de um shell. Se `backprop` estiver faltando, os botões indicam isso. RunForge não baixa, instala ou inclui o backpropagate.

A curva é o `loss_history` armazenado, em ordem de arquivo, no máximo o número de amostras que o treinador manteve. `final_loss` é uma coluna. Não é anexado à linha. Uma amostra nula é uma lacuna, não um zero.

## Modelo de ameaças

RunForge lê uma pasta que você seleciona. Ele abre `run_history.json` nessa pasta ou `output/run_history.json` um nível abaixo e pode exportar a tabela, a curva ou uma entrada. As preferências, a última pasta e o tema são gravados no pacote LocalState quando o aplicativo é empacotado e, além disso, ao lado do arquivo executável quando não está. Os comandos Treinar, Avaliar e Exportar modelo iniciam `backprop` somente quando você pressiona o botão e esse programa já está no PATH. O log é a saída desse programa. Parar encerra a árvore de processos que esta janela iniciou.

Dados que não são acessados: o treinador, o download de um modelo, a instalação do backpropagate, um shell, uma cópia do ambiente ou telemetria. Não há nenhuma conta.

As permissões permanecem na pasta que você abriu, no caminho de exportação que você seleciona, no arquivo de dados que você seleciona e no arquivo de preferências. O aplicativo não solicita nenhuma capacidade de rede.

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
