use crate::markdown::{ResolvedOptions, fence_marker, render_markdown_fragment};
use crate::sanitize::escape_html;

const MAX_CONTAINER_DEPTH: usize = 16;

pub(crate) fn render(markdown: &str, options: &ResolvedOptions) -> String {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut index = 0;
    render_lines(&lines, &mut index, false, 0, options, markdown.len()).0
}

fn render_lines(
    lines: &[&str],
    index: &mut usize,
    nested: bool,
    depth: usize,
    options: &ResolvedOptions,
    capacity: usize,
) -> (String, bool) {
    let mut output = String::with_capacity(capacity);
    let mut pending = String::with_capacity(capacity);
    let mut fence: Option<(char, usize)> = None;

    while *index < lines.len() {
        let line = lines[*index];
        if let Some((marker, minimum)) = fence {
            pending.push_str(line);
            pending.push('\n');
            if let Some((closing, count)) = fence_marker(line)
                && closing == marker
                && count >= minimum
            {
                fence = None;
            }
            *index += 1;
            continue;
        }

        if let Some(opening) = fence_marker(line) {
            fence = Some(opening);
            pending.push_str(line);
            pending.push('\n');
            *index += 1;
            continue;
        }

        if nested && line.trim() == ":::" {
            flush_pending(&mut output, &mut pending, options);
            *index += 1;
            return (output, true);
        }

        if depth < MAX_CONTAINER_DEPTH
            && let Some((name, attributes)) = parse_opening(line)
            && is_supported(&name)
        {
            flush_pending(&mut output, &mut pending, options);
            *index += 1;
            if matches!(name.as_str(), "tabs" | "snippets") {
                let (tabs, closed) = render_tabs(lines, index, &name, &attributes, options);
                if closed {
                    output.push_str(&tabs);
                    output.push('\n');
                } else {
                    output.push_str(line);
                    output.push('\n');
                    output.push_str(&tabs);
                }
                continue;
            }
            let (body, closed) = render_lines(lines, index, true, depth + 1, options, 0);
            if closed {
                output.push_str(&render_container(&name, &attributes, &body));
                output.push('\n');
            } else {
                output.push_str(line);
                output.push('\n');
                output.push_str(&body);
            }
            continue;
        }

        pending.push_str(line);
        pending.push('\n');
        *index += 1;
    }

    flush_pending(&mut output, &mut pending, options);
    (output, false)
}

fn flush_pending(output: &mut String, pending: &mut String, options: &ResolvedOptions) {
    if !pending.is_empty() {
        output.push_str(&render_markdown_fragment(pending, options));
        pending.clear();
    }
}

fn parse_opening(line: &str) -> Option<(String, Vec<(String, String)>)> {
    let trimmed = line.trim();
    let directive = trimmed.strip_prefix(":::")?;
    let name_end = directive
        .find(|character: char| character.is_ascii_whitespace())
        .unwrap_or(directive.len());
    let name = directive[..name_end].to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    let attributes = parse_attributes(&directive[name_end..]);
    Some((name, attributes))
}

fn parse_attributes(source: &str) -> Vec<(String, String)> {
    let mut attributes = Vec::new();
    let mut chars = source.trim().chars().peekable();

    while chars.peek().is_some() {
        while chars
            .peek()
            .is_some_and(|character| character.is_ascii_whitespace())
        {
            chars.next();
        }
        let mut key = String::new();
        while chars
            .peek()
            .is_some_and(|character| !character.is_ascii_whitespace() && *character != '=')
        {
            key.push(chars.next().unwrap_or_default());
        }
        if key.is_empty() {
            chars.next();
            continue;
        }
        while chars
            .peek()
            .is_some_and(|character| character.is_ascii_whitespace())
        {
            chars.next();
        }
        let mut value = String::new();
        if chars.peek() == Some(&'=') {
            chars.next();
            while chars
                .peek()
                .is_some_and(|character| character.is_ascii_whitespace())
            {
                chars.next();
            }
            if let Some(quote @ ('"' | '\'')) = chars.peek().copied() {
                chars.next();
                for character in chars.by_ref() {
                    if character == quote {
                        break;
                    }
                    value.push(character);
                }
            } else {
                while chars
                    .peek()
                    .is_some_and(|character| !character.is_ascii_whitespace())
                {
                    value.push(chars.next().unwrap_or_default());
                }
            }
        }
        attributes.push((key, value));
    }

    attributes
}

