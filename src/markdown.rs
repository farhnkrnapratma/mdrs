use std::collections::HashMap;

use katex::{KatexContext, Settings};
use pulldown_cmark::{Options, Parser, html};
use serde_json::Value;

use crate::RenderError;
use crate::RenderOptions;
use crate::sanitize::{find_attr_value_start, tag_close_offset};

use crate::containers;

pub(crate) struct ResolvedOptions {
    pub(crate) internal_origins: Vec<String>,
}

impl ResolvedOptions {
    pub(crate) fn new(options: &RenderOptions) -> Self {
        Self {
            internal_origins: options
                .internal_origins
                .iter()
                .filter_map(|origin| url::Url::parse(origin).ok())
                .map(|url| url.origin().ascii_serialization())
                .collect(),
        }
    }
}

pub(crate) fn prepare_source(markdown: &str) -> String {
    let mut source = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    if let Some(after_opening) = source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))
        && let Some(end) = find_frontmatter_end(after_opening)
    {
        source = &after_opening[end..];
        source = source.trim_start_matches(['\r', '\n']);
    }

    source.to_owned()
}

fn find_frontmatter_end(source: &str) -> Option<usize> {
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']).trim();
        if trimmed == "---" || trimmed == "..." {
            return Some(offset + line.len());
        }
        offset += line.len();
    }
    None
}

pub(crate) fn resolve_defines(source: &str) -> Result<String, RenderError> {
    let mut definitions = HashMap::new();
    let mut output = Vec::new();
    let mut in_define = false;
    let mut fence: Option<(char, usize)> = None;
    let mut dropping_leading_blank_lines = false;

    for line in source.lines() {
        if let Some((marker, minimum)) = fence {
            if let Some((closing, count)) = fence_marker(line)
                && closing == marker
                && count >= minimum
            {
                fence = None;
            }
            output.push(line);
            continue;
        }

        if let Some(opening) = fence_marker(line) {
            fence = Some(opening);
            output.push(line);
            continue;
        }

        if !in_define && line.trim() == ":::define" {
            in_define = true;
            dropping_leading_blank_lines = true;
            continue;
        }

        if in_define {
            if line.trim() == ":::" {
                in_define = false;
            } else if let Some((name, expression)) = line.split_once(':') {
                let name = name.trim();
                let expression = expression.trim();
                if !valid_identifier(name) {
                    return Err(RenderError::UnsupportedDefineValue {
                        name: name.to_owned(),
                        expression: expression.to_owned(),
                    });
                }
                definitions.insert(
                    name.to_owned(),
                    parse_scalar(expression).ok_or_else(|| {
                        RenderError::UnsupportedDefineValue {
                            name: name.to_owned(),
                            expression: expression.to_owned(),
                        }
                    })?,
                );
            } else if !line.trim().is_empty() {
                return Err(RenderError::UnsupportedDefineValue {
                    name: String::new(),
                    expression: line.trim().to_owned(),
                });
            }
            continue;
        }

        if dropping_leading_blank_lines && line.trim().is_empty() {
            continue;
        }
        dropping_leading_blank_lines = false;
        output.push(line);
    }

    if in_define {
        return Err(RenderError::UnclosedDefineBlock);
    }

    let mut rendered = String::new();
    let mut in_fence: Option<(char, usize)> = None;
    for line in output {
        if let Some((marker, minimum)) = in_fence {
            rendered.push_str(line);
            rendered.push('\n');
            if let Some((closing, count)) = fence_marker(line)
                && closing == marker
                && count >= minimum
            {
                in_fence = None;
            }
            continue;
        }
        if let Some(opening) = fence_marker(line) {
            in_fence = Some(opening);
            rendered.push_str(line);
            rendered.push('\n');
            continue;
        }
        let line = interpolate_line(line, &definitions)?;
        crate::check_rendered_output_size(&line)?;
        rendered.push_str(&line);
        rendered.push('\n');
        crate::check_rendered_output_size(&rendered)?;
    }

    if rendered.ends_with('\n') {
        rendered.pop();
    }
    Ok(rendered)
}

