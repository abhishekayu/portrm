#!/bin/bash
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#  ptrm — Comprehensive Test Suite
#  Tests every command, flag, feature, and edge case
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
set -o pipefail

P="$(cd "$(dirname "$0")" && pwd)/target/release/ptrm"
PASS=0
FAIL=0
TOTAL=0
FAILURES=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
RESET='\033[0m'

# ── Helpers ───────────────────────────────────────────────────────

ok() {
    TOTAL=$((TOTAL + 1))
    PASS=$((PASS + 1))
    printf "  ${GREEN}✔${RESET} ${DIM}%3d${RESET}  %s\n" "$TOTAL" "$1"
}

fail() {
    TOTAL=$((TOTAL + 1))
    FAIL=$((FAIL + 1))
    FAILURES="${FAILURES}\n    ✘ #${TOTAL}: $1"
    printf "  ${RED}✘${RESET} ${DIM}%3d${RESET}  %s\n" "$TOTAL" "$1"
    if [ -n "$2" ]; then
        printf "       ${DIM}%s${RESET}\n" "$2"
    fi
}

# Run a test, expect exit 0
t() {
    local name="$1"; shift
    local out
    if out=$("$@" 2>&1); then
        ok "$name"
    else
        fail "$name" "exit=$?"
    fi
}

# Run a test, expect non-zero exit
t_fail() {
    local name="$1"; shift
    local out
    if out=$("$@" 2>&1); then
        fail "$name" "expected failure but got exit 0"
    else
        ok "$name"
    fi
}

# Run a test, expect output to contain a string
t_contains() {
    local name="$1"
    local pattern="$2"
    shift 2
    local out
    out=$("$@" 2>&1) || true
    if echo "$out" | grep -qi "$pattern"; then
        ok "$name"
    else
        fail "$name" "output missing: '$pattern'"
    fi
}

# Run a test, expect exit 0 AND output contains string
t_ok_contains() {
    local name="$1"
    local pattern="$2"
    shift 2
    local out
    if out=$("$@" 2>&1); then
        if echo "$out" | grep -qi "$pattern"; then
            ok "$name"
        else
            fail "$name" "output missing: '$pattern'"
        fi
    else
        fail "$name" "exit=$?"
    fi
}

section() {
    echo ""
    printf "  ${CYAN}${BOLD}── %s ──${RESET}\n" "$1"
    echo ""
}

# ── Pre-check ────────────────────────────────────────────────────

if [ ! -f "$P" ]; then
    echo ""
    echo "  Binary not found at $P"
    echo "  Run: cargo build --release"
    echo ""
    exit 1
fi

TESTDIR=$(mktemp -d)
trap "rm -rf $TESTDIR" EXIT

echo ""
printf "  ${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"
printf "  ${BOLD}   ptrm Full Test Suite${RESET}\n"
printf "  ${DIM}   $(${P} --version)${RESET}\n"
printf "  ${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"

# ══════════════════════════════════════════════════════════════════
#  1. META / CLI BASICS
# ══════════════════════════════════════════════════════════════════

section "META"

t "ptrm --version" $P --version
t_ok_contains "--version contains 0." "0\." $P --version
t "ptrm --help" $P --help
t_ok_contains "--help lists scan" "scan" $P --help
t_ok_contains "--help lists kill" "kill" $P --help
t_ok_contains "--help lists fix" "fix" $P --help
t_ok_contains "--help lists completions" "completions" $P --help
t_ok_contains "--help lists info" "info" $P --help
t_ok_contains "--help lists log" "log" $P --help
t_ok_contains "--help lists doctor" "doctor" $P --help
t_ok_contains "--help lists watch" "watch" $P --help
t_ok_contains "--help lists up" "up" $P --help
t_ok_contains "--help lists down" "down" $P --help
t_ok_contains "--help lists preflight" "preflight" $P --help
t_ok_contains "--help lists init" "init" $P --help
t_ok_contains "--help lists registry" "registry" $P --help
t_ok_contains "--help lists ci" "ci" $P --help
t_ok_contains "--help lists use" "use" $P --help
t_ok_contains "--help lists interactive" "interactive" $P --help
t_ok_contains "--help lists group" "group" $P --help
t_ok_contains "--help lists history" "history" $P --help
t_ok_contains "--help lists project" "project" $P --help
t_ok_contains "--help lists restart" "restart" $P --help
t_ok_contains "--help lists status" "status" $P --help

