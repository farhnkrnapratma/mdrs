use std::env;
use std::fs;

fn main() {
    let mut args = env::args().skip(1);
    let markdown_path = args.next().expect("usage: showcase_render <markdown>");
    let markdown = fs::read_to_string(markdown_path).expect("read Markdown fixture");
    let data = serde_json::json!({
        "api": {
            "v1": {
                "author": {
                    "profile": {
                        "name": "Ann Example",
                        "siteUrl": "https://example.com"
                    }
                },
                "blog": [
                    {"title": "First post", "slug": "first"},
                    {"title": "Second post", "slug": "second"},
                    {"title": "Third post", "slug": "third"}
                ]
            }
        }
    });
    let html = mdrs::render_markdown_with_data(
        &markdown,
        &data,
        &mdrs::RenderOptions {
            internal_origins: vec!["https://my-site.example".to_owned()],
        },
    )
    .expect("render showcase");
    print!("{html}");
}
