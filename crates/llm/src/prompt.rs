//! Prompt templates for archaeologus queries.
//!
//! These templates are used by the CLI `ask` command to construct
//! context-rich messages for any [`LLMProvider`].

use crate::provider::ChatMessage;
use archaeologus_core::models::{File, Repository, Symbol, SymbolDependency};
use archaeologus_evidence::{EvidenceItem, EvidenceSource, Explanation};
use std::fmt::Write as _;

// ── System prompt ─────────────────────────────────────────────────────────────

/// Returns the system prompt that establishes the AI's persona.
#[must_use]
pub fn system_prompt() -> ChatMessage {
    ChatMessage::system(
        "You are an AI Software Archaeologus. \
Your job is to help developers understand why their codebase looks the way it does. \
You are given structured evidence gathered from git history, source code analysis, \
commit messages, blame information, file locations, and dependency graphs. \
\n\n\
Constraints:\n\
- Only reason from the evidence provided. Do NOT invent facts.\n\
- If confidence is low, say so clearly.\n\
- Cite commit SHAs or file:line references when available.\n\
- Be concise: lead with the key insight, then support it with evidence.\n\
- When multiple symbols are relevant, address each one.\n\
- Format your response in plain text (no markdown headers with #).",
    )
}

// ── Context builders ──────────────────────────────────────────────────────────

/// Build a compact context block describing the repository a symbol lives in.
#[must_use]
pub fn repo_context(repo: &Repository) -> String {
    let mut s = format!("Repository: {} ({})\n", repo.name, repo.url);
    if let Some(desc) = &repo.description {
        if !desc.trim().is_empty() {
            let _ = writeln!(s, "  Description: {}", desc.trim());
        }
    }
    s
}

/// Build a compact context block describing the source file.
#[must_use]
pub fn file_context(file: &File) -> String {
    format!(
        "File: {} ({})\n",
        file.path,
        file.language.as_deref().unwrap_or("unknown language"),
    )
}

/// Build a context block from a single [`Symbol`].
///
/// Shows: name, type, language, visibility, doc-comment, full source text
/// with line-number anchors, and the `file:line` location.
#[must_use]
pub fn symbol_context(symbol: &Symbol, file: Option<&File>) -> String {
    let location = file.map_or_else(
        || format!("lines {}-{}", symbol.line_start, symbol.line_end),
        |f| format!("{}:{}-{}", f.path, symbol.line_start, symbol.line_end),
    );

    let mut s = format!(
        "Symbol: `{}` ({} {}, {}) at {}\n",
        symbol.name,
        symbol.language,
        symbol.symbol_type,
        symbol.visibility.as_deref().unwrap_or("unknown visibility"),
        location,
    );

    if let Some(doc) = &symbol.doc_comment {
        if !doc.trim().is_empty() {
            let _ = writeln!(s, "  Doc comment: {}", doc.trim());
        }
    }

    let raw = symbol.raw_text.trim();
    if !raw.is_empty() {
        // Show up to 600 chars — enough for a full small function/struct.
        let limit = 600;
        let excerpt: String = raw.chars().take(limit).collect();
        let excerpt = if raw.chars().count() > limit {
            format!("{excerpt}…")
        } else {
            excerpt
        };
        let _ = writeln!(s, "  Source code:\n{excerpt}");
    }
    s
}

/// Build a context block listing symbol dependencies.
#[must_use]
pub fn deps_context(deps: &[SymbolDependency]) -> String {
    if deps.is_empty() {
        return String::new();
    }
    let mut s = "  Dependencies:\n".to_string();
    for d in deps.iter().take(20) {
        let _ = writeln!(s, "    - {} ({})", d.dependency_name, d.dependency_type);
    }
    s
}

