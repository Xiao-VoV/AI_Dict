export type UiLanguage = "zh-CN" | "en-US";

export type AppSettings = {
  baseUrl: string;
  apiKey: string;
  model: string;
  temperature: number;
  targetLanguage: string;
  uiLanguage: UiLanguage;
};

export type WordProfile = {
  source: string;
  lemma: string;
  translated: string;
  phonetics: {
    uk?: string | null;
    us?: string | null;
    audio?: string | null;
  };
  definitions: Array<{ partOfSpeech: string; meaning: string; source: string }>;
  forms: {
    plural?: string | null;
    thirdPerson?: string | null;
    past?: string | null;
    pastParticiple?: string | null;
    presentParticiple?: string | null;
    comparative?: string | null;
    superlative?: string | null;
  };
  examples: Array<{ en: string; zh?: string | null; source: string }>;
  phrases: Array<{ phrase: string; meaning: string }>;
  synonyms: string[];
  antonyms: string[];
  memoryHint?: string | null;
  examTags: string[];
  importedCards: Array<{
    dictionaryId: number;
    dictionaryName: string;
    headword: string;
    plainText: string;
  }>;
  sources: string[];
};

export type DictionaryMetadata = {
  id: number;
  name: string;
  kind: string;
  path?: string | null;
  entryCount: number;
  createdAt: number;
};

export type ImportSummary = {
  dictionaryId?: number | null;
  name: string;
  kind: string;
  importedEntries: number;
  skippedEntries: number;
};

export type LookupResult =
  | { kind: "word"; profile: WordProfile }
  | { kind: "translation"; source: string; translated: string };
