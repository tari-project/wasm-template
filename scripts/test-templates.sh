#!/usr/bin/env bash
set -euo pipefail

NO_BAIL=false
NO_CLEAN=false
FILTER=""
OUTPUT_DIR=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-bail) NO_BAIL=true; shift ;;
        --no-clean) NO_CLEAN=true; shift ;;
        --template) FILTER="$2"; shift 2 ;;
        --template=*) FILTER="${1#--template=}"; shift ;;
        --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
        --output-dir=*) OUTPUT_DIR="${1#--output-dir=}"; shift ;;
        *) echo "Unknown option: $1"; echo "Usage: $0 [--no-bail] [--no-clean] [--template <name>] [--output-dir <path>]"; exit 1 ;;
    esac
done

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ -n "$OUTPUT_DIR" ]; then
    TMPDIR_BASE="$(cd "$OUTPUT_DIR" 2>/dev/null && pwd || mkdir -p "$OUTPUT_DIR" && cd "$OUTPUT_DIR" && pwd)"
else
    TMPDIR_BASE="${TMPDIR:-/tmp}/tari-template-tests"
fi
WASM_TARGET="wasm32-unknown-unknown"

# Templates listed in wasm_templates/cargo-generate.toml
WASM_TEMPLATES=(empty no_std counter fungible nft swap meme_coin airdrop stable_coin)

# Templates that have tests (swap has no tests)
TEMPLATES_WITH_TESTS=(empty no_std counter fungible nft meme_coin airdrop stable_coin)

GREEN='\033[0;32m'
RED='\033[0;31m'
BOLD='\033[1m'
RESET='\033[0m'

passed=0
failed=0
failures=()

log() { echo -e "${BOLD}>>> $1${RESET}"; }

print_summary() {
    local total=$((passed + failed))
    echo ""
    log "Test Summary"
    echo "  Total:  $total"
    echo -e "  ${GREEN}Passed: $passed${RESET}"
    echo -e "  ${RED}Failed: $failed${RESET}"
    if [ "$failed" -gt 0 ]; then
        echo ""
        echo -e "${RED}Failures:${RESET}"
        for f in "${failures[@]}"; do
            echo "  - $f"
        done
    fi
    if $NO_CLEAN; then
        echo ""
        echo "  Generated templates: $TMPDIR_BASE"
    fi
}
pass() { echo -e "${GREEN}PASS${RESET}: $1"; passed=$((passed + 1)); }
fail() {
    echo -e "${RED}FAIL${RESET}: $1"
    failed=$((failed + 1))
    failures+=("$1")
    if ! $NO_BAIL; then
        echo ""
        print_summary
        exit 1
    fi
}

cleanup() {
    if ! $NO_CLEAN && [ -d "$TMPDIR_BASE" ]; then
        rm -rf "$TMPDIR_BASE"
    fi
}
trap cleanup EXIT

# Check prerequisites
for cmd in cargo cargo-generate; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: $cmd is not installed"
        exit 1
    fi
done

# Ensure wasm target is installed
rustup target add "$WASM_TARGET" 2>/dev/null || true

rm -rf "$TMPDIR_BASE"
mkdir -p "$TMPDIR_BASE"

# Validate --template filter
if [ -n "$FILTER" ]; then
    found=false
    for t in "${WASM_TEMPLATES[@]}"; do
        if [ "$t" = "$FILTER" ]; then
            found=true
            break
        fi
    done
    if ! $found; then
        echo "Error: unknown template '$FILTER'"
        echo "Available templates: ${WASM_TEMPLATES[*]}"
        exit 1
    fi
fi

# --- Test WASM templates via cargo-generate ---
log "Testing WASM templates"

for template in "${WASM_TEMPLATES[@]}"; do
    if [ -n "$FILTER" ] && [ "$template" != "$FILTER" ]; then
        continue
    fi
    log "Generating template: $template"
    dest="$TMPDIR_BASE/$template"

    if ! cargo generate --path "$REPO_ROOT/wasm_templates" "$template" \
        --name "test-$template" \
        --destination "$TMPDIR_BASE" \
        --define "authors=CI" \
        --define "in_cargo_workspace=false" 2>&1; then
        fail "$template (generate)"
        continue
    fi

    # cargo-generate normalises the project name (e.g. underscores become hyphens)
    normalized_name="test-${template//_/-}"
    generated_dir="$TMPDIR_BASE/$normalized_name"
    if [ ! -d "$generated_dir" ]; then
        fail "$template (generate - output dir not found)"
        continue
    fi

    log "Building WASM: $template"
    if (cd "$generated_dir" && cargo build --target "$WASM_TARGET" --release 2>&1); then
        pass "$template (build)"
    else
        fail "$template (build)"
    fi

    # Run tests if the template has them
    has_tests=false
    for t in "${TEMPLATES_WITH_TESTS[@]}"; do
        if [ "$t" = "$template" ]; then
            has_tests=true
            break
        fi
    done

    if $has_tests; then
        log "Testing: $template"
        if (cd "$generated_dir" && cargo test 2>&1); then
            pass "$template (test)"
        else
            fail "$template (test)"
        fi
    fi
done

# --- Test examples (skipped when filtering by template) ---
if [ -n "$FILTER" ]; then
    print_summary

    if [ "$failed" -gt 0 ]; then
        exit 1
    fi

    echo ""
    echo -e "${GREEN}All checks passed.${RESET}"
    exit 0
fi

log "Testing examples/guessing_game/template"

if (cd "$REPO_ROOT/examples/guessing_game/template" && cargo build --target "$WASM_TARGET" --release 2>&1); then
    pass "guessing_game/template (build)"
else
    fail "guessing_game/template (build)"
fi

if (cd "$REPO_ROOT/examples/guessing_game/template" && cargo test 2>&1); then
    pass "guessing_game/template (test)"
else
    fail "guessing_game/template (test)"
fi

log "Testing examples/guessing_game/cli"

if (cd "$REPO_ROOT/examples/guessing_game/cli" && cargo build 2>&1); then
    pass "guessing_game/cli (build)"
else
    fail "guessing_game/cli (build)"
fi

if (cd "$REPO_ROOT/examples/guessing_game/cli" && cargo test 2>&1); then
    pass "guessing_game/cli (test)"
else
    fail "guessing_game/cli (test)"
fi

# --- Summary ---
print_summary

if [ "$failed" -gt 0 ]; then
    exit 1
fi

echo ""
echo -e "${GREEN}All checks passed.${RESET}"