/// Build a context block listing sibling symbols defined in the same file.
///
/// Symbols whose `raw_text` contains the matched symbol's name are shown with
/// their full source body so the LLM can see usages in context.
#[must_use]
pub fn siblings_context(siblings: &[Symbol], symbol_name: &str) -> String {
    if siblings.is_empty() {
        return String::new();
    }
    let mut s = "  Other symbols in the same file:\n".to_string();
    for sib in siblings.iter().take(15) {
        // Include the full source body when the sibling references the symbol
        // by name — this surfaces actual usage patterns.
        let mentions = sib.raw_text.contains(symbol_name);
        if mentions && !sib.raw_text.is_empty() {
            let limit = 800;
            let excerpt: String = sib.raw_text.chars().take(limit).collect();
            let excerpt = if sib.raw_text.chars().count() > limit {
                format!("{excerpt}…")
            } else {
                excerpt
            };
            let _ = writeln!(
                s,
                "    - `{}` ({} {}) lines {}-{} [uses `{}`]:\n{}",
                sib.name,
                sib.language,
                sib.symbol_type,
                sib.line_start,
                sib.line_end,
                symbol_name,
                excerpt
                    .lines()
                    .map(|l| format!("      {l}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        } else {
            let _ = writeln!(
                s,
                "    - `{}` ({} {}) lines {}-{}",
                sib.name, sib.language, sib.symbol_type, sib.line_start, sib.line_end
            );
        }
    }
    s
}

/// Build a compact context block from a slice of [`EvidenceItem`]s.
#[must_use]
pub fn evidence_context(items: &[EvidenceItem]) -> String {
    if items.is_empty() {
        return "No evidence available.\n".to_string();
    }

    let mut s = String::new();
    for (i, item) in items.iter().enumerate() {
        let src_label = match item.source {
            EvidenceSource::Commit => "GIT COMMIT",
            EvidenceSource::Code => "SOURCE CODE",
            EvidenceSource::Blame => "GIT BLAME",
            EvidenceSource::Database => "DATABASE",
        };
        let ref_str = item
            .source_ref
            .as_deref()
            .map(|r| format!(" [ref: {r}]"))
            .unwrap_or_default();
        let _ = writeln!(s, "  {}. [{}]{ref_str} {}", i + 1, src_label, item.content);
    }
    s
}

/// Build a compact context block from an already-rendered [`Explanation`].
#[must_use]
pub fn explanation_context(explanation: &Explanation) -> String {
    format!(
        "Pre-analysis summary: {}\nConfidence: {} ({}/100)\n",
        explanation.summary,
        explanation.confidence,
        explanation.confidence.score(),
    )
}

// ── Ask prompt builder ────────────────────────────────────────────────────────

/// All context gathered for a single matched symbol, to include in the prompt.
pub struct SymbolContext<'a> {
    /// The matched symbol itself.
    pub symbol: &'a Symbol,
    /// The file the symbol lives in (fetched from DB).
    pub file: Option<&'a File>,
    /// The repository the symbol belongs to (fetched from DB).
    pub repo: Option<&'a Repository>,
    /// Dependency edges recorded for this symbol.
    pub deps: &'a [SymbolDependency],
    /// Other symbols defined in the same source file.
    pub siblings: &'a [Symbol],
    /// Ranked evidence items (commits, blame, code, DB).
    pub evidence: &'a [EvidenceItem],
    /// Pre-computed rule-based explanation.
    pub explanation: &'a Explanation,
}

/// Build the full user message for the `ask` command.
///
/// Packs the question plus — for each matched symbol — its repository, file
/// path, full source text, line numbers, doc-comment, dependency names,
/// sibling symbol list, all evidence items, and the pre-computed summary.
#[must_use]
pub fn build_ask_prompt(question: &str, contexts: &[SymbolContext<'_>]) -> ChatMessage {
    let mut body = format!("Question: {question}\n\n");

    if contexts.is_empty() {
        body.push_str("No matching symbols were found in the indexed codebase.\n");
        body.push_str(
            "Please answer based on general software engineering principles if possible, \
             but note that there is no specific evidence available.\n",
        );
    } else {
        let _ = writeln!(
            body,
            "The archaeologus found {} relevant symbol(s). \
             Here is all gathered context:\n",
            contexts.len()
        );

        for (i, ctx) in contexts.iter().enumerate() {
            let _ = writeln!(body, "═══ Symbol {} ═══", i + 1);

            // Repository & file
            if let Some(repo) = ctx.repo {
                body.push_str(&repo_context(repo));
            }
            if let Some(file) = ctx.file {
                body.push_str(&file_context(file));
            }

            // Symbol detail (name, type, location, full source)
            body.push_str(&symbol_context(ctx.symbol, ctx.file));

            // Dependencies
            let dep_block = deps_context(ctx.deps);
            if !dep_block.is_empty() {
                body.push_str(&dep_block);
            }

            // Sibling symbols in the same file (with source bodies for those that use this symbol)
            let sib_block = siblings_context(ctx.siblings, ctx.symbol.name.as_str());
            if !sib_block.is_empty() {
                body.push_str(&sib_block);
            }

            // Pre-computed summary
            body.push_str(&explanation_context(ctx.explanation));

            // Evidence
            body.push_str("Evidence:\n");
            body.push_str(&evidence_context(ctx.evidence));
            body.push('\n');
        }

        body.push_str(
            "Using only the context and evidence above, answer the question. \
             Be specific and cite file paths, line numbers, or commit SHAs where available.",
        );
    }

    ChatMessage::user(body)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use archaeologus_core::models::{File, Repository, Symbol, SymbolDependency};
    use archaeologus_evidence::{EvidenceItem, EvidenceSource};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_symbol(name: &str, doc: Option<&str>, raw: &str) -> Symbol {
        Symbol {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            repository_id: Uuid::new_v4(),
            name: name.to_string(),
            symbol_type: "function".to_string(),
            language: "rust".to_string(),
            line_start: 1,
            line_end: 10,
            col_start: 0,
            col_end: 0,
            visibility: Some("pub".to_string()),
            doc_comment: doc.map(ToString::to_string),
            raw_text: raw.to_string(),
            created_at: Utc::now(),
        }
    }

    fn make_file(path: &str) -> File {
        File {
            id: Uuid::new_v4(),
            repository_id: Uuid::new_v4(),
            path: path.to_string(),
            language: Some("rust".to_string()),
            size_bytes: 512,
            content_hash: "abc".to_string(),
            indexed_at: Utc::now(),
        }
    }

    fn make_repo(name: &str) -> Repository {
        Repository {
            id: Uuid::new_v4(),
            name: name.to_string(),
            url: format!("https://github.com/org/{name}"),
            local_path: None,
            description: Some("A test repository".to_string()),
            default_branch: "main".to_string(),
            indexed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_dep(name: &str, dep_type: &str) -> SymbolDependency {
        SymbolDependency {
            id: Uuid::new_v4(),
            symbol_id: Uuid::new_v4(),
            depends_on_symbol_id: None,
            dependency_name: name.to_string(),
            dependency_type: dep_type.to_string(),
        }
    }

    fn make_evidence_item(source: EvidenceSource, content: &str) -> EvidenceItem {
        EvidenceItem {
            source,
            content: content.to_string(),
            source_ref: Some("abc123".to_string()),
            weight: 2,
        }
    }

    #[test]
    fn system_prompt_is_system_role() {
        use crate::provider::Role;
        let msg = system_prompt();
        assert_eq!(msg.role, Role::System);
        assert!(msg.content.contains("Software Archaeologus"));
    }

    #[test]
    fn symbol_context_includes_name_language_and_location() {
        let sym = make_symbol(
            "authenticate",
            Some("Validates credentials."),
            "fn authenticate() {}",
        );
        let file = make_file("src/auth.rs");
        let ctx = symbol_context(&sym, Some(&file));
        assert!(ctx.contains("authenticate"));
        assert!(ctx.contains("rust"));
        assert!(ctx.contains("Validates credentials."));
        assert!(ctx.contains("src/auth.rs"));
        assert!(ctx.contains("fn authenticate()"));
    }

    #[test]
    fn symbol_context_no_file_uses_line_numbers() {
        let sym = make_symbol("foo", None, "fn foo() {}");
        let ctx = symbol_context(&sym, None);
        assert!(ctx.contains("lines 1-10"));
    }

    #[test]
    fn symbol_context_truncates_long_raw_text() {
        let long = "x".repeat(800);
        let sym = make_symbol("big_fn", None, &long);
        let ctx = symbol_context(&sym, None);
        assert!(ctx.contains('…'));
    }

    #[test]
    fn repo_context_includes_name_and_url() {
        let repo = make_repo("my-service");
        let ctx = repo_context(&repo);
        assert!(ctx.contains("my-service"));
        assert!(ctx.contains("github.com/org/my-service"));
    }

    #[test]
    fn file_context_includes_path() {
        let file = make_file("internal/user/user.go");
        let ctx = file_context(&file);
        assert!(ctx.contains("internal/user/user.go"));
    }

    #[test]
    fn deps_context_empty_returns_empty_string() {
        assert_eq!(deps_context(&[]), "");
    }

    #[test]
    fn deps_context_lists_dependencies() {
        let deps = vec![make_dep("fmt", "import"), make_dep("io.Writer", "call")];
        let ctx = deps_context(&deps);
        assert!(ctx.contains("fmt"));
        assert!(ctx.contains("io.Writer"));
        assert!(ctx.contains("import"));
        assert!(ctx.contains("call"));
    }

    #[test]
    fn siblings_context_empty_returns_empty_string() {
        assert_eq!(siblings_context(&[], "User"), "");
    }

    #[test]
    fn siblings_context_lists_names() {
        let siblings = vec![
            make_symbol("NewUser", None, "func NewUser() {}"),
            make_symbol("DeleteUser", None, "func DeleteUser() {}"),
        ];
        let ctx = siblings_context(&siblings, "User");
        assert!(ctx.contains("NewUser"));
        assert!(ctx.contains("DeleteUser"));
    }

    #[test]
    fn siblings_context_shows_body_for_referencing_symbol() {
        let user = make_symbol("main", None, "func main() { u := User{Name: \"Alice\"} }");
        let ctx = siblings_context(&[user], "User");
        // main references User, so its body should be shown
        assert!(ctx.contains("uses `User`"));
        assert!(ctx.contains("func main()"));
    }

    #[test]
    fn evidence_context_empty_returns_placeholder() {
        let ctx = evidence_context(&[]);
        assert!(ctx.contains("No evidence"));
    }

    #[test]
    fn evidence_context_labels_sources_correctly() {
        let items = vec![
            make_evidence_item(EvidenceSource::Commit, "add timeout logic"),
            make_evidence_item(EvidenceSource::Blame, "Alice authored 5 lines"),
        ];
        let ctx = evidence_context(&items);
        assert!(ctx.contains("GIT COMMIT"));
        assert!(ctx.contains("GIT BLAME"));
        assert!(ctx.contains("abc123"));
    }

    #[test]
    fn build_ask_prompt_contains_question_and_repo() {
        use archaeologus_evidence::{aggregate_evidence, explain_symbol};
        let sym = make_symbol("retry", None, "fn retry() {}");
        let file = make_file("src/retry.rs");
        let repo = make_repo("my-service");
        let evidence = aggregate_evidence(sym.id, Some(&sym), &[], &[], &[]);
        let expl = explain_symbol(&sym.name, &evidence);
        let contexts = vec![SymbolContext {
            symbol: &sym,
            file: Some(&file),
            repo: Some(&repo),
            deps: &[],
            siblings: &[],
            evidence: &evidence,
            explanation: &expl,
        }];
        let msg = build_ask_prompt("why does retry exist?", &contexts);
        assert!(msg.content.contains("why does retry exist?"));
        assert!(msg.content.contains("retry"));
        assert!(msg.content.contains("my-service"));
        assert!(msg.content.contains("src/retry.rs"));
    }

    #[test]
    fn build_ask_prompt_no_symbols_still_contains_question() {
        let msg = build_ask_prompt("what is the architecture?", &[]);
        assert!(msg.content.contains("what is the architecture?"));
        assert!(msg.content.contains("No matching symbols"));
    }
}
