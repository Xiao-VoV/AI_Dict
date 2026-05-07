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
  | {
      kind: "word";
      source: string;
      translated: string;
      lemma: string;
      entry: DictionaryEntry | null;
    }
  | { kind: "translation"; source: string; translated: string };