pub(crate) fn fence_marker(line: &str) -> Option<(char, usize)> {
    let line = line.trim_start();
    if line.starts_with("```") || line.starts_with("~~~") {
        let marker = line.chars().next()?;
        let count = line
            .chars()
            .take_while(|character| *character == marker)
            .count();
        return Some((marker, count));
    }
    None
}

fn valid_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
}

fn parse_scalar(value: &str) -> Option<String> {
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        let inner = &value[1..value.len() - 1];
        return Some(if inner.contains("''") {
            inner.replace("''", "'")
        } else {
            inner.to_owned()
        });
    }
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        let inner = &value[1..value.len() - 1];
        let mut output = String::new();
        let mut escaped = false;
        for character in inner.chars() {
            if escaped {
                output.push(match character {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => other,
                });
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else {
                output.push(character);
            }
        }
        if escaped {
            output.push('\\');
        }
        return Some(output);
    }
    if value.is_empty() || value.starts_with('#') {
        return Some(String::new());
    }
    if value.contains("${") || value.starts_with("api.v1.") {
        return None;
    }
    Some(value.to_owned())
}

fn interpolate_line(
    line: &str,
    definitions: &HashMap<String, String>,
) -> Result<String, RenderError> {
    let bytes = line.as_bytes();
    let mut output = String::with_capacity(line.len());
    let mut index = 0;
    let mut code_ticks = 0;

    while index < bytes.len() {
        if bytes[index] == b'`' {
            let start = index;
            while index < bytes.len() && bytes[index] == b'`' {
                index += 1;
            }
            let ticks = index - start;
            if code_ticks == 0 {
                code_ticks = ticks;
            } else if code_ticks == ticks {
                code_ticks = 0;
            }
            output.push_str(&line[start..index]);
            continue;
        }

        if code_ticks == 0
            && bytes[index] == b'$'
            && !is_escaped(bytes, index)
            && let Some((end, name)) = interpolation_name(line, index)
        {
            if let Some(value) = definitions.get(name) {
                output.push_str(value);
                index = end;
                continue;
            }
            if !definitions.is_empty() {
                return Err(RenderError::UndefinedVariable(name.to_owned()));
            }
        }

        let character = line[index..].chars().next().unwrap_or_default();
        output.push(character);
        index += character.len_utf8();
    }
    Ok(output)
}

fn is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut slashes = 0;
    let mut position = index;
    while position > 0 && bytes[position - 1] == b'\\' {
        slashes += 1;
        position -= 1;
    }
    slashes % 2 == 1
}

fn interpolation_name(line: &str, start: usize) -> Option<(usize, &str)> {
    let rest = line.get(start..)?.strip_prefix("${")?;
    let end = rest.find('}')?;
    let name = &rest[..end];
    if valid_identifier(name) {
        Some((start + 2 + end + 1, name))
    } else {
        None
    }
}

pub(crate) fn render(markdown: &str, options: &ResolvedOptions) -> String {
    containers::render(&preprocess_extensions(markdown), options)
}

pub(crate) fn resolve_conditionals(
    source: &str,
    scope: &crate::define::Scope,
    data: &Value,
) -> Result<String, RenderError> {
    let lines: Vec<&str> = source.lines().collect();
    let has_block = lines.iter().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(":::if ") || trimmed == ":::else" || trimmed == ":::endif"
    });
    let mut output = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.starts_with(":::if ") {
            let mut depth = 1;
            let mut branches: Vec<(Option<&str>, Vec<&str>)> =
                vec![(trimmed.strip_prefix(":::if "), Vec::new())];
            index += 1;
            while index < lines.len() && depth > 0 {
                let current = lines[index].trim();
                if current.starts_with(":::if ") {
                    depth += 1;
                    if let Some((_, body)) = branches.last_mut() {
                        body.push(lines[index]);
                    }
                } else if current == ":::endif" {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    if let Some((_, body)) = branches.last_mut() {
                        body.push(lines[index]);
                    }
                } else if depth == 1 && current == ":::else" {
                    branches.push((None, Vec::new()));
                } else {
                    if let Some((_, body)) = branches.last_mut() {
                        body.push(lines[index]);
                    }
                }
                index += 1;
            }
            if depth != 0 {
                return Err(RenderError::UnsupportedDefineValue {
                    name: "if".to_owned(),
                    expression: "unclosed :::if block".to_owned(),
                });
            }
            let selected = branches.into_iter().find(|(condition, _)| {
                condition.is_none_or(|condition| {
                    crate::define::eval(condition, scope, data, false)
                        .is_ok_and(|value| crate::define::value_as_bool(&value))
                })
            });
            if let Some((_, body)) = selected {
                output.extend(body);
            }
            index += 1;
            continue;
        }
        if has_block && (trimmed == ":::else" || trimmed == ":::endif") {
            index += 1;
            continue;
        }
        output.push(line);
        index += 1;
    }

    let mut rendered = output.join("\n");
    if source.ends_with('\n') {
        rendered.push('\n');
    }
    if rendered.contains("${if ") {
        rendered = resolve_inline_conditionals(&rendered, scope, data);
    }
    Ok(rendered)
}

