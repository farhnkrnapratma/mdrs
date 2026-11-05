//! MDRS (Markdown Rich Syntax): a drop-in superset of Markdown.
//!
//! MDRS (Markdown Rich Syntax) rendering.

use std::path::Path;

mod containers;
mod define;
mod markdown;
mod sanitize;

#[cfg(any(feature = "napi", feature = "wasm"))]
mod bindings;

mod headings;
/// The file extension used by MDRS documents, without the leading dot.
pub const FILE_EXTENSION: &str = "mdrs";

/// Maximum Markdown document size accepted by the renderer.
pub const MAX_DOCUMENT_LENGTH: usize = 500_000;

/// Maximum UTF-16 length of expanded definitions or rendered HTML.
pub const MAX_RENDERED_OUTPUT_LENGTH: usize = 2_000_000;

/// Rendering options shared by the public Rust API.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderOptions {
    /// URLs whose **origin** (`scheme://host:port`) matches an entry in this
    /// list are treated as internal links and will not receive the
    /// `target="_blank"` or `rel="noopener noreferrer"` attributes.
    ///
    /// Matching is origin-level only: the path component of each entry is
    /// ignored. `"https://example.com/docs"` and `"https://example.com/other"`
    /// both match any link whose origin is `https://example.com`.
    pub internal_origins: Vec<String>,
}

/// A rendered heading exposed to document navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
pub struct HeadingItem {
    pub id: String,
    pub text: String,
    pub level: u8,
}

/// Rendered HTML and its navigable headings.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
pub struct PreparedMarkdownPage {
    pub html: String,
    pub headings: Vec<HeadingItem>,
}

/// An error returned while processing an MDRS document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    DocumentTooLarge { actual: usize, maximum: usize },
    UnclosedDefineBlock,
    UnsupportedDefineValue { name: String, expression: String },
    UndefinedVariable(String),
    InvalidData(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DocumentTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "Markdown document is too large ({actual} > {maximum})."
                )
            }
            Self::UnclosedDefineBlock => formatter.write_str("The :::define block is not closed."),
            Self::UnsupportedDefineValue { name, expression } => write!(
                formatter,
                "The value for '{name}' requires a live-data resolver or expression evaluator: {expression}"
            ),
            Self::UndefinedVariable(name) => {
                write!(formatter, "Undefined MDRS variable '{name}'.")
            }
            Self::InvalidData(message) => write!(formatter, "Invalid MDRS data: {message}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Returns `true` when `path` ends in the `.mdrs` extension (case-insensitive).
pub fn is_mdrs_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(FILE_EXTENSION))
}

/// Renders Markdown to sanitized HTML using the default options.
pub fn render_markdown(markdown: &str) -> Result<String, RenderError> {
    render_markdown_with_options(markdown, &RenderOptions::default())
}

/// Renders Markdown to sanitized HTML.
pub fn render_markdown_with_options(
    markdown: &str,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    check_document_size(markdown)?;
    let resolved = markdown::ResolvedOptions::new(options);
    let source = markdown::resolve_defines(&markdown::prepare_source(markdown))?;
    check_rendered_output_size(&source)?;
    let html = markdown::render(&source, &resolved);
    check_rendered_output_size(&html)?;
    let html = markdown::restore_extensions(&html)?;
    check_rendered_output_size(&html)?;
    let sanitized = markdown::mark_references(&sanitize::sanitize_html(&html));
    Ok(markdown::add_external_link_attributes(
        &sanitized, &resolved,
    ))
}

/// Renders Markdown with JSON data available to MDRS definitions and expressions.
///
/// The data object is exposed to expressions through its properties, including
/// `api.v1.*` paths. Existing callers should use `render_markdown_with_options`
/// when no data is needed.
pub fn render_markdown_with_data(
    markdown: &str,
    data: &serde_json::Value,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    check_document_size(markdown)?;
    let resolved = markdown::ResolvedOptions::new(options);
    let (source, scope) = define::resolve_with_scope(&markdown::prepare_source(markdown), data)?;
    let source = markdown::resolve_conditionals(&source, &scope, data)?;
    check_rendered_output_size(&source)?;
    let html = markdown::render(&source, &resolved);
    check_rendered_output_size(&html)?;
    let html = markdown::restore_extensions(&html)?;
    check_rendered_output_size(&html)?;
    let sanitized = markdown::mark_references(&sanitize::sanitize_html(&html));
    Ok(markdown::add_external_link_attributes(
        &sanitized, &resolved,
    ))
}

