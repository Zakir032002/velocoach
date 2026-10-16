# Video script (≤ 3:00)

What to show: both phones, the BLE link, a spoken cue, the live verdict, the report card, and
the honest status table. It replaces the older plan in [DEMOS.md](DEMOS.md#the-video-as-the-plan-specifies-it),
which assumed pilot data and a measured earbud figure; neither exists.
Back to the [README](../README.md).

Before recording:

- Do [PHONE_CHECKLIST.md](PHONE_CHECKLIST.md) steps 1–3 once as a rehearsal.
- Copy your measured numbers into the README status table. Any number you don't have stays
  "pending" on screen, and you say "pending".
- Set do-not-disturb on both phones and hide Wi-Fi names and notifications.

The script is about 390 words, roughly 2.2 words a second. Read numbers exactly as they appear
on screen.

| Time | Shot | Voiceover |
|---|---|---|
| 0:00–0:15 | both phones and an earbud on the desk; slow push-in | "This is velocoach: a voice coach in Rust that measures whether its own cues work. The iPhone is the coach. The Android phone simulates the rider and the sensors. The sensors are simulated; the Bluetooth between the phones is real." |
| 0:15–0:35 | iQOO rig screen: `advertising true`, then `central …` and the subscribed channels; iPhone: `link: ready` | "The rig advertises a power meter, a heart-rate strap and a cadence sensor over Bluetooth Low Energy. The iPhone subscribes to them like real sensors. Every packet is followed by a timestamp, so latency is measured across the two clocks, with an error bound." |
| 0:35–1:20 | iPhone, scenario `climb_surge`: Start, then the step counter and quality line; the earcon, then the phrase (earbud close to the mic, caption "speaker-played" if you play it on the speaker); the verdict badge `✓ … L_obs` | "A tone, then the phrase. The core is a sans-IO Rust engine, the same code on the Mac and on the phone. It decided this cue from the power stream. Then it watches what happens: did power move at least half the asked amount, compared with where it was already heading, and hold for three seconds? Here: complied. About one point nine seconds of that latency is smoothing. That figure is derived from the filter, not fitted." |
| 1:20–1:40 | (optional) human mode: the iQOO slider, `I am the rider`; ease the slider after a cue | "In human mode I'm the rider, on a slider. The same observer scores my response." |
| 1:40–2:05 | iPhone **Report card**, slowly; the red banner "SIMULATED DATA — persona …, seeds …, observer_version …" | "Every ride ends in a report card. Every page says what the data is: simulated, with the persona, the seeds and the observer version. Shams, cues decided but not played, are scored the same way, so the observer's own false positives are counted too." |
| 2:05–2:45 | the README **Status** table, scrolled row by row; pause on E1 and the pending rows | "And this is the honest status. Software checks run in CI: golden rides, no allocation per packet, property tests that adaptive code can't reach safety. The observer's ground-truth test does not meet its criterion: fifteen of twenty seeds, seventeen of twenty on held-out seeds, against eighteen. Phone latency: [read your measured row, or say 'pending']. Earbud latency is not measured. The human pilot is pre-registered and not yet run." |
| 2:45–3:00 | the GitHub repository page, the README's first screen; end card with the URL | "Everything is in the repository: the code, the tests, the failing seeds, and what I'd do next." |

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