fn resolve_inline_conditionals(source: &str, scope: &crate::define::Scope, data: &Value) -> String {
    let mut output = source.to_owned();
    while let Some(start) = output.find("${if ") {
        let Some(condition_end) = output[start + 5..].find('}') else {
            break;
        };
        let condition_start = start + 5;
        let condition = output[condition_start..condition_start + condition_end].trim();
        let body_start = condition_start + condition_end + 1;
        let Some(end) = output[body_start..].find("${endif}") else {
            break;
        };
        let end = body_start + end;
        let whole_end = end + "${endif}".len();
        let body = output[body_start..end].to_owned();
        let (yes, no) = body.split_once("${else}").unwrap_or((body.as_str(), ""));
        let selected = crate::define::eval(condition, scope, data, false)
            .is_ok_and(|value| crate::define::value_as_bool(&value));
        let replacement = if selected { yes } else { no };
        output.replace_range(start..whole_end, replacement);
    }
    output
}

const PARSER_OPTIONS: Options = Options::from_bits_truncate(
    Options::ENABLE_TABLES.bits()
        | Options::ENABLE_FOOTNOTES.bits()
        | Options::ENABLE_STRIKETHROUGH.bits()
        | Options::ENABLE_TASKLISTS.bits()
        | Options::ENABLE_SMART_PUNCTUATION.bits()
        | Options::ENABLE_MATH.bits()
        | Options::ENABLE_GFM.bits(),
);

pub(crate) fn render_markdown_fragment(markdown: &str, options: &ResolvedOptions) -> String {
    let parser = Parser::new_ext(markdown, PARSER_OPTIONS);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    add_external_link_attributes(&output, options)
}

pub(crate) fn restore_extensions(html: &str) -> Result<String, RenderError> {
    if !html.contains("class=\"math ")
        && !html.contains("<pre><code class=\"language-mermaid\">")
        && !html.contains("<pre><code class=\"language-chart\">")
        && !html.contains("<li")
        && !html.contains("<h2>References</h2>")
        && !html.contains("<p>REFERENCESPLACEMENT</p>")
    {
        return Ok(html.to_owned());
    }
    let mut output = render_math(html)?;
    output = restore_code_block(&output, "mermaid", "mdrs-mermaid");
    let output = restore_code_block(&output, "chart", "mdrs-chart");
    let output = restore_task_states(&output);
    let output = output.replace(
        "<h2>References</h2>",
        "<h2><span class=\"mdrs-references\">References</span></h2>",
    );
    Ok(output.replace(
        "<p>REFERENCESPLACEMENT</p>",
        "<section class=\"mdrs-references\"><h2>References</h2></section>",
    ))
}

fn render_math(html: &str) -> Result<String, RenderError> {
    let context = KatexContext::default();
    let inline_settings = Settings::builder().display_mode(false).build();
    let block_settings = Settings::builder().display_mode(true).build();
    let html = replace_math_element(
        html,
        "<div class=\"mdrs-math mdrs-math--block\">",
        "</div>",
        &context,
        &block_settings,
        true,
    )?;
    let html = replace_math_element(
        &html,
        "<span class=\"math math-inline\">",
        "</span>",
        &context,
        &inline_settings,
        false,
    )?;
    replace_math_element(
        &html,
        "<span class=\"math math-display\">",
        "</span>",
        &context,
        &inline_settings,
        false,
    )
}

