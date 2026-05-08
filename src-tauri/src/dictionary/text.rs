use super::util::normalize_word;
use std::path::Path;

pub(super) fn mdx_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("MDX Dictionary")
        .to_string()
}

pub(super) fn is_indexable_mdx_headword(headword: &str) -> bool {
    let normalized = normalize_word(headword);
    !normalized.is_empty()
        && normalized.len() <= 80
        && normalized
            .chars()
            .all(|char| char.is_ascii_alphabetic() || char == '\'' || char == '-')
}

pub(super) fn html_to_safe_text(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag = String::new();
    let mut skip_until: Option<&'static str> = None;

    for char in html.chars() {
        if let Some(end_tag) = skip_until {
            tag.push(char.to_ascii_lowercase());
            if tag.ends_with(end_tag) {
                skip_until = None;
                tag.clear();
            }
            continue;
        }

        match char {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                let tag_name = tag
                    .trim()
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if tag_name == "script" {
                    skip_until = Some("</script>");
                } else if tag_name == "style" {
                    skip_until = Some("</style>");
                }
                in_tag = false;
                output.push(' ');
                tag.clear();
            }
            _ if in_tag => tag.push(char),
            _ => output.push(char),
        }
    }

    decode_basic_entities(&output)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_basic_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}
