#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

# Contract tests for the lock-sync gate's self-protecting workflow shape.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKFLOW="$PROJECT_DIR/.github/workflows/lock-sync-gate.yml"

PASS=0
FAIL=0

pass() {
  printf 'PASS: %s\n' "$1"
  PASS=$((PASS + 1))
}

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  FAIL=$((FAIL + 1))
}

assert_present() {
  local label="$1"
  local pattern="$2"

  if grep -Eq "$pattern" "$WORKFLOW"; then
    pass "$label"
  else
    fail "$label"
  fi
}

assert_absent() {
  local label="$1"
  local pattern="$2"

  if grep -Eq "$pattern" "$WORKFLOW"; then
    fail "$label"
  else
    pass "$label"
  fi
}

if [ ! -f "$WORKFLOW" ]; then
  printf 'FAIL: workflow not found: %s\n' "$WORKFLOW" >&2
  exit 1
fi

assert_present 'runs for pull requests' '^[[:space:]]{2}pull_request:[[:space:]]*$'
assert_present 'runs for pushes' '^[[:space:]]{2}push:[[:space:]]*$'
assert_present 'can be dispatched manually after an unrerunnable startup failure' '^[[:space:]]{2}workflow_dispatch:[[:space:]]*$'
assert_present 'limits push runs to main' '^[[:space:]]+branches:[[:space:]]*\[main\][[:space:]]*$'
assert_present 'uses read-only repository contents permission' '^[[:space:]]+contents:[[:space:]]*read[[:space:]]*$'
assert_absent 'has no path filter that could suppress a required check' '^[[:space:]]+paths(-ignore)?:'
assert_absent 'has no action dependency that could itself require a lock entry' '^[[:space:]]*-?[[:space:]]*uses:'
assert_absent 'does not weaken failures with continue-on-error' '^[[:space:]]*continue-on-error:'
assert_present 'verifies that the checker exists and is executable' 'test -x scripts/check-lock-sync\.sh'
assert_present 'executes the repository checker directly' '^[[:space:]]*\./scripts/check-lock-sync\.sh[[:space:]]*$'

printf '\nlock-sync-gate tests: %d passed, %d failed\n' "$PASS" "$FAIL"
exit "$FAIL"
