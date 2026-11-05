use std::collections::HashSet;

use crate::HeadingItem;
use crate::sanitize::{find_attr_value_start, has_attribute, tag_close_offset};

pub(crate) fn inject_heading_ids(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;
    let mut used_ids = HashSet::new();
    let mut fallback_index = 0;

    while let Some((start, level, opening_end)) = find_heading(remaining, 0) {
        output.push_str(&remaining[..start]);
        let close = closing_heading(remaining, opening_end, level);
        let Some(close) = close else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let opening = &remaining[start..opening_end];
        let content = &remaining[opening_end..close];
        let existing_id = attribute_value(opening, "id");
        let has_id = existing_id.is_some();
        let id = existing_id
            .map(str::to_owned)
            .unwrap_or_else(|| unique_slug(content, &mut used_ids, &mut fallback_index));
        used_ids.insert(id.clone());

        if has_id {
            output.push_str(opening);
        } else {
            output.push_str(&opening[..opening.len() - 1]);
            output.push_str(" id=\"");
            output.push_str(&id);
            output.push_str("\">");
        }
        output.push_str(content);
        let close_end = close + 5;
        output.push_str(&remaining[close..close_end]);
        remaining = &remaining[close_end..];
    }

    output.push_str(remaining);
    output
}

pub(crate) fn extract_headings(html: &str) -> Vec<HeadingItem> {
    let mut headings = Vec::new();
    let mut cursor = 0;
    while let Some((start, level, opening_end)) = find_heading(html, cursor) {
        let Some(close) = closing_heading(html, opening_end, level) else {
            break;
        };
        let opening = &html[start..opening_end];
        let visually_hidden = attribute_value(opening, "class").is_some_and(|class| {
            class
                .split_ascii_whitespace()
                .any(|name| name == "mdrs-sr-only")
        });
        let aria_hidden =
            attribute_value(opening, "aria-hidden").is_some_and(|value| value == "true");
        if (2..=6).contains(&level)
            && !visually_hidden
            && !has_attribute(opening, "hidden")
            && !aria_hidden
            && let Some(id) = attribute_value(opening, "id")
        {
            let text = plain_text(&html[opening_end..close]);
            headings.push(HeadingItem {
                id: id.to_owned(),
                text,
                level,
            });
        }
        cursor = close + 5;
    }
    headings
}

pub(crate) fn prefix_hash_links(html: &str) -> String {
    let ids = attribute_values(html, "id");
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find("<a ") {
        output.push_str(&remaining[..start]);
        let Some(relative_end) = tag_close_offset(&remaining[start..]) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let end = start + relative_end + 1;
        let tag = &remaining[start..end];
        if let Some((href_start, href_end)) = quoted_attribute_range(tag, "href")
            && let Some(target) = tag
                .get(href_start..href_end)
                .and_then(|href| href.strip_prefix('#'))
            && !ids.contains(target)
            && !target.starts_with("mdrs-")
        {
            let mut prefixed = String::with_capacity("mdrs-".len() + target.len());
            prefixed.push_str("mdrs-");
            prefixed.push_str(target);
            if ids.contains(prefixed.as_str()) {
                output.push_str(&tag[..href_start]);
                output.push('#');
                output.push_str(&prefixed);
                output.push_str(&tag[href_end..]);
            } else {
                output.push_str(tag);
            }
        } else {
            output.push_str(tag);
        }
        remaining = &remaining[end..];
    }

    output.push_str(remaining);
    output
}

fn find_heading(html: &str, from: usize) -> Option<(usize, u8, usize)> {
    let mut cursor = from;
    while let Some(relative) = html[cursor..].find("<h") {
        let start = cursor + relative;
        let digit = *html.as_bytes().get(start + 2)?;
        if (b'1'..=b'6').contains(&digit)
            && html
                .as_bytes()
                .get(start + 3)
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'>')
        {
            let opening_end = start + tag_close_offset(&html[start..])? + 1;
            return Some((start, digit - b'0', opening_end));
        }
        cursor = start + 2;
    }
    None
}

