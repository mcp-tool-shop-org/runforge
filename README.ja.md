<p align="center">
  <a href="README.md">English</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
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

RunForgeは、トレーニング記録用のWindowsアプリケーションです。まず、1つのフォルダーを開きます。シリーズフォルダーには、保存されているすべてのサンプル、共有レシピ、およびプレーンテキスト形式のレポートが表示されます。バックプロパゲーションフォルダーを開くと、履歴ウィンドウが表示されます。履歴ウィンドウには、実行リスト、保存された損失、2つの行の比較、およびエクスポート機能が含まれます。

上記の画像は、履歴ウィンドウです。シリーズフォルダーは、別の画面に表示されます。

バックプロパゲーションは、トレーナーです。このアプリケーションにはトレーナーが含まれておらず、モデルをダウンロードしたり、PyTorchを同梱したりすることはありません。もし`backprop`がすでにPATH環境変数に設定されている場合、履歴ウィンドウの「Train（トレーニング）」、「Eval（評価）」、および「Export model（モデルのエクスポート）」ボタンをクリックすると、そのコマンドが実行され、ログが表示されます。引数はアプリケーションによって自動的に生成されます。シェル経由で引数を渡すことはありません。もし`backprop`が見つからない場合、ボタンにはその旨が表示されます。RunForgeは、バックプロパゲーションをダウンロード、インストール、または同梱することはありません。

## シリーズフォルダー

ファイルは、開いたフォルダー内に`run-config*.json`として存在し、その1階層下のサブフォルダーにも同じ名前で存在します。RunForgeは、それ以上の階層を検索したり、ディスク全体を検索したりすることはありません。1つの不正なファイルが見つかった場合、そのファイルはスキップされ、カウントされます。重複するキーがある場合、そのファイルのみが拒否されます。

各サンプルは、記録として保持されます。記録には、エポックまたはステップ、損失、学習率、およびログに記録されたその他のすべてのフィールドが含まれます。すべての有限サンプルが表示されます。表示は再サンプリングされません。nullまたは有限でない損失は、ギャップとして扱われ、ゼロとして扱われることはありません。ファイルに`training_summary.final_loss`が含まれている場合、それは曲線に隣接するマーカーとして表示されます。マーカーは行に追加されず、レポートはそれに基づいてランク付けされません。

サイドカーは、これらの測定値から1つのレポートを出力します。ペインと「レポートを保存」は同じ意味です。保存機能は、履歴エクスポートと同じダイアログを使用します。確認時に、ページを読み方に関する短いメモを追加できます。そのメモには、数字を含めることはできず、設定の名前を記述したり、結果を記述したりすることはできません。制限を超えるメモは破棄され、ペインにその旨が表示されます。ローカルモデルが見つからない場合でも、レポートは有効です。

要求された場合、モデルはローカルのOllamaで、ポートは`127.0.0.1`、ポートは`11434`です。クラウドとしてタグ付けされた名前は選択されません。サイドカーは「Train」ボタンを押すことはありません。質問には、フォルダーパスは含まれません。保存されたメモは、設定とともに保持されます。メモは、シリーズファイルに書き戻されることはありません。

## 履歴ウィンドウ

`run_history.json`を含むフォルダー、または`output`ディレクトリの1つ上のフォルダーを開きます。両方のファイルが存在する場合、開いたフォルダー内のファイルが優先されます。ウィンドウには、実行、保存された損失、2つの行の比較、およびテーブルまたは曲線のエクスポートが表示されます。

グラフは、保存された`loss_history`のデータであり、ファイル順に表示され、トレーニングツールが保持したサンプルの最大数までです。`final_loss`は列です。行に追加されることはありません。nullサンプルは、ゼロではなく、データの欠損を表します。

「Train（トレーニング）」、「Eval（評価）」、および「Export model（モデルのエクスポート）」は、この画面に表示されます。シリーズ画面には表示されません。

## 脅威モデル

RunForgeは、選択したフォルダーを読み込みます。履歴フォルダーは、そこに`run_history.json`、または1階層下の場所に`output/run_history.json`として開きます。シリーズフォルダーは、そのフォルダーとその直接の子フォルダーに`run-config*.json`として開きます。アプリケーションは、残りのディスク全体を検索したり、2つの種類の記録をマージしたりすることはありません。「エクスポート」および「レポートを保存」は、選択したパスに書き込みます。設定（最後のフォルダーとテーマ）は、アプリケーションがパッケージ化された場合にパッケージのLocalStateに、パッケージ化されていない場合は実行ファイルの横に書き込まれます。

「Train（トレーニング）」、「Eval（評価）」、および「Export model（モデルのエクスポート）」は、履歴ウィンドウのボタンをクリックし、そのプログラムがすでにPATH環境変数に設定されている場合にのみ、`backprop`が開始されます。ログは、そのプログラムの出力です。停止すると、このウィンドウによって開始されたプロセスツリーが終了します。サイドカーは、これらのボタンを押すことはありません。

サイドカーは、ループバックアドレス上のモデルサーバーに接続できます。パッケージマニフェストは、`internetClient`を要求しません。テレメトリーやアカウントはありません。レポートの参照リストは、プログラム内にあります。外部から取得されることはありません。

アプリケーションが操作しないデータ：トレーナー、モデルのダウンロード、バックプロパゲーションのインストール、シェル、環境のコピー、クラウドモデル、またはシリーズファイルや`run_history.json`への書き戻し。

権限は、開いたフォルダー、選択したエクスポートパス、選択したデータファイル、設定ファイル、およびローカルモデルに要求した場合のループバックポート`11434`に保持されます。

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
