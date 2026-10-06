<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.md">English</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
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

RunForge 是一款用于训练记录的 Windows 工具。打开一个文件夹。一系列文件夹会显示每个存储的样本、共享的配方以及纯文本报告。一个反向传播文件夹会打开历史记录面板：运行列表、存储的损失值、两个行的比较以及导出功能。

上图是历史记录面板。一系列文件夹是一个不同的界面。

反向传播是训练器。此应用程序不包含训练器，不下载模型，也不包含 PyTorch。当 `backprop` 已经在 PATH 环境变量中时，在历史记录面板上点击“训练”、“评估”和“导出模型”按钮，将启动该命令并跟踪其日志。参数由应用程序构建。没有任何内容会通过 shell 传递。如果缺少 `backprop`，按钮会显示相应的提示。RunForge 不会下载、安装或包含反向传播。

## 一系列文件夹

该文件位于您打开的文件夹中的 `run-config*.json`，并且在子文件夹中，下一级也有相同名称的文件。RunForge 不会进一步搜索，也不会搜索整个磁盘。如果发现一个损坏的文件，它将被跳过并计数。如果存在重复的键，则只会拒绝该文件。

每个样本都保留为一条记录：包括 epoch 或 step、损失值、学习率以及所有其他已记录的字段。每个有限的样本都会被显示。视图不会重新采样。空值或非有限的损失值表示一个间隙，而不是零。当文件包含 `training_summary.final_loss` 时，它会在曲线旁边显示一个标记。它不会附加到行中，并且报告也不会根据它进行排序。

侧边栏会从这些测量值中生成一份报告。面板和“保存报告”使用相同的措辞。“保存”使用与历史记录导出相同的对话框。提示可能会添加一个简短的注释，说明如何阅读该页面。该注释可能不包含数字、命名设置或命名结论。如果注释超出了范围，则会被删除，并且面板会显示相应的提示。如果没有任何本地模型可用，报告仍然有效。

当您要求时，该模型是一个位于 `127.0.0.1` 端口 `11434` 上的本地 Ollama。带有“云”标签的名称不会被选择。侧边栏不会点击“训练”。问题不包括文件夹路径。保存的注释将与首选项一起保存。它不会写回到系列文件中。

## 历史记录面板

打开包含 `run_history.json` 的文件夹，或者打开包含 `output` 目录的上一级文件夹。如果两个文件夹都存在，则打开的文件夹中的文件优先。该窗口会列出运行情况、显示存储的损失值、比较两个行，并导出表格或曲线。

曲线是存储的 `loss_history`，按文件顺序排列，最多为训练器保留的样本数量。`final_loss` 是一列。它不会附加到行中。空样本是一个间隙，而不是零。

“训练”、“评估”和“导出模型”按钮将保留在此屏幕上。它们不会显示在系列屏幕上。

## 威胁模型

RunForge 读取您选择的文件夹。历史记录文件夹会在那里打开 `run_history.json`，或者在下一级打开 `output/run_history.json`。系列文件夹会在该文件夹及其直接子文件夹中打开 `run-config*.json`。该应用程序不会搜索磁盘的其余部分，也不会合并这两种记录类型。“导出”和“保存报告”会将内容写入您选择的路径。首选项（包括上次使用的文件夹和主题）将在应用程序打包时写入 LocalState 包中，如果未打包，则写入可执行文件旁边。

“训练”、“评估”和“导出模型”按钮只有在您点击历史记录面板上的按钮并且该程序已经位于 PATH 环境变量中时才会启动 `backprop`。日志是该程序的输出。停止会结束此窗口启动的进程树。侧边栏不会点击这些按钮。

侧边栏可以连接到环回地址上的模型服务器。软件包清单不请求 `internetClient`。没有遥测数据，也没有帐户。报告的参考列表位于程序内部。它不会被提取。

它不会触及的数据：训练器、模型下载、反向传播的安装、shell、环境的副本、云模型或写回到系列文件或 `run_history.json`。

权限将保留在您打开的文件夹、您选择的导出路径、您选择的数据文件、首选项文件以及您请求本地模型时使用的环回端口 `11434` 上。

如何报告漏洞，请参阅 [SECURITY.md](SECURITY.md)。

## 构建

Rust 1.98.1，2024 版本。工具链文件会固定该版本。

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

CI 运行代码覆盖率命令并上传 `lcov.info`。当行覆盖率低于 90% 时，Codecov 会使状态显示失败。文件对话框不会被测试打开。

## 商店

发布的列表是产品 `9PHL1HX0CGMF`，包 `mcp-tool-shop.RunForge-Desktop`。版本 2 替换了早期的分类器应用程序，并且列表文本必须在同一提交中说明这一点。此仓库尚未包含该包。构建包时，包标识不会更改。

记录的设计在 [docs/CONTRACT.md](docs/CONTRACT.md) 中。此仓库支持 2.0.0 源代码构建。发布的商店应用程序将保持 1.0.1 分类器，直到提交高于 `1.0.1.0` 的包。

由 [MCP Tool Shop](https://mcp-tool-shop.github.io/) 构建。
