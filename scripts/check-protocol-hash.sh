#!/usr/bin/env bash
# scripts/check-protocol-hash.sh: the app's hash must be STUDY.md's latest commit.
set -euo pipefail
cd "$(dirname "$0")/.."
want=$(git log -1 --format=%h -- STUDY.md)
grep -q "protocolHash = \"$want\"" apps/ios/VeloCoach/StudyConfig.swift \
  && echo "ok: $want" || { echo "MISMATCH: STUDY.md is $want"; exit 1; }
