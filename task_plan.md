# 划词翻译软件项目计划

## 目标
开发一款基于 Tauri 的跨平台划词翻译软件，支持 macOS、Linux、Windows。用户选中文本后可触发翻译：单词走本地词典并展示详细释义，句子/段落走用户自配 AI 大模型接口翻译。

## 当前结论
方案可行，但不是纯前端 Tauri 应用。核心能力需要 Rust 后端结合平台原生 API、权限引导、全局快捷键、剪贴板兜底策略和悬浮窗口管理。

## 阶段

| 阶段 | 状态 | 目标 | 产出 |
|---|---|---|---|
| 1. 可行性评估 | complete | 判断 Tauri、跨平台取词、词典和 AI 翻译是否可落地 | findings.md 中记录关键判断 |
| 2. 产品范围定义 | complete | 明确 MVP 与后续版本边界 | 本计划中的 MVP/非 MVP |
| 3. 技术架构规划 | complete | 拆分前端、Rust 后端、平台适配、词典、模型接入 | 架构方案 |
| 4. 开发路线图 | complete | 安排里程碑与验证顺序 | 分阶段开发计划 |
| 5. 风险与验证清单 | complete | 列出高风险点和 PoC | 风险表与 PoC 建议 |
| 6. MVP 工程实现 | complete | 搭建 Tauri + React + Tailwind + Rust MVP | 可编译的桌面应用骨架 |
| 7. 词典运行时重构 | complete | 移除运行时 CSV 解析，改为内置 ECDICT SQLite + 用户 MDX 索引 | 内置 SQLite 查询、用户 MDX 管理 |
| 8. 词典模块拆分 | complete | 拆分过长的 `dictionary.rs`，按职责维护词典逻辑 | `dictionary/` 子模块与瘦门面 |

## MVP 范围
- 桌面端支持 macOS、Windows、主流 Linux 桌面环境。
- 托盘常驻应用。
- 全局快捷键触发翻译，例如 `Cmd/Ctrl + Shift + E`。
- 优先通过模拟复制/读取剪贴板获取选中文本，保留原剪贴板内容。
- 单词识别后查本地词典，展示音标、词性、中文释义、例句、变形。
- 句子或多词文本调用用户配置的 AI 模型接口翻译。
- 支持 OpenAI-compatible API 配置：base URL、API key、model、temperature。
- 悬浮翻译窗口显示在鼠标或选区附近，可复制结果、固定窗口。
- 基础历史记录与收藏。

## 非 MVP / 后续版本
- OCR 屏幕取词。
- 浏览器扩展联动。
- 移动端。
- 完整术语库、翻译记忆、团队同步。
- 多引擎词典聚合与离线大模型。
- 深度原生辅助功能 API 取选区坐标与文本。

## 推荐技术栈
- 桌面框架：Tauri v2。
- 后端：Rust。
- 前端：React/Vue/Svelte 均可，建议 React + TypeScript 或 Vue + TypeScript。
- UI：轻量组件库 + 自定义紧凑浮窗。
- 存储：SQLite 或 sled；MVP 建议 SQLite。
- 配置密钥：系统 Keychain/Credential Manager/Secret Service，短期可用 Tauri store + 加密兜底。
- 词典格式：内置 ECDICT 使用资源目录中的预构建 SQLite；用户导入只支持 MDX；ECDICT CSV/脚本仅作为上游源码或构建材料。
- AI 接入：OpenAI-compatible client 抽象，预留 provider adapter。

## 验证优先级
1. macOS/Windows/Linux 上通过全局快捷键获取当前选中文本的 PoC。
2. 悬浮窗口定位、置顶、失焦关闭、托盘常驻体验。
3. 词典查询性能与数据授权。
4. AI API 配置、流式返回、错误重试与超时。
5. 打包签名、自动更新、权限提示。

## 已知高风险
- macOS 辅助功能权限、输入监控权限可能影响模拟快捷键和取词。
- Linux Wayland 对全局快捷键、焦点窗口、模拟按键、剪贴板访问限制更强。
- Windows 部分应用内选区无法稳定通过复制获得。
- 词典版权风险，需要选择可商用/可分发的数据源。
- API key 本地存储和日志脱敏必须认真处理。

## 当前实现状态
- 已创建 Tauri v2 + React + TypeScript + Tailwind CSS 工程。
- 已实现 Rust 命令：`capture_selection_and_lookup`、`lookup_text`、`get_settings`、`save_settings`。
- 已实现全局快捷键 `CommandOrControl+Shift+E`。
- 已实现剪贴板取词兜底：保存剪贴板、模拟复制、读取文本、恢复剪贴板。
- 已实现本地示例词典查询和 OpenAI-compatible 翻译调用。
- 已通过 `npm run build` 和 `cargo check`。
- 已将词典运行时重构为内置 ECDICT SQLite + 用户 MDX：内置词典直接只读查询资源目录中的 `ecdict.db`，用户 MDX 导入后写入应用 SQLite，CSV 导入已移除。
- 已为用户 MDX 导入增加后台进度事件，并在设置页显示索引进度条。
- 已修复用户 MDX 索引初期无反馈问题：打开 MDX 文件阶段显示不确定进度，进入写入后显示条目进度；索引循环仅处理当前单词查询需要的英文词头。
- 已增加用户 MDX 索引单任务保护：后端保存当前索引进度并拒绝重复导入，设置页重新进入时先恢复运行中任务状态，避免重复触发索引和 SQLite `database is locked`。
- 已将过长的 `src-tauri/src/dictionary.rs` 拆为 `dictionary/` 子模块：公开模型、错误、store/schema、内置 ECDICT SQLite、用户 MDX 导入、进度事件、文本清洗和 WordProfile 构建分离维护。
