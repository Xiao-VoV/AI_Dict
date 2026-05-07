# 划词翻译项目进度

## 2026-05-07
- 创建文件化规划系统：`task_plan.md`、`findings.md`、`progress.md`。
- 完成方案可行性判断：Tauri 可作为跨平台桌面框架，关键风险集中在取选中文本、权限和 Linux Wayland 兼容性。
- 明确 MVP 范围：托盘、全局快捷键、剪贴板取词、单词本地词典、句子 AI 翻译、悬浮窗、基础配置。
- 明确后续增强：原生辅助功能 API、OCR、浏览器扩展、翻译记忆、离线模型。
- 搭建 Tauri v2 + React + TypeScript + Tailwind CSS 工程。
- 实现 Rust 后端 MVP：全局快捷键、剪贴板取词、本地示例词典、OpenAI-compatible 翻译、配置读写。
- 实现前端 MVP：手动查询、读取当前选中、模型配置、结果展示。
- 执行验证：`npm run build` 通过，`cargo check` 通过。
- 启动 Vite 预览服务：http://127.0.0.1:1420/。
- 执行 `npm audit --audit-level=high`：无 high/critical；存在 Vite/esbuild 开发服务器相关 moderate 提示，修复需要升级到 breaking 版本，暂未自动 force。
- 修复桌面应用启动 panic：替换无效占位 `src-tauri/icons/icon.png`，解决 Tauri `invalid icon` 错误。
- 重新验证：`cargo check` 通过，`npm run tauri:dev` 不再出现 icon panic，直接运行 `target/debug/selection-translator` 可进入运行状态。
- 调整桌面端 UI：禁止窗口级上下左右滚动，首页只保留查询/取词/结果，模型配置拆到独立设置页，通过首页齿轮按钮进入。
- 重新验证：`npm run build` 通过，`cargo check` 通过。
- 替换 AI 调用实现：从手写 `reqwest` 请求改为 `async-openai` 的 chat-completion 客户端，支持自定义 OpenAI-compatible Base URL，并自动裁剪误填的 `/chat/completions` 后缀。
- 增加 AI 设置校验和 Base URL 规范化单元测试；验证 `cargo test` 与 `npm run build` 通过。
- 修复 DeepSeek-compatible 非标准响应兼容问题：开启 `async-openai` 的 `byot` feature，改用宽松响应解析以接受 `message.role: ""`，新增回归测试覆盖该响应。
- 重新验证：`cargo test` 通过，`npm run build` 通过。
- 接入日志体系：新增 `tauri-plugin-log` 与 `log`，日志输出到 stdout 和系统日志目录文件，覆盖启动、快捷键、取词、设置读写、词典/AI 路由、AI 请求与错误等关键路径。
- 重新验证：`cargo check`、`cargo test`、`npm run build` 通过。
- 修复前端结果展示：后端 `LookupResult` 使用 `serde(rename_all = "camelCase")`，实际 `kind` 为 `dictionary`、`translation`、`dictionaryMiss`；前端此前判断大写枚举名导致翻译结果不渲染。已统一为 camelCase。
- 重新验证：`npm run build`、`cargo test` 通过。
- 设置页增强：新增响应式/容器自适应布局，小窗口下自动单列并在面板内部滚动，避免窗口级滚动。
- 模型设置新增 API 联通检测：后端新增 `test_ai_connection` 命令，复用当前表单配置发起极短 AI 请求，前端显示检测中/成功/失败状态。
- UI 新增轻量国际化：内置 `zh-CN` 与 `en-US` 文案字典，设置中可切换界面语言，并将 `uiLanguage` 保存到设置文件；旧设置通过 serde default 兼容。
- 重新验证：`npm run build`、`cargo test` 通过。
- 前端模块化重构：将 `App.tsx` 收缩为应用状态/命令编排，新增 `types.ts`、`settingsDefaults.ts`、`i18n/messages.ts`，并拆出 `HomePage`、`SettingsPage`、`ResultPanel`、`Field` 组件。
- 设置页调整：界面语言从模型配置卡片中移出，改为独立的界面设置卡片；设置页继续保持窗口级不滚动，卡片栈在内部滚动。
- 重新验证：`npm run build` 通过。
- 设置页横向布局调整：移除设置内容 `44rem` 最大宽度限制，设置卡片左右随窗口宽度铺开，仅保留页面外边距。
- 重新验证：`npm run build` 通过。

## 待办
- 运行 `npm run tauri:dev` 做真实桌面取词体验测试。
- 调研并选择可分发词典数据源，替换当前示例词典。
- 增加悬浮窗定位、固定窗口、托盘菜单、历史记录和收藏。
- 将 API Key 存储迁移到系统安全凭据服务。