fn attribute<'a>(attributes: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn is_supported(name: &str) -> bool {
    matches!(
        name,
        "note"
            | "tip"
            | "important"
            | "warning"
            | "caution"
            | "details"
            | "steps"
            | "callout"
            | "tabs"
            | "snippets"
    )
}

fn render_tabs(
    lines: &[&str],
    index: &mut usize,
    name: &str,
    attributes: &[(String, String)],
    options: &ResolvedOptions,
) -> (String, bool) {
    let mut tabs = Vec::<(Vec<(String, String)>, String)>::new();
    let mut current: Option<(Vec<(String, String)>, String)> = None;
    let mut fence: Option<(char, usize)> = None;

    while *index < lines.len() {
        let line = lines[*index];
        *index += 1;

        if let Some((marker, minimum)) = fence {
            if let Some((closing, count)) = fence_marker(line)
                && closing == marker
                && count >= minimum
            {
                fence = None;
            }
            if let Some((_, content)) = current.as_mut() {
                content.push_str(line);
                content.push('\n');
            }
            continue;
        }

        if let Some(opening) = fence_marker(line) {
            fence = Some(opening);
            if let Some((_, content)) = current.as_mut() {
                content.push_str(line);
                content.push('\n');
            }
            continue;
        }

        if line.trim() == ":::" {
            if let Some(tab) = current.take() {
                tabs.push(tab);
            }
            return (format_tabs(name, attributes, &tabs, options), true);
        }

        if let Some(source) = line.trim().strip_prefix("::tab")
            && source
                .chars()
                .next()
                .is_none_or(|character| character.is_ascii_whitespace())
        {
            if let Some(tab) = current.take() {
                tabs.push(tab);
            }
            current = Some((parse_attributes(source), String::new()));
            continue;
        }

        if let Some((_, content)) = current.as_mut() {
            content.push_str(line);
            content.push('\n');
        }
    }

    if let Some(tab) = current {
        tabs.push(tab);
    }
    (format_tabs(name, attributes, &tabs, options), false)
}

fn format_tabs(
    name: &str,
    attributes: &[(String, String)],
    tabs: &[(Vec<(String, String)>, String)],
    options: &ResolvedOptions,
) -> String {
    if tabs.is_empty()
        || tabs
            .iter()
            .any(|(attrs, _)| attribute(attrs, "label").is_none())
    {
        return "<span class=\"mdrs-define-error\">Tabs require at least one ::tab with a label.</span>"
            .to_owned();
    }

    let block = if name == "snippets" {
        "mdrs-snippets"
    } else {
        "mdrs-tabs"
    };
    let key = attribute(attributes, "key")
        .map(|value| format!(" data-mdrs-tab-key=\"{}\"", escape_html(value)))
        .unwrap_or_default();
    let buttons = tabs
        .iter()
        .enumerate()
        .map(|(index, (attrs, _))| {
            let selected = index == 0;
            let label = escape_html(attribute(attrs, "label").unwrap_or_default());
            let icon = attribute(attrs, "icon").map(icon_markup).unwrap_or_default();
            format!(
                "<button type=\"button\" role=\"tab\" class=\"mdrs-tab{}\" data-mdrs-tab-label=\"{label}\" aria-selected=\"{}\">{icon}{label}</button>",
                if selected { " mdrs-is-active" } else { "" },
                if selected { "true" } else { "false" }
            )
        })
        .collect::<String>();
    let panels = tabs
        .iter()
        .enumerate()
        .map(|(index, (_, content))| {
            format!(
                "<div role=\"tabpanel\" class=\"mdrs-tab-panel\"{}>\n\n{}\n\n</div>",
                if index == 0 { "" } else { " hidden" },
                render_markdown_fragment(content.trim(), options)
            )
        })
        .collect::<String>();
    let actions = if name == "snippets" {
        format!("<div class=\"{block}-actions\"></div>")
    } else {
        String::new()
    };
    format!(
        "<div class=\"{block}\"{key}>\n<div class=\"{block}-header\" role=\"tablist\"><div class=\"{block}-nav\">{buttons}</div>{actions}</div>\n<div class=\"{block}-body\">{panels}</div>\n</div>"
    )
}

