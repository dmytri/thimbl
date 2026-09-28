#!/bin/sh
# Runs the thimbl cucumber suite inside the identity fixture: uid-mapped
# user namespace, /etc/passwd shadowed with the qmx999 probe line, temp
# HOME. The fixture variant is THIMBL_IDENTITY_GECOS; the default
# comment "QM GECOS Probe" serves the tier's plain scenarios, and the
# empty variant serves the @empty-comment-fixture scenarios.
#
# Usage: run.sh [NAME-PATTERN]
#   no argument: default-tier enumeration sweep at the plain variant
#   argument:    focused run, --name NAME-PATTERN (regex or exact name)
#
# Fixture variants the suite must stay runnable at:
#   run.sh                                                   # plain
#   THIMBL_IDENTITY_GECOS= run.sh '@empty-comment-fixture$'  # empty comment
# The tier sweep at the plain variant excludes @empty-comment-fixture,
# which reddens under the populated comment; the empty variant runs
# only the scenarios tagged with it.
set -eu
cd "$(dirname "$0")/../../.." || exit 9
# Default applies only when unset, so THIMBL_IDENTITY_GECOS= selects
# the empty-comment variant.
export THIMBL_IDENTITY_GECOS="${THIMBL_IDENTITY_GECOS-QM GECOS Probe}"
export HOME="${THIMBL_FIXTURE_HOME:-/tmp/qmhome}"
if [ "$#" -gt 0 ]; then
    exec python3 tests/support/identity-fixture/nsrun.py \
        env CUCUMBER_FILTER_TAGS="not @captain and not @shipwright" \
        cargo test --test cucumber -- --name "$1"
fi
exec python3 tests/support/identity-fixture/nsrun.py \
    env CUCUMBER_FILTER_TAGS="not @empty-comment-fixture and not @captain and not @shipwright" \
    cargo test --test cucumber
