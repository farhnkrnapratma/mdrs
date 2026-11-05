use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{
    HeadingItem, PreparedMarkdownPage, RenderOptions, prepare_markdown_page,
    render_markdown_with_options,
};

#[derive(Default)]
#[cfg_attr(test, allow(dead_code))]
#[napi(object)]
pub struct BindingRenderOptions {
    pub internal_origins: Option<Vec<String>>,
}

#[derive(Clone)]
#[cfg_attr(test, allow(dead_code))]
#[napi(object)]
pub struct BindingHeadingItem {
    pub id: String,
    pub text: String,
    pub level: u8,
}

#[derive(Clone)]
#[cfg_attr(test, allow(dead_code))]
#[napi(object)]
pub struct BindingPreparedMarkdownPage {
    pub html: String,
    pub headings: Vec<BindingHeadingItem>,
}

impl From<HeadingItem> for BindingHeadingItem {
    fn from(heading: HeadingItem) -> Self {
        Self {
            id: heading.id,
            text: heading.text,
            level: heading.level,
        }
    }
}

impl From<PreparedMarkdownPage> for BindingPreparedMarkdownPage {
    fn from(page: PreparedMarkdownPage) -> Self {
        Self {
            html: page.html,
            headings: page.headings.into_iter().map(Into::into).collect(),
        }
    }
}

#[napi(js_name = "renderMarkdown")]
#[cfg_attr(test, allow(dead_code))]
pub fn render(markdown: String, options: Option<BindingRenderOptions>) -> Result<String> {
    let options = options.unwrap_or_default();
    let render_options = RenderOptions {
        internal_origins: options.internal_origins.unwrap_or_default(),
    };
    render_markdown_with_options(&markdown, &render_options)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi(js_name = "renderMarkdownWithData")]
#[cfg_attr(test, allow(dead_code))]
pub fn render_with_data(
    markdown: String,
    data_json: String,
    options: Option<BindingRenderOptions>,
) -> Result<String> {
    let options = options.unwrap_or_default();
    let render_options = RenderOptions {
        internal_origins: options.internal_origins.unwrap_or_default(),
    };
    let data = serde_json::from_str(&data_json)
        .map_err(|error| Error::from_reason(format!("Invalid MDRS data: {error}")))?;
    crate::render_markdown_with_data(&markdown, &data, &render_options)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi(js_name = "isMdrsPath")]
#[cfg_attr(test, allow(dead_code))]
pub fn is_path(path: String) -> bool {
    crate::is_mdrs_path(std::path::Path::new(&path))
}

#[napi(js_name = "prepareMarkdownPage")]
#[cfg_attr(test, allow(dead_code))]
pub fn prepare_page(
    markdown: String,
    options: Option<BindingRenderOptions>,
) -> Result<BindingPreparedMarkdownPage> {
    let options = options.unwrap_or_default();
    let render_options = RenderOptions {
        internal_origins: options.internal_origins.unwrap_or_default(),
    };
    prepare_markdown_page(&markdown, &render_options)
        .map(Into::into)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi(js_name = "prepareMarkdownPageWithData")]
#[cfg_attr(test, allow(dead_code))]
pub fn prepare_page_with_data(
    markdown: String,
    data_json: String,
    options: Option<BindingRenderOptions>,
) -> Result<BindingPreparedMarkdownPage> {
    let options = options.unwrap_or_default();
    let render_options = RenderOptions {
        internal_origins: options.internal_origins.unwrap_or_default(),
    };
    let data = serde_json::from_str(&data_json)
        .map_err(|error| Error::from_reason(format!("Invalid MDRS data: {error}")))?;
    crate::prepare_markdown_page_with_data(&markdown, &data, &render_options)
        .map(Into::into)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi(js_name = "resolveDefines")]
#[cfg_attr(test, allow(dead_code))]
pub fn resolve(markdown: String) -> Result<String> {
    crate::resolve_defines(&markdown).map_err(|error| Error::from_reason(error.to_string()))
}

#[napi(js_name = "resolveDefinesWithData")]
#[cfg_attr(test, allow(dead_code))]
pub fn resolve_with_data(markdown: String, data_json: String) -> Result<String> {
    let data = serde_json::from_str(&data_json)
        .map_err(|error| Error::from_reason(format!("Invalid MDRS data: {error}")))?;
    crate::resolve_defines_with_data(&markdown, &data)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_binding_api_smoke() {
        assert!(is_path("README.mdrs".to_owned()));
        assert!(!is_path("README.md".to_owned()));

        let html = render("# Hello".to_owned(), None).expect("render should succeed");
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello"));

        let page = prepare_page("# Title\n\n## Hello".to_owned(), None)
            .expect("page preparation should succeed");
        assert_eq!(page.headings.len(), 1);
        assert_eq!(page.headings[0].text, "Hello");

        let data = r#"{"api":{"v1":{"author":{"profile":{"name":"Ada"}}}}}"#;
        let source = ":::define\ngreeting: api.v1.author.profile.name\n:::\n\n${greeting}";
        let html = render_with_data(source.to_owned(), data.to_owned(), None).expect("data render");
        assert!(html.contains("Ada"));

        let page = prepare_page_with_data(source.to_owned(), data.to_owned(), None)
            .expect("data page preparation");
        assert!(page.html.contains("Ada"));

        let resolved = resolve(":::define\nname: Ada\n:::\n\n${name}".to_owned())
            .expect("define resolution should succeed");
        assert!(resolved.contains("Ada"));
        let resolved = resolve_with_data(source.to_owned(), data.to_owned())
            .expect("data define resolution should succeed");
        assert!(resolved.contains("Ada"));
    }
}
