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

## 待办
- 运行 `npm run tauri:dev` 做真实桌面取词体验测试。
- 调研并选择可分发词典数据源，替换当前示例词典。
- 增加悬浮窗定位、固定窗口、托盘菜单、历史记录和收藏。
- 将 API Key 存储迁移到系统安全凭据服务。
