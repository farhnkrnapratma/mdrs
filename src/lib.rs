// SPDX-FileCopyrightText: 2026 Farhan Kurnia Pratama
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! MDRS (Markdown Rich Syntax): a drop-in superset of Markdown.
//!
//! This release only reserves the name and exposes the file extension helpers.

use std::path::Path;

/// The file extension used by MDRS documents, without the leading dot.
pub const FILE_EXTENSION: &str = "mdrs";

/// Returns `true` when `path` ends in the `.mdrs` extension (case-insensitive).
pub fn is_mdrs_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(FILE_EXTENSION))
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
}
