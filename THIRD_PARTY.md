# Third-party material

What the repository uses but did not write, with licences. Back to the [README](README.md).

| What | Where | Licence | Notes |
|---|---|---|---|
| Piper TTS (the engine, `piper-tts` 1.8.0) | scripts/render_cues.sh, tools/render_phrases.py (build-time tool; neither the engine nor the model is vendored) | GPL-3.0 (OHF-Voice/piper1-gpl); only its audio output is committed | version recorded in assets/cues/RENDERED_WITH.txt |
| Piper voice `en_US-ljspeech-medium` | assets/cues/*.wav (rendered) | trained on LJ Speech, public domain ("All text, audio, and annotations are in the public domain", keithito.com/LJ-Speech-Dataset) | model card, dataset and exact command: [assets/cues/VOICE_LICENSE.md](assets/cues/VOICE_LICENSE.md). Not lessac or its derivatives; not macOS `say` |
| Rust crates | Cargo.lock | see `cargo tree`; direct dependencies include anyhow, heapless, libm, serde, uniffi | all dependencies must be MIT / Apache-2.0 compatible: confirm with `cargo deny` or `cargo license` before tagging |
| UniFFI | vc-ffi | MPL-2.0 | used as a library and as the binding generator (apps/ios/VcCore, tools/uniffi-bindgen) |
| GoldenCheetah WPrime.cpp | consulted, not copied (docs/WPRIME_GC.md) | GPL-2.0 | the rule was read; tools/gc_wbal.py and WbalGc are independent code |
| Aki Kakko's posts (Alphanome Knowledge Hub) | parameter names, profiles, earcon-first rule, phrase rules | cited with URL and UTC date in the README's Credits | velocoach responds to / operationalises them; no endorsement implied; not copied as text |
| Apache-2.0 licence text | LICENSE-APACHE | Apache-2.0 | verbatim from apache.org |
