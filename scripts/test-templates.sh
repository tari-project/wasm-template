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
    TMPDIR_BASE="${TMPDIR:-/tmp}"
    TMPDIR_BASE="${TMPDIR_BASE%/}/tari-template-tests"
fi
WASM_TARGET="wasm32-unknown-unknown"

# Templates listed in wasm_templates/cargo-generate.toml
WASM_TEMPLATES=(empty no_std counter fungible nft swap meme_coin airdrop ico stable_coin)

# Templates that have tests (swap has no tests)
TEMPLATES_WITH_TESTS=(empty no_std counter fungible nft meme_coin airdrop ico stable_coin)

# Deterministic non-interactive value for the copyright-holder placeholder (Issue #6).
# Includes a regex metacharacter (the trailing period) so copyright checks below prove
# they compare literal strings rather than accidentally matching as a regex.
COPYRIGHT_HOLDER_CI_VALUE="CI Test Co."
GUESSING_GAME_SENTINEL=""

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

# Checks that a generated file contains an exact, whole-line, literal match for
# $expected_line and does not contain the literal old fixed-holder $forbidden_line.
# Uses `grep -xF` (fixed-string, whole-line) throughout so a value containing regex
# metacharacters (e.g. the period in COPYRIGHT_HOLDER_CI_VALUE) cannot be misread as a
# pattern and cannot produce a false-positive near-match.
check_copyright_line() {
    local file="$1"
    local expected_line="$2"
    local forbidden_line="$3"
    local label="$4"

    if [ ! -f "$file" ]; then
        fail "$label (file not found)"
        return
    fi
    if ! grep -qxF -- "$expected_line" "$file"; then
        fail "$label (expected copyright line not found)"
        return
    fi
    if grep -qxF -- "$forbidden_line" "$file"; then
        fail "$label (old fixed copyright line still present)"
        return
    fi
    pass "$label"
}

cleanup() {
    if [ -n "$GUESSING_GAME_SENTINEL" ] && [ -e "$GUESSING_GAME_SENTINEL" ]; then
        rm -f "$GUESSING_GAME_SENTINEL"
    fi
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
    if ! [[ " ${WASM_TEMPLATES[*]} " =~ " $FILTER " ]]; then
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
        --define "in_cargo_workspace=false" \
        --define "copyright-holder=$COPYRIGHT_HOLDER_CI_VALUE" 2>&1; then
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

    # Issue #6: the copyright-holder placeholder only affects nft and stable_coin, whose
    # source files carry a fixed "The Tari Project" holder. Other templates ignore the
    # define entirely, so this check is scoped to just those two.
    if [ "$template" = "nft" ]; then
        check_copyright_line "$generated_dir/src/lib.rs" \
            "//   Copyright 2022. $COPYRIGHT_HOLDER_CI_VALUE" \
            "//   Copyright 2022. The Tari Project" \
            "nft (copyright holder rendered)"
    elif [ "$template" = "stable_coin" ]; then
        check_copyright_line "$generated_dir/src/user_data.rs" \
            "// Copyright 2024 $COPYRIGHT_HOLDER_CI_VALUE" \
            "// Copyright 2024 The Tari Project" \
            "stable_coin/user_data.rs (copyright holder rendered)"
        check_copyright_line "$generated_dir/src/wrapped_exchange_token.rs" \
            "// Copyright 2024 $COPYRIGHT_HOLDER_CI_VALUE" \
            "// Copyright 2024 The Tari Project" \
            "stable_coin/wrapped_exchange_token.rs (copyright holder rendered)"
    fi

    log "Building WASM: $template"
    if (cd "$generated_dir" && cargo build --target "$WASM_TARGET" --release 2>&1); then
        pass "$template (build)"
    else
        fail "$template (build)"
    fi

    if [[ " ${TEMPLATES_WITH_TESTS[*]} " =~ " $template " ]]; then
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

log "Testing examples/guessing_game/template (cargo-generate copyright + build-artifact omission)"

GUESSING_GAME_SRC="$REPO_ROOT/examples/guessing_game/template"

# Collision-safe controlled sentinel, in addition to whatever real target/ the build above
# just produced. Together these cover both required scenarios: a real build artifact and a
# synthetic one. $$ (this script's PID) keeps concurrent runs from colliding; cleanup() removes
# it on success, failure, and interruption.
GUESSING_GAME_SENTINEL="$GUESSING_GAME_SRC/target/.cg-omission-sentinel-$$"
mkdir -p "$(dirname "$GUESSING_GAME_SENTINEL")"
echo "build-artifact-sentinel-$$" > "$GUESSING_GAME_SENTINEL"

if cargo generate --path "$GUESSING_GAME_SRC" \
    --name test-guessing-game-copyright \
    --destination "$TMPDIR_BASE" \
    --define "copyright-holder=$COPYRIGHT_HOLDER_CI_VALUE" 2>&1; then

    guessing_generated_dir="$TMPDIR_BASE/test-guessing-game-copyright"
    if [ ! -d "$guessing_generated_dir" ]; then
        fail "guessing_game/template (generate - output dir not found)"
    else
        check_copyright_line "$guessing_generated_dir/src/lib.rs" \
            "//   Copyright 2026 $COPYRIGHT_HOLDER_CI_VALUE" \
            "//   Copyright 2026 The Tari Project" \
            "guessing_game/template src/lib.rs (copyright holder rendered)"
        check_copyright_line "$guessing_generated_dir/tests/test.rs" \
            "//   Copyright 2025 $COPYRIGHT_HOLDER_CI_VALUE" \
            "//   Copyright 2025 The Tari Project" \
            "guessing_game/template tests/test.rs (copyright holder rendered)"

        if find "$guessing_generated_dir" -iname target -o -iname "*.cg-omission-sentinel-*" | grep -q .; then
            fail "guessing_game/template (build artifacts leaked into generated output)"
        else
            pass "guessing_game/template (build artifacts omitted from generated output)"
        fi
    fi
else
    fail "guessing_game/template (generate)"
fi

rm -f "$GUESSING_GAME_SENTINEL"
GUESSING_GAME_SENTINEL=""

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
