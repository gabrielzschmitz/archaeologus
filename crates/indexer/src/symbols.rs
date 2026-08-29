//! Symbol extraction from tree-sitter ASTs.
//!
//! Each language has its own set of node kinds that map to [`SymbolKind`]
//! values.  [`extract_symbols`] is the single public entry point.

use serde::{Deserialize, Serialize};
use tree_sitter::Node;

use crate::languages::Lang;
use crate::parser::{walk_named, ParseResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Class,
    Interface,
    Type,
    Constructor,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Module => "module",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Type => "type",
            Self::Constructor => "constructor",
        };
        write!(f, "{s}")
    }
}

/// A symbol extracted from a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: Option<String>,
    pub doc_comment: Option<String>,
    /// Full source text of the symbol node (capped at 4 KiB to stay DB-friendly).
    pub raw_text: String,
    pub line_start: usize,
    pub line_end: usize,
    pub col_start: usize,
    pub col_end: usize,
}

#[must_use]
pub fn extract_symbols(result: &ParseResult, source: &[u8]) -> Vec<ExtractedSymbol> {
    let root = result.tree.root_node();
    match result.language {
        Lang::Rust => extract_rust(root, source),
        Lang::Python => extract_python(root, source),
        Lang::JavaScript => extract_javascript(root, source),
        Lang::TypeScript => extract_typescript(root, source),
        Lang::Go => extract_go(root, source),
        Lang::Java => extract_java(root, source),
        Lang::C => extract_c(root, source),
        Lang::Cpp => extract_cpp(root, source),
    }
}

fn node_text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

fn child_text<'a>(node: Node<'_>, kind: &str, source: &'a [u8]) -> Option<&'a str> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(node_text(child, source));
        }
    }
    None
}

/// Maximum bytes we store for `raw_text` per symbol (4 KiB).
const RAW_TEXT_CAP: usize = 4096;

fn make_sym(
    name: String,
    kind: SymbolKind,
    node: Node<'_>,
    source: &[u8],
    visibility: Option<String>,
    doc_comment: Option<String>,
) -> ExtractedSymbol {
    let start = node.start_position();
    let end = node.end_position();

    // Slice the raw bytes for this node and cap at RAW_TEXT_CAP.
    let node_bytes = &source[node.start_byte()..node.end_byte().min(source.len())];
    let raw_text = if node_bytes.len() > RAW_TEXT_CAP {
        let truncated = &node_bytes[..RAW_TEXT_CAP];
        // Back up to the last valid UTF-8 boundary.
        let valid = std::str::from_utf8(truncated)
            .unwrap_or_else(|e| std::str::from_utf8(&truncated[..e.valid_up_to()]).unwrap_or(""));
        format!("{valid}…")
    } else {
        String::from_utf8_lossy(node_bytes).into_owned()
    };

    ExtractedSymbol {
        name,
        kind,
        visibility,
        doc_comment,
        raw_text,
        line_start: start.row,
        line_end: end.row,
        col_start: start.column,
        col_end: end.column,
    }
}