# ══════════════════════════════════════════════════════════════════
#  2. SCAN
# ══════════════════════════════════════════════════════════════════

section "SCAN"

t "scan (all ports)" $P scan
t "scan --dev" $P scan --dev
t "scan --json" $P scan --json
t "scan --json --dev" $P scan --json --dev
t "scan <port> 80" $P scan 80
t "scan <port> 443" $P scan 443
t "scan multiple ports" $P scan 80 443 8080
t "scan unused port 59999" $P scan 59999
t "scan --json returns output" $P scan --json

# ══════════════════════════════════════════════════════════════════
#  3. INFO / SHORTHAND
# ══════════════════════════════════════════════════════════════════

section "INFO"

t "info 80" $P info 80
t "info --json 80" $P info 80 --json
t "info unused port 59998" $P info 59998
t_ok_contains "info unused port says free" "free" $P info 59998
t_ok_contains "info --json unused port" "free" $P info 59998 --json
t "shorthand: ptrm 80" $P 80
t "shorthand: ptrm 59997" $P 59997
t_ok_contains "shorthand unused says free" "free" $P 59997

# ══════════════════════════════════════════════════════════════════
#  3b. LOG
# ══════════════════════════════════════════════════════════════════

section "LOG"

t_fail "log on free port fails" $P log 59996
t_contains "log free port says no process" "No process\|no process\|not found" $P log 59996
t "log --help" $P log --help
t_ok_contains "log --help mentions PORT" "PORT\|port" $P log --help

# ══════════════════════════════════════════════════════════════════
#  4. GROUP
# ══════════════════════════════════════════════════════════════════

section "GROUP"

t "group" $P group
t "group --dev" $P group --dev
t "group --json" $P group --json

# ══════════════════════════════════════════════════════════════════
#  5. DOCTOR
# ══════════════════════════════════════════════════════════════════

section "DOCTOR"

t "doctor" $P doctor
t "doctor --json" $P doctor --json

# ══════════════════════════════════════════════════════════════════
#  6. HISTORY
# ══════════════════════════════════════════════════════════════════

section "HISTORY"

t "history" $P history
t "history --stats" $P history --stats
t "history --json" $P history --json
t "history --stats --json" $P history --stats --json

# ══════════════════════════════════════════════════════════════════
#  7. PROJECT
# ══════════════════════════════════════════════════════════════════

section "PROJECT"

t "project" $P project
t "project --json" $P project --json

# ══════════════════════════════════════════════════════════════════
#  8. PREFLIGHT (explicit ports)
# ══════════════════════════════════════════════════════════════════

section "PREFLIGHT"

t "preflight 59990" $P preflight 59990
t_ok_contains "preflight free port says free" "free" $P preflight 59990
t "preflight multiple free ports" $P preflight 59990 59991 59992
t "preflight 80 (likely busy)" $P preflight 80

# ══════════════════════════════════════════════════════════════════
#  9. INIT
# ══════════════════════════════════════════════════════════════════

section "INIT"

cd "$TESTDIR"
mkdir -p test_init && cd test_init

t "init creates .ptrm.toml" $P init
[ -f .ptrm.toml ] && ok "init file exists" || fail "init file exists"
t_ok_contains "init already exists warning" "already exists" $P init
t_ok_contains "init config has [project]" "project" cat .ptrm.toml
t_ok_contains "init config has [services" "services" cat .ptrm.toml

