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
    fn decodes_percent_escapes_leniently() {
        assert_eq!(decode("a%20b%E2%9C%93"), "a b\u{2713}");
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz%4"), "%zz%4");
    }
}
