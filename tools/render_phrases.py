#!/usr/bin/env python3
"""Render the phrase bank (`vc-cli phrases`, tab-separated: key, text, earcons) to one WAV per
phrase with a Piper voice, through the piper-tts Python API (>= 1.3).

usage: render_phrases.py <phrases.tsv> <voice.onnx> <out-dir>
"""
import sys
import wave
from pathlib import Path

from piper import PiperVoice


def main(tsv, model, out):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    voice = PiperVoice.load(model)
    rows = [line.split("\t") for line in Path(tsv).read_text().splitlines() if "\t" in line]
    for key, text, *_ in rows:
        with wave.open(str(out / f"{key}.wav"), "wb") as w:
            voice.synthesize_wav(text, w)
        print(f"rendered {key}: {text}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