const CLOSING_TAGS: [&str; 6] = ["</h1>", "</h2>", "</h3>", "</h4>", "</h5>", "</h6>"];

fn closing_heading(html: &str, from: usize, level: u8) -> Option<usize> {
    let tag = CLOSING_TAGS.get(level as usize - 1)?;
    html[from..].find(tag).map(|offset| from + offset)
}

fn unique_slug(
    content: &str,
    used_ids: &mut HashSet<String>,
    fallback_index: &mut usize,
) -> String {
    let text = plain_text(content).to_ascii_lowercase();
    let mut base = String::new();
    let mut needs_separator = false;
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            if needs_separator && !base.is_empty() && !base.ends_with('-') {
                base.push('-');
            }
            base.push(character);
            needs_separator = false;
        } else if character.is_ascii_whitespace() || character == '-' {
            needs_separator = true;
        }
    }

    if base.is_empty() {
        base = format!("heading-{}", *fallback_index);
        *fallback_index += 1;
    }
    let base = format!("mdrs-{base}");
    let mut id = base.clone();
    let mut suffix = 1;
    while used_ids.contains(&id) {
        id = format!("{base}-{suffix}");
        suffix += 1;
    }
    id
}

fn plain_text(html: &str) -> String {
    let mut stripped = String::with_capacity(html.len());
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => stripped.push(character),
            _ => {}
        }
    }
    decode_html_entities(&stripped)
}

fn decode_html_entities(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(amp) = remaining.find('&') {
        output.push_str(&remaining[..amp]);
        let after = &remaining[amp + 1..];
        if let Some(semi) = after.find(';') {
            let entity = &after[..semi];
            let decoded = match entity {
                "amp" => Some("&"),
                "lt" => Some("<"),
                "gt" => Some(">"),
                "quot" => Some("\""),
                "apos" => Some("'"),
                "nbsp" => Some("\u{00A0}"),
                "ndash" => Some("\u{2013}"),
                "mdash" => Some("\u{2014}"),
                "laquo" => Some("\u{00AB}"),
                "raquo" => Some("\u{00BB}"),
                "hellip" => Some("\u{2026}"),
                "lsquo" => Some("\u{2018}"),
                "rsquo" => Some("\u{2019}"),
                "ldquo" => Some("\u{201C}"),
                "rdquo" => Some("\u{201D}"),
                _ => None,
            };
            if let Some(s) = decoded {
                output.push_str(s);
                remaining = &remaining[amp + 1 + semi + 1..];
                continue;
            }
            if let Some(digits) = entity.strip_prefix('#') {
                let codepoint = if let Some(hex) = digits
                    .strip_prefix('x')
                    .or_else(|| digits.strip_prefix('X'))
                {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    digits.parse::<u32>().ok()
                };
                if let Some(cp) = codepoint
                    && let Some(ch) = char::from_u32(cp)
                {
                    output.push(ch);
                    remaining = &remaining[amp + 1 + semi + 1..];
                    continue;
                }
            }
        }
        output.push('&');
        remaining = after;
    }
    output.push_str(remaining);
    output
}

fn attribute_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let (start, end) = quoted_attribute_range(tag, name)?;
    tag.get(start..end)
}

fn quoted_attribute_range(tag: &str, name: &str) -> Option<(usize, usize)> {
    let start = find_attr_value_start(tag, name)?;
    let end = start + tag[start..].find('"')?;
    Some((start, end))
}

