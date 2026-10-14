# Third-party material

What the repository uses but did not write, with licences. Back to the [README](README.md).

| What | Where | Licence | Notes |
|---|---|---|---|
| Piper TTS (the engine) | scripts/render-phrases.sh (build-time tool, not shipped) | MIT (rhasspy/piper) | rendered offline; confirm the licence of the version you installed |
| Piper voice "en_US-lessac-medium" (the script's default) | apps/ios/VeloCoach/Cues/*.caf (rendered) | **__ (not verified)** | read the voice's MODEL_CARD before publishing; some Piper voices are trained on datasets with restricted (non-commercial) terms, and a trial demo for a company is a commercial context. If the voice is restricted, say so here and in the README, or re-render with a permissively licensed voice |
| Rust crates | Cargo.lock | see `cargo tree`; direct dependencies include anyhow, heapless, libm, serde, uniffi | all dependencies must be MIT / Apache-2.0 compatible: confirm with `cargo deny` or `cargo license` before tagging |
| UniFFI | vc-ffi | MPL-2.0 | used as a library and as the binding generator (apps/ios/VcCore, tools/uniffi-bindgen) |
| GoldenCheetah WPrime.cpp | consulted, not copied (docs/WPRIME_GC.md) | GPL-2.0 | the rule was read; tools/gc_wbal.py and WbalGc are independent code |
| Aki Kakko's Knowledge Hub | parameter names, profiles, earcon table, phrase rules | cited | used with credit, not copied as text |
| Apache-2.0 licence text | LICENSE-APACHE | Apache-2.0 | verbatim from apache.org |
