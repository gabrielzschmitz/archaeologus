//! Source-code parsing via tree-sitter.
//!
//! Provides [`parse`] (full) and [`parse_incremental`] (incremental) entry
//! points that return a typed [`ParseResult`].

use tree_sitter::{InputEdit, Node, Parser, Tree};

use crate::languages::Lang;

#[derive(Debug)]
pub struct ParseResult {
    pub tree: Tree,
    pub has_errors: bool,
    pub language: Lang,
}

impl ParseResult {
    fn new(tree: Tree, language: Lang) -> Self {
        let has_errors = tree.root_node().has_error();
        Self {
            tree,
            has_errors,
            language,
        }
    }
}

/// Parse `source` with the grammar for `language`.
///
/// # Errors
/// Returns an error if the parser fails to produce a tree (should be rare
/// with tree-sitter – it always produces *something*, even for broken input).
pub fn parse(source: &[u8], language: Lang) -> anyhow::Result<ParseResult> {
    let mut parser = Parser::new();
    parser
        .set_language(&language.tree_sitter_language())
        .map_err(|e| anyhow::anyhow!("failed to set language: {e}"))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter returned None for {language:?}"))?;
    Ok(ParseResult::new(tree, language))
}

/// Re-parse `new_source` incrementally given a previously parsed `old_tree`.
///
/// `edit` describes what changed between the old and new source.  When the old
/// tree is not available, falls back to a full parse.
///
/// # Errors
/// Same conditions as [`parse`].
pub fn parse_incremental(
    new_source: &[u8],
    language: Lang,
    old_tree: Option<(&mut Tree, InputEdit)>,
) -> anyhow::Result<ParseResult> {
    let mut parser = Parser::new();
    parser
        .set_language(&language.tree_sitter_language())
        .map_err(|e| anyhow::anyhow!("failed to set language: {e}"))?;

    let tree = if let Some((tree, edit)) = old_tree {
        tree.edit(&edit);
        parser
            .parse(new_source, Some(tree))
            .ok_or_else(|| anyhow::anyhow!("incremental parse returned None for {language:?}"))?
    } else {
        parser
            .parse(new_source, None)
            .ok_or_else(|| anyhow::anyhow!("parse returned None for {language:?}"))?
    };

    Ok(ParseResult::new(tree, language))
}

/// Walk `node` and all its descendants, calling `cb` for every named node.
pub fn walk_named<'a, F>(node: Node<'a>, cb: &mut F)
where
    F: FnMut(Node<'a>),
{
    if node.is_named() {
        cb(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_named(child, cb);
    }
}
