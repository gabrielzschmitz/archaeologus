//! Language detection and tree-sitter grammar resolution.

use std::path::Path;
use tree_sitter::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
}

impl Lang {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Java => "java",
            Self::C => "c",
            Self::Cpp => "cpp",
        }
    }

    /// Return the tree-sitter [`Language`] for this variant.
    #[must_use]
    pub fn tree_sitter_language(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        }
    }
}

#[must_use]
pub fn detect_language(path: &str) -> Option<Lang> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some(Lang::Rust),
        "py" => Some(Lang::Python),
        "js" | "mjs" | "cjs" => Some(Lang::JavaScript),
        "ts" | "mts" | "cts" | "tsx" => Some(Lang::TypeScript),
        "go" => Some(Lang::Go),
        "java" => Some(Lang::Java),
        "c" | "h" => Some(Lang::C),
        "cpp" | "hpp" | "cc" | "cxx" | "hxx" => Some(Lang::Cpp),
        _ => None,
    }
}