fn attribute_values<'a>(html: &'a str, name: &str) -> HashSet<&'a str> {
    let mut values = HashSet::new();
    let mut remaining = html;
    while let Some(start) = remaining.find('<') {
        remaining = &remaining[start..];
        let Some(tag_end) = tag_close_offset(remaining).map(|offset| offset + 1) else {
            break;
        };
        let tag = &remaining[..tag_end];
        if let Some(value_start) = find_attr_value_start(tag, name)
            && let Some(value_end) = tag[value_start..].find('"')
        {
            values.insert(&tag[value_start..value_start + value_end]);
        }
        remaining = &remaining[tag_end..];
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_unique_prefixed_ids_and_prefixes_hash_links() {
        let source =
            "<h1>Title</h1><h2>Same name</h2><h2>Same name</h2><a href=\"#same-name\">jump</a>";
        let with_ids = inject_heading_ids(source);
        assert!(with_ids.contains("id=\"mdrs-title\""));
        assert!(with_ids.contains("id=\"mdrs-same-name\""));
        assert!(with_ids.contains("id=\"mdrs-same-name-1\""));
        let output = prefix_hash_links(&with_ids);
        assert!(output.contains("href=\"#mdrs-same-name\""));
    }

    #[test]
    fn extracts_navigation_headings_without_h1() {
        let html = inject_heading_ids("<h1>Title</h1><h2>Some <em>text</em></h2>");
        let headings = extract_headings(&html);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "Some text");
        assert_eq!(headings[0].level, 2);
    }

    #[test]
    fn does_not_truncate_anchor_tag_at_gt_inside_href_value_in_prefix_hash_links() {
        let html = "<h2 id=\"mdrs-sec\">Section</h2><a href=\"#sec\" title=\"a>b\">link</a>";
        let output = prefix_hash_links(html);
        assert!(output.contains("href=\"#mdrs-sec\""));
    }

    #[test]
    fn does_not_truncate_heading_id_at_gt_inside_attribute_value() {
        let html = r#"<h2 data-x="a>b">Title</h2>"#;
        let with_ids = inject_heading_ids(html);
        assert!(with_ids.contains("id=\"mdrs-title\""));
        assert!(with_ids.contains("data-x=\"a>b\""));
    }

    #[test]
    fn decodes_named_and_numeric_entities_in_heading_text() {
        assert_eq!(decode_html_entities("a &amp; b"), "a & b");
        assert_eq!(decode_html_entities("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_html_entities("&ndash;"), "\u{2013}");
        assert_eq!(decode_html_entities("&#39;"), "'");
        assert_eq!(decode_html_entities("&#x2019;"), "\u{2019}");
        assert_eq!(decode_html_entities("no entities"), "no entities");
        assert_eq!(decode_html_entities("bare & ampersand"), "bare & ampersand");
    }

    #[test]
    fn quoted_attribute_range_skips_attribute_like_text_in_quoted_value() {
        let tag = r#"<h2 data-x='id="fake"' id="real">"#;
        assert_eq!(attribute_value(tag, "id"), Some("real"));
    }

    #[test]
    fn injects_heading_id_when_id_like_text_is_inside_another_attribute() {
        let html = "<h2 data-x='foo id=\"fake\"'>Title</h2>";
        let with_ids = inject_heading_ids(html);
        assert!(with_ids.contains("id=\"mdrs-title\""));
        assert!(with_ids.contains("data-x='foo id=\"fake\"'"));
    }

    #[test]
    fn does_not_match_attribute_name_suffixes() {
        assert_eq!(attribute_value(r#"<h2 data-id="fake">"#, "id"), None);
    }

    #[test]
    fn ignores_id_like_plain_text_when_prefixing_hash_links() {
        let html = r##"<p>id="real"</p><h2 id="mdrs-real">Real</h2><a href="#real">jump</a>"##;
        assert!(prefix_hash_links(html).contains("href=\"#mdrs-real\""));
    }

    #[test]
    fn does_not_hide_heading_for_marker_text_in_another_attribute() {
        let html = r#"<h2 id="mdrs-visible" title=" hidden mdrs-sr-only">Visible</h2>"#;
        let headings = extract_headings(html);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "Visible");
    }

    #[test]
    fn skips_headings_with_actual_hidden_attributes() {
        let html = r#"<h2 id="mdrs-hidden" hidden>Hidden</h2><h2 id="mdrs-visible">Visible</h2>"#;
        let headings = extract_headings(html);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].id, "mdrs-visible");
    }
}