fn replace_math_element(
    html: &str,
    opening: &str,
    closing: &str,
    context: &KatexContext,
    settings: &Settings,
    display: bool,
) -> Result<String, RenderError> {
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;
    while let Some(start) = remaining.find(opening) {
        output.push_str(&remaining[..start]);
        let body_start = start + opening.len();
        let Some(end) = remaining[body_start..].find(closing) else {
            output.push_str(&remaining[start..]);
            return Ok(output);
        };
        let body_end = body_start + end;
        let source = decode_code_block_entities(&remaining[body_start..body_end]);
        match katex::render_to_string(context, &source, settings) {
            Ok(rendered) => {
                crate::check_rendered_output_size(&rendered)?;
                if display {
                    output.push_str("<div class=\"mdrs-math mdrs-math--block\">");
                    output.push_str(&rendered);
                    output.push_str("</div>");
                } else {
                    output.push_str(&rendered);
                }
            }
            Err(_) => {
                output.push_str(opening);
                output.push_str(&remaining[body_start..body_end]);
                output.push_str(closing);
            }
        }
        remaining = &remaining[body_end + closing.len()..];
    }
    output.push_str(remaining);
    crate::check_rendered_output_size(&output)?;
    Ok(output)
}

pub(crate) fn mark_references(html: &str) -> String {
    if !html.contains("<h2>References</h2>") {
        return html.to_owned();
    }
    html.replace(
        "<h2>References</h2>",
        "<h2><span class=\"mdrs-references\">References</span></h2>",
    )
}

