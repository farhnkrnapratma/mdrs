use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{RenderOptions, render_markdown_with_options};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingRenderOptions {
    pub internal_origins: Option<Vec<String>>,
}

#[wasm_bindgen(js_name = renderMarkdown)]
pub fn render(markdown: &str, options: JsValue) -> Result<String, JsError> {
    let options: BindingRenderOptions = if options.is_undefined() || options.is_null() {
        BindingRenderOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|error| JsError::new(&error.to_string()))?
    };
    let render_options = RenderOptions {
        internal_origins: options.internal_origins.unwrap_or_default(),
    };
    render_markdown_with_options(markdown, &render_options)
        .map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen(js_name = renderMarkdownWithData)]
pub fn render_with_data(
    markdown: &str,
    data_json: &str,
    options: JsValue,
) -> Result<String, JsError> {
    let options: BindingRenderOptions = if options.is_undefined() || options.is_null() {
        BindingRenderOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|error| JsError::new(&error.to_string()))?
    };
    let render_options = RenderOptions {
        internal_origins: options.internal_origins.unwrap_or_default(),
    };
    let data: serde_json::Value =
        serde_json::from_str(data_json).map_err(|error| JsError::new(&error.to_string()))?;
    crate::render_markdown_with_data(markdown, &data, &render_options)
        .map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen(js_name = isMdrsPath)]
pub fn is_path(path: &str) -> bool {
    crate::is_mdrs_path(std::path::Path::new(path))
}

#[wasm_bindgen(js_name = prepareMarkdownPage)]
pub fn prepare_page(markdown: &str, options: JsValue) -> Result<JsValue, JsError> {
    let options: BindingRenderOptions = if options.is_undefined() || options.is_null() {
        BindingRenderOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|error| JsError::new(&error.to_string()))?
    };
    let render_options = RenderOptions {
        internal_origins: options.internal_origins.unwrap_or_default(),
    };
    let page = crate::prepare_markdown_page(markdown, &render_options)
        .map_err(|error| JsError::new(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&page).map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen(js_name = prepareMarkdownPageWithData)]
pub fn prepare_page_with_data(
    markdown: &str,
    data_json: &str,
    options: JsValue,
) -> Result<JsValue, JsError> {
    let options: BindingRenderOptions = if options.is_undefined() || options.is_null() {
        BindingRenderOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|error| JsError::new(&error.to_string()))?
    };
    let render_options = RenderOptions {
        internal_origins: options.internal_origins.unwrap_or_default(),
    };
    let data: serde_json::Value =
        serde_json::from_str(data_json).map_err(|error| JsError::new(&error.to_string()))?;
    let page = crate::prepare_markdown_page_with_data(markdown, &data, &render_options)
        .map_err(|error| JsError::new(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&page).map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen(js_name = resolveDefines)]
pub fn resolve(markdown: &str) -> Result<String, JsError> {
    crate::resolve_defines(markdown).map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen(js_name = resolveDefinesWithData)]
pub fn resolve_with_data(markdown: &str, data_json: &str) -> Result<String, JsError> {
    let data: serde_json::Value =
        serde_json::from_str(data_json).map_err(|error| JsError::new(&error.to_string()))?;
    crate::resolve_defines_with_data(markdown, &data)
        .map_err(|error| JsError::new(&error.to_string()))
}
