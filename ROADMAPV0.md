# AI Software Archaeologist — Implementation Plan V0

## Vision

Build an **AI Software Archaeologist** that answers:

> **"Why is the code like this?"**

Instead of replacing developers, it reconstructs the **context, history, decisions, dependencies, and risks** behind a codebase.

---

## Part 1 — Decisions

### 1.1 Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Performance, safety, ecosystem |
| CLI Framework | clap 4 (derive) | Declarative arg parsing with derive macros |
| Serialization | serde + serde_json | Universal Rust serialization |
| Date-Time | jiff 0.2 | Modern, correct,serde-supported datetime |
| HTTP Client | reqwest 0.12 | For future API integrations (GitHub, etc.) |
| Database | PostgreSQL 16 | JSONB, full-text search, pg_trgm |
| DB Access | sqlx 0.8 | Compile-time checked SQL, async |
| Git Operations | git2 0.21 | libgit2 bindings, full git API |
| Code Parsing | tree-sitter 0.26 | Incremental parsing, 300+ languages |
| Parallelism | rayon 1.10 | Data parallelism, par_iter |
| Benchmarking | criterion 0.5 | Statistical benchmarking with HTML reports |
| OpenAPI Docs | utoipa 5 + axum 0.8 | Compile-time OpenAPI generation |
| Web Framework | axum 0.8 | Type-safe async web framework |
| Dev Environment | Docker + Docker Compose | Reproducible, no Nix dependency |
| Error Handling | thiserror 2 + anyhow 1 | Structured errors + context propagation |
| Logging | tracing 0.1 + tracing-subscriber 0.3 | Structured logging |

### 1.2 Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| MVP Interface | CLI only first | Faster iteration, API added in Phase 2 |
| Search Engine | SQL + pg_trgm | Simpler MVP, PostgreSQL handles search well |
| Code Parsing | Individual tree-sitter grammar crates | No network dependency, compiled in, deterministic |
| Workspace Layout | Virtual workspace with 8 crates | Clear separation of concerns, incremental compilation |
| Error Strategy | anyhow for app code, thiserror for library code | Flexible context in apps, structured in libs |
| Async Runtime | tokio (full features) | Industry standard, sqlx/axum require it |
| Migration Strategy | sqlx migrate (compile-time + runtime) | Schema versioning, offline support |

### 1.3 Language Support (All 8 from Start)

| Language | Grammar Crate | Version |
|----------|--------------|---------|
| Rust | tree-sitter-rust | 0.24 |
| Python | tree-sitter-python | 0.23 |
| JavaScript | tree-sitter-javascript | 0.23 |
| TypeScript | tree-sitter-typescript | 0.23 |
| Go | tree-sitter-go | 0.23 |
| Java | tree-sitter-java | 0.23 |
| C | tree-sitter-c | 0.23 |
| C++ | tree-sitter-cpp | 0.23 |

### 1.4 Docker Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| PostgreSQL Image | postgres:16-alpine | Small, stable, production-ready |
| Compose Version | Compose V2 (compose.yaml) | Modern standard |
| App Container | Multi-stage Dockerfile | Small production images |
| Data Persistence | Named volumes | Data survives container restarts |
| Health Checks | pg_isready | Reliable PostgreSQL readiness check |
| Init Strategy | sqlx migrate run on startup | Automatic schema migration |

---

## Part 2 — Project Structure

```
ai-software-archaeologist/
├── Cargo.toml                          # Virtual workspace root
├── compose.yaml                        # Docker Compose (PostgreSQL + app)
├── Dockerfile                          # Multi-stage Rust build
├── .dockerignore
├── .env                                # DATABASE_URL, RUST_LOG
├── .env.example                        # Template for .env
├── .gitignore
├── ROADMAPV0.md                        # This file
├── migrations/                         # sqlx migrations
│   ├── 20260826000001_create_repositories.up.sql
│   ├── 20260826000001_create_repositories.down.sql
│   ├── 20260826000002_create_files.up.sql
│   ├── 20260826000002_create_files.down.sql
│   ├── 20260826000003_create_symbols.up.sql
│   ├── 20260826000003_create_symbols.down.sql
│   ├── 20260826000004_create_commits.up.sql
│   ├── 20260826000004_create_commits.down.sql
│   ├── 20260826000005_create_commit_files.up.sql
│   ├── 20260826000005_create_commit_files.down.sql
│   ├── 20260826000006_create_branches.up.sql
│   ├── 20260826000006_create_branches.down.sql
│   ├── 20260826000007_create_tags.up.sql
│   ├── 20260826000007_create_tags.down.sql
│   ├── 20260826000008_create_symbol_commits.up.sql
│   ├── 20260826000008_create_symbol_commits.down.sql
│   ├── 20260826000009_create_symbol_dependencies.up.sql
│   ├── 20260826000009_create_symbol_dependencies.down.sql
│   ├── 20260826000010_create_evidence.up.sql
│   └── 20260826000010_create_evidence.down.sql
├── crates/
│   ├── core/                           # Domain types, config, errors
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── config.rs
│   │   │   ├── error.rs
│   │   │   └── models/
│   │   │       ├── mod.rs
│   │   │       ├── repository.rs
│   │   │       ├── file.rs
│   │   │       ├── symbol.rs
│   │   │       ├── commit.rs
│   │   │       ├── evidence.rs
│   │   │       └── query.rs
│   │   └── tests/
│   │       └── model_tests.rs
│   ├── db/                             # Database access layer
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── pool.rs
│   │   │   ├── migrations.rs
│   │   │   └── repositories/
│   │   │       ├── mod.rs
│   │   │       ├── repo_repository.rs
│   │   │       ├── file_repository.rs
│   │   │       ├── symbol_repository.rs
│   │   │       ├── commit_repository.rs
│   │   │       └── evidence_repository.rs
│   │   ├── tests/
│   │   │   └── repository_tests.rs
│   │   └── benches/
│   │       └── query_bench.rs
│   ├── git/                            # Git operations
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── clone.rs
│   │   │   ├── history.rs
│   │   │   ├── blame.rs
│   │   │   └── diff.rs
│   │   └── tests/
│   │       └── git_tests.rs
│   ├── indexer/                        # Source code parsing
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── parser.rs
│   │   │   ├── languages.rs
│   │   │   ├── symbols.rs
│   │   │   └── dependencies.rs
│   │   ├── tests/
│   │   │   └── indexer_tests.rs
│   │   └── benches/
│   │       └── parse_bench.rs
│   ├── search/                         # Search functionality
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── symbol_search.rs
│   │   │   └── code_search.rs
│   │   └── tests/
│   │       └── search_tests.rs
│   ├── evidence/                       # Evidence engine
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── aggregator.rs
│   │   │   ├── confidence.rs
│   │   │   └── explainer.rs
│   │   └── tests/
│   │       └── evidence_tests.rs
│   ├── cli/                            # CLI binary
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── commands/
│   │   │       ├── mod.rs
│   │   │       ├── index.rs
│   │   │       ├── ask.rs
│   │   │       ├── explain.rs
│   │   │       ├── history.rs
│   │   │       ├── impact.rs
│   │   │       └── search.rs
│   │   └── tests/
│   │       └── cli_tests.rs
│   └── api/                            # HTTP API (Phase 2)
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs
│       │   ├── routes/
│       │   │   ├── mod.rs
│       │   │   ├── health.rs
│       │   │   ├── repositories.rs
│       │   │   ├── symbols.rs
│       │   │   ├── search.rs
│       │   │   └── evidence.rs
│       │   ├── openapi.rs
│       │   └── state.rs
│       └── tests/
│           └── api_tests.rs
```