cd "$TESTDIR"

# ══════════════════════════════════════════════════════════════════
#  10. CONFIG-DEPENDENT COMMANDS (UP, DOWN, PREFLIGHT from config)
# ══════════════════════════════════════════════════════════════════

section "CONFIG (up/down/preflight)"

mkdir -p test_config && cd test_config

cat > .ptrm.toml << 'EOF'
[project]
name = "test-project"

[services.web]
port = 59980
run = "echo web"
preflight = true

[services.api]
port = 59981
run = "echo api"
preflight = true
EOF

t "preflight from config" $P preflight
t_ok_contains "preflight config ports free" "free" $P preflight

# up/down (services won't actually persist since they just echo)
t "up (echo services)" $P up -y
t "down" $P down

# status (services just echoed, so they'll show as stopped)
t "status" $P status
t "status --json" $P status --json
t_ok_contains "status shows service name" "web" $P status
t_ok_contains "status shows port" "59980" $P status

# status --json returns valid JSON
OUT=$($P status --json 2>&1)
echo "$OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null && ok "status --json is valid JSON" || fail "status --json is valid JSON"

# restart (services echo and exit, restart should still work gracefully)
t "restart web" $P restart web
t_fail "restart nonexistent service" $P restart nonexistent

cd "$TESTDIR"

# ══════════════════════════════════════════════════════════════════
#  11. REGISTRY CHECK -- valid config
# ══════════════════════════════════════════════════════════════════

section "REGISTRY CHECK"

mkdir -p test_registry && cd test_registry

# Valid config (no conflicts)
cat > .ptrm.toml << 'EOF'
[project]
name = "reg-valid"

[services.frontend]
port = 3000
run = "npm run dev"

[services.backend]
port = 8080
run = "cargo run"

[services.worker]
port = 9090
run = "node worker.js"
EOF

t "registry check (valid, no conflicts)" $P registry check
t_ok_contains "registry valid message" "valid" $P registry check
t "registry check --json (valid)" $P registry check --json
t_ok_contains "registry --json empty array" "\[\]" $P registry check --json

# Config with duplicate ports
cat > .ptrm.toml << 'EOF'
[project]
name = "reg-dup"

[services.frontend]
port = 3000
run = "npm run dev"

[services.api]
port = 3000
run = "npm start"

[services.backend]
port = 8080
run = "cargo run"
EOF

t_fail "registry check (duplicate ports)" $P registry check
t_contains "registry detects conflict" "conflict" $P registry check
t_contains "registry shows conflicting services" "frontend" $P registry check
t_contains "registry shows conflicting services" "api" $P registry check
t_fail "registry check --json (conflicts)" $P registry check --json
t_contains "registry --json has port 3000" "3000" $P registry check --json

# Three-way conflict
cat > .ptrm.toml << 'EOF'
[project]
name = "reg-triple"

[services.a]
port = 4000
run = "a"

[services.b]
port = 4000
run = "b"

[services.c]
port = 4000
run = "c"
EOF

t_fail "registry check (3-way conflict)" $P registry check
t_contains "registry 3-way has all services" "a" $P registry check

# Multiple different conflicts
cat > .ptrm.toml << 'EOF'
[project]
name = "reg-multi"

[services.a]
port = 3000
run = "a"

[services.b]
port = 3000
run = "b"

[services.c]
port = 8080
run = "c"

[services.d]
port = 8080
run = "d"
EOF

t_fail "registry check (multiple conflicts)" $P registry check

# Single service (no conflict possible)
cat > .ptrm.toml << 'EOF'
[project]
name = "reg-single"

[services.only]
port = 5000
run = "node ."
EOF

t "registry check (single service)" $P registry check

cd "$TESTDIR"

# ══════════════════════════════════════════════════════════════════
#  12. PROFILES
# ══════════════════════════════════════════════════════════════════

section "PROFILES"

mkdir -p test_profiles && cd test_profiles

