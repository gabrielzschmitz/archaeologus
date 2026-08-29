# Solution statement

**Archaeologus — an AI that digs through code history**

## The problem

Every software project accumulates a hidden archaeology: undocumented design
decisions, long-dead features, layered workarounds, and dependencies that
survive only because "nobody knew what it was for." New engineers routinely
spend hours, even days, digging through git logs, blame output, and source
files just to answer a single question: *why does this code exist?* Names get
renamed, docstrings decay, and the reasoning behind a module dies when its
original author leaves. The knowledge is buried in the repository itself — but
no tool surfaces it.

## The solution

Archaeologus is an AI Software Archaeologus: a toolchain that indexes a
repository and answers natural-language questions about it with evidence-based
explanations. It combines static analysis, git history mining, and an LLM to
reconstruct *why* a codebase looks the way it does, citing answers to commits,
file paths, and line numbers.

The `index` command parses source into symbols across eight languages using
tree-sitter, and walks git history to record commits, blame, diffs, branches,
tags, and dependencies into a PostgreSQL model. The `search` engine
fuzzy-matches symbols with language and repository filters. The `evidence`
crate aggregates code, commits, blame, and DB data into ranked items with a
confidence score. Finally, the `ask` command bundles per-symbol context into a
prompt sent to an LLM provider — **IBM watsonx.ai** by default, using
`ibm/granite-4-h-small` — with a rule-based fallback when no LLM is configured.
Results are exposed via a CLI, a REST API, and an MCP server.

Crucially, this approach uses **far fewer tokens than handing a code agent the
entire codebase** and asking "why?". Instead of shipping every file to the
model, Archaeologus first curates a small, targeted slice of context in the
database — only the matched symbols, their files and repositories,
dependencies, sibling symbols, and ranked evidence — so the LLM reasons over
what actually matters, not megabytes of untouched source. That keeps each `ask`
prompt tiny, fast, and cheap while still grounding the answer in verifiable,
cited facts.
