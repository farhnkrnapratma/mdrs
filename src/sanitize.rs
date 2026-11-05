use std::collections::HashSet;
use std::sync::LazyLock;

static ALLOWED_ATTRIBUTES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "aria-describedby",
        "aria-hidden",
        "aria-label",
        "aria-selected",
        "class",
        "data-mdrs-chart",
        "data-mdrs-source",
        "data-mdrs-tab-key",
        "data-mdrs-tab-label",
        "hidden",
        "stroke",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-width",
        "viewBox",
        "width",
        "role",
        "title",
        "fill",
        "height",
        "x",
        "x1",
        "x2",
        "y",
        "y1",
        "y2",
        "d",
        "cx",
        "cy",
        "r",
        "xmlns",
    ]
    .into_iter()
    .collect()
});

pub(crate) fn escape_html(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
    }
    output
}

pub(crate) fn sanitize_html(html: &str) -> String {
    let mut sanitizer = ammonia::Builder::default();
    sanitizer.add_tags(&["svg", "path", "rect", "line", "circle"]);
    sanitizer.add_generic_attributes(&*ALLOWED_ATTRIBUTES);
    filter_classes(&sanitizer.clean(html).to_string())
}

fn filter_classes(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;
    while let Some(start) = remaining.find('<') {
        output.push_str(&remaining[..start]);
        let Some(relative_end) = tag_close_offset(&remaining[start..]) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let end = start + relative_end + 1;
        let tag = &remaining[start..end];
        output.push_str(&filter_tag_classes(tag));
        remaining = &remaining[end..];
    }
    output.push_str(remaining);
    output
}

pub(crate) fn find_attr_value_start(tag: &str, name: &str) -> Option<usize> {
    let mut needle = String::with_capacity(name.len() + 2);
    needle.push_str(name);
    needle.push_str("=\"");
    let nlen = needle.len();
    let mut in_dq = false;
    let mut in_sq = false;
    let mut i = 0;
    while i + nlen <= tag.len() {
        let c = tag[i..].chars().next()?;
        match c {
            '"' if !in_sq => {
                in_dq = !in_dq;
                i += 1;
                continue;
            }
            '\'' if !in_dq => {
                in_sq = !in_sq;
                i += 1;
                continue;
            }
            _ => {}
        }
        let starts_attribute = i > 0 && tag.as_bytes()[i - 1].is_ascii_whitespace();
        if starts_attribute && !in_dq && !in_sq && tag[i..].starts_with(needle.as_str()) {
            return Some(i + nlen);
        }
        i += c.len_utf8();
    }
    None
}

pub(crate) fn has_attribute(tag: &str, name: &str) -> bool {
    let mut in_dq = false;
    let mut in_sq = false;
    let mut i = 0;
    while i < tag.len() {
        let c = match tag[i..].chars().next() {
            Some(c) => c,
            None => return false,
        };
        match c {
            '"' if !in_sq => in_dq = !in_dq,
            '\'' if !in_dq => in_sq = !in_sq,
            _ => {}
        }
        if !in_dq
            && !in_sq
            && i > 0
            && tag.as_bytes()[i - 1].is_ascii_whitespace()
            && tag[i..]
                .get(..name.len())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
        {
            let end = i + name.len();
            if tag.as_bytes().get(end).is_some_and(|byte| {
                byte.is_ascii_whitespace() || matches!(byte, b'=' | b'>' | b'/')
            }) {
                return true;
            }
        }
        i += c.len_utf8();
    }
    false
}

pub(crate) fn tag_close_offset(tag: &str) -> Option<usize> {
    let mut chars = tag.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        match ch {
            '>' => return Some(i),
            '"' | '\'' => {
                let quote = ch;
                for (_, c) in chars.by_ref() {
                    if c == quote {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn filter_tag_classes(tag: &str) -> String {
    let Some(class_start) = tag.find("class=\"") else {
        return tag.to_owned();
    };
    let value_start = class_start + "class=\"".len();
    let Some(relative_end) = tag[value_start..].find('"') else {
        return tag.to_owned();
    };
    let value_end = value_start + relative_end;

    let mut allowed = String::new();
    for class in tag[value_start..value_end].split_ascii_whitespace() {
        if allowed_class(class) {
            if !allowed.is_empty() {
                allowed.push(' ');
            }
            allowed.push_str(class);
        }
    }

    let mut output = String::with_capacity(tag.len());
    let before = &tag[..class_start];
    if allowed.is_empty() {
        output.push_str(before.trim_end());
    } else {
        output.push_str(before);
        output.push_str("class=\"");
        output.push_str(&allowed);
        output.push('"');
    }
    output.push_str(&tag[value_end + 1..]);
    output
}

fn allowed_class(class: &str) -> bool {
    if matches!(class, "mdrs" | "katex" | "hljs" | "bi") {
        return true;
    }
    if let Some(suffix) = class.strip_prefix("mdrs-") {
        return valid_class_suffix(suffix);
    }
    ["katex-", "hljs-", "language-", "bi-"]
        .iter()
        .any(|prefix| class.strip_prefix(prefix).is_some_and(valid_class_suffix))
}

fn valid_class_suffix(suffix: &str) -> bool {
    !suffix.is_empty()
        && suffix.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && suffix
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_unsafe_urls() {
        let html = sanitize_html(r#"<a href="javascript:alert(1)">link</a>"#);
        assert!(!html.contains("javascript:"));
        assert!(html.contains("link"));
    }

    #[test]
    fn removes_script_content_event_handlers_and_encoded_unsafe_urls() {
        let html = sanitize_html(
            r#"<script>alert(1)</script><img src="x" onerror="alert(2)"><a href="java&#x73;cript:alert(3)" onclick="alert(4)">link</a>"#,
        );
        assert!(!html.contains("alert("));
        assert!(!html.contains("onerror"));
        assert!(!html.contains("onclick"));
        assert!(!html.contains("javascript:"));
        assert!(html.contains("link"));
    }

    #[test]
    fn restricts_classes_to_known_engine_namespaces() {
        let html =
            sanitize_html(r#"<div class="mdrs-tabs arbitrary"><p class="fixed">text</p></div>"#);
        assert!(html.contains("class=\"mdrs-tabs\""));
        assert!(!html.contains("arbitrary"));
        assert!(!html.contains("fixed"));
    }

    #[test]
    fn does_not_truncate_tag_at_gt_inside_attribute_value() {
        let html = sanitize_html(r#"<div title="a>b" class="mdrs-alert arbitrary">text</div>"#);
        assert!(html.contains("class=\"mdrs-alert\""));
        assert!(!html.contains("arbitrary"));
        assert!(html.contains("text"));
    }

    #[test]
    fn removes_empty_class_attribute_without_leaving_double_space() {
        let html = filter_tag_classes(r#"<div class="arbitrary">text"#);
        assert!(!html.contains("class="));
        assert!(!html.contains("  "));
    }
}