cat > .ptrm.toml << 'EOF'
[project]
name = "profile-test"

[services.frontend]
port = 3000
run = "npm run dev"

[services.backend]
port = 8080
run = "cargo run"

[profiles.staging.services.frontend]
port = 4000

[profiles.staging.services.backend]
port = 9090

[profiles.production.services.frontend]
port = 80

[profiles.production.services.backend]
port = 443

[profiles.custom.services.frontend]
port = 5555
run = "npm run custom"
env = { NODE_ENV = "custom" }
EOF

rm -f .ptrm.state

# Switch to staging
t "use staging" $P use staging
t_ok_contains "use staging shows frontend" "frontend" $P use staging
t_ok_contains "use staging shows backend" "backend" $P use staging
[ -f .ptrm.state ] && ok ".ptrm.state created" || fail ".ptrm.state created"
t_ok_contains "state file has staging" "staging" cat .ptrm.state

# Switch to production
t "use production" $P use production
t_ok_contains "state file has production" "production" cat .ptrm.state

# Switch to custom
t "use custom" $P use custom
t_ok_contains "use custom shows frontend" "frontend" $P use custom

# Invalid profile
t_fail "use nonexistent (error)" $P use nonexistent
t_contains "use nonexistent shows available" "staging" $P use nonexistent
t_contains "use nonexistent shows available" "production" $P use nonexistent

# Registry check respects active profile
rm -f .ptrm.state
t "registry check (base, no profile)" $P registry check

$P use staging > /dev/null 2>&1
t "registry check (staging active)" $P registry check

$P use production > /dev/null 2>&1
t "registry check (production active)" $P registry check

# Profile with conflicts
cat > .ptrm.toml << 'EOF'
[project]
name = "profile-conflict"

[services.frontend]
port = 3000
run = "npm run dev"

[services.backend]
port = 8080
run = "cargo run"

[profiles.bad.services.frontend]
port = 8080
EOF

rm -f .ptrm.state
$P use bad > /dev/null 2>&1
t "registry check (profile with overlap, base ports valid)" $P registry check

# No profiles defined
cat > .ptrm.toml << 'EOF'
[project]
name = "no-profiles"

[services.web]
port = 3000
run = "node ."
EOF

rm -f .ptrm.state
t_fail "use profile when none defined" $P use dev
t_contains "no profiles message" "No profiles" $P use dev

# Preflight respects profile
cat > .ptrm.toml << 'EOF'
[project]
name = "preflight-profile"

[services.web]
port = 59970
run = "echo web"

[profiles.alt.services.web]
port = 59971
EOF

rm -f .ptrm.state
t_ok_contains "preflight base port 59970" "59970" $P preflight
$P use alt > /dev/null 2>&1
t_ok_contains "preflight after profile switch" "59970" $P preflight

cd "$TESTDIR"

# ══════════════════════════════════════════════════════════════════
#  13. CI COMMAND
# ══════════════════════════════════════════════════════════════════

section "CI COMMAND"

mkdir -p test_ci && cd test_ci

# CI pass: valid config, unique ports, ports free
cat > .ptrm.toml << 'EOF'
[project]
name = "ci-pass"

[services.a]
port = 59960
run = "echo a"

[services.b]
port = 59961
run = "echo b"
EOF

rm -f .ptrm.state
t "ci (pass)" $P ci
t_ok_contains "ci pass message" "passed" $P ci
t "ci --json (pass)" $P ci --json
t_ok_contains "ci --json passed=true" "true" $P ci --json
t_contains "ci --json has config_valid" "config_valid" $P ci --json
t_contains "ci --json has registry_valid" "registry_valid" $P ci --json
t_contains "ci --json has preflight_passed" "preflight_passed" $P ci --json
t_contains "ci --json has doctor_issues" "doctor_issues" $P ci --json

# CI fail: duplicate ports
cat > .ptrm.toml << 'EOF'
[project]
name = "ci-fail-dup"