---

## Part 3 — Database Schema

### repositories
```sql
CREATE TABLE repositories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    local_path TEXT,
    description TEXT,
    default_branch TEXT NOT NULL DEFAULT 'main',
    indexed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(url)
);
```

### files
```sql
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    language TEXT,
    size_bytes BIGINT NOT NULL DEFAULT 0,
    content_hash TEXT NOT NULL,
    indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(repository_id, path)
);
CREATE INDEX idx_files_repo ON files(repository_id);
CREATE INDEX idx_files_language ON files(language);
```

### symbols
```sql
CREATE TABLE symbols (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    symbol_type TEXT NOT NULL,
    language TEXT NOT NULL,
    line_start INT NOT NULL,
    line_end INT NOT NULL,
    col_start INT NOT NULL,
    col_end INT NOT NULL,
    visibility TEXT,
    doc_comment TEXT,
    raw_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_symbols_repo ON symbols(repository_id);
CREATE INDEX idx_symbols_file ON symbols(file_id);
CREATE INDEX idx_symbols_name ON symbols(name);
CREATE INDEX idx_symbols_type ON symbols(symbol_type);
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX idx_symbols_name_trgm ON symbols USING gin(name gin_trgm_ops);
```

### commits
```sql
CREATE TABLE commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    sha TEXT NOT NULL,
    author_name TEXT,
    author_email TEXT,
    author_date TIMESTAMPTZ NOT NULL,
    committer_name TEXT,
    committer_email TEXT,
    committer_date TIMESTAMPTZ NOT NULL,
    message TEXT NOT NULL,
    parent_shas TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(repository_id, sha)
);
CREATE INDEX idx_commits_repo ON commits(repository_id);
CREATE INDEX idx_commits_author_date ON commits(author_date);
CREATE INDEX idx_commits_sha ON commits(sha);
```

### commit_files
```sql
CREATE TABLE commit_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_id UUID NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    status TEXT NOT NULL,
    additions INT NOT NULL DEFAULT 0,
    deletions INT NOT NULL DEFAULT 0,
    old_path TEXT
);
CREATE INDEX idx_commit_files_commit ON commit_files(commit_id);
CREATE INDEX idx_commit_files_path ON commit_files(file_path);
```

### branches
```sql
CREATE TABLE branches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    head_sha TEXT NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(repository_id, name)
);
```

### tags
```sql
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    target_sha TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(repository_id, name)
);
```

### symbol_commits
```sql
CREATE TABLE symbol_commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol_id UUID NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    commit_id UUID NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    change_type TEXT NOT NULL,
    UNIQUE(symbol_id, commit_id)
);
CREATE INDEX idx_symbol_commits_symbol ON symbol_commits(symbol_id);
CREATE INDEX idx_symbol_commits_commit ON symbol_commits(commit_id);
```

### symbol_dependencies
```sql
CREATE TABLE symbol_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol_id UUID NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    depends_on_symbol_id UUID REFERENCES symbols(id) ON DELETE SET NULL,
    dependency_name TEXT NOT NULL,
    dependency_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_symbol_deps_symbol ON symbol_dependencies(symbol_id);
CREATE INDEX idx_symbol_deps_depends ON symbol_dependencies(depends_on_symbol_id);
```

### evidence
```sql
CREATE TABLE evidence (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    evidence_type TEXT NOT NULL,
    source_ref TEXT,
    content TEXT NOT NULL,
    confidence TEXT NOT NULL DEFAULT 'UNKNOWN',
    symbol_id UUID REFERENCES symbols(id) ON DELETE SET NULL,
    commit_id UUID REFERENCES commits(id) ON DELETE SET NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_evidence_repo ON evidence(repository_id);
CREATE INDEX idx_evidence_symbol ON evidence(symbol_id);
CREATE INDEX idx_evidence_type ON evidence(evidence_type);
```

---

## Part 4 — Docker Configuration

### compose.yaml
```yaml
services:
  db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: archaeologist
      POSTGRES_PASSWORD: archaeologist_dev
      POSTGRES_DB: archaeologist
    ports:
      - "5432:5432"
    volumes:
      - pg_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U archaeologist -d archaeologist"]
      interval: 5s
      timeout: 5s
      retries: 5

  app:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://archaeologist:archaeologist_dev@db:5432/archaeologist
      RUST_LOG: info
    depends_on:
      db:
        condition: service_healthy
    volumes:
      - ./repos:/app/repos

volumes:
  pg_data:
```

### Dockerfile
```dockerfile
# Stage 1: Builder
FROM rust:1.82-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin archaeologist

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/archaeologist .
COPY migrations ./migrations

EXPOSE 8080
CMD ["./archaeologist", "serve"]
```

### .env
```
DATABASE_URL=postgres://archaeologist:archaeologist_dev@localhost:5432/archaeologist
RUST_LOG=info,sqlx=warn
```

### .env.example
```
DATABASE_URL=postgres://archaeologist:archaeologist_dev@localhost:5432/archaeologist
RUST_LOG=info,sqlx=warn
```

---

## Part 5 — Cargo Configuration

### Root Cargo.toml
```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
# Internal crates
archaeologist-core = { path = "crates/core" }
archaeologist-db = { path = "crates/db" }
archaeologist-git = { path = "crates/git" }
archaeologist-indexer = { path = "crates/indexer" }
archaeologist-search = { path = "crates/search" }
archaeologist-evidence = { path = "crates/evidence" }

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# Date-time
jiff = { version = "0.2", features = ["serde"] }

# HTTP client
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Database
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "tls-rustls",
    "postgres",
    "macros",
    "migrate",
    "uuid",
    "json",
    "offline"
] }

# Git
git2 = { version = "0.21", features = ["vendored-libgit2"] }

# Code parsing
tree-sitter = "0.26"
tree-sitter-rust = "0.24"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-go = "0.23"
tree-sitter-java = "0.23"
tree-sitter-c = "0.23"
tree-sitter-cpp = "0.23"

# Parallelism
rayon = "1.10"

# Benchmarking
criterion = { version = "0.5", features = ["html_reports"] }

# OpenAPI
utoipa = { version = "5", features = ["axum_extras", "uuid"] }
utoipa-swagger-ui = { version = "0.3", features = ["axum"] }

# Web framework
axum = "0.8"

# Error handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
walkdir = "2"
regex = "1"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
pedantic = "warn"
```

---

## Part 6 — MVP Implementation Phases