fn preceding_doc_comment(node: Node<'_>, source: &[u8]) -> Option<String> {
    let parent = node.parent()?;
    let idx = (0..parent.child_count()).find(|&i| {
        parent
            .child(u32::try_from(i).unwrap())
            .is_some_and(|c| c.id() == node.id())
    })?;

    let mut lines = Vec::new();
    let mut i = idx.checked_sub(1)?;
    loop {
        let sibling = parent.child(u32::try_from(i).unwrap())?;
        if sibling.kind() == "line_comment"
            || sibling.kind() == "block_comment"
            || sibling.kind() == "comment"
        {
            let text = node_text(sibling, source).trim().to_string();
            lines.push(text);
        } else {
            break;
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    lines.reverse();
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn extract_rust(root: Node<'_>, source: &[u8]) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| {
        let kind = match node.kind() {
            "function_item" => SymbolKind::Function,
            "struct_item" => SymbolKind::Struct,
            "enum_item" => SymbolKind::Enum,
            "trait_item" => SymbolKind::Trait,
            "impl_item" => SymbolKind::Impl,
            "mod_item" => SymbolKind::Module,
            "type_item" => SymbolKind::Type,
            _ => return,
        };

        let name = if node.kind() == "impl_item" {
            child_text(node, "type_identifier", source)
                .or_else(|| child_text(node, "generic_type", source))
                .unwrap_or("(impl)")
                .to_string()
        } else {
            child_text(node, "identifier", source)
                .or_else(|| child_text(node, "type_identifier", source))
                .unwrap_or("")
                .to_string()
        };

        if name.is_empty() {
            return;
        }

        let vis = child_text(node, "visibility_modifier", source).map(String::from);
        let doc = preceding_doc_comment(node, source);
        out.push(make_sym(name, kind, node, source, vis, doc));
    });
    out
}

fn extract_python(root: Node<'_>, source: &[u8]) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| {
        let kind = match node.kind() {
            "function_definition" => SymbolKind::Function,
            "class_definition" => SymbolKind::Class,
            _ => return,
        };
        let adjusted_kind = if kind == SymbolKind::Function {
            if node
                .parent()
                .and_then(|p| p.parent())
                .is_some_and(|gp| gp.kind() == "class_definition")
            {
                SymbolKind::Method
            } else {
                kind
            }
        } else {
            kind
        };

        let name = child_text(node, "identifier", source)
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            return;
        }
        let doc = preceding_doc_comment(node, source);
        out.push(make_sym(name, adjusted_kind, node, source, None, doc));
    });
    out
}

fn extract_javascript(root: Node<'_>, source: &[u8]) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(
                name,
                SymbolKind::Function,
                node,
                source,
                None,
                doc,
            ));
        }
        "class_declaration" | "class" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(name, SymbolKind::Class, node, source, None, doc));
        }
        "method_definition" => {
            let name = child_text(node, "property_identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let kind = if name == "constructor" {
                SymbolKind::Constructor
            } else {
                SymbolKind::Method
            };
            out.push(make_sym(name, kind, node, source, None, None));
        }
        _ => {}
    });
    out
}

fn extract_typescript(root: Node<'_>, source: &[u8]) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(
                name,
                SymbolKind::Function,
                node,
                source,
                None,
                doc,
            ));
        }
        "class_declaration" | "class" => {
            let name = child_text(node, "type_identifier", source)
                .or_else(|| child_text(node, "identifier", source))
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(name, SymbolKind::Class, node, source, None, doc));
        }
        "method_definition" => {
            let name = child_text(node, "property_identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let kind = if name == "constructor" {
                SymbolKind::Constructor
            } else {
                SymbolKind::Method
            };
            out.push(make_sym(name, kind, node, source, None, None));
        }
        "interface_declaration" => {
            let name = child_text(node, "type_identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(
                name,
                SymbolKind::Interface,
                node,
                source,
                None,
                doc,
            ));
        }
        "type_alias_declaration" => {
            let name = child_text(node, "type_identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(name, SymbolKind::Type, node, source, None, doc));
        }
        _ => {}
    });
    out
}

fn extract_go(root: Node<'_>, source: &[u8]) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "function_declaration" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(
                name,
                SymbolKind::Function,
                node,
                source,
                None,
                doc,
            ));
        }
        "method_declaration" => {
            let name = child_text(node, "field_identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(name, SymbolKind::Method, node, source, None, doc));
        }
        "type_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_spec" {
                    let name = child_text(child, "type_identifier", source)
                        .unwrap_or("")
                        .to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let kind = if child
                        .children(&mut child.walk())
                        .any(|c| c.kind() == "struct_type")
                    {
                        SymbolKind::Struct
                    } else if child
                        .children(&mut child.walk())
                        .any(|c| c.kind() == "interface_type")
                    {
                        SymbolKind::Interface
                    } else {
                        SymbolKind::Type
                    };
                    let doc = preceding_doc_comment(node, source);
                    // Use `node` (type_declaration) for full source span.
                    out.push(make_sym(name, kind, node, source, None, doc));
                }
            }
        }
        _ => {}
    });
    out
}