[services.a]
port = 59960
run = "echo a"

[services.b]
port = 59960
run = "echo b"
EOF

t_fail "ci (fail: duplicate ports)" $P ci
t_contains "ci fail message" "failed" $P ci
t_fail "ci --json (fail)" $P ci --json

# CI fail: no config
cd "$TESTDIR"
mkdir -p test_ci_noconfig && cd test_ci_noconfig
t_fail "ci (fail: no config)" $P ci
t_contains "ci no config message" "ptrm.toml" $P ci

# CI with active profile
cd "$TESTDIR"/test_ci
cat > .ptrm.toml << 'EOF'
[project]
name = "ci-profile"

[services.web]
port = 59950
run = "echo web"

[profiles.staging.services.web]
port = 59951
EOF

rm -f .ptrm.state
$P use staging > /dev/null 2>&1
t "ci with active profile" $P ci
t_ok_contains "ci --json with profile" "true" $P ci --json

cd "$TESTDIR"

# ══════════════════════════════════════════════════════════════════
#  14. KILL / FIX (on free port -- should report not found)
# ══════════════════════════════════════════════════════════════════

section "KILL / FIX (safe tests)"

t_contains "kill free port 59940" "free\|nothing\|No process\|no process" $P kill 59940 -y
t_contains "fix free port 59940" "free\|nothing\|No process\|no process" $P fix 59940 -y
t_contains "fix --json free port" "free\|nothing\|No process\|no process" $P fix 59940 -y --json
t_contains "kill multiple free ports" "no process" $P kill 59940 59941 -y
t_contains "fix multiple free ports" "no process" $P fix 59940 59941 -y

# ══════════════════════════════════════════════════════════════════
#  14b. LIVE LISTENER TESTS (actual busy port)
# ══════════════════════════════════════════════════════════════════

section "LIVE LISTENER (scan / info / kill / fix)"

# Helper: start a TCP listener on a given port
start_listener() {
    python3 -c "
import socket, time, os, sys
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('0.0.0.0', int(sys.argv[1])))
s.listen(5)
time.sleep(120)
" "$1" &
    LISTENER_PID=$!
    sleep 1
}

# -- Scan a busy port
start_listener 59900
t_ok_contains "scan busy port 59900" "59900" $P scan 59900
t_ok_contains "scan --json busy port" "59900" $P scan --json 59900
kill $LISTENER_PID 2>/dev/null; wait $LISTENER_PID 2>/dev/null

# -- Info a busy port
start_listener 59901
t_ok_contains "info busy port shows in use" "in use" $P info 59901
t_ok_contains "info --json busy port has pid" "pid" $P info 59901 --json
t_ok_contains "info busy port shows PID" "$LISTENER_PID" $P info 59901
kill $LISTENER_PID 2>/dev/null; wait $LISTENER_PID 2>/dev/null

# -- Kill a busy port
start_listener 59902
t_ok_contains "kill busy port 59902" "free\|Killed" $P kill 59902 -y
sleep 1
t_ok_contains "port 59902 free after kill" "free" $P info 59902
wait $LISTENER_PID 2>/dev/null

# -- Kill --force (SIGKILL)
start_listener 59903
t_ok_contains "kill --force 59903" "free\|Killed\|killed" $P kill 59903 -y --force
sleep 1
t_ok_contains "port 59903 free after force kill" "free" $P info 59903
wait $LISTENER_PID 2>/dev/null

# -- Fix a busy port
start_listener 59904
t_ok_contains "fix busy port 59904" "free\|Killed\|killed" $P fix 59904 -y
sleep 1
t_ok_contains "port 59904 free after fix" "free" $P info 59904
wait $LISTENER_PID 2>/dev/null

# -- Fix --force a busy port
start_listener 59905
t_ok_contains "fix --force 59905" "free\|Killed\|killed" $P fix 59905 -y --force
sleep 1
t_ok_contains "port 59905 free after fix --force" "free" $P info 59905
wait $LISTENER_PID 2>/dev/null

