#!/usr/bin/env python3
"""d_route from a loopback video. The iPhone flashes white and clicks at the same
host time; the iQOO films both. Prints, for each flash, the delay to the click's
onset, then the median and IQR.

Run on the earbuds video and on the speaker video:
  d_route(earbuds) = median(earbuds) - median(speaker) + d_route(speaker)

usage: loopback.py video.mp4
needs: ffmpeg + ffprobe on PATH, numpy
"""
import json
import subprocess
import sys

import numpy as np

SR = 48_000


def frame_times(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "frame=best_effort_timestamp_time", "-of", "json", path],
        capture_output=True, text=True, check=True,
    ).stdout
    return np.array([float(f["best_effort_timestamp_time"]) for f in json.loads(out)["frames"]])


def frame_luma(path):
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-vsync", "passthrough", "-vf", "scale=16:16,format=gray", "-f", "rawvideo", "-"],
        capture_output=True, check=True,
    ).stdout
    return np.frombuffer(raw, np.uint8).reshape(-1, 256).mean(axis=1)


def audio(path):
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-ac", "1", "-ar", str(SR), "-f", "s16le", "-"],
        capture_output=True, check=True,
    ).stdout
    return np.frombuffer(raw, np.int16).astype(np.float64)


def rising(times, signal, threshold, refractory):
    edges, last = [], -1e9
    above = signal > threshold
    for i in np.flatnonzero(above[1:] & ~above[:-1]) + 1:
        if times[i] - last >= refractory:
            edges.append(times[i])
            last = times[i]
    return np.array(edges)


def main(path):
    t_frames, luma = frame_times(path), frame_luma(path)
    n = min(len(t_frames), len(luma))
    t_frames, luma = t_frames[:n], luma[:n]
    flashes = rising(t_frames, luma, (luma.min() + luma.max()) / 2, 1.0)

    x = audio(path)
    env = np.convolve(np.abs(x), np.ones(24) / 24, mode="same")  # 0.5 ms
    t_audio = np.arange(len(env)) / SR
    clicks = rising(t_audio, env, 8 * np.median(env) + 1e-9, 1.0)

    deltas = []
    for f in flashes:
        after = clicks[(clicks >= f - 0.1) & (clicks <= f + 0.6)]
        if len(after):
            deltas.append((after[0] - f) * 1000)
    d = np.array(deltas)
    print(f"{path}: {len(flashes)} flashes, {len(clicks)} clicks, {len(d)} pairs")
    for i, v in enumerate(d, 1):
        print(f"  {i:2d}  {v:7.1f} ms")
    if len(d):
        q1, med, q3 = np.percentile(d, [25, 50, 75])
        print(f"median {med:.1f} ms  IQR {q1:.1f}-{q3:.1f} ms  (frame ~ {1000 * np.median(np.diff(t_frames)):.1f} ms)")


if __name__ == "__main__":
    main(sys.argv[1])