### MVP-1: Core Domain Types + Database

**Goal:** Establish foundation — types, schema, connection

**Crates:** `core`, `db`

**Tasks:**
1. Create workspace Cargo.toml with all workspace dependencies
2. Create `core` crate with domain models:
   - `repository.rs` — Repository, RepositoryCreate
   - `file.rs` — File, FileCreate
   - `symbol.rs` — Symbol, SymbolCreate, SymbolType enum
   - `commit.rs` — Commit, CommitCreate, CommitFile
   - `evidence.rs` — Evidence, EvidenceType, Confidence
   - `query.rs` — SearchQuery, QueryResult
3. Create `core::config` — AppConfig with DATABASE_URL, RUST_LOG
4. Create `core::error` — AppError enum with thiserror
5. Create all migration SQL files (10 pairs)
6. Create `db` crate:
   - `pool.rs` — create_pool(), PgPool wrapper
   - `migrations.rs` — run_migrations()
   - `repositories/` — one file per table with CRUD operations
7. Create Docker compose.yaml with PostgreSQL
8. Create .env and .env.example
9. Write unit tests for all core models
10. Write integration tests for db repositories (requires running PostgreSQL)

**Tests:**
- Model construction and validation
- Serialization roundtrips (serde)
- Database CRUD operations
- Migration execution

**Benchmarks:** None (foundation only)

---

### MVP-2: Git Operations

**Goal:** Clone repos, walk history, extract blame/diff

**Crates:** `git`

**Tasks:**
1. Create `git` crate
2. Implement `clone.rs`:
   - `clone_repository(url, dest) -> Result<PathBuf>`
   - Support HTTPS and SSH
   - Progress callbacks
3. Implement `history.rs`:
   - `walk_commits(repo_path) -> Vec<CommitInfo>`
   - Extract author, date, message, parents
   - Filter by date range, author
4. Implement `blame.rs`:
   - `blame_file(repo_path, file_path) -> Vec<BlameHunk>`
   - Line-by-line attribution
5. Implement `diff.rs`:
   - `diff_commit(repo_path, sha) -> Vec<CommitFile>`
   - File-level diffs with additions/deletions
   - Detect added/modified/deleted/renamed
6. Write unit tests with test git repositories
7. Write integration tests

**Tests:**
- Clone a test repository
- Walk commit history
- Blame a file
- Diff between commits
- Handle edge cases (empty repos, single commit)

**Benchmarks:**
- Commit walking throughput (1k, 10k, 100k commits)
- Blame performance on large files

---

### MVP-3: Code Indexer

**Goal:** Parse source code, extract symbols, detect dependencies

**Crates:** `indexer`

**Tasks:**
1. Create `indexer` crate
2. Implement `languages.rs`:
   - Language detection from file extension
   - Map extensions to tree-sitter grammars
   - Support all 8 languages
3. Implement `parser.rs`:
   - Parse source code to AST
   - Handle parse errors gracefully
   - Incremental parsing support
4. Implement `symbols.rs`:
   - Extract functions, structs, enums, traits, impls, modules
   - Capture name, type, visibility, doc comments
   - Line/column positions
5. Implement `dependencies.rs`:
   - Detect function calls
   - Detect use/import statements
   - Detect trait implementations
   - Build dependency graph
6. Implement parallel indexing with rayon:
   - `index_directory(path) -> Vec<IndexedFile>`
   - Parallel file processing
   - Progress reporting
7. Write unit tests per language
8. Write benchmarks

**Tests:**
- Parse each of the 8 languages
- Extract symbols from sample files
- Detect dependencies
- Handle syntax errors
- Parallel indexing correctness

**Benchmarks:**
- Parse throughput per language (1k, 10k, 100k lines)
- Symbol extraction latency

---

### MVP-4: Index Command

**Goal:** Wire up clone -> parse -> store pipeline

**Crates:** `cli` (index command)

**Tasks:**
1. Create `cli` crate with clap
2. Implement `commands/index.rs`:
   - Accept repository URL or local path
   - Clone if remote
   - Walk files
   - Parse with indexer
   - Store in database
   - Show progress
3. Implement `main.rs` with subcommand dispatch
4. Wire up:
   - git::clone_repository
   - indexer::index_directory
   - db::repositories::create_repository
   - db::repositories::create_file
   - db::repositories::create_symbol
   - db::repositories::create_commit
5. Handle errors gracefully
6. Write tests

**Tests:**
- Index a local test repository
- Index a remote repository (mock)
- Idempotent indexing (re-index same repo)
- Error handling (invalid repo, parse failures)

---

### MVP-5: Search

**Goal:** Search symbols and code with pg_trgm

**Crates:** `search`

**Tasks:**
1. Create `search` crate
2. Implement `symbol_search.rs`:
   - `search_symbols(query, repo_id) -> Vec<Symbol>`
   - Fuzzy matching with pg_trgm
   - Filter by type, language
   - Rank by relevance
3. Implement `code_search.rs`:
   - `search_code(query, repo_id) -> Vec<File>`
   - Full-text search in raw_text
   - Search in file contents
4. Add search commands to CLI
5. Write tests

**Tests:**
- Fuzzy symbol search
- Exact match search
- Filter by type
- Filter by language
- Pagination

---

### MVP-6: Evidence Engine

**Goal:** Aggregate evidence, compute confidence

**Crates:** `evidence`

**Tasks:**
1. Create `evidence` crate
2. Implement `aggregator.rs`:
   - Collect evidence from commits
   - Collect evidence from code
   - Collect evidence from blame
   - Deduplicate and rank
3. Implement `confidence.rs`:
   - Score evidence by type
   - FACT: direct from commit/PR/code
   - INFERENCE: derived from multiple sources
   - UNKNOWN: no evidence
4. Implement `explainer.rs`:
   - Generate human-readable explanations
   - Include evidence sources
   - Show confidence level
5. Write tests

**Tests:**
- Evidence collection
- Confidence scoring
- Explanation generation
- Edge cases (no evidence, conflicting evidence)

---

### MVP-7: Query Commands

**Goal:** Answer the three killer questions

**Crates:** `cli` (ask, explain, history, impact commands)

**Tasks:**
1. Implement `commands/ask.rs`:
   - Accept natural language question
   - Search for relevant symbols
   - Aggregate evidence
   - Generate explanation
2. Implement `commands/explain.rs`:
   - Accept file path or symbol name
   - Show purpose, origin, author, history
   - Show related commits, tests, dependencies
3. Implement `commands/history.rs`:
   - Accept symbol name
   - Show all commits that touched it
   - Show blame information
   - Show evolution over time
4. Implement `commands/impact.rs`:
   - Accept symbol name
   - Find all callers (direct + indirect)
   - Find all tests
   - Show affected services
   - Estimate risk level
5. Implement `commands/search.rs`:
   - Search symbols
   - Search code
   - Filter options
6. Write tests

**Tests:**
- Ask command with various questions
- Explain command for files and symbols
- History command output
- Impact analysis accuracy
- Search command with filters

---