fn render_container(name: &str, attributes: &[(String, String)], body: &str) -> String {
    match name {
        "details" => {
            let summary = attribute(attributes, "summary").unwrap_or("Details");
            format!(
                "<details>\n<summary>{}</summary>\n\n{}\n\n</details>",
                escape_html(summary),
                body.trim()
            )
        }
        "steps" => format!("<div class=\"mdrs-steps\">\n\n{}\n\n</div>", body.trim()),
        "callout" => {
            let title = attribute(attributes, "title").unwrap_or("Callout");
            let icon = attribute(attributes, "icon")
                .map(icon_markup)
                .unwrap_or_default();
            format!(
                "<div class=\"mdrs-callout\">\n  <div class=\"mdrs-callout-title\">\n    {}<span>{}</span>\n  </div>\n  <div class=\"mdrs-callout-content\">\n\n{}\n\n</div>\n</div>",
                icon,
                escape_html(title),
                body.trim()
            )
        }
        kind => {
            let title = match kind {
                "note" => "Note",
                "tip" => "Tip",
                "important" => "Important",
                "warning" => "Warning",
                _ => "Caution",
            };
            format!(
                "<div class=\"mdrs-alert mdrs-alert--{kind}\">\n  <div class=\"mdrs-alert-title\">\n    {icon}\n    <span>{title}</span>\n  </div>\n  <div class=\"mdrs-alert-content\">\n\n{}\n\n</div>\n</div>",
                body.trim(),
                icon = note_icon(kind)
            )
        }
    }
}

fn note_icon(kind: &str) -> &'static str {
    match kind {
        "note" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mdrs-icon mdrs-icon-info"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>"#
        }
        "tip" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mdrs-icon mdrs-icon-lightbulb"><path d="M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5"/><path d="M9 18h6"/><path d="M10 22h4"/></svg>"#
        }
        "important" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mdrs-icon mdrs-icon-message-square-warning"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/><path d="M12 7v2"/><path d="M12 13h.01"/></svg>"#
        }
        "warning" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mdrs-icon mdrs-icon-alert-triangle"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" x2="12" y1="9" y2="13"/><line x1="12" x2="12.01" y1="17" y2="17"/></svg>"#
        }
        _ => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mdrs-icon mdrs-icon-circle-alert"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>"#
        }
    }
}

fn icon_markup(name: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return String::new();
    }
    format!("<span class=\"mdrs-icon mdrs-icon-{name}\" aria-hidden=\"true\"></span>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RenderOptions;

    fn default_opts() -> ResolvedOptions {
        ResolvedOptions::new(&RenderOptions::default())
    }

    #[test]
    fn renders_note_and_details_directives() {
        let source = ":::note\nA **note**.\n:::\n\n:::details summary=\"More\"\nHidden text.\n:::";
        let output = render(source, &default_opts());
        assert!(output.contains("mdrs-alert--note"));
        assert!(output.contains("<strong>note</strong>"));
        assert!(output.contains("<summary>More</summary>"));
        assert!(output.contains("Hidden text."));
    }

    #[test]
    fn does_not_parse_directives_inside_fenced_code() {
        let source = "```md\n:::note\ntext\n:::\n```";
        let output = render(source, &default_opts());
        assert!(output.contains(":::note"));
        assert!(!output.contains("mdrs-alert"));
    }

    #[test]
    fn escapes_directive_attributes() {
        let output = render(
            ":::details summary=\"<script>\"\ncontent\n:::",
            &default_opts(),
        );
        assert!(output.contains("&lt;script&gt;"));
        assert!(!output.contains("<summary><script>"));
    }

    #[test]
    fn renders_labeled_tabs_and_preserves_tab_code() {
        let output = render(
            ":::tabs\n::tab label=\"Rust\"\n`let x = 1;`\n::tab label=\"JS\"\nconst x = 1;\n:::",
            &default_opts(),
        );
        assert!(output.contains("mdrs-tabs"));
        assert!(output.contains("data-mdrs-tab-label=\"Rust\""));
        assert!(output.contains("<code>let x = 1;</code>"));
        assert!(output.contains("const x = 1;"));
    }
}
