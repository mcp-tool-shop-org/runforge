<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.md">English</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
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

RunForge 是一个用于查看 [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate) 输出文件夹的 Windows 应用程序。打开包含 `run_history.json` 的文件夹，或者包含 `output` 目录的上一级文件夹。该窗口会列出运行结果、绘制存储的损失值、比较两行数据，并导出表格或曲线。

Backpropagate 是训练器。此应用程序不包含训练器，不下载模型，也不包含 PyTorch。如果 `backprop` 已经位于 PATH 环境变量中，点击“训练”、“评估”和“导出模型”按钮将启动该命令并跟踪其日志。参数由应用程序构建。没有任何内容会通过 shell 传递。如果缺少 `backprop`，按钮会显示相应的提示。RunForge 不会下载、安装或包含 backpropagate。

曲线是存储的 `loss_history`，按文件顺序排列，最多为训练器保留的样本数量。`final_loss` 是一列。它不会附加到行中。空样本是一个间隙，而不是零。

## 威胁模型

RunForge 读取您选择的文件夹。它会打开该文件夹中的 `run_history.json`，或者打开下一级文件夹中的 `output/run_history.json`，并且可以导出表格、曲线或单个条目。首选项（包括上次使用的文件夹和主题）在应用程序打包时写入到包的 LocalState 目录中，而在未打包时则写入到可执行文件所在的目录中。点击“训练”、“评估”和“导出模型”按钮时，只有当您按下按钮并且该程序已经位于 PATH 环境变量中时，才会启动 `backprop`。日志是该程序的输出。点击“停止”按钮会结束此窗口启动的进程树。

它不会访问以下数据：训练器、模型下载、backpropagate 的安装、shell、环境的副本或遥测数据。没有账户。

权限仅保留在您打开的文件夹、您选择的导出路径、您选择的数据文件以及首选项文件中。该应用程序不会请求任何网络权限。

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
