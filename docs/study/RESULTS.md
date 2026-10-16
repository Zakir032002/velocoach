# Pilot results

**Human pilot: planned, pre-registered ([STUDY.md](../../STUDY.md)), not yet run.** Every blank
below stays blank until it is run.

Feasibility pilot · slider proxy, not cycling · personas are assumptions.
n = __ decision points from __ people (R01-R0__) in __ sessions; __ excluded
(__ Confounded, __ Invalid). Not powered to detect differences under ~0.2.
No p-values are reported. Analysis: `vc-report data/study/*.jsonl --exclude-rider R00 --audit` at commit __.

## Data quality (vc-report --audit)
M2 arm frequencies sham __ / direct __ / autonomy __ (99 % bounds: pass/fail);
seed replay __ mismatches. M3 __ P1 records with an arm. M4 __ policy_hash
value(s). M5 __ % of decision points not Invalid; p95 sync bound __ us.
Every session's capture replayed identically: yes/no.

## Primary outcomes
1. Cue vs sham: complied __/__ (__; Wilson 95 % __-__) with a cue,
   __/__ (__; __-__) with a sham. Pooled within-person difference __
   (bootstrap 95 % CI __ to __, B = 10,000, seed 20261013).
2. Autonomy (B) vs direct (A): __/__ (__) vs __/__ (__). Pooled within-person
   difference __ (95 % CI __ to __).

## Secondary outcomes
L_onset median (IQR): direct __ s (__-__), autonomy __ s (__-__).
L_end: __. Sham false-positive rate: __ (Wilson __-__).
Verdict mix: Partial __, Reversed __, Ignored __.
Coincidental first moves: __.

## Per person
(the per-person table from the report)

## Deviations
(each, as recorded, with its date: the aborted-session rule; the publication line;
anything else)

## Limitations
- A slider proxy, not cycling; simulated heart rate; one pair of earbuds.
- __ people, __ decision points: estimates are wide by design.
- R01 is the author and not blind to the arms or the hypotheses.
- Participants were briefed identically but were friends of the author.
- Compliance is not benefit: a cue followed is not a cue that improved anything.
- Reaction-time references are floors only (Zak et al. 2023; Buchholtz &
  Burgess 2020); neither involves understanding a 9-10 syllable phrase.
- Personas used to validate the observer (E1, E8) are assumptions, not data.

## Consent
__ participants, __ signed consent sheets (M8); __ agreed to publication of
their pseudonymous per-cue log.
