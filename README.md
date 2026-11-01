# MDRS

**MDRS (Markdown Rich Syntax)** is a superset of Markdown. It is meant as a drop-in replacement: any CommonMark or GitHub Flavored Markdown document should render the same way. On top of that it adds directives (`:::note`, `:::tabs`, `:::details`, `:::steps`, …), math, diagrams, charts, references and document variables (`:::define` with `${…}` interpolation).

MDRS files use the `.mdrs` extension.

## Status

The name is reserved and the project is under active development. This `0.0.0` release only exposes the file extension helpers below; the parser and renderer will follow in a later release.

## Packages

| Registry | Name |
| --- | --- |
| npm | [`@mdrs/core`](https://www.npmjs.com/package/@mdrs/core) |
| crates.io | [`mdrs`](https://crates.io/crates/mdrs) |

### JavaScript / TypeScript

```sh
bun add @mdrs/core
```

```ts
import { FILE_EXTENSION, isMdrsPath } from '@mdrs/core';

FILE_EXTENSION; // "mdrs"
isMdrsPath('docs/intro.mdrs'); // true
```

### Rust

```sh
cargo add mdrs
```

```rust
use std::path::Path;

assert_eq!(mdrs::FILE_EXTENSION, "mdrs");
assert!(mdrs::is_mdrs_path(Path::new("docs/intro.mdrs")));
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.

Copyright © 2026 Farhan Kurnia Pratama
