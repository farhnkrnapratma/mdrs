use crate::Diagrams;

const PLACEHOLDER: &str = "<div class=\"mdrs-mermaid\"><div class=\"mdrs-mermaid-display\"><div class=\"mdrs-mermaid-loading\"><span class=\"mdrs-spinner\"></span><span>Rendering diagram…</span></div></div><div class=\"mdrs-mermaid-source\" hidden=\"\" data-mdrs-source=\"";

pub(crate) fn draw(html: &str, diagrams: Diagrams) -> String {
    if diagrams == Diagrams::Placeholder || !html.contains(PLACEHOLDER) {
        return html.to_owned();
    }
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;
    let mut count = 0;
    while let Some(start) = remaining.find(PLACEHOLDER) {
        output.push_str(&remaining[..start]);
        let source_start = start + PLACEHOLDER.len();
        let encoded_end = remaining[source_start..]
            .find('"')
            .map_or(remaining.len(), |end| source_start + end);
        count += 1;
        match svg(&decode(&remaining[source_start..encoded_end]), count) {
            Some(svg) => {
                output.push_str("<div class=\"mdrs-mermaid mdrs-mermaid--static\"><div class=\"mdrs-mermaid-display\">");
                output.push_str(&svg);
                output.push_str(
                    "</div><div class=\"mdrs-mermaid-source\" hidden=\"\" data-mdrs-source=\"",
                );
            }
            None => output.push_str(PLACEHOLDER),
        }
        remaining = &remaining[source_start..];
    }
    output.push_str(remaining);
    output
}

fn svg(source: &str, index: usize) -> Option<String> {
    let model = pelers::parse_with(source, &pelers::ParseOptions::default()).ok()?;
    let options = pelers::RenderOptions {
        id_prefix: Some(format!("mdrs-diagram-{index}")),
        ..pelers::RenderOptions::default()
    };
    pelers::render_with(&model, &options)
        .ok()
        .map(|rendered| rendered.svg)
        .filter(|svg| is_safe(svg))
}

const SVG_TAGS: [&str; 20] = [
    "svg",
    "g",
    "text",
    "tspan",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "title",
    "desc",
    "a",
    "image",
    "defs",
    "marker",
    "lineargradient",
    "stop",
    "clippath",
];

fn is_safe(svg: &str) -> bool {
    let mut rest = svg;
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else {
            return false;
        };
        let tag = attribute_text(&rest[open + 1..open + close]);
        let name = tag
            .trim_start_matches('/')
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or_default();
        if !SVG_TAGS.contains(&name) || has_event_handler(&tag) || has_unsafe_scheme(&tag) {
            return false;
        }
        rest = &rest[open + close + 1..];
    }
    true
}

fn attribute_text(tag: &str) -> String {
    let mut text = String::with_capacity(tag.len());
    let mut rest = tag;
    while let Some(start) = rest.find("&#") {
        text.push_str(&rest[..start]);
        let entity = &rest[start + 2..];
        let end = entity.find(';').unwrap_or(entity.len());
        let code = match entity[..end].strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok(),
            None => entity[..end].parse().ok(),
        };
        text.extend(code.and_then(char::from_u32));
        rest = &entity[(end + 1).min(entity.len())..];
    }
    text.push_str(rest);
    text.replace("&colon;", ":").to_ascii_lowercase()
}

fn has_event_handler(tag: &str) -> bool {
    tag.match_indices("on").any(|(at, _)| {
        let after = &tag[at + 2..];
        let name_end = after
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(after.len());
        tag[..at].ends_with(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '/')
            && name_end > 0
            && after[name_end..].trim_start().starts_with('=')
    })
}

fn has_unsafe_scheme(tag: &str) -> bool {
    let compact: String = tag
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect();
    ["javascript:", "vbscript:", "data:text", "data:image/svg"]
        .iter()
        .any(|scheme| compact.contains(scheme))
}

fn decode(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                index += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholder(source: &str) -> String {
        format!("{PLACEHOLDER}{source}\"></div></div>")
    }

    #[test]
    fn draws_each_diagram_with_its_own_ids() {
        let source =
            "flowchart%20LR%0A%20%20accTitle%3A%20Steps%0A%20%20A%5BStart%5D%20--%3E%20B%0A";
        let html = format!("<p>a</p>{}{}", placeholder(source), placeholder(source));
        let output = draw(&html, Diagrams::Pelers);
        assert_eq!(output.matches("mdrs-mermaid--static").count(), 2);
        assert!(output.contains("<svg"));
        assert!(output.contains("id=\"mdrs-diagram-1-title\""));
        assert!(output.contains("id=\"mdrs-diagram-2-title\""));
        assert!(output.contains(&format!("data-mdrs-source=\"{source}\"></div></div>")));
        assert!(!output.contains("Rendering diagram"));
    }

    #[test]
    fn keeps_the_placeholder_when_pelers_cannot_draw() {
        let html = placeholder("not%20a%20diagram");
        assert_eq!(draw(&html, Diagrams::Pelers), html);
        let unclosed = format!("{PLACEHOLDER}nope");
        assert_eq!(draw(&unclosed, Diagrams::Pelers), unclosed);
    }

    #[test]
    fn leaves_placeholders_when_asked() {
        let html = placeholder("flowchart%20LR%0AA--%3EB");
        assert_eq!(draw(&html, Diagrams::Placeholder), html);
        assert_eq!(draw("<p>x</p>", Diagrams::Pelers), "<p>x</p>");
    }

    #[test]
    fn rejects_svg_that_could_run_script() {
        assert!(is_safe(
            "<svg class=\"pelers\"><g><text>&lt;script&gt; onclick=x javascript:</text></g><a href=\"https://x\"><title>t</title></a></svg>"
        ));
        for svg in [
            "<svg><script>alert(1)</script></svg>",
            "<svg><foreignObject></foreignObject></svg>",
            "<svg><g onclick=\"x\"></g></svg>",
            "<svg><g\tONLOAD = 'x'></g></svg>",
            "<svg><g x=\"1\"/onload=x></g></svg>",
            "<svg><a href=\"&#106;ava&#x73;cript&colon;x\">y</a></svg>",
            "<svg><a href=\" java\tscript:x\">y</a></svg>",
            "<svg><image href=\"data:image/svg+xml;base64,x\"/></svg>",
            "<svg><g",
        ] {
            assert!(!is_safe(svg), "{svg}");
        }
        assert!(is_safe(
            "<svg><g data-x=\"&#;&#xZZ\" font=\"one onward\"></g></svg>"
        ));
    }

    #[test]
    fn draws_labels_that_only_look_like_script() {
        let source = "flowchart%20LR%0A%20%20A%5B%22javascript%3Aalert(1)%22%5D%20--%3E%20B%0A";
        let html = placeholder(source);
        let output = draw(&html, Diagrams::Pelers);
        assert!(output.contains("mdrs-mermaid--static"));
        assert!(output.contains("javascript:alert(1)"));
    }

    #[test]
    fn decodes_percent_escapes_leniently() {
        assert_eq!(decode("a%20b%E2%9C%93"), "a b\u{2713}");
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz%4"), "%zz%4");
    }
}