### MVP-8: HTTP API

**Goal:** Expose functionality via REST API with OpenAPI docs

**Crates:** `api`

**Tasks:**
1. Create `api` crate with axum
2. Implement `state.rs` — AppState with pool
3. Implement `routes/health.rs` — health check endpoint
4. Implement `routes/repositories.rs`:
   - GET /repositories — list
   - POST /repositories — create/index
   - GET /repositories/:id — get one
5. Implement `routes/symbols.rs`:
   - GET /repositories/:id/symbols — list
   - GET /symbols/:id — get one
   - GET /symbols/:id/history — commit history
   - GET /symbols/:id/impact — impact analysis
6. Implement `routes/search.rs`:
   - GET /search?q=... — search
7. Implement `routes/evidence.rs`:
   - GET /evidence?symbol_id=... — get evidence
8. Implement `openapi.rs` — OpenAPI spec with utoipa
9. Implement Swagger UI
10. Write tests

**Tests:**
- All endpoint responses
- OpenAPI spec generation
- Error responses
- Pagination

---

## Part 7 — CLI Interface

```
USAGE: archaeologist [OPTIONS] <COMMAND>

SUBCOMMANDS:
  index       Index a git repository
  ask         Ask a natural language question about a codebase
  explain     Explain a file, function, or symbol
  history     Show the history of a symbol or file
  impact      Analyze the impact of changing a symbol
  search      Search for symbols or code
  serve       Start the HTTP API server

OPTIONS:
  -v, --verbose       Enable verbose output
  -q, --quiet         Suppress non-essential output
  --db-url <URL>      Database connection URL (overrides DATABASE_URL env)
```

### Command Examples

```bash
# Index a repository
archaeologist index /path/to/repo
archaeologist index https://github.com/user/repo --branch main

# Ask a question
archaeologist ask "why does this function exist?"
archaeologist ask "how does authentication work?"

# Explain a file or symbol
archaeologist explain src/auth/token.rs
archaeologist explain src/auth/token.rs:42

# History of a symbol
archaeologist history calculateSessionDuration
archaeologist history --file src/utils.rs calculateSessionDuration

# Impact analysis
archaeologist impact calculateSessionDuration

# Search
archaeologist search "timeout" --type symbol
archaeologist search "database" --type file

# Start API server
archaeologist serve --port 8080
```

---

## Part 8 — Testing Strategy

### Per-Crate Testing

| Crate | Test Type | What to Test |
|-------|-----------|--------------|
| core | Unit | Model construction, validation, serialization |
| db | Integration | CRUD operations, migrations, pool management |
| git | Integration | Clone, history, blame, diff |
| indexer | Unit + Integration | Parsing, symbol extraction, dependencies |
| search | Integration | Search queries, ranking, filters |
| evidence | Unit | Aggregation, confidence scoring |
| cli | Integration | Command parsing, output formatting |
| api | Integration | Endpoint responses, OpenAPI spec |

### Test Commands

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p archaeologist-core
cargo test -p archaeologist-db

# Run with output
cargo test --workspace -- --nocapture

# Run benchmarks
cargo bench --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check
```

### Benchmark Targets

| Crate | Benchmark | What it Measures |
|-------|-----------|------------------|
| db | query_bench | Symbol search, commit lookup |
| indexer | parse_bench | tree-sitter parsing throughput |

---

## Part 9 — Future Roadmap (Post-MVP)

### Phase 2: Code Archaeology + API

- HTTP API with OpenAPI docs (utoipa)
- Swagger UI for API exploration
- IDE integration support
- Enhanced evidence from PRs and issues

### Phase 3: Knowledge Graph

- Entity relationship mapping
- Graph queries
- Visual graph representation

### Phase 4: Impact Analysis

- Full call graph analysis
- Dependency graph
- Risk assessment
- Recommended tests

### Phase 5: Runtime Archaeology

- Log integration
- Metrics collection
- Trace analysis
- Incident correlation

### Phase 6: Architecture Archaeology

- Architecture evolution tracking
- Decision documentation
- Service dependency mapping

### Phase 7: Technical Debt Detection

- TODO/FIXME tracking
- Deprecated API detection
- Dead code identification
- Debt prioritization

### Phase 8: Institutional Memory

- Knowledge preservation
- Decision history
- Team knowledge base

### Phase 9: Living Documentation

- Auto-generated docs
- Continuous verification
- Confidence scoring

### Phase 10: Developer Interfaces

- IDE plugins
- MCP server for AI agents
- Web dashboard

---

## Part 10 — Quick Start

```bash
# 1. Clone the repository
git clone https://github.com/your-org/ai-software-archaeologist.git
cd ai-software-archaeologist

# 2. Start PostgreSQL
docker compose up -d db

# 3. Set up environment
cp .env.example .env

# 4. Build the project
cargo build

# 5. Run migrations
cargo run -- migrate

# 6. Index a repository
cargo run -- index /path/to/your/repo

# 7. Ask a question
cargo run -- ask "why does this function exist?"

# 8. Run tests
cargo test --workspace

# 9. Run benchmarks
cargo bench --workspace

# 10. Start API server (optional)
cargo run -- serve --port 8080
```

---

## Appendix A — Dependency Graph

```
cli ──┬── core
      ├── db ──── core
      ├── git ─── core
      ├── indexer ── core
      ├── search ──┬── core
      │            └── db
      └── evidence ──┬── core
                     ├── db
                     └── git

api ──┬── core
      ├── db
      ├── search
      └── evidence
```

## Appendix B — Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| DATABASE_URL | postgres://localhost:5432/archaeologist | PostgreSQL connection string |
| RUST_LOG | info | Log level (trace, debug, info, warn, error) |
| RUST_BACKTRACE | 1 | Enable backtraces |

## Appendix C — Docker Commands

```bash
# Start PostgreSQL
docker compose up -d

# Stop PostgreSQL
docker compose down

# Stop and remove data
docker compose down -v

# View logs
docker compose logs -f db

# Connect to database
docker compose exec db psql -U archaeologist -d archaeologist

