#!/usr/bin/env bash
# Render the phrase bank with Piper, then build the earcon + speech buffers for the iPhone.
set -euo pipefail
cd "$(dirname "$0")/.."
VOICE="${PIPER_VOICE:-voices/en_US-lessac-medium.onnx}"

command -v piper >/dev/null || { echo "piper not found: install it and put it on PATH"; exit 1; }
[[ -f "$VOICE" && -f "$VOICE.json" ]] || { echo "voice missing: $VOICE and $VOICE.json"; exit 1; }

mkdir -p build/phrases
cargo run -q -p vc-cli -- phrases > build/phrases/phrases.tsv

while IFS=$'\t' read -r key text _ _; do
  printf '%s' "$text" | piper --model "$VOICE" --output_file "build/phrases/$key.wav" 2>/dev/null
  echo "rendered $key"
done < build/phrases/phrases.tsv

python3 tools/make_earcons.py build/phrases apps/ios/VeloCoach/Cues
