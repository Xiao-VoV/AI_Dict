# 划词翻译项目发现记录

## 可行性判断
- 使用 Tauri v2 开发跨平台桌面划词翻译软件可行。
- Tauri 自身适合负责窗口、托盘、打包、插件体系和 Rust/前端通信；划词翻译的关键能力需要 Rust 侧封装平台 API。
- “选中单词查词典，选中句子走 AI 翻译”的产品逻辑清晰，适合做成本地判别路由。

## Tauri 相关事实
- Tauri v2 提供 global-shortcut 插件，可注册全局快捷键，文档标注支持 Windows、Linux、macOS，并要求 Rust 至少 1.77.2。
- Tauri v2 支持系统托盘能力，可用于常驻应用入口。
- Tauri v2 有 updater 插件体系，可支持后续自动更新分发。
- Tauri v2 的能力权限配置需要显式开放插件命令，安全模型比 v1 更严格。

## 取词策略
- 最现实的 MVP 策略是“用户选中文本 -> 按全局快捷键 -> 应用保存原剪贴板 -> 模拟复制 -> 读取剪贴板 -> 恢复剪贴板”。
- 更理想的原生策略是按平台辅助功能/可访问性 API 读取当前焦点应用选区，但实现复杂且覆盖率不稳定，适合作为后续增强。
- Linux 需要区分 X11 与 Wayland。Wayland 环境下全局快捷键、模拟按键和剪贴板访问通常限制更多，应设计清晰的兼容性提示。

## 词典策略
- 本地词典必须重视版权。建议优先使用可分发的开源词典，或让用户自行导入词典。
- 数据层建议转换为 SQLite 表，并为英文单词建立规范化索引，例如小写、lemma、变形映射。
- 单词判别不宜只靠空格数量，应结合语言检测、标点、token 数量和长度。

## AI 翻译策略
- 支持 OpenAI-compatible API 可以覆盖 OpenAI、DeepSeek、Moonshot、通义千问兼容接口、Ollama/LM Studio 等。
- 应提供流式响应、超时、取消、重试、模型测试按钮和错误可读提示。
- Prompt 需要区分翻译、润色、解释、术语约束；MVP 可以先做翻译与双语展示。

## 产品体验判断
- 入口应以托盘 + 全局快捷键为主，而不是常驻主窗口。
- 悬浮窗要小、快、可复制、可固定，并避免抢焦点。
- 首次启动应做权限引导，特别是 macOS 辅助功能权限、Linux Wayland 限制、Windows 安全软件误报说明。

## 实现发现
- Tauri v2.11 编译时即使配置中未显式指定图标，也会查找 `src-tauri/icons/icon.png`，当前已放入占位图标用于开发编译。
- 之前的占位 `src-tauri/icons/icon.png` 只有 PNG 头信息可被 `file`/`sips` 识别，但实际 RGBA 像素数据无效；Tauri 启动时会解码图标像素，因此触发 `invalid icon` panic。已替换为包含完整 32x32 RGBA 像素数据的 PNG。
- `tauri::path::Error` 在当前版本中不可直接作为公开错误类型使用，配置路径错误应使用 `tauri::Error` 承接。
- `tauri-plugin-global-shortcut` 的错误类型不能直接用 `?` 转成 `tauri::Error`，当前注册失败时记录错误但不阻塞应用启动。
- `npm install` 在无输出时仍可能正常下载包；本次首次安装需要等待较久。
- `async-openai` 0.37 默认只启用 TLS，不会启用 API 分组；使用 chat completions 需要显式开启 `chat-completion` feature。
- OpenAI-compatible Base URL 应配置为 API base，例如 `https://api.openai.com/v1`；如果用户误填到 `/chat/completions`，当前实现会自动裁剪该后缀。
- 某些 OpenAI-compatible 服务会返回非标准响应，例如 `message.role` 为空字符串；`async-openai` 的强类型 `CreateChatCompletionResponse` 会因此反序列化失败。当前使用 `byot` feature 让库负责请求与鉴权，但用宽松响应结构只读取 `choices[].message.content`。
- 使用 `tauri-plugin-log` 写 stdout 和系统日志目录，debug 构建默认 Debug 级别，release 构建默认 Info 级别。日志应避免记录 API Key 和完整原文，只记录长度、路由、模型、Base URL 等必要排障信息。

## 参考来源
- Tauri global-shortcut 插件文档：https://v2.tauri.app/zh-cn/plugin/global-shortcut/
- Tauri 系统托盘文档：https://v2.tauri.org.cn/learn/system-tray/
- Tauri updater 插件文档：https://v2.tauri.app/ko/plugin/updater/
