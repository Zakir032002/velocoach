# Video script (≤ 3:00)

What to show: both phones and the BLE link, a spoken cue with its live verdict, safety winning,
the observer refusing to guess, the tuner learning, the report card, and the honest status
table. Each scene is one feature; their setups are in
[WHAT_WE_BUILT.md § 3](WHAT_WE_BUILT.md#3-the-demo-scenes-one-feature-each). It replaces the
older plan in [DEMOS.md](DEMOS.md#the-video-as-the-plan-specifies-it), which assumed pilot data
and a measured earbud figure; neither exists.
Back to the [README](../README.md).

Before recording:

- Do [PHONE_CHECKLIST.md](PHONE_CHECKLIST.md) steps 1–3 once as a rehearsal.
- Copy your measured numbers into the README status table. Any number you don't have stays
  "pending" on screen, and you say "pending".
- Set do-not-disturb on both phones and hide Wi-Fi names and notifications.

The script is about 400 words, roughly 2.2 words a second. Read numbers exactly as they appear
on screen. Speed up every wait (4× or 8×); never speed up a cue or a verdict.

| Time | Shot (scene) | Voiceover |
|---|---|---|
| 0:00–0:12 | both phones and an earbud on the desk | "This is velocoach: a voice coach in Rust that measures whether its own cues work. The sensors are simulated; the Bluetooth between these two phones is real." |
| 0:12–0:27 | iQOO rig: `advertising true`, then `central …`; iPhone: `link: ready`, the latency panel | "The Android phone plays the rider, the power meter and the heart-rate strap. The iPhone is the coach. Every packet carries a timestamp, so latency is measured across both clocks, with an error bound." |
| 0:27–1:00 | **D1** iPhone `climb_surge` at 8:50: tone, "Ease off now, your reserve is at twenty", then `✓ … s` | "When the rider's reserve runs low, a tone, then one short phrase. Then the coach checks: did power move at least half the asked amount, compared with where it was already heading, and hold for three seconds? Complied. About one point nine seconds of that number is the filter's smoothing, derived, not fitted." |
| 1:00–1:22 | **D3** iPhone `recovery_drift`, redline at 847 s: a phrase cut off by the safety cue; then **D3b** terminal: the property tests passing | "Safety comes first. When heart rate stays near maximum, the safety cue interrupts anything playing. The adaptive code can't touch it: property tests throw random histories at the engine to check that." |
| 1:22–1:40 | **D4** terminal: `-> confounded … ? grade` | "If something else could explain the change, like a hill, a sensor dropout or coasting, the verdict is 'confounded'. It refuses to guess." |
| 1:40–2:00 | **D2** terminal: the `tuner_change` line | "In coach mode it adapts, inside Aki Kakko's published ranges. This simulated rider ignores orders but follows suggestions; after sixteen cues the tuner switches phrasing, and logs why." |
| 2:00–2:20 | **D7** iPhone **Report card**, the red SIMULATED DATA banner; the sham row | "Every ride ends in a report card, labelled with what the data is. Shams, cues decided but not played, are scored too, which measures the observer's own false positives." |
| 2:20–2:50 | **D8** README **Status** table, row by row; pause on E1 and latency | "The honest status: software checks run in CI. The ground-truth test does not meet its criterion: fifteen of twenty seeds, seventeen of twenty held out, against eighteen. Phone latency: [your row, or 'pending']. Earbud latency is not measured. The human pilot is pre-registered and not yet run." |
| 2:50–3:00 | the GitHub page; end card with the URL | "Everything is in the repository: the code, the tests, the failing seeds, and what I'd do next." |

Short of time: drop D3's phone shot and keep D3b (terminal only). Optional extra if it fits:
**D6** (you on the iQOO slider, "the same observer scores my own response").

## Do not say

- That the coach works, helps, or improves anything. Nothing here measures benefit.
- "validated", "proven", "users", "efficacy shown", or "E1 passed".
- Any earbud or audio-output latency number.
- Anything implying that Aki Kakko or Domesly endorsed this. Wording: velocoach "responds to"
  his posts.
- Any study result: the human pilot has not been run.

## Edit checklist

1. Voiceover first, end to end. Its length is the video's length; cut words if it runs over
   3:00.
2. Speed up waiting at 4× or 8× with a small label. Never speed up the cue or the verdict.
3. Put a caption on screen for every number. Each number must equal the README's.
4. Title card (≤ 2 s): "velocoach-rs · Zakir Shaik". End card: the repository URL.
5. Export 1080p H.264. Watch it once muted and once with sound. Check that no frame shows a
   notification, phone number, Wi-Fi name, email or anyone's name.
