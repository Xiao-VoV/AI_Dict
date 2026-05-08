use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordProfile {
    pub source: String,
    pub lemma: String,
    pub translated: String,
    pub phonetics: Phonetics,
    pub definitions: Vec<Definition>,
    pub forms: WordForms,
    pub examples: Vec<Example>,
    pub phrases: Vec<Phrase>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
    pub memory_hint: Option<String>,
    pub exam_tags: Vec<String>,
    pub imported_cards: Vec<ImportedCard>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phonetics {
    pub uk: Option<String>,
    pub us: Option<String>,
    pub audio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub part_of_speech: String,
    pub meaning: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub en: String,
    pub zh: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordForms {
    pub plural: Option<String>,
    pub third_person: Option<String>,
    pub past: Option<String>,
    pub past_participle: Option<String>,
    pub present_participle: Option<String>,
    pub comparative: Option<String>,
    pub superlative: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phrase {
    pub phrase: String,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCard {
    pub dictionary_id: i64,
    pub dictionary_name: String,
    pub headword: String,
    pub plain_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryMetadata {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub path: Option<String>,
    pub entry_count: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub dictionary_id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub imported_entries: usize,
    pub skipped_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryIndexProgress {
    pub name: String,
    pub kind: String,
    pub phase: String,
    pub processed_entries: usize,
    pub total_entries: usize,
    pub imported_entries: usize,
    pub skipped_entries: usize,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWordCard {
    pub short_definition: Option<String>,
    #[serde(default)]
    pub memory_hint: Option<String>,
    #[serde(default)]
    pub phrases: Vec<Phrase>,
    #[serde(default)]
    pub examples: Vec<Example>,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub antonyms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WordLookupSeed {
    pub source: String,
    pub translated: String,
    pub lemma: String,
}
