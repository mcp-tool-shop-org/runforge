<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.md">English</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
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

RunForge 是一款 Windows 应用程序，用于微调实验结果。打开一个包含实验结果的文件夹，它会绘制出所有存储的样本，生成一份报告，报告以其结论为重点，并为本地模型提供一个工作台。在工作台上，模型会构建自己的公式工具，提出每个参数的作用，并在多个文件夹中收集证据，直到得出最终结论。

RunForge 读取训练记录。它不进行训练。它不会下载模型、安装 PyTorch 或调用云模型。

## 报告

打开一个包含 `run-config*.json` 文件的文件夹。这些文件可以位于文件夹本身或其下一级目录中。RunForge 会绘制出每个有限样本，不进行重采样，并且间隙仍然是间隙。`training_summary.final_loss` 是曲线旁边的标记，而不是曲线上的点，并且报告不会根据它进行排名。

报告以“简而言之”开始：说明哪个实验结果胜出，以及原因。对于一组包含五个种子的实验结果，报告可能如下所示：

> 没有实验结果胜出。种子 512 具有最低的单一点，在第 3 个 epoch 时为 0.0674。种子 1024 在其最低点附近具有最平缓的曲线：中间值为 0.3474，而种子 512 的最低点附近为 0.4041。这些实验结果的中间值彼此相差不超过 0.1508：比最平缓的实验结果的中间一半（0.1133）更宽，但比最嘈杂的实验结果（0.3238）更窄。这些实验结果仅部分分离。

之后，报告会列出：
- 实验结果
- 哪个实验结果胜出，或者没有实验结果胜出
- 哪些参数发生了变化，哪些参数没有变化
- 之前对相同实验结果的评估
- 下一步应该做什么
- 报告无法提供哪些信息
- 每个公式的来源

如果某个部分没有内容需要说明，则不会打印该部分。如果某个设置在所有实验结果中都相同，则会将其列为未测试，而不是一个参数。面板和“保存报告”按钮显示相同的文字。

## 工作台

按下“询问”按钮，本地模型将通过五个工具来分析实验结果：衡量一个公式、比较一个变化的参数、保留一个新的工具、提出一个假设，然后完成。程序会计算出每个结果，并记录每个包含数字的句子。模型会选择要查看的内容，并将其转化为文字。其最后的注释将被标记为模型的观点，而不是一个测量结果。

**模型构建的工具。** 一个工具是在一个小型语言中定义的公式，每个实验结果都会对其进行一次评估，例如：
- `last / low`：曲线在其最低点之后上升了多少
- `slope_between(end_epoch - 1, end_epoch)`：最后一个 epoch 的陡峭程度如何
- `knob('lora_r')`：一个实验结果的值

一个公式不能读取文件、打开网络或运行代码。只有当它在每个实验结果中都提供一个值，并且不是重复项时，才会保留一个新的工具。它将保持临时状态，直到在第二个文件夹中使用。后续的公式可以通过名称使用它，因此库会随着数据的增长而扩展。你可以在面板的公式框中自行尝试一个公式。

**关于参数的假设。** 一个假设会命名一个参数、一个公式和一个方向，例如“当 LoRA 秩增加时，`last / low` 会降低”。当它被提出时，其测试是固定的。对于每个文件夹，程序会将其标记为以下之一：
- 无法测试：该参数没有变化
- 混淆：另一个参数与其同时变化
- 结论不明确
- 或者仅针对这些实验结果的结论

当实验结果无法确定一个假设时，RunForge 会规划一组最小的实验结果，以满足以下条件：一个参数、两个设置，每个设置至少有三个种子。它不会启动这些实验。

**跨文件夹的证据。** 每个文件夹都会提供一个 e 值：这是一个证据指标，当参数不起作用时，其平均值为 1，因此可以在多个文件夹中相乘，而不会损失有效性。以下两种类型的文件夹将被排除在外：
- 任何包含 RunForge 在假设注册时已经看到的实验结果的文件夹
- 已经计数的实验结果

