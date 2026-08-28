# AI Software Archaeologist

> **"Why is the code like this?"**

An AI-powered tool that reconstructs the context, history, decisions, dependencies, and risks behind a codebase.

---

## Prerequisites

### Required

| Tool | Version | Install |
|------|---------|---------|
| **Rust** | 1.82+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Docker** | 24+ | [docs.docker.com/get-docker](https://docs.docker.com/get-docker/) |
| **Docker Compose** | V2 | Included with Docker Desktop or `docker-compose-plugin` |
| **Git** | 2.30+ | Usually pre-installed |
| **CMake** | 3.x | Required by `git2` (libgit2 vendored build) |
| **OpenSSL dev** | 1.1+ | Required by `reqwest` / TLS |

### Verify installation

```bash
rustc --version    # rustc 1.82.0 or later
cargo --version    # cargo 1.82.0 or later
docker --version   # Docker 24+
docker compose version  # Docker Compose V2
git --version      # git 2.30+
cmake --version    # cmake 3.x
```

### System-specific install

**Arch Linux:**
```bash
sudo pacman -S rust docker docker-compose cmake openssl pkg-config base-devel git
sudo systemctl enable --now docker
```

If `docker compose` (V2 plugin) isn't available:
```bash
sudo pacman -S docker-compose
# or
sudo pacman -S docker-buildx
```

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev cmake git
# Docker Compose V2 plugin
sudo apt install docker-compose-plugin
```

**Fedora/RHEL:**
```bash
sudo dnf install -y gcc cmake openssl-devel pkg-config git
sudo dnf install docker-compose-plugin
```

**macOS:**
```bash
brew install cmake openssl pkg-config git
export PKG_CONFIG_PATH="/opt/homebrew/opt/openssl/lib/pkgconfig"
```

---

## Quick Start

### 1. Clone and build

```bash
git clone https://github.com/gabrielzschmitz/archaeologist.git
cd archaeologist
cargo build
```

### 2. Start PostgreSQL

```bash
docker compose up -d db
```

Wait for healthy:
```bash
docker compose ps   # db should show "healthy"
```

### 3. Run migrations

```bash
cargo run -- migrate
```

### 4. Index a repository

```bash
cargo run -- index https://github.com/rust-lang/rust
```

### 5. Query the codebase

```bash
cargo run -- explain main
cargo run -- history calculate_hash
cargo run -- search "fn process"
```

---

## Configuration

### Environment variables

Copy `.env.example` to `.env` and edit:

```bash
cp .env.example .env
```

**Required:**
```bash
DATABASE_URL=postgres://archaeologist:archaeologist_dev@localhost:5432/archaeologist
```

**Optional:**
```bash
RUST_LOG=info,sqlx=warn
```

---

## CLI Commands

```bash
# Index a repository
cargo run -- index <git-url> [--branch main]

# Explain a symbol
cargo run -- explain <symbol-name>

# Show commit history
cargo run -- history <symbol-name>

# Analyze impact
cargo run -- impact <symbol-name>

# Search symbols
cargo run -- search "query" [--symbol-type function] [--language rust]

# Run database migrations
cargo run -- migrate
```

---

## Docker

### Start database only

```bash
docker compose up -d db
```

### View logs

```bash
docker compose logs -f db
```

### Stop and clean

```bash
docker compose down -v
```

### Reset database

```bash
docker compose down -v
docker compose up -d db
cargo run -- migrate
```

---

## Development

```bash
cargo build            # Build
cargo build --release  # Release build
cargo test             # Run tests
cargo clippy           # Lint
cargo fmt              # Format
```

### Database access

```bash
docker compose exec db psql -U archaeologist -d archaeologist
```

---

## Troubleshooting

### Docker Compose V2 not found

```
unknown shorthand flag: 'd' in -d
```

The `docker compose` subcommand (V2 plugin) isn't installed. Fix:

```bash
# Arch Linux
sudo pacman -S docker-compose

# Ubuntu/Debian
sudo apt install docker-compose-plugin

# Then verify
docker compose version
```

If you can't install the plugin, use the standalone binary:
```bash
docker-compose up -d db
```

### OpenSSL not found

```
Could not find directory of OpenSSL installation
```

```bash
# Arch Linux
sudo pacman -S openssl pkg-config

# Ubuntu/Debian
sudo apt install libssl-dev pkg-config

# macOS
brew install openssl
export PKG_CONFIG_PATH="/opt/homebrew/opt/openssl/lib/pkgconfig"
```

### CMake not found

```
CMake not found
```

```bash
# Arch Linux
sudo pacman -S cmake

# Ubuntu/Debian
sudo apt install cmake

# macOS
brew install cmake
```

### Docker connection refused

```
Connection refused (os error 111)
```

PostgreSQL isn't running yet:
```bash
docker compose up -d db
docker compose ps   # check health status
```

### cargo run: no binary found

```
error: `cargo run` could not determine which binary to run
```

The workspace has one binary (`archaeologist`). If this error occurs:
```bash
cargo run --bin archaeologist -- <command>
```

---

## Project Structure

```
ai-software-archaeologist/
├── Cargo.toml                  # Workspace root
├── compose.yaml                # Docker Compose (PostgreSQL)
├── Dockerfile                  # Multi-stage build
├── .env                        # Local config (git-ignored)
├── .env.example                # Config template
├── .gitignore
├── migrations/                 # SQL migrations (10 tables)
├── crates/
│   ├── core/                   # Domain types, config, errors
│   ├── db/                     # Database access (sqlx)
│   ├── git/                    # Git operations (git2)
│   ├── indexer/                 # Source code parsing (tree-sitter)
│   ├── search/                 # Search functionality
│   ├── evidence/               # Evidence engine
│   └── cli/                    # CLI binary (clap)
└── ROADMAPV0.md                # Implementation plan
```

---

## License

MIT
