export type UiLanguage = "zh-CN" | "en-US";

export type AppSettings = {
  baseUrl: string;
  apiKey: string;
  model: string;
  temperature: number;
  targetLanguage: string;
  uiLanguage: UiLanguage;
};

export type DictionaryEntry = {
  word: string;
  phonetic: string;
  definitions: Array<{ partOfSpeech: string; meaning: string }>;
  examples: string[];
};

export type LookupResult =
  | { kind: "dictionary"; source: string; entry: DictionaryEntry }
  | { kind: "translation"; source: string; translated: string }
  | { kind: "dictionaryMiss"; source: string; message: string };