只有在检查点时，才会发布支持或反驳的结论，每个五个新的文件夹会进行一次检查。每个检查点都会应用 e-BH 方法，以 5% 的假发现率来评估工作台上每个假设的两个方向。一个假设大约需要三个干净的文件夹，每个文件夹包含三个实验结果。该方法、其来源以及外部审查可在 [docs/sidecar-workbench.md](docs/sidecar-workbench.md) 和 [docs/evidence.consult.response.md](docs/evidence.consult.response.md) 中找到。

模型是一个本地的 Ollama，位于 `127.0.0.1:11434`。RunForge 仅使用 Ollama 报告可以调用工具的模型，并跳过带有“云”标签的名称。一个会话最多可以进行六次请求和十次工具调用。如果没有本地模型，报告仍然有效，公式框也一样。

## 历史记录工作台

一个包含反向传播 `run_history.json` 的文件夹会打开历史记录工作台，或者位于 `output` 目录上方的文件夹。工作台会列出实验结果，按文件顺序绘制存储的 `loss_history`，比较两行，并导出表格或曲线。

如果 `backprop` 已经在 PATH 中，则“训练”、“评估”和“导出模型”按钮会启动该命令并跟踪其日志。应用程序会构建参数，并且没有任何内容会通过 shell 传递。如果缺少 `backprop`，按钮会显示相应的提示。RunForge 不会下载、安装或提供反向传播。

<p align="center"><img src="docs/bench-dark.png" alt="The history bench in the dark theme, open on a fixture folder" width="720"></p>

## 威胁模型

**它读取的内容。** 你选择的文件夹：
- 历史记录文件夹：`run_history.json` 在其中，或者 `output/run_history.json` 在其下一级目录中
- 系列文件夹：`run-config*.json` 在其中，以及在其直接子目录中

它不会遍历磁盘的其余部分，并且它绝不会将内容写回这些文件。导出和保存报告会将内容写入你选择的路径。

**它保留的内容。** 首选项（最后一个文件夹和主题）以及 `sidecar-memory.json` 位于一起：当应用程序打包时，位于包的 LocalState 中；当应用程序未打包时，位于可执行文件旁边。内存文件不包含任何文件夹路径。它包含：
- 模型的注释
- 每个测量的权重
- 学习到的公式工具（最多 50 个）
- 带有其测试结果的假设（最多 60 个）
- 检查点

**传递给本地模型的内容。** 运行名称、配方值以及程序自身工具的结果会发送到环回端口 `11434`。文件夹路径不会发送。模型只能调用五个工作台工具，并且程序会验证每次调用。

**启动时会发生什么。** 只有在您按下历史记录工作台上的按钮并且程序已添加到 PATH 环境变量中时，才会启动训练、评估和导出模型 `backprop`。停止会结束此窗口启动的进程树。辅助进程绝不会按下这些按钮。

**程序不会触及的内容。** 程序包清单不会请求 `internetClient`。没有遥测数据，也没有帐户，并且报告的参考列表已内置到程序中。该应用程序没有云模型、没有 shell、没有环境副本、没有训练器，也没有模型下载。

如何报告漏洞，请参见 [SECURITY.md](SECURITY.md)。

## 构建

Rust 1.98.1，2024 版本。工具链文件会固定此版本。

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

代码覆盖率要求：
- CI 在代码覆盖率低于 90% 时会失败。
- Codecov 会将项目和每个拉取请求的新代码行都限制在 90%。

测试会在环回接口上针对一个模拟的 Ollama 运行模型循环。针对真实本地模型的会话是可选的：

```bash
RUNFORGE_LIVE_FOLDER=<series folder> cargo test -p runforge --features live live_workbench -- --nocapture
```

## 商店

发布的列表是产品 `9PHL1HX0CGMF`，程序包 `mcp-tool-shop.RunForge-Desktop`。版本 2 替换了早期的分类器应用程序，并且列表文本必须在同一提交中说明这一点。发布的商店应用程序将保持为 1.0.1 分类器，直到提交高于 `1.0.1.0` 的程序包。设计规范位于 [docs/CONTRACT.md](docs/CONTRACT.md)。

由 [MCP Tool Shop](https://mcp-tool-shop.github.io/) 构建。