/// Renders a page, assigning MDRS-prefixed heading IDs and resolving local hash links.
pub fn prepare_markdown_page(
    markdown: &str,
    options: &RenderOptions,
) -> Result<PreparedMarkdownPage, RenderError> {
    let html = render_markdown_with_options(markdown, options)?;
    let html = headings::inject_heading_ids(&html);
    let html = headings::prefix_hash_links(&html);
    let headings = headings::extract_headings(&html);
    Ok(PreparedMarkdownPage { html, headings })
}

/// Renders a page with JSON-backed definitions and heading navigation.
pub fn prepare_markdown_page_with_data(
    markdown: &str,
    data: &serde_json::Value,
    options: &RenderOptions,
) -> Result<PreparedMarkdownPage, RenderError> {
    let html = render_markdown_with_data(markdown, data, options)?;
    let html = headings::inject_heading_ids(&html);
    let html = headings::prefix_hash_links(&html);
    let headings = headings::extract_headings(&html);
    Ok(PreparedMarkdownPage { html, headings })
}

/// Resolves simple scalar variables in a `:::define` block without external data.
pub fn resolve_defines(markdown: &str) -> Result<String, RenderError> {
    check_document_size(markdown)?;
    let resolved = markdown::resolve_defines(&markdown::prepare_source(markdown))?;
    check_rendered_output_size(&resolved)?;
    Ok(resolved)
}

/// Resolves definitions and expressions against a JSON data object.
pub fn resolve_defines_with_data(
    markdown: &str,
    data: &serde_json::Value,
) -> Result<String, RenderError> {
    check_document_size(markdown)?;
    let resolved = define::resolve(&markdown::prepare_source(markdown), data)?;
    check_rendered_output_size(&resolved)?;
    Ok(resolved)
}

fn check_document_size(markdown: &str) -> Result<(), RenderError> {
    let actual = markdown.encode_utf16().count();
    if actual > MAX_DOCUMENT_LENGTH {
        return Err(RenderError::DocumentTooLarge {
            actual,
            maximum: MAX_DOCUMENT_LENGTH,
        });
    }
    Ok(())
}

