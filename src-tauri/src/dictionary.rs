use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub word: String,
    pub phonetic: String,
    pub definitions: Vec<Definition>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub part_of_speech: String,
    pub meaning: String,
}

pub fn lookup(word: &str) -> Option<DictionaryEntry> {
    let normalized = word.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "hello" => Some(entry(
            "hello",
            "/he'lo/",
            vec![("interj.", "你好；喂；表示问候"), ("n.", "问候；招呼")],
            vec!["Hello, nice to meet you."],
        )),
        "world" => Some(entry(
            "world",
            "/wɜːrld/",
            vec![("n.", "世界；地球；领域；世人")],
            vec!["The world is changing quickly."],
        )),
        "translate" => Some(entry(
            "translate",
            "/træns'leɪt/",
            vec![("v.", "翻译；转化；解释；转换")],
            vec!["Please translate this sentence into Chinese."],
        )),
        "run" => Some(entry(
            "run",
            "/rʌn/",
            vec![
                ("v.", "跑；运行；经营；持续"),
                ("n.", "跑步；行程；连续一段时间"),
            ],
            vec!["The service can run in the background."],
        )),
        "selection" => Some(entry(
            "selection",
            "/sɪ'lekʃn/",
            vec![("n.", "选择；选区；被选中的内容")],
            vec!["The app reads the current text selection."],
        )),
        "dictionary" => Some(entry(
            "dictionary",
            "/'dɪkʃəneri/",
            vec![("n.", "词典；字典；专业术语表")],
            vec!["A local dictionary works without network access."],
        )),
        _ => None,
    }
}

fn entry(
    word: &str,
    phonetic: &str,
    definitions: Vec<(&str, &str)>,
    examples: Vec<&str>,
) -> DictionaryEntry {
    DictionaryEntry {
        word: word.to_string(),
        phonetic: phonetic.to_string(),
        definitions: definitions
            .into_iter()
            .map(|(part_of_speech, meaning)| Definition {
                part_of_speech: part_of_speech.to_string(),
                meaning: meaning.to_string(),
            })
            .collect(),
        examples: examples.into_iter().map(str::to_string).collect(),
    }
}