# Rebuild app
docker compose up --build app
```

---

## Part 11 — AI Integration Architecture

### 11.1 Design Goals

| Goal | Description |
|------|-------------|
| IBM First | IBM watsonx (Bob 2.0) is the primary AI for hackathon |
| Interchangeable | Swap between watsonx, OpenCode, ChatGPT/Codex, Claude/Claude Code |
| Tool Access | Each AI has full access to all archaeologist capabilities |
| Standard Protocol | MCP (Model Context Protocol) for universal compatibility |
| Local + Remote | Support both local (stdio) and remote (HTTP) connections |

### 11.2 Architecture Overview

```
                    ┌─────────────────────────────────────┐
                    │        AI Software Archaeologist     │
                    │                                     │
                    │  ┌─────────────────────────────┐   │
                    │  │       MCP Server             │   │
                    │  │  (rmcp crate)                │   │
                    │  │                              │   │
                    │  │  Tools:                       │   │
                    │  │  - index_repository           │   │
                    │  │  - search_symbols             │   │
                    │  │  - explain_symbol             │   │
                    │  │  - get_history                │   │
                    │  │  - analyze_impact             │   │
                    │  │  - get_evidence               │   │
                    │  │  - search_code                │   │
                    │  └──────────┬──────────────────┘   │
                    │             │                       │
                    │  ┌──────────▼──────────────────┐   │
                    │  │    Archaeologist Core        │   │
                    │  │  (db, git, indexer, etc.)    │   │
                    │  └─────────────────────────────┘   │
                    └─────────────────────────────────────┘
                                   │
         ┌─────────────────────────┼─────────────────────────┐
         │                         │                         │
         ▼                         ▼                         ▼
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│  IBM watsonx    │      │   Claude Code   │      │  ChatGPT/Codex  │
│  (Bob 2.0)      │      │   (stdio)       │      │  (HTTP)         │
│  PRIMARY ★      │      │                  │      │                  │
└─────────────────┘      └─────────────────┘      └─────────────────┘
         │                         │                         │
         └─────────────────────────┼─────────────────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │         OpenCode            │
                    │         (stdio)             │
                    └─────────────────────────────┘
```

### 11.3 Provider Matrix (IBM First)

| Priority | Provider | Transport | MCP Support | Tool Access | Use Case |
|----------|----------|-----------|-------------|-------------|----------|
| **★ P0** | **IBM watsonx (Bob 2.0)** | HTTP | ✅ Native | Full | **Hackathon primary** |
| P1 | OpenCode | stdio | ✅ Native | Full | Development |
| P2 | Claude Desktop / Code | stdio | ✅ Native | Full | Development |
| P3 | ChatGPT / Codex | HTTP | Via HTTP | Full | Alternative |
| P4 | Ollama (local) | stdio | ✅ | Full | Offline / demo |

### 11.4 Two-Layer AI Design

```
Layer 1: MCP Server (rmcp) ──→ Exposes tools to EXTERNAL AI clients
         │
         ├── IBM watsonx (Bob 2.0)  ← PRIMARY
         ├── Claude Code
         ├── ChatGPT/Codex
         └── OpenCode

Layer 2: LLM Abstraction (custom) ──→ Powers INTERNAL AI explanations
         │
         ├── IBM watsonx (Bob 2.0)  ← PRIMARY
         ├── OpenAI
         ├── Anthropic
         └── Ollama
```

---

## Part 12 — IBM watsonx (Bob 2.0) Integration — PRIMARY

### 12.1 Why IBM First

| Reason | Detail |
|--------|--------|
| Hackathon requirement | IBM Bob 2.0 is the hackathon platform |
| Enterprise-grade | watsonx is IBM's enterprise AI platform |
| OpenAI-compatible | watsonx has OpenAI-compatible API endpoint |
| MCP support | watsonx supports MCP protocol natively |
| Granite models | IBM Granite models for code understanding |

### 12.2 watsonx Connection Methods

#### Method A: MCP Server (Recommended for hackathon)

The archaeologist runs as an MCP server. watsonx connects as MCP client.

```bash
# Start archaeologist MCP server
archaeologist mcp --transport http --port 8080

# watsonx connects to http://localhost:8080/mcp
# watsonx discovers tools automatically
# watsonx calls tools to answer questions
```

#### Method B: LLM Provider (For internal explanations)

The archaeologist calls watsonx to generate explanations.

```rust
// crates/llm/src/providers/watsonx.rs
pub struct WatsonxProvider {
    client: reqwest::Client,
    api_key: String,
    project_id: String,
    model: String,          // e.g., "ibm/granite-3-3-8b-instruct"
    base_url: String,       // e.g., "https://us-south.ml.cloud.ibm.com"
}

impl WatsonxProvider {
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("WATSONX_API_KEY")?;
        let project_id = std::env::var("WATSONX_PROJECT_ID")?;
        let model = std::env::var("WATSONX_MODEL")
            .unwrap_or_else(|_| "ibm/granite-3-3-8b-instruct".to_string());
        let base_url = std::env::var("WATSONX_BASE_URL")
            .unwrap_or_else(|_| "https://us-south.ml.cloud.ibm.com".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            project_id,
            model,
            base_url,
        })
    }

    /// Authenticate with IBM Cloud IAM
    async fn get_token(&self) -> Result<String, LLMError> {
        let resp = self.client
            .post("https://iam.cloud.ibm.com/identity/token")
            .form(&[
                ("grant_type", "urn:ibm:params:oauth:grant-type:apikey"),
                ("apikey", &self.api_key),
            ])
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        data["access_token"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LLMError::AuthError("Failed to get IAM token".into()))
    }

    /// Chat completion using watsonx text generation API
    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse, LLMError> {
        let token = self.get_token().await?;

        let body = serde_json::json!({
            "model_id": self.model,
            "project_id": self.project_id,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "parameters": {
                "temperature": temperature.unwrap_or(0.7),
                "max_new_tokens": max_tokens.unwrap_or(4096),
            },
        });

        let resp = self.client
            .post(format!(
                "{}/ml/v1/text/generation?version=2025-02-06",
                self.base_url
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let content = data["results"][0]["generated_text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            content,
            tokens_used: data["results"][0]["input_token_count"]
                .as_u64()
                .map(|v| v as u32),
            model: self.model.clone(),
        })
    }

    /// Chat with tool calling support
    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        temperature: Option<f32>,
    ) -> Result<ChatResponse, LLMError> {
        let token = self.get_token().await?;

        let body = serde_json::json!({
            "model_id": self.model,
            "project_id": self.project_id,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "tools": tools.iter().map(|t| serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                },
            })).collect::<Vec<_>>(),
            "parameters": {
                "temperature": temperature.unwrap_or(0.7),
                "max_new_tokens": 4096,
            },
        });

        let resp = self.client
            .post(format!(
                "{}/ml/v1/text/generation?version=2025-02-06",
                self.base_url
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let content = data["results"][0]["generated_text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            content,
            tokens_used: data["results"][0]["input_token_count"]
                .as_u64()
                .map(|v| v as u32),
            model: self.model.clone(),
        })
    }
}
```

### 12.3 watsonx Configuration

```bash
# IBM watsonx (Bob 2.0) — PRIMARY
WATSONX_API_KEY=your-ibm-api-key
WATSONX_PROJECT_ID=your-project-id
WATSONX_MODEL=ibm/granite-3-3-8b-instruct
WATSONX_BASE_URL=https://us-south.ml.cloud.ibm.com

# Or use Dallas region
# WATSONX_BASE_URL=https://dal.ml.cloud.ibm.com

# Or use London region
# WATSONX_BASE_URL=https://eu-gb.ml.cloud.ibm.com
```

### 12.4 watsonx Hackathon Demo

```bash
# 1. Set watsonx credentials
export WATSONX_API_KEY="your-key"
export WATSONX_PROJECT_ID="your-project"
export LLM_PROVIDER=watsonx

# 2. Index a repository
cargo run -- index https://github.com/your-org/your-repo

