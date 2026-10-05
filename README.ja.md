<p align="center">
  <a href="README.md">English</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
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

RunForgeは、[backpropagate](https://github.com/mcp-tool-shop-org/backpropagate)の出力フォルダーを扱うWindowsアプリケーションです。`run_history.json`を含むフォルダー、または`output`ディレクトリの親フォルダーを開きます。このウィンドウには、実行履歴、保存された損失のグラフ、2つの行の比較が表示され、テーブルまたはグラフをエクスポートできます。

Backpropagateはトレーニングツールです。このアプリにはトレーニングツールが含まれておらず、モデルをダウンロードしたり、PyTorchをインストールしたりすることはありません。`backprop`がすでにPATHに含まれている場合、[Train]、[Eval]、[Export model]ボタンを押すと、対応するコマンドが実行され、そのログが表示されます。引数はアプリによって生成されます。シェル経由で引数を渡すことはありません。`backprop`が存在しない場合、ボタンにその旨が表示されます。RunForgeはbackpropagateをダウンロード、インストール、またはバンドルしません。

グラフは、保存された`loss_history`のデータであり、ファイル順に表示され、トレーニングツールが保持したサンプルの最大数までです。`final_loss`は列です。行に追加されることはありません。nullサンプルは、ゼロではなく、データの欠損を表します。

## 脅威モデル

RunForgeは、ユーザーが選択したフォルダーを読み込みます。そのフォルダー内の`run_history.json`、または1レベル下の`output/run_history.json`を開き、テーブル、グラフ、または1つのエントリをエクスポートできます。設定、最後のフォルダー、テーマは、アプリがパッケージ化されている場合はパッケージのLocalStateに、そうでない場合は実行ファイルの近くに書き込まれます。[Train]、[Eval]、[Export model]ボタンを押すと、`backprop`が実行され、そのプログラムがすでにPATHに含まれている必要があります。ログは、そのプログラムの出力です。[Stop]を押すと、このウィンドウによって開始されたプロセスツリーが終了します。

アプリがアクセスしないデータ：トレーニングツール、モデルのダウンロード、backpropagateのインストール、シェル、環境のコピー、またはテレメトリー。アカウントは使用しません。

アクセス許可は、開いたフォルダー、選択したエクスポートパス、選択したデータファイル、および設定ファイルに適用されます。アプリはネットワーク機能の要求を行いません。

脆弱性を報告する方法については、[SECURITY.md](SECURITY.md)を参照してください。

## ビルド

Rust 1.98.1、エディション2024。ツールチェーンファイルでバージョンが固定されています。

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

CIはカバレッジコマンドを実行し、`lcov.info`をアップロードします。Codecovは、行カバレッジが90%未満の場合、ステータスを失敗とします。ファイルダイアログは、テストによって開かれません。

## ストア

公開されているリストは、製品`9PHL1HX0CGMF`、パッケージ`mcp-tool-shop.RunForge-Desktop`です。バージョン2は、以前の分類アプリを置き換えます。リストのテキストには、同じ提出時にその旨を記載する必要があります。このリポジトリには、まだそのパッケージは含まれていません。パッケージのビルド時に、パッケージIDは変更されません。

設計仕様は、[docs/CONTRACT.md](docs/CONTRACT.md)に記載されています。このリポジトリは、2.0.0のソースビルドをサポートします。公開されているストアアプリは、パッケージのバージョンが`1.0.1.0`を超えるものが提出されるまで、1.0.1の分類アプリのままです。

[MCP Tool Shop](https://mcp-tool-shop.github.io/)によってビルドされました。