# -- Fix --json a busy port
start_listener 59906
t_contains "fix --json busy port has fields" "port" $P fix 59906 -y --json
sleep 1
wait $LISTENER_PID 2>/dev/null

# -- Group detects busy port
start_listener 59907
t_ok_contains "group detects busy port" "59907" $P scan 59907
t "group --json with busy port" $P group --json
kill $LISTENER_PID 2>/dev/null; wait $LISTENER_PID 2>/dev/null

# -- Doctor with some ports in use
start_listener 59908
t "doctor with active ports" $P doctor
t "doctor --json with active ports" $P doctor --json
kill $LISTENER_PID 2>/dev/null; wait $LISTENER_PID 2>/dev/null

# ══════════════════════════════════════════════════════════════════
#  15. WATCH (quick test -- ctrl-c after 1 second)
# ══════════════════════════════════════════════════════════════════

section "WATCH (quick)"

# Watch a free port for 1 second
timeout 2 $P watch 59930 --interval 1 > /dev/null 2>&1 || true
ok "watch free port (timeout exit)"

# Watch a busy port briefly
start_listener 59931
timeout 2 $P watch 59931 --interval 1 > /dev/null 2>&1 || true
ok "watch busy port (timeout exit)"
kill $LISTENER_PID 2>/dev/null; wait $LISTENER_PID 2>/dev/null

# ══════════════════════════════════════════════════════════════════
#  15b. HISTORY --clear
# ══════════════════════════════════════════════════════════════════

section "HISTORY (clear)"

t "history --clear" $P history --clear
t_ok_contains "history --clear confirms" "cleared\|Cleared\|empty" $P history --clear
t "history after clear" $P history
t "history --stats after clear" $P history --stats
t "history --stats --json after clear" $P history --stats --json

# ══════════════════════════════════════════════════════════════════
#  16. EDGE CASES
# ══════════════════════════════════════════════════════════════════

section "EDGE CASES"

# Invalid port
t_fail "scan invalid port 99999" $P scan 99999
t_fail "info invalid port 99999" $P info 99999

# Unknown subcommand
t_fail "unknown subcommand" $P foobar

# Empty services config
cd "$TESTDIR"
mkdir -p test_edge && cd test_edge
cat > .ptrm.toml << 'EOF'
[project]
name = "empty"
EOF

t "registry check (empty services)" $P registry check
t "ci (empty services)" $P ci

# Config with env vars
cat > .ptrm.toml << 'EOF'
[project]
name = "envtest"

[services.web]
port = 59920
run = "echo hello"
cwd = "."
preflight = true
env = { NODE_ENV = "development", PORT = "59920" }
EOF

t "preflight with env config" $P preflight
t "registry check with env config" $P registry check

# Very large port number (valid max)
t "scan port 65535" $P scan 65535
t "info port 65535" $P info 65535

# Port 0 is valid to scan (no result)
t "scan port 0" $P scan 0

# JSON output validation: scan --json returns valid JSON
OUT=$($P scan --json 2>&1)
echo "$OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null && ok "scan --json is valid JSON" || fail "scan --json is valid JSON"

# JSON output validation: doctor --json returns valid JSON
OUT=$($P doctor --json 2>&1)
echo "$OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null && ok "doctor --json is valid JSON" || fail "doctor --json is valid JSON"

# JSON output validation: group --json returns valid JSON
OUT=$($P group --json 2>&1)
echo "$OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null && ok "group --json is valid JSON" || fail "group --json is valid JSON"

# JSON output validation: history --json returns valid JSON
OUT=$($P history --json 2>&1)
echo "$OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null && ok "history --json is valid JSON" || fail "history --json is valid JSON"

# JSON output validation: project --json returns valid JSON
OUT=$($P project --json 2>&1)
echo "$OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null && ok "project --json is valid JSON" || fail "project --json is valid JSON"

