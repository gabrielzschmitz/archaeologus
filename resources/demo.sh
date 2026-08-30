#!/usr/bin/env bash

set -e

# ============================================================
# Colors
# ============================================================

RESET="\033[0m"
BOLD="\033[1m"
WHITE="\033[97m"

DATABASE="\033[38;5;39m"
INDEX="\033[38;5;208m"
SEARCH="\033[38;5;45m"
ASK="\033[38;5;141m"
EXPLAIN="\033[38;5;220m"
HISTORY="\033[38;5;81m"
IMPACT="\033[38;5;196m"
MCP="\033[38;5;75m"
SERVE="\033[38;5;27m"

# ============================================================
# Header
# ============================================================

echo
echo -e "${BOLD}${WHITE}"
echo "╔════════════════════════════════════════╗"
echo "║              ARCHAEOLOGUS              ║"
echo "║            Tomato.C CLI demo           ║"
echo "╚════════════════════════════════════════╝"
echo -e "${RESET}"
echo
sleep 3


# ============================================================
# Index
# ============================================================

echo -e "${INDEX}${BOLD}━━━ Index ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

echo -e "${INDEX}Indexing Tomato.C...${RESET}"
cargo run -- index https://github.com/gabrielzschmitz/Tomato.C

echo
sleep 3


# ============================================================
# Search
# ============================================================

echo -e "${SEARCH}${BOLD}━━━ Search ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

echo -e "${SEARCH}Searching Tomato.C source files...${RESET}"
cargo run -- search tomato --mode files

echo -e "${SEARCH}Searching functions...${RESET}"
cargo run -- search timer --symbol-type function

echo
sleep 3


# ============================================================
# Explain
# ============================================================

echo -e "${EXPLAIN}${BOLD}━━━ Explain ━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

echo -e "${EXPLAIN}Explaining the timer implementation...${RESET}"
cargo run -- explain timer

echo
sleep 3


# ============================================================
# Ask
# ============================================================

echo -e "${ASK}${BOLD}━━━ Ask ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

echo -e "${ASK}Asking about log...${RESET}"
cargo run -- ask \
    "How does Tomato.C logging works?"

echo
sleep 3


# ============================================================
# History
# ============================================================

echo -e "${HISTORY}${BOLD}━━━ History ━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

echo -e "${HISTORY}Checking timer history...${RESET}"
cargo run -- history timer

echo
sleep 3


# ============================================================
# Impact
# ============================================================

echo -e "${IMPACT}${BOLD}━━━ Impact ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

echo -e "${IMPACT}Analyzing timer impact...${RESET}"
cargo run -- impact timer

echo
sleep 3


# ============================================================
# REST API
# ============================================================

echo -e "${SERVE}${BOLD}━━━ REST API ━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

API_LOG="$(mktemp)"

cleanup() {
    if [[ -n "${API_PID:-}" ]]; then
        echo
        echo -e "${SERVE}Stopping REST API...${RESET}"
        kill "$API_PID" 2>/dev/null || true
        wait "$API_PID" 2>/dev/null || true
    fi

    rm -f "$API_LOG"
}

trap cleanup EXIT INT TERM

echo -e "${SERVE}Starting REST API server...${RESET}"

cargo run -- serve \
    --addr 127.0.0.1:3000 \
    >"$API_LOG" 2>&1 &

API_PID=$!

echo -e "${SERVE}Waiting for REST API...${RESET}"

for _ in {1..30}; do
    if grep -q "API server listening" "$API_LOG" 2>/dev/null; then
        break
    fi

    if ! kill -0 "$API_PID" 2>/dev/null; then
        echo -e "${SERVE}REST API failed to start:${RESET}"
        cat "$API_LOG"
        exit 1
    fi

    sleep 1
done

if ! kill -0 "$API_PID" 2>/dev/null; then
    echo -e "${SERVE}REST API failed to start:${RESET}"
    cat "$API_LOG"
    exit 1
fi

sleep 3
echo
echo -e "${BOLD}${WHITE}"
echo "╔════════════════════════════════════════╗"
echo "║          REST API IS RUNNING           ║"
echo "╚════════════════════════════════════════╝"
echo -e "${RESET}"
echo
echo -e "${SERVE}API:${RESET}     http://127.0.0.1:3000"
echo -e "${SERVE}Swagger UI:${RESET} http://127.0.0.1:3000/swagger-ui"
echo
echo -e "${WHITE}The API will remain running for the demo.${RESET}"
echo -e "${WHITE}Press Ctrl+C when finished.${RESET}"
echo

wait "$API_PID"
