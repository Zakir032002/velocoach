# velocoach-rs mini-study: does a cue move slider effort, and does phrasing matter?

**Human pilot: planned, pre-registered, not yet run.** No human session has taken place.

Status: pre-registration, committed before the first counted session. This file is frozen:
results go to [docs/study/RESULTS.md](docs/study/RESULTS.md), and any later change is a dated
deviation below.
Protocol hash: the git short hash of the last commit that changed this file
(`git log -1 --format=%h -- STUDY.md`), written on every Study-mode record
(`scripts/check-protocol-hash.sh`).
Author: Zakir Shaik. Date: 2026-10-16.

This is a feasibility pilot of an instrument, not evidence of efficacy.
It uses a slider proxy, not cycling. It is not medical research and has not
been reviewed by an ethics board.

## Instrument (frozen for the pilot)
- Observer `vc-efficacy 0.3.0` (`OBSERVER_VERSION`); no observer change during the pilot.
- Its validation against a known simulated rider does **not** meet its criterion: E1 gives
  15/20 seeds within tolerance on development seeds and 17/20 on held-out seeds, against
  >= 18/20 ([EFFICACY.md](docs/EFFICACY.md#e1-the-observer-recovers-a-known-rider--criterion-not-met)).
  Every compliance proportion from this pilot is reported with that caveat.
- The whole protocol has been run end to end in software, with simulated personas standing
  in for people ([docs/sim-study/report.html](docs/sim-study/report.html),
  `scripts/sim-study.sh`). That is a pipeline check, not data about people.

## Question
Does a spoken cue change slider-proxy effort within 15 s, and does phrasing matter?

## Design
Within-person, micro-randomised.
- Decision points: eligible P2 directives (they pass staleness, profile,
  cooldown and the observer window). Solo profile, 20 s cooldown.
- Allocation at each decision point, from PCG32(session seed, draw index):
  sham 0.2 (no audio, no CUE_PLAYED; the cooldown still starts),
  direct phrasing 0.4, autonomy-preserving phrasing 0.4. Propensity logged.
- Both phrasings follow the same rules: verb first, at most one number,
  8-12 syllables. The earcon is identical and always on.
- P1 cues (safety, sensor dropout, step transitions) are never randomised
  or suppressed. The tuner is frozen (Study mode); policy_hash is constant.

## Participants
The author plus up to 2 consenting adult friends, pseudonymised R01-R03.
No health data: heart rate is simulated; the only input is the slider.
R00 is the uncounted dry run and is excluded from every analysis. A session in which the author is
the only rider, before the pilot, is labelled "n=1 author dry run (engineering feasibility, not
evidence of efficacy)"; its ledgers live in `data/dry-run/`, apart from `data/study/`.

## Sessions
- 3 x 20 min per person on the `creep_pilot` scenario in human mode: 18 cycles of
  a slow power creep (0.8 W/s, at most 1 W/s, no grade change; 40 s, then 20 s
  easy) that the rider counteracts with the slider. The slider starts at 70 %.
  At least 20 minutes between a person's sessions.
- Session seed = 1000 x rider number + session number (R02 session 3 -> 2003).
- Expected 15-20 decision points per session: 45-60 per person, 100-180 total.
- The rig shows no cue information; the only signal is the earbud.

## Outcomes
Primary:
1. Complied proportion, cue (direct or autonomy) vs sham.
2. Complied proportion, autonomy (B) vs direct (A).
Secondary: L_onset median and IQR per arm; L_end; verdict mix; sham
false-positive rate; Partial and Reversed rates.

## Analysis (pre-specified; implemented in vc-report)
- Wilson 95 % intervals on every proportion.
- Within-person differences, pooled as the mean of per-person differences.
- Person-stratified bootstrap 95 % CI: B = 10,000, seed 20261013.
- Confounded and Invalid decision points are excluded and counted.
- No headline p-values.

## Data quality checks (vc-report --audit)
M2 arm frequencies within 99 % binomial bounds and seed replay; M3 no P1
record with an arm; M4 one policy_hash across Study sessions; M5 at least
90 % of decision points not Invalid and the 95th-percentile clock-sync bound
within +/-15 ms.

## Reporting language (mandatory)
"feasibility pilot"; "slider proxy, not cycling"; "n = [decision points]
from [people] people"; "not powered to detect differences under ~0.2";
"personas are assumptions". Reaction-time figures are floor references only:
Zak et al. 2023 (Sensors 23(8):3898) measured simple visual reaction with a
brake-lever response (median 0.348 s riding); Buchholtz & Burgess 2020
(S Afr J Sports Med 32(1)) measured online simple reaction while cycling
(medians 0.37 s males, 0.42 s females). Neither involves understanding a
9-10 syllable phrase.

## Data handling
Pseudonymous JSONL ledgers (velocoach.irp.v1), kept locally and, aggregated,
in the public repository. Any participant can stop at any time and have their
data deleted on request.

## Consent (read and signed before a participant's first session)
I agree to take part in a short, informal usability test of a personal software
project (velocoach-rs) by Zakir. I will move a slider on a phone in response to
spoken prompts in earbuds for three 20-minute sessions. No health or
physiological data is collected; only slider positions, timings and the app's
decisions are recorded under a pseudonym (e.g., R02). I can stop at any time
without giving a reason, and I can ask for my data to be deleted at any time.
Aggregated, pseudonymous results may be published in a public GitHub repository
and shown to a prospective employer. This is not medical research and has not
been reviewed by an ethics board. Keep earbud volume at a comfortable level.
Optional (default no): [ ] I agree that my pseudonymous per-cue log (slider
timings and the app's decisions, under R0x) may be published.
Name: ________ Date: ________

## Deviations
Every change after the commit above is listed here, dated, with its reason.

### 2026-10-16 (before the first counted session): aborted sessions
If a session ends early for a technical reason (an unrecovered disconnect, a
crash, audio failing), its data up to that moment is kept and analysed like any
other, and the session is not repeated: repeating it would give that person
more exposure than the protocol allows. The time and cause are logged. If a
participant stops a session themselves, the same rule applies.

### 2026-10-16 (before the first counted session): publishing pseudonymous ledgers
STUDY.md's data-handling section and the consent text allow publishing
aggregated, pseudonymous results. Criterion M6 (every figure regenerated from
committed pseudonymous JSONL) needs the per-cue ledgers themselves. The consent
sheet therefore gains one optional line: "I agree that my pseudonymous per-cue
log (slider timings and the app's decisions, under R0x) may be published."
Ledgers are committed only for participants who tick it; for the others, only
aggregated figures are published, and M6 is reported for the published subset.

## Results
Not yet run. The template is [docs/study/RESULTS.md](docs/study/RESULTS.md).