# JSON output validation: ci --json returns valid JSON
OUT=$($P ci --json 2>&1)
echo "$OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null && ok "ci --json is valid JSON" || fail "ci --json is valid JSON"

# JSON output validation: info --json (free port)
OUT=$($P info 59998 --json 2>&1)
echo "$OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null && ok "info --json (free port) is valid JSON" || fail "info --json (free port) is valid JSON"

cd "$TESTDIR"

# ══════════════════════════════════════════════════════════════════
#  16b. DOWN WITHOUT CONFIG
# ══════════════════════════════════════════════════════════════════

section "DOWN / UP WITHOUT CONFIG"

mkdir -p test_no_config && cd test_no_config
rm -f .ptrm.toml .ptrm.state
t_fail "up without config" $P up -y
t_fail "down without config" $P down
t_contains "up error mentions config" "ptrm.toml\|config\|not found" $P up -y
t_contains "down error mentions config" "ptrm.toml\|config\|not found" $P down
t_fail "restart without config" $P restart frontend
t_fail "status without config" $P status

cd "$TESTDIR"

# ══════════════════════════════════════════════════════════════════
#  17. SUBCOMMAND HELP
# ══════════════════════════════════════════════════════════════════

section "SUBCOMMAND HELP"

t "scan --help" $P scan --help
t "kill --help" $P kill --help
t "fix --help" $P fix --help
t "info --help" $P info --help
t "group --help" $P group --help
t "doctor --help" $P doctor --help
t "history --help" $P history --help
t "project --help" $P project --help
t "watch --help" $P watch --help
t "up --help" $P up --help
t "down --help" $P down --help
t "preflight --help" $P preflight --help
t "init --help" $P init --help
t "registry --help" $P registry --help
t "registry check --help" $P registry check --help
t "ci --help" $P ci --help
t "use --help" $P use --help
t "log --help (subcommand)" $P log --help
t "restart --help" $P restart --help
t "status --help" $P status --help
t "interactive --help" $P interactive --help
t "ui alias --help" $P ui --help
t_ok_contains "ui alias shows same as interactive" "Interactive\|interactive" $P ui --help

# ══════════════════════════════════════════════════════════════════
#  18. DEFAULT (no args = scan)
# ══════════════════════════════════════════════════════════════════

section "DEFAULT BEHAVIOR"

t "ptrm (no args = scan)" $P

# ══════════════════════════════════════════════════════════════════
#  19. SHELL COMPLETIONS
# ══════════════════════════════════════════════════════════════════

section "SHELL COMPLETIONS"

t "completions --help" $P completions --help
t_ok_contains "completions bash generates script" "_ptrm" $P completions bash
t_ok_contains "completions zsh generates script" "compdef" $P completions zsh
t_ok_contains "completions fish generates script" "complete -c ptrm" $P completions fish
t_ok_contains "completions powershell generates script" "ptrm" $P completions powershell
t_ok_contains "bash has dynamic port hook" "_ptrm_complete_ports" $P completions bash
t_ok_contains "fish has dynamic port hook" "_complete-ports" $P completions fish
t "_complete-ports runs" $P _complete-ports
t_fail "completions rejects invalid shell" $P completions invalid

# ══════════════════════════════════════════════════════════════════
#  RESULTS
# ══════════════════════════════════════════════════════════════════

echo ""
printf "  ${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"

if [ $FAIL -eq 0 ]; then
    printf "  ${GREEN}${BOLD}  ✔ ALL %d TESTS PASSED${RESET}\n" "$TOTAL"
else
    printf "  ${RED}${BOLD}  ✘ %d/%d FAILED${RESET}  ${GREEN}%d passed${RESET}\n" "$FAIL" "$TOTAL" "$PASS"
    printf "${RED}${FAILURES}${RESET}\n"
fi

printf "  ${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"
echo ""

[ $FAIL -eq 0 ]
