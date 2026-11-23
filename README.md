# MDRS

[![CircleCI](https://dl.circleci.com/status-badge/img/circleci/stZqvYiNAK17m5zy7ERCb/EGBRyRQ9CWeyLiSDGguM5Z/tree/main.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/circleci/stZqvYiNAK17m5zy7ERCb/EGBRyRQ9CWeyLiSDGguM5Z/tree/main) [![codecov](https://codecov.io/gh/farhnkrnapratma/mdrs/graph/badge.svg)](https://codecov.io/gh/farhnkrnapratma/mdrs) [![npm](https://img.shields.io/npm/v/@mdrs/core?label=npm)](https://www.npmjs.com/package/@mdrs/core) [![crates.io](https://img.shields.io/crates/v/mdrs?label=crates.io)](https://crates.io/crates/mdrs) [![docs.rs](https://img.shields.io/docsrs/mdrs)](https://docs.rs/mdrs) ![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)

**MDRS (Markdown Rich Syntax)** is a superset of Markdown. A CommonMark or GitHub Flavored Markdown document renders as it would anywhere else, and MDRS adds callouts, collapsible sections, steps, tabs, math, diagrams, extended task lists and document variables on top. This repository is the MDRS renderer: it turns MDRS into sanitized HTML, is written in Rust and is available as a crate and as an npm package for Node.js and Bun.

MDRS files use the `.mdrs` extension.

> **Alpha.** The API may still change before 1.0.0.

## A taste

```mdrs
:::define
project: "MDRS"
:::

# Hello from ${project}

Plain **Markdown** works as usual.

:::tip
Directives open with `:::name` and close with a bare `:::`.
:::

:::details summary="What is rendered on the server?"
Everything except diagrams and charts, including math: $$ e^{i\pi} + 1 = 0 $$
:::
```

The [MDRS syntax guide](https://github.com/farhnkrnapratma/idstead/tree/main/docs/008-mdrs) describes every construct. It is written for the original TypeScript engine; see [Limitations](#limitations) for what the Rust renderer does not support yet.

## Install

```sh
npm install @mdrs/core         # or: bun add @mdrs/core
cargo add mdrs                 # Rust
```

The npm package ships prebuilt native addons as optional dependencies, so nothing is compiled on install:

| OS      | Architectures                                                   |
| ------- | --------------------------------------------------------------- |
| Linux   | x64 and arm64 (glibc and musl), armv7, ppc64le, s390x, riscv64 |
| macOS   | arm64, x64                                                      |
| Windows | x64, arm64, ia32                                                |

## Usage

### Node.js and Bun

```ts
import { prepareMarkdownPage, renderMarkdown, renderMarkdownWithData } from "@mdrs/core";

const html = renderMarkdown("# Hello\n\nA **rich** document.");

const page = prepareMarkdownPage(source, { internalOrigins: ["https://example.com"] });
page.html;     // the rendered HTML
page.headings; // [{ id, text, level }] for a table of contents

renderMarkdownWithData("Hello ${name}!", JSON.stringify({ name: "Ada" }));
```

| Function                                                                          | Returns                                                                                                    |
| --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `renderMarkdown(source, options?)`                                                | HTML                                                                                                       |
| `prepareMarkdownPage(source, options?)`                                           | `{ html, headings }`                                                                                       |
| `resolveDefines(source)`                                                          | The source with `:::define` variables substituted                                                          |
| `renderMarkdownWithData`, `prepareMarkdownPageWithData`, `resolveDefinesWithData` | The same, with a JSON string of values that expressions can read                                           |
| `isMdrsPath(path)`, `FILE_EXTENSION`                                              | Whether a path ends in `.mdrs`, and the extension itself                                                   |

`internalOrigins` lists origins whose links open in the same tab; other links get `target="_blank"`. Invalid definitions and expressions, such as a variable a `:::define` block does not declare, throw an error that names the problem.

### Rust

```rust
use mdrs::{RenderOptions, prepare_markdown_page, render_markdown};

let html = render_markdown("# Hello\n\nA **rich** document.")?;
let page = prepare_markdown_page("# Hello", &RenderOptions::default())?;
```

The Rust API mirrors the JavaScript one with snake_case names. Documents are limited to 500,000 bytes of input and 2,000,000 bytes of output.

## Output

The HTML is sanitized: raw HTML is reduced to a safe subset, event handlers are stripped and unsafe URLs are removed. Generated classes start with `mdrs-`. Math is typeset to KaTeX HTML on the server. Mermaid diagrams and charts are emitted as placeholders that carry their source, for a browser-side script to render.

## Limitations

The Rust renderer is a port of the TypeScript engine described in the syntax guide and does not match it byte for byte yet. Known gaps:

- GitHub alert blockquotes (`> [!NOTE]`) render as plain blockquotes; use `:::note` and the other callout directives.
- Live data (`get(url, path)`) and other asynchronous resolvers are not available, because the bindings are synchronous.

## Contributing

Bug reports and pull requests are welcome. [`CONTRIBUTING.md`](https://github.com/farhnkrnapratma/mdrs/blob/main/CONTRIBUTING.md) explains how to build, test and submit changes.

## License

Licensed under either of the [Apache License, Version 2.0](https://github.com/farhnkrnapratma/mdrs/blob/main/LICENSE-APACHE) or the [MIT License](https://github.com/farhnkrnapratma/mdrs/blob/main/LICENSE-MIT), at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
