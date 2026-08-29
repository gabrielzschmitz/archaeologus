//! Prompt templates for archaeologist queries.
//!
//! These templates are used by the CLI `ask` command to construct
//! context-rich messages for any [`LLMProvider`].

use crate::provider::ChatMessage;
use archaeologist_evidence::{EvidenceItem, EvidenceSource, Explanation};
use archaeologist_core::models::Symbol;

// ── System prompt ─────────────────────────────────────────────────────────────

/// Returns the system prompt that establishes the AI's persona.
#[must_use]
pub fn system_prompt() -> ChatMessage {
    ChatMessage::system(
        "You are an AI Software Archaeologist. \
Your job is to help developers understand why their codebase looks the way it does. \
You are given structured evidence gathered from git history, source code analysis, \
commit messages, and blame information. \
\n\n\
Constraints:\n\
- Only reason from the evidence provided. Do NOT invent facts.\n\
- If confidence is low, say so clearly.\n\
- Cite commit SHAs or file references when they are available in the evidence.\n\
- Be concise: lead with the key insight, then support it with evidence.\n\
- Prefer factual git archaeology over speculation.\n\
- When multiple symbols are relevant, address each one.\n\
- Format your response in plain text (no markdown, no headers with #).",
    )
}

// ── Context builders ──────────────────────────────────────────────────────────

/// Build a compact context block from a single [`Symbol`].
#[must_use]
pub fn symbol_context(symbol: &Symbol) -> String {
    let mut s = format!(
        "Symbol: `{}` ({} {}, {})\n",
        symbol.name,
        symbol.language,
        symbol.symbol_type,
        symbol.visibility.as_deref().unwrap_or("unknown visibility")
    );
    if let Some(doc) = &symbol.doc_comment {
        if !doc.trim().is_empty() {
            s.push_str(&format!("  Doc comment: {}\n", doc.trim()));
        }
    }
    let raw = symbol.raw_text.trim();
    if !raw.is_empty() {
        let excerpt: String = raw.chars().take(300).collect();
        let excerpt = if raw.len() > 300 {
            format!("{excerpt}…")
        } else {
            excerpt
        };
        s.push_str(&format!("  Source excerpt: {excerpt}\n"));
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
        s.push_str(&format!(
            "  {}. [{}]{ref_str} {}\n",
            i + 1,
            src_label,
            item.content
        ));
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

/// Context gathered for a single symbol to include in the `ask` prompt.
pub struct SymbolContext<'a> {
    pub symbol: &'a Symbol,
    pub evidence: &'a [EvidenceItem],
    pub explanation: &'a Explanation,
}

/// Build the full user message for the `ask` command.
///
/// Includes:
/// - The natural-language question
/// - Symbol metadata (name, language, type, doc, code excerpt) for each hit
/// - All evidence items (commit messages, blame, code)
/// - The pre-computed summary/confidence from the evidence engine
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
        body.push_str(&format!(
            "The archaeologist found {} relevant symbol(s). \
             Here is the evidence gathered for each:\n\n",
            contexts.len()
        ));

        for (i, ctx) in contexts.iter().enumerate() {
            body.push_str(&format!("--- Symbol {} ---\n", i + 1));
            body.push_str(&symbol_context(ctx.symbol));
            body.push_str(&explanation_context(ctx.explanation));
            body.push_str("Evidence:\n");
            body.push_str(&evidence_context(ctx.evidence));
            body.push('\n');
        }

        body.push_str(
            "Using only the evidence above, answer the question. \
             Be specific and cite commit SHAs or source references where available.",
        );
    }

    ChatMessage::user(body)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use archaeologist_core::models::Symbol;
    use archaeologist_evidence::{EvidenceItem, EvidenceSource};
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
        assert!(msg.content.contains("Software Archaeologist"));
    }

    #[test]
    fn symbol_context_includes_name_and_language() {
        let sym = make_symbol(
            "authenticate",
            Some("Validates credentials."),
            "fn authenticate() {}",
        );
        let ctx = symbol_context(&sym);
        assert!(ctx.contains("authenticate"));
        assert!(ctx.contains("rust"));
        assert!(ctx.contains("Validates credentials."));
    }

    #[test]
    fn symbol_context_truncates_long_raw_text() {
        let long = "x".repeat(400);
        let sym = make_symbol("big_fn", None, &long);
        let ctx = symbol_context(&sym);
        assert!(ctx.contains('…'));
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
    fn build_ask_prompt_contains_question() {
        use archaeologist_evidence::{aggregate_evidence, explain_symbol};
        let sym = make_symbol("retry", None, "fn retry() {}");
        let evidence = aggregate_evidence(sym.id, Some(&sym), &[], &[], &[]);
        let expl = explain_symbol(&sym.name, &evidence);
        let contexts = vec![SymbolContext {
            symbol: &sym,
            evidence: &evidence,
            explanation: &expl,
        }];
        let msg = build_ask_prompt("why does retry exist?", &contexts);
        assert!(msg.content.contains("why does retry exist?"));
        assert!(msg.content.contains("retry"));
    }

    #[test]
    fn build_ask_prompt_no_symbols_still_contains_question() {
        let msg = build_ask_prompt("what is the architecture?", &[]);
        assert!(msg.content.contains("what is the architecture?"));
        assert!(msg.content.contains("No matching symbols"));
    }
}