fn restore_task_states(html: &str) -> String {
    let states = [
        (
            'x',
            "done",
            "Done",
            r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="m9 12 2 2 4-4"/>"#,
        ),
        (
            '*',
            "in-progress",
            "In progress",
            r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M12 8v8"/><path d="m8.5 14 7-4"/><path d="m8.5 10 7 4"/>"#,
        ),
        (
            '/',
            "failed",
            "Failed",
            r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>"#,
        ),
        (
            '-',
            "skipped",
            "Skipped",
            r#"<rect width="18" height="18" x="3" y="3" rx="2"/><line x1="8" x2="16" y1="12" y2="12"/>"#,
        ),
        (
            '!',
            "important",
            "Important",
            r#"<rect width="18" height="18" x="3" y="3" rx="2"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/>"#,
        ),
        (
            '?',
            "question",
            "Question",
            r#"<rect width="18" height="18" x="3" y="3" rx="2"/><rect width="8" height="8" x="8" y="8" rx="1"/>"#,
        ),
    ];
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;
    while let Some(start) = remaining.find("<li") {
        output.push_str(&remaining[..start]);
        let Some(open_relative_end) = remaining[start..].find('>') else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let open_end = start + open_relative_end + 1;
        let Some(close) = matching_list_item_end(remaining, open_end) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let mut opening = remaining[start..open_end].to_owned();
        let mut body = restore_task_states(&remaining[open_end..close]);
        let mut status = None;
        if let Some(input_end) = body.find('>') {
            let input = &body[..=input_end];
            if input.starts_with("<input") {
                status = Some(if input.contains("checked") { 'x' } else { ' ' });
                body.replace_range(..=input_end, "");
            }
        }
        if status.is_none() {
            status = states
                .iter()
                .find_map(|(marker, ..)| {
                    let marker_text = format!("[{marker}]");
                    body.find(&marker_text).and_then(|position| {
                        (body[..position].trim().is_empty() || body[..position].ends_with('>'))
                            .then_some((*marker, position))
                    })
                })
                .map(|(marker, position)| {
                    body.replace_range(position..position + 3, "");
                    marker
                });
        }
        if let Some(status) = status {
            let (state, label, icon) = match status {
                'x' => (
                    "done",
                    "Done",
                    r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="m9 12 2 2 4-4"/>"#,
                ),
                '*' => (
                    "in-progress",
                    "In progress",
                    r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M12 8v8"/><path d="m8.5 14 7-4"/><path d="m8.5 10 7 4"/>"#,
                ),
                '/' => (
                    "failed",
                    "Failed",
                    r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>"#,
                ),
                '-' => (
                    "skipped",
                    "Skipped",
                    r#"<rect width="18" height="18" x="3" y="3" rx="2"/><line x1="8" x2="16" y1="12" y2="12"/>"#,
                ),
                '!' => (
                    "important",
                    "Important",
                    r#"<rect width="18" height="18" x="3" y="3" rx="2"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/>"#,
                ),
                '?' => (
                    "question",
                    "Question",
                    r#"<rect width="18" height="18" x="3" y="3" rx="2"/><rect width="8" height="8" x="8" y="8" rx="1"/>"#,
                ),
                _ => (
                    "todo",
                    "To do",
                    r#"<rect width="18" height="18" x="3" y="3" rx="2"/>"#,
                ),
            };
            body = body.trim().to_owned();
            opening = r#"<li class="mdrs-list-item mdrs-task mdrs-task--"#.to_owned();
            opening.push_str(state);
            opening.push_str(r#"">"#);
            body = format!(
                r#"<span class="mdrs-task-icon" aria-hidden="true"><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mdrs-icon">{icon}</svg></span><span class="mdrs-sr-only">{label}: </span><div class="mdrs-task-content">{body}</div>"#
            );
        }
        output.push_str(&opening);
        output.push_str(&body);
        output.push_str("</li>");
        remaining = &remaining[close + 5..];
    }
    output.push_str(remaining);
    output
}

fn matching_list_item_end(html: &str, body_start: usize) -> Option<usize> {
    let mut depth = 1;
    let mut cursor = body_start;
    while cursor < html.len() {
        let relative = html[cursor..].find('<')?;
        let tag_start = cursor + relative;
        if html[tag_start..].starts_with("<li") {
            depth += 1;
        } else if html[tag_start..].starts_with("</li>") {
            depth -= 1;
            if depth == 0 {
                return Some(tag_start);
            }
        }
        cursor = tag_start + 1;
    }
    None
}

fn restore_code_block(html: &str, language: &str, _class_name: &str) -> String {
    let marker = format!("<pre><code class=\"language-{language}\">");
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;
    while let Some(start) = remaining.find(&marker) {
        output.push_str(&remaining[..start]);
        let body_start = start + marker.len();
        let Some(end) = remaining[body_start..].find("</code></pre>") else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let body_end = body_start + end;
        let body = &remaining[body_start..body_end];
        let encoded = encode_uri_component(&decode_code_block_entities(body));
        if language == "mermaid" {
            output.push_str("<div class=\"mdrs-mermaid\"><div class=\"mdrs-mermaid-display\"><div class=\"mdrs-mermaid-loading\"><span class=\"mdrs-spinner\"></span><span>Rendering diagram…</span></div></div><div class=\"mdrs-mermaid-source\" hidden data-mdrs-source=\"");
            output.push_str(&encoded);
            output.push_str("\"></div></div>");
        } else {
            output.push_str("<div class=\"mdrs-chart\" data-mdrs-chart=\"");
            output.push_str(&encoded);
            output.push_str("\"><div class=\"mdrs-chart-loading\"><span class=\"mdrs-spinner\"></span>Loading chart visualization…</div></div>");
        }
        remaining = &remaining[body_end + "</code></pre>".len()..];
    }

    fn encode_uri_component(value: &str) -> String {
        let mut output = String::with_capacity(value.len());
        for byte in value.bytes() {
            if byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
                )
            {
                output.push(byte as char);
            } else {
                output.push('%');
                output.push_str(&format!("{byte:02X}"));
            }
        }
        output
    }
    output.push_str(remaining);
    output
}

