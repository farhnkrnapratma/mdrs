# Contributing to MDRS

## Setup

You need Rust (stable) with the `wasm32-unknown-unknown` target, and [Bun](https://bun.sh). `bun.lock` is the only lockfile.

```sh
bun install    # JavaScript dependencies, commitlint and the git hooks
```

## Build and test

```sh
cargo test --locked                                         # Rust tests
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt --check
cargo deny check
cargo llvm-cov --locked --summary-only                      # must report no uncovered lines or functions
cargo check --features napi
cargo check --target wasm32-unknown-unknown --features wasm
bun run build:release                                       # Node.js addon for this platform
bun run test:bindings                                       # binding checks
bun run test:native                                         # build:release, then test:bindings
```

`test:bindings` checks the TypeScript declarations against both binding modules and the `napi` and `wasm` Cargo features. It reports, without failing, when the local `napi` or `wasm-pack` packagers are unavailable.

## The TypeScript engine as the reference

The behaviour reference is the TypeScript engine in the [idstead](https://github.com/farhnkrnapratma/idstead) repository (`packages/mdrs/core`), and its [syntax guide](https://github.com/farhnkrnapratma/idstead/tree/main/docs/008-mdrs) is the specification. With idstead checked out next to this repository:

```sh
bun scripts/showcase-parity.ts                 # compare the showcase output with the TypeScript engine
cargo bench --locked --offline --bench render  # rendering baseline
```

The parity script reports output hashes, marker counts and the first difference. The Rust output does not match byte for byte yet; keep the gaps listed in the README up to date when you close one.

To render a `.mdrs` file to HTML:

```sh
cargo run --example showcase_render -- path/to/file.mdrs > out.html
```

## Repository layout

```
src/lib.rs          public API
src/markdown.rs     CommonMark and GFM rendering
src/containers.rs   block directives (:::note, :::tabs, …)
src/define.rs       :::define variables, expressions and conditionals
src/headings.rs     heading ids and structure
src/sanitize.rs     HTML sanitization
src/bindings/       Node.js (napi) and WebAssembly (wasm) glue
examples/           showcase_render: renders a .mdrs file to HTML
benches/            rendering baseline
scripts/            binding checks and parity tooling
index.js            JavaScript entry over the generated native loader
```

## Submitting changes

- Branch from `develop` as `feature/<name>` or `bugfix/<name>`. The repository uses [git-flow-next](https://git-flow.sh/), and `main` only receives releases and hotfixes.
- Write commit messages in the [Conventional Commits](https://www.conventionalcommits.org) format and sign them off with `git commit -s`. The `commit-msg` hook and CI check both.
- Keep the tests, clippy, formatting and coverage passing, and add a regression test with every fix.

CI runs on CircleCI: `reuse lint`, formatting and lint, tests, coverage, `cargo deny`, a secret scan and commitlint on every push.

## Reporting security issues

Do not open a public issue for a vulnerability. Report it privately through [GitHub's security advisories](https://github.com/farhnkrnapratma/mdrs/security/advisories/new).
