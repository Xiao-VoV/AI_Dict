import type { AppSettings } from "./types";

export const defaultSettings: AppSettings = {
  baseUrl: "https://api.openai.com/v1",
  apiKey: "",
  model: "gpt-4o-mini",
  temperature: 0.2,
  targetLanguage: "简体中文",
  uiLanguage: "zh-CN",
};