fn decode_code_block_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn preprocess_extensions(source: &str) -> String {
    let mut output = Vec::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(":::math") {
            let mut body = Vec::new();
            for next in lines.by_ref() {
                if next.trim() == ":::" {
                    break;
                }
                body.push(next);
            }
            output.push(format!(
                "<div class=\"mdrs-math mdrs-math--block\">{}</div>",
                crate::sanitize::escape_html(&body.join("\n"))
            ));
        } else if trimmed.to_ascii_lowercase().starts_with(":::embed ") {
            let url = trimmed
                .split_once("url=")
                .map(|(_, value)| value.trim().trim_matches(['"', '\'']))
                .unwrap_or_default();
            output.push(format!(
                "<div class=\"mdrs-embed\" data-mdrs-embed-url=\"{}\"></div>",
                crate::sanitize::escape_html(url)
            ));
        } else if trimmed.eq_ignore_ascii_case(":::references") {
            let mut definitions = Vec::new();
            for next in lines.by_ref() {
                if next.trim() == ":::" {
                    break;
                }
                definitions.push(next);
            }
            output.push(String::new());
            output.push("REFERENCESPLACEMENT".to_owned());
            output.push(String::new());
            output.extend(definitions.into_iter().map(str::to_owned));
        } else if trimmed.eq_ignore_ascii_case(":::verbatim")
            || trimmed.eq_ignore_ascii_case("::::verbatim")
        {
            let marker = if trimmed.starts_with("::::") {
                "::::"
            } else {
                ":::"
            };
            let mut body = Vec::new();
            for next in lines.by_ref() {
                if next.trim() == marker {
                    break;
                }
                body.push(next);
            }
            output.push(format!(
                "<pre class=\"mdrs-verbatim\"><code>{}</code></pre>",
                crate::sanitize::escape_html(&body.join("\n"))
            ));
        } else {
            output.push(line.to_owned());
        }
    }
    output.join("\n")
}

pub(crate) fn add_external_link_attributes(html: &str, options: &ResolvedOptions) -> String {
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find("<a ") {
        output.push_str(&remaining[..start]);
        let tag_end = match tag_close_offset(&remaining[start..]) {
            Some(offset) => start + offset + 1,
            None => {
                output.push_str(&remaining[start..]);
                return output;
            }
        };
        let tag = &remaining[start..tag_end];
        let href = find_attr_value_start(tag, "href")
            .and_then(|vs| tag[vs..].find('"').map(|end| &tag[vs..vs + end]));
        if let Some(href) = href
            && let Ok(url) = url::Url::parse(
                if href.contains("&amp;") {
                    href.replace("&amp;", "&")
                } else {
                    href.to_owned()
                }
                .as_str(),
            )
            && matches!(url.scheme(), "http" | "https")
            && !options
                .internal_origins
                .contains(&url.origin().ascii_serialization())
        {
            output.push_str(&decorate_external_link(tag));
        } else {
            output.push_str(tag);
        }
        remaining = &remaining[tag_end..];
    }

    output.push_str(remaining);
    output
}

fn tag_has_attrs(tag: &str, a: &str, b: &str) -> (bool, bool) {
    (
        find_attr_value_start(tag, a).is_some(),
        find_attr_value_start(tag, b).is_some(),
    )
}