# 3. Ask with watsonx
cargo run -- ask "why does this function exist?"

# 4. Or start MCP server for watsonx to connect
cargo run -- mcp --transport http --port 8080
# watsonx connects to http://localhost:8080/mcp
```

---

## Part 13 — MCP Server Implementation

### 13.1 New Crate: `mcp`

```
crates/mcp/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── server.rs          # MCP server setup
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── index.rs       # index_repository tool
│   │   ├── search.rs      # search_symbols, search_code tools
│   │   ├── explain.rs     # explain_symbol tool
│   │   ├── history.rs     # get_history tool
│   │   ├── impact.rs      # analyze_impact tool
│   │   ├── evidence.rs    # get_evidence tool
│   │   └── repos.rs       # list_repositories tool
│   └── transport/
│       ├── mod.rs
│       ├── stdio.rs       # stdio transport (local)
│       └── http.rs        # HTTP transport (remote)
└── tests/
    └── mcp_tests.rs
```

### 13.2 Cargo.toml

```toml
[package]
name = "archaeologist-mcp"
version.workspace = true
edition.workspace = true

[dependencies]
archaeologist-core = { workspace = true }
archaeologist-db = { workspace = true }
archaeologist-git = { workspace = true }
archaeologist-indexer = { workspace = true }
archaeologist-search = { workspace = true }
archaeologist-evidence = { workspace = true }

rmcp = { version = "3", features = [
    "server",
    "macros",
    "transport-io",
    "transport-streamable-http-server",
    "schemars",
] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
uuid = { workspace = true }
```

### 13.3 MCP Tool Definitions

```rust
// crates/mcp/src/tools/mod.rs

use rmcp::tool;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Symbol {
    pub id: uuid::Uuid,
    pub name: String,
    pub symbol_type: String,
    pub file_path: String,
    pub line_start: i32,
    pub line_end: i32,
    pub language: String,
    pub visibility: Option<String>,
    pub doc_comment: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Explanation {
    pub symbol: Symbol,
    pub purpose: String,
    pub origin: OriginInfo,
    pub history: Vec<CommitInfo>,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub evidence: Vec<Evidence>,
    pub confidence: String,
}

#[derive(Serialize, Deserialize)]
pub struct OriginInfo {
    pub author: String,
    pub created_at: String,
    pub first_commit: String,
    pub commit_message: String,
}

#[derive(Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub files_changed: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: String,
    pub content: String,
    pub source_ref: Option<String>,
    pub confidence: String,
}

#[derive(Serialize, Deserialize)]
pub struct ImpactReport {
    pub symbol: Symbol,
    pub direct_callers: Vec<Symbol>,
    pub indirect_callers: Vec<Symbol>,
    pub affected_tests: Vec<String>,
    pub risk_level: String,
    pub recommended_tests: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CodeMatch {
    pub file_path: String,
    pub line: i32,
    pub content: String,
    pub context: String,
}
```

### 13.4 Server Implementation

```rust
// crates/mcp/src/server.rs
use rmcp::{ServerHandler, ServiceExt, tool, model::{ServerCapabilities, ServerInfo}};
use std::sync::Arc;
use sqlx::PgPool;

#[derive(Clone)]
pub struct ArchaeologistServer {
    pub pool: PgPool,
}

#[rmcp::server]
impl ServerHandler for ArchaeologistServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "ai-software-archaeologist".to_string(),
            version: "0.1.0".to_string(),
            instructions: Some(
                "AI Software Archaeologist - answers 'why is the code like this?'\n\
                 Index repositories, search symbols, analyze history, and understand code.".to_string()
            ),
        }
    }

    #[tool(description = "Index a git repository for analysis")]
    async fn index_repository(
        &self,
        url: String,
        branch: Option<String>,
    ) -> Result<String, rmcp::ErrorData> {
        let path = archaeologist_git::clone_repository(&url, "/tmp/repos").await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let files = archaeologist_indexer::index_directory(&path).await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        Ok(format!("Indexed {} files from {}", files.len(), url))
    }

    #[tool(description = "Search for symbols (functions, structs, enums, traits) in the codebase")]
    async fn search_symbols(
        &self,
        query: String,
        repository_id: Option<uuid::Uuid>,
        symbol_type: Option<String>,
        language: Option<String>,
    ) -> Result<String, rmcp::ErrorData> {
        let results = archaeologist_search::search_symbols(
            &self.pool, &query, repository_id, symbol_type, language
        ).await.map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&results)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    #[tool(description = "Explain a symbol's purpose, origin, author, history, and dependencies")]
    async fn explain_symbol(
        &self,
        symbol_name: String,
        file_path: Option<String>,
        repository_id: Option<uuid::Uuid>,
    ) -> Result<String, rmcp::ErrorData> {
        let explanation = archaeologist_evidence::explain_symbol(
            &self.pool, &symbol_name, file_path.as_deref(), repository_id
        ).await.map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&explanation)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    #[tool(description = "Get commit history for a symbol or file")]
    async fn get_history(
        &self,
        target: String,
        repository_id: Option<uuid::Uuid>,
        limit: Option<u32>,
    ) -> Result<String, rmcp::ErrorData> {
        let history = archaeologist_evidence::get_history(
            &self.pool, &target, repository_id, limit.unwrap_or(50)
        ).await.map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&history)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    #[tool(description = "Analyze the impact of changing a symbol (callers, tests, risk)")]
    async fn analyze_impact(
        &self,
        symbol_name: String,
        repository_id: Option<uuid::Uuid>,
    ) -> Result<String, rmcp::ErrorData> {
        let impact = archaeologist_evidence::analyze_impact(
            &self.pool, &symbol_name, repository_id
        ).await.map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&impact)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    #[tool(description = "Get evidence for why a piece of code exists")]
    async fn get_evidence(
        &self,
        symbol_name: String,
        repository_id: Option<uuid::Uuid>,
    ) -> Result<String, rmcp::ErrorData> {
        let evidence = archaeologist_evidence::get_evidence(
            &self.pool, &symbol_name, repository_id
        ).await.map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&evidence)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    #[tool(description = "Search code content across the codebase")]
    async fn search_code(
        &self,
        query: String,
        repository_id: Option<uuid::Uuid>,
        language: Option<String>,
    ) -> Result<String, rmcp::ErrorData> {
        let results = archaeologist_search::search_code(
            &self.pool, &query, repository_id, language
        ).await.map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&results)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    #[tool(description = "List all indexed repositories")]
    async fn list_repositories(&self) -> Result<String, rmcp::ErrorData> {
        let repos = archaeologist_db::repositories::list_repositories(&self.pool).await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&repos)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
}
```

### 13.5 Transport Selection

```rust
// crates/mcp/src/main.rs
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("archaeologist=info".parse()?)
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let pool = archaeologist_db::create_pool(&database_url).await?;
    archaeologist_db::run_migrations(&pool).await?;

    let server = ArchaeologistServer { pool };

    let transport = std::env::var("MCP_TRANSPORT")
        .unwrap_or_else(|_| "stdio".to_string());

    match transport.as_str() {
        "stdio" => {
            tracing::info!("Starting MCP server in stdio mode");
            let service = server.serve(rmcp::transport::io::stdio()).await?;
            service.waiting().await?;
        }
        "http" => {
            let addr = std::env::var("MCP_HTTP_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string());
            tracing::info!("Starting MCP server in HTTP mode on {}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            let service = server.serve(
                rmcp::transport::streamable_http_server::StreamableHttpService::new(listener)
            ).await?;
            service.waiting().await?;
        }
        _ => {
            eprintln!("Unknown transport: {}. Use 'stdio' or 'http'", transport);
            std::process::exit(1);
        }
    }

    Ok(())
}
```

---

## Part 14 — LLM Provider Abstraction (for Internal AI)

### 14.1 Purpose

The MCP server exposes tools to external AI models. But the archaeologist also needs to **generate AI-powered explanations** internally (e.g., when the CLI `ask` command needs to synthesize evidence into a natural language answer).

### 14.2 New Crate: `llm`

```
crates/llm/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── provider.rs        # LLMProvider trait
│   ├── providers/
│   │   ├── mod.rs
│   │   ├── watsonx.rs     # IBM watsonx — PRIMARY
│   │   ├── openai.rs      # OpenAI / ChatGPT
│   │   ├── anthropic.rs   # Anthropic / Claude
│   │   ├── ollama.rs      # Local models
│   │   └── mock.rs        # For testing
│   ├── config.rs          # Provider configuration
│   └── prompt.rs          # Prompt templates
└── tests/
    └── llm_tests.rs