pub(crate) fn check_rendered_output_size(output: &str) -> Result<(), RenderError> {
    let actual = output.encode_utf16().count();
    if actual > MAX_RENDERED_OUTPUT_LENGTH {
        return Err(RenderError::DocumentTooLarge {
            actual,
            maximum: MAX_RENDERED_OUTPUT_LENGTH,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_mdrs_paths() {
        assert!(is_mdrs_path(Path::new("docs/intro.mdrs")));
        assert!(is_mdrs_path(Path::new("INTRO.MDRS")));
        assert!(!is_mdrs_path(Path::new("intro.md")));
        assert!(!is_mdrs_path(Path::new("mdrs")));
    }

    #[test]
    fn renders_and_sanitizes_markdown() {
        let html = render_markdown("# Hello\n\nA **rich** document.").unwrap();
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>rich</strong>"));
    }

    #[test]
    fn removes_frontmatter_and_define_blocks_before_rendering() {
        let source = "---\ntitle: Example\n---\n:::define\nname: Ada\n:::\n\nHello.";
        let html = render_markdown(source).unwrap();
        assert!(!html.contains("title: Example"));
        assert!(!html.contains("name: Ada"));
        assert!(html.contains("Hello."));
    }

    #[test]
    fn rejects_documents_over_the_size_limit() {
        let source = "x".repeat(MAX_DOCUMENT_LENGTH + 1);
        assert!(matches!(
            render_markdown(&source),
            Err(RenderError::DocumentTooLarge { .. })
        ));
    }

    #[test]
    fn rejects_expanded_define_output_over_the_size_limit() {
        let source = ":::define\nvalue: api.value\n:::\n\n${value}";
        let data =
            serde_json::json!({ "api": { "value": "x".repeat(MAX_RENDERED_OUTPUT_LENGTH + 1) } });
        let result = render_markdown_with_data(source, &data, &RenderOptions::default());
        assert!(
            matches!(result, Err(RenderError::DocumentTooLarge { maximum, .. }) if maximum == MAX_RENDERED_OUTPUT_LENGTH),
            "{result:?}"
        );
    }

    #[test]
    fn rejects_katex_output_over_the_size_limit() {
        let source = "$x^2$ ".repeat(10_000);
        assert!(matches!(
            render_markdown(&source),
            Err(RenderError::DocumentTooLarge { maximum, .. }) if maximum == MAX_RENDERED_OUTPUT_LENGTH
        ));
    }

    #[test]
    fn renders_data_backed_defines_expressions_and_conditionals() {
        let source = r#":::define
site: "MDRS"
features: ["math", "charts"]
greeting: "Welcome to " + this.site
stable: true
:::

${greeting}: ${features.length} features.
${if stable}stable${else}preview${endif}.
${features[1]}.
"#;
        let html =
            render_markdown_with_data(source, &serde_json::json!({}), &RenderOptions::default())
                .unwrap();
        assert!(html.contains("Welcome to MDRS: 2 features."));
        assert!(html.contains("stable."));
        assert!(html.contains("charts."));
    }

    #[test]
    fn resolves_api_data_and_renders_browser_extension_markers() {
        let source = r#":::define
author: api.v1.author.profile.name
posts: api.v1.blog.length
:::

${author} has ${posts} posts.

:::math
x^2 + 1
:::

```mermaid
flowchart LR
A --> B
```

```chart
type: bar
```

:::embed url="https://example.com/video"

See the source[^spec].

:::references
[^spec]: The MDRS reference.
:::
"#;
        let data = serde_json::json!({
            "api": {
                "v1": {
                    "author": {"profile": {"name": "Ada"}},
                    "blog": [{"title": "One"}, {"title": "Two"}]
                }
            }
        });
        let html = render_markdown_with_data(source, &data, &RenderOptions::default()).unwrap();
        assert!(html.contains("Ada has 2 posts."));
        assert!(html.contains("mdrs-math--block"));
        assert!(html.contains("mdrs-mermaid"));
        assert!(html.contains("mdrs-chart"));
        assert!(html.contains("mdrs-embed"));
        assert!(html.contains("mdrs-references"));
        assert!(!html.contains("REFERENCESPLACEMENT"));
    }

    #[test]
    fn renders_extended_tasks_icons_and_verbatim_source() {
        let source = r#"- [*] In progress
- [/] Failed

:::callout title="Build" icon="rocket"
Keep this source visible: ${not_interpolated}
:::

::::verbatim
:::tip
${literal}
:::
::::
"#;
        let html = render_markdown(source).unwrap();
        assert!(html.contains("mdrs-task--in-progress"));
        assert!(html.contains("mdrs-task--failed"));
        assert!(html.contains("mdrs-task-content"));
        assert!(html.contains("mdrs-icon-rocket"));
        assert!(html.contains("mdrs-verbatim"));
        assert!(html.contains("${literal}"));
    }

    #[test]
    fn renders_checked_and_nested_task_states() {
        let html = render_markdown("- [x] Done\n- [ ] Todo\n  - [*] Active\n").unwrap();
        assert!(html.contains("mdrs-task--done"));
        assert!(html.contains("mdrs-task--todo"));
        assert!(html.contains("mdrs-task--in-progress"));
        assert!(html.contains("<svg"));
        assert!(html.contains("mdrs-task-content"));
    }

    #[test]
    fn preserves_diagram_sources_for_browser_renderers() {
        let html =
            render_markdown("```mermaid\nflowchart LR\nA --> B\n```\n\n```chart\ntype: bar\n```")
                .unwrap();
        assert!(html.contains("mdrs-mermaid-display"));
        assert!(
            html.contains("data-mdrs-source=\"flowchart%20LR%0AA%20--%3E%20B%0A\""),
            "{html}"
        );
        assert!(html.contains("data-mdrs-chart=\"type%3A%20bar%0A\""));
        assert!(html.contains("mdrs-chart-loading"));
    }

    #[test]
    fn renders_inline_and_block_math_with_katex() {
        let html = render_markdown("Inline $$x^2$$.\n\n:::math\n\\frac{1}{2}\n:::").unwrap();
        assert!(html.contains("mdrs-math mdrs-math--block"));
        assert!(html.contains("class=\"katex\""));
        assert!(html.contains("katex-display"));
        assert!(!html.contains("class=\"math math-display\""));
    }

    #[test]
    fn strips_script_content_from_raw_html() {
        let html = render_markdown("<script>alert(1)</script><p>safe</p>").unwrap();
        assert!(!html.contains("alert(1)"));
        assert!(html.contains("safe"));
    }

    #[test]
    fn preserves_generated_mdrs_container_markup() {
        let html = render_markdown(":::note\nA note.\n:::").unwrap();
        assert!(html.contains("mdrs-alert--note"));
        assert!(html.contains("A note."));
    }

    #[test]
    fn marks_external_links_but_not_internal_origins() {
        let options = RenderOptions {
            internal_origins: vec!["https://internal.example/docs".to_owned()],
        };
        let html = render_markdown_with_options(
            "[external](https://outside.example/page) [internal](https://internal.example/page)",
            &options,
        )
        .unwrap();
        assert!(html.contains("target=\"_blank\""), "{html}");
        assert!(html.contains("mdrs-external-link"), "{html}");
        assert_eq!(html.matches("target=\"_blank\"").count(), 1);
    }

    #[test]
    fn prepares_heading_navigation_and_local_links() {
        let page = prepare_markdown_page(
            "# Title\n\n## Getting Started\n\n[Jump](#getting-started)",
            &RenderOptions::default(),
        )
        .unwrap();
        assert!(page.html.contains("id=\"mdrs-title\""));
        assert!(page.html.contains("id=\"mdrs-getting-started\""));
        assert!(page.html.contains("href=\"#mdrs-getting-started\""));
        assert_eq!(page.headings.len(), 1);
        assert_eq!(page.headings[0].text, "Getting Started");
    }

    #[test]
    fn describes_every_error() {
        assert_eq!(
            RenderError::DocumentTooLarge {
                actual: 2,
                maximum: 1
            }
            .to_string(),
            "Markdown document is too large (2 > 1)."
        );
        assert_eq!(
            RenderError::UnclosedDefineBlock.to_string(),
            "The :::define block is not closed."
        );
        assert_eq!(
            RenderError::UnsupportedDefineValue {
                name: "a".to_owned(),
                expression: "b".to_owned()
            }
            .to_string(),
            "The value for 'a' requires a live-data resolver or expression evaluator: b"
        );
        assert_eq!(
            RenderError::UndefinedVariable("v".to_owned()).to_string(),
            "Undefined MDRS variable 'v'."
        );
        assert_eq!(
            RenderError::InvalidData("bad".to_owned()).to_string(),
            "Invalid MDRS data: bad"
        );
    }

    #[test]
    fn resolves_definitions_without_and_with_data() {
        let source = ":::define\nname: 'MDRS'\ncount: api.n\n:::\n\n${name} ${count}";
        assert_eq!(resolve_defines(source).unwrap(), "MDRS api.n");
        assert!(resolve_defines(":::define\nn: api.v1.x\n:::\n\n${n}").is_err());
        assert_eq!(
            resolve_defines(":::define\nname: 'MDRS'\n:::\n\nhi ${name}").unwrap(),
            "hi MDRS"
        );
        let data = serde_json::json!({"api": {"n": 3}});
        assert_eq!(resolve_defines_with_data(source, &data).unwrap(), "MDRS 3");
        let oversized = "x".repeat(MAX_DOCUMENT_LENGTH + 1);
        assert!(resolve_defines(&oversized).is_err());
        assert!(resolve_defines_with_data(&oversized, &data).is_err());
    }

    #[test]
    fn prepares_pages_with_data() {
        let data = serde_json::json!({"api": {"title": "Data page"}});
        let page = prepare_markdown_page_with_data(
            ":::define\nt: api.title\n:::\n\n## ${t}\n\n[go](#data-page)",
            &data,
            &RenderOptions::default(),
        )
        .unwrap();
        assert!(page.html.contains("id=\"mdrs-data-page\""));
        assert!(page.html.contains("href=\"#mdrs-data-page\""));
        assert_eq!(page.headings[0].text, "Data page");
    }
}