fn decorate_external_link(tag: &str) -> String {
    let close = tag.len().saturating_sub(1);
    let attrs_end = if tag.ends_with('>') { close } else { tag.len() };

    let mut out = String::with_capacity(tag.len() + 64);

    if let Some(value_start) = find_attr_value_start(tag, "class") {
        if let Some(rel_end) = tag[value_start..].find('"') {
            let value_end = value_start + rel_end;
            let existing = &tag[value_start..value_end];
            let has_class = existing
                .split_ascii_whitespace()
                .any(|c| c == "mdrs-external-link");
            out.push_str(&tag[..value_end]);
            if !has_class {
                out.push_str(" mdrs-external-link");
            }
            out.push_str(&tag[value_end..attrs_end]);
        } else {
            out.push_str(&tag[..attrs_end]);
        }
    } else {
        out.push_str(&tag[..attrs_end]);
        out.push_str(" class=\"mdrs-external-link\"");
    }

    let (has_target, has_rel) = tag_has_attrs(tag, "target", "rel");
    if !has_target {
        out.push_str(" target=\"_blank\"");
    }
    if !has_rel {
        out.push_str(" rel=\"noopener noreferrer\"");
    }

    if tag.ends_with('>') {
        out.push('>');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_truncate_anchor_tag_at_gt_inside_attribute_value() {
        let options = ResolvedOptions {
            internal_origins: vec![],
        };
        let html = add_external_link_attributes(
            r#"<a href="https://example.com/" title="a>b">link</a>"#,
            &options,
        );
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("noopener"));
    }

    #[test]
    fn does_not_skip_target_attr_when_another_attr_value_contains_target_eq() {
        let tag = r#"<a href="https://x.com/" title="target=\"foo\"">"#;
        let decorated = decorate_external_link(tag);
        assert!(decorated.contains("target=\"_blank\""));
    }

    #[test]
    fn adds_rel_when_single_quoted_attr_value_contains_rel_eq() {
        let tag = r#"<a href="https://x.com/" title='rel="nofollow"'>"#;
        let decorated = decorate_external_link(tag);
        assert!(decorated.contains("rel=\"noopener noreferrer\""));
    }

    #[test]
    fn does_not_match_class_attr_inside_another_attrs_value() {
        let tag = r#"<a href="https://x.com/" data-x='class="foo"'>"#;
        let decorated = decorate_external_link(tag);
        assert!(decorated.contains("class=\"mdrs-external-link\""));
    }

    #[test]
    fn does_not_match_href_attr_inside_another_attrs_value() {
        let options = ResolvedOptions {
            internal_origins: vec![],
        };
        let html = add_external_link_attributes(
            r#"<a title='href="https://evil.com/"' href="https://real.com/">link</a>"#,
            &options,
        );
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("https://real.com/"));
    }

    #[test]
    fn does_not_skip_target_attr_when_single_quoted_value_contains_target_eq() {
        let tag = r#"<a href="https://x.com/" title='target="foo"'>"#;
        let decorated = decorate_external_link(tag);
        assert!(decorated.contains("target=\"_blank\""));
    }

    #[test]
    fn decorate_external_link_builds_tag_without_double_shift() {
        let tag = r#"<a href="https://x.com/">"#;
        let decorated = decorate_external_link(tag);
        assert!(decorated.contains("mdrs-external-link"));
        assert!(decorated.contains("target=\"_blank\""));
        assert!(decorated.contains("rel=\"noopener noreferrer\""));
        assert!(decorated.ends_with('>'));
        assert_eq!(decorated.matches('>').count(), 1);
    }

    #[test]
    fn strips_only_closed_frontmatter() {
        assert_eq!(prepare_source("---\ntitle: T\n---\n\n# Hi"), "# Hi");
        assert_eq!(
            prepare_source("---\nnot closed\n# Hi"),
            "---\nnot closed\n# Hi"
        );
    }

    #[test]
    fn leaves_define_like_text_in_code_fences() {
        let source = "```md\n:::define\nx: y\n:::\n```";
        assert_eq!(resolve_defines(source).unwrap(), source);
    }

    #[test]
    fn resolves_static_defines_outside_code() {
        let source = ":::define\nname: 'Ada'\n:::\n\nHello ${name}, `${name}`.";
        assert_eq!(resolve_defines(source).unwrap(), "Hello Ada, `${name}`.");
    }

    #[test]
    fn resolves_variables_defined_after_their_use() {
        let source = "Hello ${name}.\n\n:::define\nname: Ada\n:::\n\nGoodbye.";
        assert_eq!(resolve_defines(source).unwrap(), "Hello Ada.\n\nGoodbye.");
    }

    #[test]
    fn rejects_live_data_values_without_resolvers() {
        let source = ":::define\nname: api.v1.author.name\n:::\n\nHello ${name}.";
        assert!(matches!(
            resolve_defines(source),
            Err(RenderError::UnsupportedDefineValue { .. })
        ));
    }

    #[test]
    fn reports_undefined_variables_when_definitions_exist() {
        let source = ":::define\nname: Ada\n:::\n\nHello ${missing}.";
        assert!(matches!(
            resolve_defines(source),
            Err(RenderError::UndefinedVariable(name)) if name == "missing"
        ));
    }

    #[test]
    fn preserves_escaped_and_fenced_interpolation_tokens() {
        let source = ":::define\nname: Ada\n:::\n\n\\${name}\n\n```md\n${name}\n```";
        assert_eq!(
            resolve_defines(source).unwrap(),
            "\\${name}\n\n```md\n${name}\n```"
        );
    }
}