fn extract_java(root: Node<'_>, source: &[u8]) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "class_declaration" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let vis = modifiers_visibility(node, source);
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(name, SymbolKind::Class, node, source, vis, doc));
        }
        "interface_declaration" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let vis = modifiers_visibility(node, source);
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(
                name,
                SymbolKind::Interface,
                node,
                source,
                vis,
                doc,
            ));
        }
        "method_declaration" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let vis = modifiers_visibility(node, source);
            out.push(make_sym(name, SymbolKind::Method, node, source, vis, None));
        }
        "constructor_declaration" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let vis = modifiers_visibility(node, source);
            out.push(make_sym(
                name,
                SymbolKind::Constructor,
                node,
                source,
                vis,
                None,
            ));
        }
        "enum_declaration" => {
            let name = child_text(node, "identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let vis = modifiers_visibility(node, source);
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(name, SymbolKind::Enum, node, source, vis, doc));
        }
        _ => {}
    });
    out
}

fn modifiers_visibility(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let text = node_text(child, source);
            for token in text.split_whitespace() {
                if matches!(token, "public" | "protected" | "private") {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

fn extract_c(root: Node<'_>, source: &[u8]) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();

    walk_named(root, &mut |node| match node.kind() {
        "function_definition" => {
            if let Some(name) = c_function_name(node, source) {
                out.push(make_sym(
                    name,
                    SymbolKind::Function,
                    node,
                    source,
                    None,
                    None,
                ));
            }
        }

        "type_definition" => {
            let name = c_typedef_name(node, source);
            if let Some(n) = name {
                let kind = if node
                    .children(&mut node.walk())
                    .any(|c| c.kind() == "struct_specifier")
                {
                    SymbolKind::Struct
                } else if node
                    .children(&mut node.walk())
                    .any(|c| c.kind() == "enum_specifier")
                {
                    SymbolKind::Enum
                } else {
                    SymbolKind::Type
                };
                out.push(make_sym(n, kind, node, source, None, None));
            }
        }

        "struct_specifier" if node.parent().is_none_or(|p| p.kind() != "type_definition") => {
            if let Some(name) = child_text(node, "type_identifier", source) {
                out.push(make_sym(
                    name.to_string(),
                    SymbolKind::Struct,
                    node,
                    source,
                    None,
                    None,
                ));
            }
        }

        _ => {}
    });

    out
}

fn c_function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            let mut c2 = child.walk();
            for inner in child.children(&mut c2) {
                if inner.kind() == "identifier" || inner.kind() == "field_identifier" {
                    return Some(node_text(inner, source).to_string());
                }
            }
        }
    }
    None
}

fn c_typedef_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let count = node.named_child_count();
    for i in (0..count).rev() {
        if let Some(child) = node.named_child(u32::try_from(i).unwrap()) {
            if child.kind() == "type_identifier" {
                return Some(node_text(child, source).to_string());
            }
        }
    }
    None
}

fn extract_cpp(root: Node<'_>, source: &[u8]) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "function_definition" => {
            if let Some(name) = cpp_function_name(node, source) {
                out.push(make_sym(
                    name,
                    SymbolKind::Function,
                    node,
                    source,
                    None,
                    None,
                ));
            }
        }
        "class_specifier" => {
            let name = child_text(node, "type_identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(name, SymbolKind::Class, node, source, None, doc));
        }
        "struct_specifier" => {
            if node.parent().is_some_and(|p| p.kind() == "class_specifier") {
                return;
            }
            let name = child_text(node, "type_identifier", source)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return;
            }
            let doc = preceding_doc_comment(node, source);
            out.push(make_sym(name, SymbolKind::Struct, node, source, None, doc));
        }
        _ => {}
    });
    out
}

fn cpp_function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            let mut c2 = child.walk();
            for inner in child.children(&mut c2) {
                match inner.kind() {
                    "identifier" | "field_identifier" | "qualified_identifier" => {
                        return Some(node_text(inner, source).to_string());
                    }
                    _ => {}
                }
            }
        }
    }
    None
}
