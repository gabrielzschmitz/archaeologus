//! Dependency extraction from tree-sitter ASTs.
//!
//! Detects:
//! - Use / import statements
//! - Function calls
//! - Trait implementations (Rust)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tree_sitter::Node;

use crate::languages::Lang;
use crate::parser::{walk_named, ParseResult};

/// A single detected dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// What is being referenced (module path, function name, …).
    pub target: String,
    pub kind: DependencyKind,
    /// 0-based line where the reference occurs.
    pub line: usize,
}

/// The nature of a dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Import,
    Call,
    TraitImpl,
}

pub type DependencyGraph = HashMap<String, Vec<Dependency>>;

#[must_use]
pub fn extract_dependencies(result: &ParseResult, source: &[u8]) -> Vec<Dependency> {
    let root = result.tree.root_node();
    match result.language {
        Lang::Rust => extract_rust_deps(root, source),
        Lang::Python => extract_python_deps(root, source),
        Lang::JavaScript | Lang::TypeScript => extract_js_deps(root, source),
        Lang::Go => extract_go_deps(root, source),
        Lang::Java => extract_java_deps(root, source),
        Lang::C | Lang::Cpp => extract_c_deps(root, source),
    }
}

#[must_use]
pub fn build_graph(deps: Vec<Dependency>) -> DependencyGraph {
    let mut graph: DependencyGraph = HashMap::new();
    graph.entry("(file)".to_string()).or_default().extend(deps);
    graph
}

fn node_text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

fn dep(target: impl Into<String>, kind: DependencyKind, node: Node<'_>) -> Dependency {
    Dependency {
        target: target.into(),
        kind,
        line: node.start_position().row,
    }
}

fn extract_rust_deps(root: Node<'_>, source: &[u8]) -> Vec<Dependency> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "use_declaration" => {
            let text = node_text(node, source)
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .trim()
                .to_string();
            out.push(dep(text, DependencyKind::Import, node));
        }
        "call_expression" => {
            if let Some(func) = node.child(0) {
                let name = node_text(func, source);
                if !name.is_empty() {
                    out.push(dep(name, DependencyKind::Call, node));
                }
            }
        }
        "impl_item" => {
            let mut cursor = node.walk();
            let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
            let has_for = children.iter().any(|c| c.kind() == "for");
            if has_for {
                for i in 0..children.len() {
                    if children[i].kind() == "for" && i > 0 {
                        let trait_name = node_text(children[i - 1], source);
                        if !trait_name.is_empty() {
                            out.push(dep(trait_name, DependencyKind::TraitImpl, node));
                        }
                        break;
                    }
                }
            }
        }
        _ => {}
    });
    out
}

fn extract_python_deps(root: Node<'_>, source: &[u8]) -> Vec<Dependency> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "import_statement" | "import_from_statement" => {
            let text = node_text(node, source).to_string();
            out.push(dep(text, DependencyKind::Import, node));
        }
        "call" => {
            if let Some(func) = node.child_by_field_name("function") {
                let name = node_text(func, source);
                if !name.is_empty() {
                    out.push(dep(name, DependencyKind::Call, node));
                }
            }
        }
        _ => {}
    });
    out
}

fn extract_js_deps(root: Node<'_>, source: &[u8]) -> Vec<Dependency> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "import_statement" => {
            let text = node_text(node, source).to_string();
            out.push(dep(text, DependencyKind::Import, node));
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                let name = node_text(func, source);
                if !name.is_empty() && name != "require" {
                    out.push(dep(name, DependencyKind::Call, node));
                }
            }
        }
        _ => {}
    });
    out
}

fn extract_go_deps(root: Node<'_>, source: &[u8]) -> Vec<Dependency> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "import_declaration" => {
            let text = node_text(node, source).to_string();
            out.push(dep(text, DependencyKind::Import, node));
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                let name = node_text(func, source);
                if !name.is_empty() {
                    out.push(dep(name, DependencyKind::Call, node));
                }
            }
        }
        _ => {}
    });
    out
}

fn extract_java_deps(root: Node<'_>, source: &[u8]) -> Vec<Dependency> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "import_declaration" => {
            let text = node_text(node, source)
                .trim_start_matches("import ")
                .trim_end_matches(';')
                .trim()
                .to_string();
            out.push(dep(text, DependencyKind::Import, node));
        }
        "method_invocation" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(name_node, source);
                if !name.is_empty() {
                    out.push(dep(name, DependencyKind::Call, node));
                }
            }
        }
        _ => {}
    });
    out
}

fn extract_c_deps(root: Node<'_>, source: &[u8]) -> Vec<Dependency> {
    let mut out = Vec::new();
    walk_named(root, &mut |node| match node.kind() {
        "preproc_include" => {
            let text = node_text(node, source).to_string();
            out.push(dep(text, DependencyKind::Import, node));
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                let name = node_text(func, source);
                if !name.is_empty() {
                    out.push(dep(name, DependencyKind::Call, node));
                }
            }
        }
        _ => {}
    });
    out
}