```

### 14.3 Cargo.toml

```toml
[package]
name = "archaeologist-llm"
version.workspace = true
edition.workspace = true

[dependencies]
archaeologist-core = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
async-trait = "0.1"
```

### 14.4 Provider Trait

```rust
// crates/llm/src/provider.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub tokens_used: Option<u32>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    Watsonx,
    OpenAI,
    Anthropic,
    Ollama,
    Mock,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Watsonx => write!(f, "watsonx"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::Mock => write!(f, "mock"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = LLMError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "watsonx" | "ibm" => Ok(ProviderType::Watsonx),
            "openai" | "chatgpt" => Ok(ProviderType::OpenAI),
            "anthropic" | "claude" => Ok(ProviderType::Anthropic),
            "ollama" | "local" => Ok(ProviderType::Ollama),
            "mock" | "test" => Ok(ProviderType::Mock),
            _ => Err(LLMError::ProviderUnavailable(format!("Unknown provider: {}", s))),
        }
    }
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn provider_type(&self) -> ProviderType;

    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse, LLMError>;

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        temperature: Option<f32>,
    ) -> Result<ChatResponse, LLMError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),
}
```

### 14.5 Provider Factory

```rust
// crates/llm/src/config.rs
use super::provider::*;
use super::providers::*;

pub struct LLMConfig {
    pub provider_type: ProviderType,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl LLMConfig {
    pub fn from_env() -> Result<Self, LLMError> {
        let provider_type = std::env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "watsonx".to_string())  // Default to watsonx
            .parse()?;

        Ok(Self {
            provider_type,
            model: std::env::var("LLM_MODEL").ok(),
            temperature: std::env::var("LLM_TEMPERATURE")
                .ok()
                .and_then(|v| v.parse().ok()),
            max_tokens: std::env::var("LLM_MAX_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok()),
        })
    }
}

pub fn create_provider(config: &LLMConfig) -> Result<Box<dyn LLMProvider>, LLMError> {
    match config.provider_type {
        ProviderType::Watsonx => Ok(Box::new(WatsonxProvider::from_env()?)),
        ProviderType::OpenAI => Ok(Box::new(OpenAIProvider::from_env()?)),
        ProviderType::Anthropic => Ok(Box::new(AnthropicProvider::from_env()?)),
        ProviderType::Ollama => Ok(Box::new(OllamaProvider::from_env()?)),
        ProviderType::Mock => Ok(Box::new(MockProvider::new())),
    }
}
```

---

## Part 15 — Other AI Providers (Secondary)

### 15.1 OpenAI / ChatGPT

```rust
// crates/llm/src/providers/openai.rs
pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| LLMError::AuthError("OPENAI_API_KEY not set".into()))?;
        let model = std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-4o".to_string());
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        Ok(Self { client: reqwest::Client::new(), api_key, model, base_url })
    }
}
```

**Config:**
```bash
LLM_PROVIDER=openai
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o
```

### 15.2 Anthropic / Claude

```rust
// crates/llm/src/providers/anthropic.rs
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| LLMError::AuthError("ANTHROPIC_API_KEY not set".into()))?;
        let model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

        Ok(Self { client: reqwest::Client::new(), api_key, model })
    }
}
```

**Config:**
```bash
LLM_PROVIDER=anthropic
ANTHROPIC_API_KEY=sk-ant-...
ANTHROPIC_MODEL=claude-sonnet-4-20250514
```

### 15.3 Ollama (Local)

```rust
// crates/llm/src/providers/ollama.rs
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn from_env() -> Result<Self, LLMError> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "llama3".to_string());

        Ok(Self { client: reqwest::Client::new(), base_url, model })
    }
}
```

**Config:**
```bash
LLM_PROVIDER=ollama
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3
```

---

## Part 16 — Connecting AI Clients

### 16.1 IBM watsonx (Bob 2.0) — PRIMARY

**Option A: watsonx connects to archaeologist MCP server**
```bash
# Start archaeologist MCP server
archaeologist mcp --transport http --port 8080

# watsonx connects to http://localhost:8080/mcp
# watsonx discovers and calls archaeologist tools
```

**Option B: archaeologist calls watsonx for explanations**
```bash
export WATSONX_API_KEY="your-key"
export WATSONX_PROJECT_ID="your-project"
export LLM_PROVIDER=watsonx

archaeologist ask "why does this function exist?"
# archaeologist calls watsonx to generate explanation
```

### 16.2 OpenCode

Add to `.opencode/config.json`:
```json
{
  "mcpServers": {
    "archaeologist": {
      "command": "archaeologist",
      "args": ["mcp", "--transport", "stdio"],
      "env": {
        "DATABASE_URL": "postgres://localhost:5432/archaeologist"
      }
    }
  }
}
```

### 16.3 Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "archaeologist": {
      "command": "archaeologist",
      "args": ["mcp", "--transport", "stdio"],
      "env": {
        "DATABASE_URL": "postgres://localhost:5432/archaeologist"
      }
    }
  }
}
```

### 16.4 Claude Code

Add to `.claude/settings.json`:
```json
{
  "mcpServers": {
    "archaeologist": {
      "command": "archaeologist",
      "args": ["mcp", "--transport", "stdio"]
    }
  }
}
```

### 16.5 ChatGPT / Codex

Start MCP server in HTTP mode:
```bash
archaeologist mcp --transport http --port 8080
```
Configure ChatGPT to connect to `http://localhost:8080/mcp`.

---

## Part 17 — Updated MVP Phases

