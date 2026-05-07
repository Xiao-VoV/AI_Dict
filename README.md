# 划词翻译

基于 Tauri v2 + React + TypeScript + Tailwind CSS 的跨平台划词翻译 MVP。

## 当前 MVP 能力

- `Cmd/Ctrl + Shift + E` 全局快捷键触发读取当前选中文本。
- 通过剪贴板兜底策略获取选区文本，并尝试恢复原剪贴板。
- 单词走本地词典查询。
- 句子/段落走 OpenAI-compatible `/chat/completions` 翻译。
- 前端提供手动测试、模型配置、目标语言、temperature 设置和结果展示。

## 开发启动

```bash
npm install
npm run tauri:dev
```

## 验证命令

```bash
npm run build
cd src-tauri && cargo check
```

## 平台说明

- macOS：快捷键取词依赖 `osascript` 模拟复制，首次使用可能需要授予辅助功能权限。
- Windows：使用 PowerShell `SendKeys` 模拟 `Ctrl+C`。
- Linux：优先使用 `xdotool`，失败后尝试 `wtype`；Wayland 环境可能需要额外配置。

## 后续建议

- 将示例词典替换为可分发的 SQLite 词库。
- 增加悬浮窗定位和固定窗口。
- 使用系统 Keychain/Credential Manager/Secret Service 保存 API Key。
- 增加历史记录、收藏和词典导入。

