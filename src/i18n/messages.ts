import type { UiLanguage } from "../types";

export const messages = {
  "zh-CN": {
    appName: "划词翻译",
    homeSubtitle: "快捷键：Cmd/Ctrl + Shift + E。单词查词典，句子走 AI 翻译。",
    openSettings: "打开设置",
    readSelection: "读取当前选中",
    manualTest: "手动测试",
    inputPlaceholder: "输入 hello 测试本地词典，或输入一句话测试 AI 翻译",
    lookup: "查询 / 翻译",
    result: "结果",
    emptyResult: "选中文本后按快捷键，或在上方输入内容测试。",
    source: "原文",
    translation: "译文",
    settings: "设置",
    settingsSubtitle: "模型、目标语言、界面语言和翻译参数",
    back: "返回",
    save: "保存",
    saved: "已保存",
    modelConfig: "模型配置",
    interfaceConfig: "界面设置",
    targetLanguage: "目标语言",
    uiLanguage: "界面语言",
    simplifiedChinese: "简体中文",
    english: "English",
    testConnection: "检测",
    testing: "检测中",
    testSuccess: "API 联通正常",
    testIdle: "保存前也可以直接检测当前表单配置。",
  },
  "en-US": {
    appName: "Selection Translator",
    homeSubtitle:
      "Shortcut: Cmd/Ctrl + Shift + E. Words use the dictionary; sentences use AI translation.",
    openSettings: "Open settings",
    readSelection: "Read selection",
    manualTest: "Manual test",
    inputPlaceholder: "Type hello for dictionary lookup, or enter a sentence for AI translation",
    lookup: "Lookup / Translate",
    result: "Result",
    emptyResult: "Select text and press the shortcut, or test with the input above.",
    source: "Source",
    translation: "Translation",
    settings: "Settings",
    settingsSubtitle: "Model, target language, UI language, and translation parameters",
    back: "Back",
    save: "Save",
    saved: "Saved",
    modelConfig: "Model configuration",
    interfaceConfig: "Interface",
    targetLanguage: "Target language",
    uiLanguage: "UI language",
    simplifiedChinese: "Simplified Chinese",
    english: "English",
    testConnection: "Test",
    testing: "Testing",
    testSuccess: "API connection works",
    testIdle: "You can test the current form settings before saving.",
  },
} satisfies Record<UiLanguage, Record<string, string>>;

export type Messages = (typeof messages)[UiLanguage];

export function normalizeUiLanguage(language: string | undefined): UiLanguage {
  return language === "en-US" ? "en-US" : "zh-CN";
}
