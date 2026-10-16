#!/usr/bin/env bash
# Render the spoken cues with Piper (voice en_US-ljspeech-medium, LJ Speech dataset: public
# domain), add the earcons, and write the app's cue bank to assets/cues/ (WAVs + manifest.json).
# The .onnx voice is downloaded to build/voices/ and never committed; the rendered WAVs are.
# Provenance and licences: assets/cues/VOICE_LICENSE.md.
set -euo pipefail
cd "$(dirname "$0")/.."

VOICE="${PIPER_VOICE:-en_US-ljspeech-medium}"
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/ljspeech/medium"
MODEL="build/voices/$VOICE.onnx"
PY="build/piper-venv/bin/python"

run() { echo "+ $*"; "$@"; }

mkdir -p build/voices build/phrases
if [[ ! -x "$PY" ]]; then
  run python3 -m venv build/piper-venv
  run "$PY" -m pip install -q "piper-tts>=1.3,<2"
fi
for ext in .onnx .onnx.json; do
  [[ -s "build/voices/$VOICE$ext" ]] && continue
  run curl -fL --retry 3 -o "build/voices/$VOICE$ext" "$BASE/$VOICE$ext?download=true"
done

run cargo run -q -p vc-cli -- phrases > build/phrases/phrases.tsv
run "$PY" tools/render_phrases.py build/phrases/phrases.tsv "$MODEL" build/phrases
run python3 tools/make_earcons.py build/phrases assets/cues
"$PY" -m pip show piper-tts | awk '/^Version/ {print "piper-tts " $2}' > assets/cues/RENDERED_WITH.txt
echo "voice $VOICE" >> assets/cues/RENDERED_WITH.txt
echo "OK: assets/cues ($(ls assets/cues/*.wav | wc -l | tr -d ' ') WAVs)"
