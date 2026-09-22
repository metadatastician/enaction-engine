#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# Regression tests for README badge claims.
#
# Usage:
#   bash tests/readme_badges_test.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
README="$PROJECT_DIR/README.adoc"
FAILURES=0

pass() { printf 'PASS: %s\n' "$1"; }
fail() {
    printf 'FAIL: %s\n' "$1" >&2
    FAILURES=$((FAILURES + 1))
}

# The removed project ID belongs to an unrelated repository. Keep its digits
# split so a repository-wide exact-ID scan ignores this regression test itself.
FOREIGN_OPENSSF_PROJECT_ID="$(printf '%s%s' 85 09)"

if grep -Fq "$FOREIGN_OPENSSF_PROJECT_ID" "$README"; then
    fail "README references foreign OpenSSF project $FOREIGN_OPENSSF_PROJECT_ID"
else
    pass "README does not reference foreign OpenSSF project $FOREIGN_OPENSSF_PROJECT_ID"
fi

# Until this repository has its own registration, neither a differently
# numbered project URL nor a badge label without that URL is an honest claim.
if grep -Eq 'bestpractices\.dev/projects/[0-9]+' "$README"; then
    fail "README advertises an unverified OpenSSF project"
else
    pass "README does not advertise an unverified OpenSSF project"
fi

if grep -Fq 'OpenSSF Best Practices' "$README"; then
    fail "README claims an unverified OpenSSF Best Practices badge"
else
    pass "README does not claim an unverified OpenSSF Best Practices badge"
fi

# Removing the final badge in the compliance block must not collapse the
# AsciiDoc separator before the first content section.
if awk '
    /^== The invariant everything follows from$/ {
        found = 1
        exit(previous == "" ? 0 : 1)
    }
    { previous = $0 }
    END { if (!found) exit 1 }
' "$README"; then
    pass "README keeps a blank separator before its first content section"
else
    fail "README is missing its first content section or its blank separator"
fi

if ((FAILURES > 0)); then
    printf '%d README badge test(s) failed.\n' "$FAILURES" >&2
    exit 1
fi

printf 'All README badge tests passed.\n'
