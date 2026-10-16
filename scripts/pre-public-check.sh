#!/usr/bin/env bash
# Run before making the repository public (docs/PRE_PUBLIC_CHECKLIST.md). Read-only.
# Exit status 1 on any FAIL.
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
pass() { printf 'PASS  %s\n' "$*"; }
bad() { printf 'FAIL  %s\n' "$*"; fail=1; }
warn() { printf 'NOTE  %s\n' "$*"; }

# 1. Secrets in the whole history.
if command -v gitleaks >/dev/null; then
    if gitleaks detect --source . --log-opts="--all" --no-banner --redact >/dev/null 2>&1; then
        pass "gitleaks: no leaks in $(git rev-list --all --count) commits"
    else
        bad "gitleaks found something: gitleaks detect --source . --log-opts=--all -v"
    fi
else
    bad "gitleaks not installed (brew install gitleaks)"
fi

# 2. Files that must never be committed.
f=$(git ls-files | grep -i -E '\.(p12|mobileprovision|keystore|jks|pem|key|env)$|local\.properties|DerivedData/|xcuserdata/|\.DS_Store' || true)
[[ -z "$f" ]] && pass "no signing files, keystores, local.properties, DerivedData or xcuserdata" || bad "tracked: $f"

# 3. Personal data in tracked text.
e=$(git grep -n -I -E '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[a-z]{2,}' -- ':!Cargo.lock' ':!*/Cargo.lock' \
    | grep -v -E 'noreply@anthropic.com|users.noreply.github.com' || true)
[[ -z "$e" ]] && pass "no email addresses in tracked files" || bad "email addresses: $e"
p=$(git grep -n -I -E '\+91[ -]?[6-9][0-9]{9}' || true)
[[ -z "$p" ]] && pass "no phone numbers" || bad "phone numbers: $p"
s=$(git ls-files data/study | grep -c . || true)
[[ "$s" == 0 ]] && pass "no human study ledgers (pilot not yet run)" || warn "data/study has $s files: check consent (M8) and the publication line for each"
warn "commit author emails are public git metadata: $(git log --format='%ae' --all | sort -u | tr '\n' ' ')"

# 4. Licences and attribution.
for x in LICENSE-MIT LICENSE-APACHE THIRD_PARTY.md assets/cues/VOICE_LICENSE.md; do
    [[ -s "$x" ]] && pass "$x present" || bad "$x missing"
done
grep -q 'en_US-ljspeech-medium' assets/cues/RENDERED_WITH.txt 2>/dev/null && pass "cue bank rendered with en_US-ljspeech-medium" || bad "assets/cues/RENDERED_WITH.txt does not name the ljspeech voice"
git ls-files | grep -q '\.onnx$' && bad "an .onnx voice model is tracked (do not vendor it)" || pass "no .onnx model vendored"
for x in DerivedData/ build/ xcuserdata/; do
    grep -qx "$x" .gitignore && pass ".gitignore has $x" || bad ".gitignore lacks $x"
done

# 5. Wording.
o=$(git grep -n -i -w -E 'validated|proven|efficacy shown|E1 (green|passed)' -- README.md STUDY.md docs/EFFICACY.md docs/LATENCY.md docs/VIDEO_SCRIPT.md docs/RELEASE_NOTES.md \
    | grep -v -i -E "do not|don't|never|not say|cannot say|\"(validated|proven|efficacy shown|E1 passed)\"" || true)
[[ -z "$o" ]] && pass "no overclaiming words in the public documents" || bad "overclaiming: $o"
# A word in quotes is a list of words not to use (VIDEO_SCRIPT.md's "Do not say"), not a claim.
u=$(git grep -n -w 'users' -- README.md STUDY.md docs/VIDEO_SCRIPT.md | grep -v -i "not say\|don't\|\"users\"" || true)
[[ -z "$u" ]] && pass "no \"users\"" || bad "\"users\": $u"
grep -q 'SIMULATED sensors\|SIMULATED' README.md && pass "README says SIMULATED" || bad "README lacks the SIMULATED claim"
grep -q 'Human pilot: planned, pre-registered, not yet run' README.md STUDY.md && pass "pilot labelled not yet run" || bad "pilot label missing"
grep -q -i 'endorsed by\|not endorsed' README.md && pass "README states no endorsement" || bad "README lacks the no-endorsement line"

echo
(( fail )) && { echo "Fix the FAIL lines before making the repository public."; exit 1; }
echo "OK: ready to make public (re-run on the day you do)."