### MVP-9: MCP Server

**Goal:** Expose archaeologist tools via MCP protocol

**Crates:** `mcp`

**Tasks:**
1. Create `mcp` crate with rmcp dependency
2. Implement MCP server with all 8 tools
3. Implement stdio transport (local AI clients)
4. Implement HTTP transport (remote/watsonx)
5. Add `mcp` subcommand to CLI
6. Write unit tests for each tool
7. Write integration tests with mock AI client

**Tests:**
- MCP server startup (stdio and HTTP)
- Tool discovery (list all tools)
- Tool execution (each tool)
- Error handling
- Transport switching

---

### MVP-10: LLM Integration

**Goal:** AI-powered explanations, IBM watsonx primary

**Crates:** `llm`

**Tasks:**
1. Create `llm` crate
2. Implement `LLMProvider` trait
3. Implement WatsonxProvider (PRIMARY)
4. Implement OpenAIProvider (secondary)
5. Implement AnthropicProvider (secondary)
6. Implement OllamaProvider (local fallback)
7. Implement MockProvider (testing)
8. Implement provider factory with env selection
9. Add prompt templates for archaeologist queries
10. Wire up to CLI `ask` command
11. Write unit tests for each provider
12. Write integration tests

**Tests:**
- Provider initialization from env
- Chat completion (each provider)
- Error handling (auth, network, rate limit)
- Provider switching via env var
- Prompt template rendering
- Mock provider for CI

---

## Part 18 — Updated Dependency Graph

```
cli ──┬── core
      ├── db ──── core
      ├── git ─── core
      ├── indexer ── core
      ├── search ──┬── core
      │            └── db
      ├── evidence ──┬── core
      │              ├── db
      │              └── git
      ├── llm ──── core          # NEW: LLM provider abstraction
      └── mcp ────┬── core       # NEW: MCP server
                  ├── db
                  ├── git
                  ├── indexer
                  ├── search
                  └── evidence

api ──┬── core
      ├── db
      ├── search
      └── evidence
```

---

## Part 19 — Complete Environment Variables

```bash
# Database
DATABASE_URL=postgres://archaeologist:archaeologist_dev@localhost:5432/archaeologist

# Logging
RUST_LOG=info,sqlx=warn
RUST_BACKTRACE=1

# LLM Provider Selection (IBM watsonx is DEFAULT)
LLM_PROVIDER=watsonx          # watsonx | openai | anthropic | ollama
LLM_MODEL=ibm/granite-3-3-8b-instruct
LLM_TEMPERATURE=0.7
LLM_MAX_TOKENS=4096

# IBM watsonx (Bob 2.0) — PRIMARY ★
WATSONX_API_KEY=your-ibm-api-key
WATSONX_PROJECT_ID=your-project-id
WATSONX_MODEL=ibm/granite-3-3-8b-instruct
WATSONX_BASE_URL=https://us-south.ml.cloud.ibm.com

# OpenAI (secondary)
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o
OPENAI_BASE_URL=https://api.openai.com/v1

# Anthropic (secondary)
ANTHROPIC_API_KEY=sk-ant-...
ANTHROPIC_MODEL=claude-sonnet-4-20250514

# Ollama (local fallback)
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3

# MCP Server
MCP_TRANSPORT=stdio          # stdio | http
MCP_HTTP_ADDR=0.0.0.0:8080
```

---

## Part 20 — Hackathon Demo Flow (IBM Bob 2.0)

### Setup

```bash
# 1. Start PostgreSQL
docker compose up -d db

# 2. Set watsonx credentials
export WATSONX_API_KEY="your-key"
export WATSONX_PROJECT_ID="your-project"
export LLM_PROVIDER=watsonx

# 3. Build the project
cargo build

# 4. Run migrations
cargo run -- migrate

# 5. Index a repository
cargo run -- index https://github.com/your-org/your-repo

# 6. Start MCP server (for watsonx to connect)
cargo run -- mcp --transport http --port 8080
```

### Demo Script

1. **Show CLI with watsonx**
   ```bash
   # Explain a symbol using watsonx
   cargo run -- explain src/auth/token.rs

   # Ask a question using watsonx
   cargo run -- ask "why does this function exist?"

   # Show history
   cargo run -- history calculateSessionDuration
   ```

2. **Show MCP connection to watsonx**
   - Start MCP server: `cargo run -- mcp --transport http --port 8080`
   - Configure watsonx to connect to `http://localhost:8080/mcp`
   - Ask watsonx: "How does authentication work in this codebase?"
   - watsonx uses archaeologist tools to answer

3. **Show provider switching**
   ```bash
   # Use watsonx
   LLM_PROVIDER=watsonx cargo run -- ask "why is this timeout 30 seconds?"

   # Switch to OpenAI
   LLM_PROVIDER=openai cargo run -- ask "why is this timeout 30 seconds?"

   # Switch to local Ollama
   LLM_PROVIDER=ollama cargo run -- ask "why is this timeout 30 seconds?"
   ```

4. **Show interchangeability**
   - Same question, different provider
   - Results are consistent (evidence-based, not provider-dependent)
   - Evidence comes from code/commits, not AI hallucination

---

## Part 21 — Updated Cargo Workspace Dependencies

```toml
# Add to workspace Cargo.toml
[workspace.dependencies]
# Internal crates
archaeologist-core = { path = "crates/core" }
archaeologist-db = { path = "crates/db" }
archaeologist-git = { path = "crates/git" }
archaeologist-indexer = { path = "crates/indexer" }
archaeologist-search = { path = "crates/search" }
archaeologist-evidence = { path = "crates/evidence" }
archaeologist-mcp = { path = "crates/mcp" }       # NEW
archaeologist-llm = { path = "crates/llm" }       # NEW

# MCP
rmcp = { version = "3", features = [
    "server",
    "macros",
    "transport-io",
    "transport-streamable-http-server",
    "schemars",
] }

# Async trait
async-trait = "0.1"

# ... existing dependencies ...
```

---

## Appendix D — AI Integration Dependencies

```toml
# crates/mcp/Cargo.toml
[dependencies]
rmcp = { version = "3", features = [
    "server",
    "macros",
    "transport-io",
    "transport-streamable-http-server",
    "schemars",
] }

# crates/llm/Cargo.toml
[dependencies]
async-trait = "0.1"
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
```

## Appendix E — AI Integration Commands

```bash
# Start MCP server (stdio mode for local AI clients)
archaeologist mcp --transport stdio

# Start MCP server (HTTP mode for watsonx and remote clients)
archaeologist mcp --transport http --port 8080

# Use different LLM provider (IBM watsonx is default)
LLM_PROVIDER=watsonx archaeologist ask "why does this exist?"
LLM_PROVIDER=openai archaeologist ask "why does this exist?"
LLM_PROVIDER=anthropic archaeologist ask "why does this exist?"
LLM_PROVIDER=ollama archaeologist ask "why does this exist?"

# IBM watsonx specific
WATSONX_API_KEY=... WATSONX_PROJECT_ID=... cargo run -- ask "question"
```
