#!/usr/bin/env python3
"""Earcons from Aki's table, then one buffer per cue variant:
earcon + 120 ms silence + speech. 16-bit mono WAV at the speech's sample rate,
plus manifest.json with every buffer's duration in ms. Standard library only.

usage: make_earcons.py <rendered-phrases-dir> <app-cues-dir>
"""
import json
import math
import struct
import sys
import wave
from pathlib import Path

GAP_MS = 120
AMP = 0.35  # earcons sit below the speech's peaks

# (start Hz, end Hz, ms, decay); a start of 0 Hz is silence.
EARCONS = {
    "ear_p0": [(440, 440, 120, 0), (0, 0, 80, 0), (440, 440, 120, 0)],  # low double beep
    "ear_p1": [(587, 587, 300, 4)],                                     # soft chime
    "ear_p2": [(300, 300, 150, 0), (200, 200, 150, 0)],                 # 300 -> 200 Hz dual-tone
    "ear_p3_high": [(880, 880, 120, 0), (660, 660, 120, 0)],            # falling: back off
    "ear_p3_low": [(660, 660, 120, 0), (880, 880, 120, 0)],             # rising: raise
    "ear_p4": [(1000, 1000, 30, 0)],                                    # click
    "ear_p5": [(523, 523, 400, 5)],                                     # bell
}


def tone(rate, segments):
    out = []
    ramp = 0.005 * rate
    for f0, f1, ms, decay in segments:
        n = int(rate * ms / 1000)
        phase = 0.0
        for i in range(n):
            if f0 == 0:
                out.append(0.0)
                continue
            f = f0 + (f1 - f0) * i / max(n - 1, 1)
            phase += 2 * math.pi * f / rate
            env = math.exp(-decay * i / n) if decay else 1.0
            fade = min(1.0, i / ramp, (n - 1 - i) / ramp)
            out.append(AMP * env * fade * math.sin(phase))
    return out


def read_wav(path):
    with wave.open(str(path), "rb") as w:
        assert w.getnchannels() == 1 and w.getsampwidth() == 2, f"{path}: need 16-bit mono"
        rate = w.getframerate()
        raw = w.readframes(w.getnframes())
    return rate, [s / 32768.0 for s in struct.unpack(f"<{len(raw) // 2}h", raw)]


def write_wav(path, rate, samples):
    clipped = [max(-1.0, min(1.0, s)) for s in samples]
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(struct.pack(f"<{len(clipped)}h", *(int(s * 32767) for s in clipped)))
    return round(len(samples) * 1000 / rate)


def main(src, out):
    src, out = Path(src), Path(out)
    out.mkdir(parents=True, exist_ok=True)
    rows = [line.rstrip("\n").split("\t") for line in (src / "phrases.tsv").read_text().splitlines() if line]

    speech, rate = {}, None
    for key, _text, _d, _e in rows:
        r, s = read_wav(src / f"{key}.wav")
        assert rate in (None, r), f"{key}: {r} Hz, others {rate} Hz"
        rate, speech[key] = r, s

    earcons = {name: tone(rate, segs) for name, segs in EARCONS.items()}
    gap = [0.0] * int(rate * GAP_MS / 1000)
    manifest = {"rate": rate, "gap_ms": GAP_MS, "buffers": {}}

    def emit(name, samples):
        manifest["buffers"][name] = {"file": f"{name}.wav", "ms": write_wav(out / f"{name}.wav", rate, samples)}

    for key, _text, default, enabled in rows:
        emit(key, speech[key])
        for ear in {default, enabled} - {"-"}:
            emit(f"{key}__{ear}", earcons[ear] + gap + speech[key])

    (out / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True))
    longest = max(b["ms"] for b in manifest["buffers"].values())
    print(f"{len(manifest['buffers'])} buffers at {rate} Hz; longest {longest} ms -> {out}")


if __name__ == "__main__":
    main(*sys.argv[1:3])
