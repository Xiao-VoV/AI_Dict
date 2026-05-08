use super::{
    constants::{BUILTIN_ECDICT_NAME, MAX_CARD_TEXT_CHARS},
    models::{AiWordCard, Definition, ImportedCard, Phonetics, WordForms, WordProfile},
    util::trim_to_chars,
};
use std::collections::BTreeSet;

pub(super) fn build_profile(
    source: String,
    translated: String,
    lemma: String,
    ai_card: Option<AiWordCard>,
    imported_cards: Vec<ImportedCard>,
) -> WordProfile {
    let mut definitions = Vec::new();
    let mut examples = Vec::new();
    let mut phrases = Vec::new();
    let mut synonyms = Vec::new();
    let mut antonyms = Vec::new();
    let mut sources = BTreeSet::new();
    let mut memory_hint = None;
    let mut phonetics = Phonetics::default();
    let mut forms = WordForms::default();
    let mut exam_tags = Vec::new();

    for card in &imported_cards {
        sources.insert(card.dictionary_name.clone());
    }

    if let Some(first_card) = imported_cards.first() {
        if first_card.dictionary_name == BUILTIN_ECDICT_NAME {
            phonetics = parse_ecdict_phonetics(&first_card.plain_text);
            forms = parse_ecdict_forms(&first_card.plain_text);
            exam_tags = parse_ecdict_tags(&first_card.plain_text);
        }
        definitions.extend(parse_plain_text_definitions(
            &first_card.plain_text,
            &first_card.dictionary_name,
        ));
    }

    if let Some(card) = ai_card {
        sources.insert("AI".to_string());
        if let Some(short_definition) = card
            .short_definition
            .filter(|value| !value.trim().is_empty())
        {
            definitions.insert(
                0,
                Definition {
                    part_of_speech: "AI".to_string(),
                    meaning: short_definition,
                    source: "AI".to_string(),
                },
            );
        }
        examples.extend(card.examples);
        phrases.extend(card.phrases);
        synonyms.extend(card.synonyms);
        antonyms.extend(card.antonyms);
        memory_hint = card.memory_hint;
    }

    WordProfile {
        source,
        lemma,
        translated,
        phonetics,
        definitions,
        forms,
        examples,
        phrases,
        synonyms,
        antonyms,
        memory_hint,
        exam_tags,
        imported_cards,
        sources: sources.into_iter().collect(),
    }
}

pub(super) fn parse_plain_text_definitions(text: &str, source: &str) -> Vec<Definition> {
    text.lines()
        .flat_map(|line| line.split("；"))
        .filter_map(|chunk| {
            let trimmed = chunk.trim();
            if trimmed.is_empty() || is_ecdict_metadata_line(trimmed) {
                return None;
            }
            let (part_of_speech, meaning) = split_definition_line(trimmed);
            if meaning.is_empty() {
                return None;
            }
            Some(Definition {
                part_of_speech,
                meaning: trim_to_chars(&meaning, 180),
                source: source.to_string(),
            })
        })
        .take(6)
        .collect()
}

fn is_ecdict_metadata_line(line: &str) -> bool {
    line.strip_prefix("phonetic:")
        .or_else(|| line.strip_prefix("exchange:"))
        .or_else(|| line.strip_prefix("tag:"))
        .or_else(|| line.strip_prefix("pos:"))
        .is_some()
}

pub(super) fn parse_ecdict_phonetics(text: &str) -> Phonetics {
    let phonetic = metadata_line_value(text, "phonetic").map(ToString::to_string);
    Phonetics {
        uk: phonetic.clone(),
        us: phonetic,
        audio: None,
    }
}

pub(super) fn parse_ecdict_forms(text: &str) -> WordForms {
    let mut forms = WordForms::default();
    let Some(exchange) = metadata_line_value(text, "exchange") else {
        return forms;
    };
    for item in exchange.split('/') {
        let Some((kind, value)) = item.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match kind {
            "p" => forms.past = Some(value.to_string()),
            "d" => forms.past_participle = Some(value.to_string()),
            "i" => forms.present_participle = Some(value.to_string()),
            "3" => forms.third_person = Some(value.to_string()),
            "s" => forms.plural = Some(value.to_string()),
            "r" => forms.comparative = Some(value.to_string()),
            "t" => forms.superlative = Some(value.to_string()),
            _ => {}
        }
    }
    forms
}

pub(super) fn parse_ecdict_tags(text: &str) -> Vec<String> {
    metadata_line_value(text, "tag")
        .map(|tags| {
            tags.split_whitespace()
                .filter(|tag| !tag.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn metadata_line_value<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    let prefix = format!("{label}:");
    text.lines()
        .find_map(|line| line.trim().strip_prefix(&prefix).map(str::trim))
        .filter(|value| !value.is_empty())
}

fn split_definition_line(line: &str) -> (String, String) {
    for marker in [
        "vt.", "vi.", "v.", "n.", "adj.", "adv.", "prep.", "conj.", "pron.", "abbr.", "a.",
    ] {
        if let Some(rest) = line.strip_prefix(marker) {
            return (marker.to_string(), rest.trim().to_string());
        }
    }
    ("".to_string(), line.to_string())
}

pub(super) fn fallback_translated(translated: &str, cards: &[ImportedCard], lemma: &str) -> String {
    let translated = translated.trim();
    if !translated.is_empty() {
        return translated.to_string();
    }

    cards
        .iter()
        .find_map(local_chinese_definition)
        .unwrap_or_else(|| lemma.to_string())
}

fn local_chinese_definition(card: &ImportedCard) -> Option<String> {
    parse_plain_text_definitions(&card.plain_text, &card.dictionary_name)
        .into_iter()
        .find(|definition| contains_cjk(&definition.meaning))
        .map(|definition| trim_to_chars(&definition.meaning, 80))
        .filter(|value| !value.is_empty())
}

fn contains_cjk(text: &str) -> bool {
    text.chars().any(|char| {
        matches!(
            char,
            '\u{3400}'..='\u{4dbf}'
                | '\u{4e00}'..='\u{9fff}'
                | '\u{f900}'..='\u{faff}'
        )
    })
}

pub(super) fn summarize_cards_for_ai(cards: &[ImportedCard]) -> Option<String> {
    let summary = cards
        .iter()
        .map(|card| {
            format!(
                "{} / {}: {}",
                card.dictionary_name,
                card.headword,
                trim_to_chars(&card.plain_text, MAX_CARD_TEXT_CHARS)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if summary.trim().is_empty() {
        None
    } else {
        Some(summary)
    }
}
