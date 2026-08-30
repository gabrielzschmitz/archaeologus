# Archaeologus

<img align="right" width="192px" src="./resources/icons/icon.png" alt="Archaeologus Logo">

<a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-green" alt="License"></a>
<a href="https://www.buymeacoffee.com/gabrielzschmitz" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: 20px !important;width: 87px;" ></a>
<a href="https://github.com/gabrielzschmitz/archaeologus"><img src="https://img.shields.io/github/stars/gabrielzschmitz/archaeologus?style=social" alt="Give me a Star"></a>

**Archaeologus** answers the question: *"Why is the code like
this?"*

It reconstructs the context, history, decisions, dependencies, and risks behind
a codebase by indexing repositories, analyzing git history, parsing source code
with tree-sitter, and providing evidence-based explanations powered by your
choice of LLM (IBM watsonx.ai by default, plus OpenAI, Anthropic, or Ollama).

---

## Quick Start

### 1. Clone and build

```sh
git clone https://github.com/gabrielzschmitz/archaeologus.git
cd archaeologus
cargo build
```

### 2. Start PostgreSQL and migrate

```sh
docker compose up -d db
cargo run -- migrate
```

### 3. Index a repository and query it

```sh
cargo run -- index https://github.com/rust-lang/rust # Your repository here
cargo run -- explain main
cargo run -- history calculate_hash
cargo run -- search "fn process"
```

---

## Features

- Index remote or local git repositories with full history
- Parse source code in 8 languages (Rust, Python, JS, TS, Go, Java, C, C++)
- Extract symbols (functions, structs, enums, traits) with line-level positions
- Track symbol evolution across commits with blame and diff
- Fuzzy search with pg_trgm for symbol and code search
- Evidence aggregation with confidence scoring
- MCP server for AI client integration (watsonx, Claude, OpenCode, ChatGPT)
- Ask-anything `ask` command that answers questions about the codebase with
  cited evidence
- REST API with OpenAPI/Swagger docs
- CLI-first, with the same engine exposed over the MCP protocol and HTTP

<p align="center">
  <a href="https://www.youtube.com/watch?v=stjPfYCJRC0">
    <img src="./resources/demo.jpg" alt="Archaeologus demonstration" style="width: 584px; padding: 0;">
  </a>
</p>
<p align="center">
  <em>
    Watch the demonstration above to see Archaeologus in action, <br>
    including its core features, AI-powered code analysis, and REST API.
  </em>
</p>

---

## Install dependencies

<details>
<summary><b>Arch Linux</b></summary>

```sh
sudo pacman -S rust docker docker-compose cmake openssl pkg-config base-devel git
sudo systemctl enable --now docker
```
</details>

<details>
<summary><b>Ubuntu / Debian</b></summary>

```sh
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev cmake git docker-compose-plugin
```
</details>

<details>
<summary><b>Fedora / RHEL</b></summary>

```sh
sudo dnf install -y gcc cmake openssl-devel pkg-config git docker-compose-plugin
```
</details>

<details>
<summary><b>macOS</b></summary>

```sh
brew install cmake openssl pkg-config git
export PKG_CONFIG_PATH="/opt/homebrew/opt/openssl/lib/pkgconfig"
```
</details>

---

## Usage

```sh
# Index a repository
cargo run -- index <git-url> [--branch main]

# Explain a symbol's purpose and history
cargo run -- explain <symbol-name>

# Show commit history for a symbol
cargo run -- history <symbol-name>

# Analyze impact of changing a symbol
cargo run -- impact <symbol-name>

# Search symbols or code
cargo run -- search "query" [--symbol-type function] [--language rust]

# Ask a question about the codebase (LLM-powered; falls back to rule-based)
cargo run -- ask "why does this function exist?"

# Start MCP server for AI clients
cargo run -- mcp --transport stdio

# Start the REST API (OpenAPI/Swagger at /docs)
cargo run -- serve

# Run database migrations
cargo run -- migrate
```

---

## Configuration

Copy `.env.example` to `.env`:

```sh
cp .env.example .env
```

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://archaeologus:archaeologus_dev@localhost:5432/archaeologus` | PostgreSQL connection |
| `RUST_LOG` | `info,sqlx=warn` | Log level |
| `LLM_PROVIDER` | `watsonx` | AI provider (`watsonx`, `openai`, `anthropic`, `ollama`) |

---

<details>
<summary><b>Database Schema</b></summary>

10 tables using PostgreSQL 16 with `pg_trgm` for fuzzy search.

| Table | Description |
|-------|-------------|
| `repositories` | Indexed git repos |
| `files` | Source files per repo |
| `symbols` | Extracted symbols (functions, structs, enums, traits) |
| `commits` | Git commits per repo |
| `commit_files` | Files changed per commit |
| `branches` | Git branches per repo |
| `tags` | Git tags per repo |
| `symbol_commits` | Links symbols to commits |
| `symbol_dependencies` | Symbol dependency graph |
| `evidence` | Aggregated evidence for explanations |

```
repositories ──┬── files ──────── symbols ──┬── symbol_commits ──── commits ──── commit_files
               ├── commits                  │
               ├── branches                 └── symbol_dependencies
               ├── tags
               └── evidence
```

</details>

<details>
<summary><b>Project Structure</b></summary>

```
archaeologus/
├── Cargo.toml                  # Workspace root
├── compose.yaml                # Docker Compose (PostgreSQL)
├── Dockerfile                  # Multi-stage build
├── .env.example                # Config template
├── migrations/                 # SQL migrations (10 tables)
└── crates/
    ├── core/                   # Domain types, config, errors
    ├── db/                     # Database access (sqlx)
    ├── git/                    # Git operations (git2)
    ├── indexer/                # Source code parsing (tree-sitter)
    ├── search/                 # Symbol and code search
    ├── evidence/               # Evidence aggregation engine
    ├── llm/                    # LLM provider abstraction (watsonx, OpenAI, …)
    ├── api/                    # REST API (Axum, OpenAPI/Swagger)
    ├── mcp/                    # MCP server exposing tools to AI clients
    └── cli/                    # CLI binary (clap)
```

</details>

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file
for details.
