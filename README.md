# MDRS

[![CircleCI](https://dl.circleci.com/status-badge/img/circleci/stZqvYiNAK17m5zy7ERCb/EGBRyRQ9CWeyLiSDGguM5Z/tree/main.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/circleci/stZqvYiNAK17m5zy7ERCb/EGBRyRQ9CWeyLiSDGguM5Z/tree/main) [![npm](https://img.shields.io/npm/v/@mdrs/core?label=npm)](https://www.npmjs.com/package/@mdrs/core) [![crates.io](https://img.shields.io/crates/v/mdrs?label=crates.io)](https://crates.io/crates/mdrs) ![Language](https://img.shields.io/badge/language-Rust-orange?logo=rust) ![Tests](https://img.shields.io/badge/tests-cargo%20test-orange?logo=rust) ![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)


**MDRS (Markdown Rich Syntax)** is a superset of Markdown. It is meant as a drop-in replacement: any CommonMark or GitHub Flavored Markdown document should render the same way. On top of that it adds directives (`:::note`, `:::tabs`, `:::details`, `:::steps`, …), math, diagrams, charts, references and document variables (`:::define` with `${…}` interpolation).

MDRS files use the `.mdrs` extension.

## Status

Alpha (`v1.0.0-alpha1`). The Rust renderer port is in progress. The original TypeScript engine at `idstead/packages/mdrs/core` remains intact and is the behavior reference.

The Rust crate supports CommonMark/GFM rendering, sanitization, selected block directives, JSON-backed `:::define` expressions and conditionals, server-side KaTeX math, diagram/chart/embed markers, references, extended task states, icons, and verbatim blocks. It is not a byte-for-byte parity port: asynchronous JavaScript resolver callbacks and browser-side Mermaid/chart execution remain runtime integrations rather than Rust implementations.

| Registry | Name |
| --- | --- |
| crates.io | [`mdrs`](https://crates.io/crates/mdrs) |
| npm | [`@mdrs/core`](https://www.npmjs.com/package/@mdrs/core) |

The `napi` and `wasm` Cargo features expose `renderMarkdown`, `prepareMarkdownPage`, `resolveDefines` and `isMdrsPath`, plus `*WithData` variants that accept a JSON string for definitions, expressions and conditionals. Async JavaScript resolver callbacks are not exposed by the synchronous Rust ABI.

## Install

```sh
cargo add mdrs              # Rust
bun add @mdrs/core          # JavaScript / TypeScript (or: npm install @mdrs/core)
```

The npm package ships prebuilt binaries for Linux (x64 and arm64, glibc and musl), macOS (x64 and arm64) and Windows (x64 and arm64) as optional dependencies. Math is rendered server-side through `katex-rs`; Mermaid and chart markers are left for browser-side execution.

## Usage

Rust:

```rust
use std::path::Path;

assert_eq!(mdrs::FILE_EXTENSION, "mdrs");
assert!(mdrs::is_mdrs_path(Path::new("docs/intro.mdrs")));

let html = mdrs::render_markdown("# Hello\n\nA **rich** document.")?;
```

JavaScript / TypeScript:

```ts
import { FILE_EXTENSION, isMdrsPath, renderMarkdown } from '@mdrs/core';

FILE_EXTENSION; // "mdrs"
isMdrsPath('docs/intro.mdrs'); // true
renderMarkdown('# Hello\n\nA **rich** document.');
```

To render a `.mdrs` file to HTML, run the bundled example with the path of the file:

```sh
cargo run --example showcase_render -- docs/intro.mdrs > intro.html
```

## Repository layout

```
src/
  lib.rs          public API
  markdown.rs     CommonMark / GFM rendering
  containers.rs   block directives (:::note, :::tabs, …)
  define.rs       :::define variables, expressions, conditionals
  headings.rs     heading ids and structure
  sanitize.rs     HTML sanitization
  bindings/       Node.js (napi) and WebAssembly (wasm) glue
examples/         showcase_render: renders a .mdrs file to HTML
benches/          rendering baseline
scripts/          binding checks and parity tooling
index.js          JavaScript entry (re-exports the generated native loader)
```

## Development

Requirements: Rust (stable) with the `wasm32-unknown-unknown` target, and [Bun](https://bun.sh) (`bun.lock` is the only lockfile).

```sh
bun install                              # JS dependencies and git hooks
cargo test --locked                      # Rust tests
cargo check --features napi
cargo check --target wasm32-unknown-unknown --features wasm
bun run build:release                    # Node artifact for this platform
bun run test:bindings                    # offline binding checks
bun run test:native                      # build:release + test:bindings
```

`test:bindings` validates the TypeScript declarations against both binding modules and checks the `napi` and `wasm` Cargo features. It reports, without failing, when the local `napi` or `wasm-pack` packagers are unavailable.

Parity and performance checks:

```sh
bun scripts/showcase-parity.ts           # compare the showcase with the TypeScript mirror (needs the idstead checkout)
cargo bench --locked --offline --bench render
```

The parity command reports output hashes, marker counts and the first difference. The Rust output is intentionally not byte-for-byte identical to the mirror yet.

Releases are cut with git-flow (`git-flow release start/finish`). Pushing a `v*` tag makes CircleCI build the native binding for every platform and publish `@mdrs/core` to npm; the crate is published with `cargo publish`.

## Contributing

This repository uses [git-flow-next](https://git-flow.sh/) (classic preset): `main` is release-only, work happens on `feature/*` and `bugfix/*` branches off `develop`. Commits are signed off (`git commit -s`) and follow [Conventional Commits](https://www.conventionalcommits.org), enforced by commitlint in a `commit-msg` hook and in CI. Install the hooks with `bash .githooks/install.sh` (`bun install` does it through the `prepare` script). CI runs on CircleCI: `reuse lint`, format and lint, tests, `cargo deny`, a TruffleHog secret scan and commitlint on every push. The branch rules are written out in [`CLAUDE.md`](CLAUDE.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.

Copyright © 2026 Farhan Kurnia Pratama
