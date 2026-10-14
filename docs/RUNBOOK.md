# Study runbook

The operating procedure for the pilot in [STUDY.md](../STUDY.md) (Days 16-18).
Print it; tick it every time. Most mistakes in small studies are a wrong rider
number or a forgotten setting, not a bug.

Rules that hold from the first counted session to the last:

- The code is frozen for anything that touches decisions, verdicts or the
  ledger. A bug found mid-study is written down; fixing it would make sessions
  before and after the fix incomparable. Interface-only fixes (a label, a
  button) are allowed and logged in DEVLOG.
- Every change to the protocol is a deviation, dated and explained in STUDY.md
  before it takes effect.
- The researcher says nothing about cues during a session. No hints, no
  reactions, no "good". The rider's only signal is the earbud.
- Raw data never leaves your machines except as STUDY.md says. Participants are
  R02 and R03 everywhere, including file names and DEVLOG.

## Before (about 10 minutes)

- [ ] `scripts/check-protocol-hash.sh` prints `ok`; the installed build is the one it checked
- [ ] Both phones charging; the rig app in front, screen on; quiet room
- [ ] Earbuds connected; the iPhone's audio route shows Bluetooth; calibration
      clip played, volume comfortable
- [ ] Route delay setting still filled in (same earbuds as the loopback)
- [ ] Rig: "I am the rider" on, slider at 60 %
- [ ] iPhone: Study on, rider R0__, session __ -> the screen shows seed __
      (1000 x rider + session)
- [ ] iPhone: Human rider on, scenario `creep`; `policy_hash` on screen equals
      the one in SESSIONS.md
- [ ] For a friend: consent signed (first session only), briefing read from the card

## During (20 minutes)

- [ ] Start, then the iPhone face down; the researcher does not watch the iPhone
- [ ] No talking about cues, effort or the slider; no reactions
- [ ] Note anything unusual with its clock time (a noise, a pause, a question asked)

## After (about 10 minutes)

- [ ] Stop after the scenario ends (the iPhone shows "Session running" until then)
- [ ] Export ride (capture + ledger) by AirDrop; file into `runs/study/R0__/`
- [ ] `cargo run -q -p vc-cli -- replay runs/study/R0__/ride-*.vcraw` -> identical
- [ ] `cargo run -q -p vc-report -- runs/study/R0__/*.jsonl --audit` -> M3, M4
      and M5 pass for this session (M2 is meaningful only over many sessions)
- [ ] SESSIONS.md entry written
- [ ] Next session for this person not before 20 minutes from now

```
mkdir -p runs/study/R0__
mv ~/Downloads/ledger-*.jsonl ~/Downloads/ride-*.vcraw runs/study/R0__/
```

`runs/` is gitignored. The pseudonymous ledgers are copied into the repository
only at the end, aggregated, as STUDY.md's data-handling section says.

## The session log

`runs/study/SESSIONS.md` (private, never committed), one entry per session:

```
## R01 S1 — 2026-10-__ __:__
- seed 1001 · protocol_hash __ · policy_hash __ · build __
- duration 20:00 · decision points __ (sham __, direct __, autonomy __)
- replay identical: yes · audit: M3 __ M4 __ M5 __ (valid __ %, p95 bound __ us)
- notes: __
- deviations: none
```

## Your three sessions (R01)

| Hour | Session | Seed |
|---|---|---|
| H3 | R01 S1 | 1001 |
| H4 | R01 S2 | 1002 |
| H5 | R01 S3 | 1003 |

You are a participant who knows everything: the arms, the phrases, the
hypotheses. That cannot be undone, so it is reported, not hidden. Two things
keep it from getting worse: ride the way you would ride if the coach were real
(respond to what a cue asks, not to which phrasing you recognise), and do not
look at your own results until all sessions are finished. EFFICACY.md's
limitations name it in one sentence: "R01 is the author and not blind to the
arms."

Between sessions, stay away from the code. The 20-minute break is for the
participant, not for a fix.

## Consent (M8)

M8 is "written consent for every participant". The text is in STUDY.md, plus the
optional publication line. Print two copies of the sheet: one for them, one for
you.

1. Confirm they are an adult (18 or over).
2. Give them the sheet and time to read it. Say the three things that matter in
   your own words: it is a test of software, not of them; they can stop at any
   time without a reason; their data is deleted on request.
3. The publication line is optional and its default is no. Do not encourage
   either answer.
4. Both sign and date both copies. They keep one.

The paper with their name stays on paper. The electronic record never contains a
name: `runs/study/CONSENT.md` holds one line per participant.

```
| Pseudonym | Consent date | Publication of per-cue log | Sessions |
|---|---|---|---|
| R01 | (author) | yes | 3 |
| R02 | 2026-10-__ | yes / no | __ |
```

## The briefing card

Read it word for word, so every participant gets the same instructions. It
describes the task and nothing else: no arms, no phrasings, no shams, no
hypotheses, no "we think".

> You'll ride a virtual bike for 20 minutes using this slider: further right is
> harder. Put the earbuds in. Now and then you'll hear a short tone and a spoken
> prompt. Respond with the slider however you think a rider would. I won't talk
> during the session, and I can't answer questions about the prompts until all
> your sessions are done. You can stop at any time.

If they ask a question during a session, the answer is "I'll tell you everything
after your last session." Write the question and its time in SESSIONS.md.

Their first few cues are where most of a newcomer's confusion shows (a delayed
reaction, a slider pushed the wrong way). That is data, not a problem to fix: no
coaching between sessions either.

## Seeds

| Person | Session 1 | Session 2 | Session 3 |
|---|---|---|---|
| R00 (dry run, uncounted) | 1 | | |
| R01 | 1001 | 1002 | 1003 |
| R02 | 2001 | 2002 | 2003 |
| R03 | 3001 | 3002 | 3003 |

## Backups, checksums, an audit without peeking

At the end of every study day, every file from every session exists in three
places, with checksums proving none changed:

```
cd runs/study
shasum -a 256 R0*/* > SHA256SUMS            # proves later that nothing was edited
zip -er ~/study-backup-day17.zip . -x '*.DS_Store'   # encrypted copy: an external disk and private cloud storage
shasum -a 256 -c SHA256SUMS                  # all OK
```

Then the interim audit. Run the data-quality checks across everything so far,
but do not render the outcome panels: looking at results mid-study is how
analyses quietly change.

```
cargo run -q -p vc-report -- runs/study/R0*/*.jsonl --exclude-rider R00 --audit --audit-only
```

M2 (arm frequencies) starts to mean something once about 60 decision points
exist; M3, M4 and M5 must pass on every run. If M4 shows two `policy_hash`
values, stop and find out why before the next session: two hashes means two
different policies, and the sessions would not be comparable.

## The last five sessions (Day 18)

| Hour | Session | Seed | First? |
|---|---|---|---|
| H1 | R02 S2 | 2002 | |
| H2 | R02 S3 | 2003 | |
| H3 | R03 S1 | 3001 | consent + briefing card |
| H4 | R03 S2 | 3002 | |
| H5 | R03 S3 | 3003 | |

Same runbook, same card, same silence. Re-check the hash and the build before
H1: overnight is when a stray rebuild or an expired profile happens.

After each participant's last session, tell them what the study was about: the
two phrasings, the shams, what was measured. Ask, and write down without
judgement, whether they noticed anything (that some prompts were worded
differently; that tones sometimes came without words). Shams play nothing at
all, so a participant can never have heard one. If someone reports noticing a
pattern, it goes in the Results as a limitation, not as a reason to drop their
data. Ask R02, debriefed after H2, not to discuss it with R03 until the evening.

### K4: no sessions after Day 18

Whatever is collected by the end of H5 is the study.

| If... | Then |
|---|---|
| A friend cannot come today | the study has two people (R01, R02). n shrinks to about 90-120 decision points; say so in the Results |
| A session aborts for a technical reason | the aborted-session rule: keep its data, do not repeat it, log time and cause |
| A participant stops partway | the same: keep what exists, no pressure, no repeat |
| A participant withdraws and asks for deletion | delete their files and their line in every backup the same day; record "one participant withdrew" (without a pseudonym) in the Results; rerun the analysis without them |
| Everything fails (no friend sessions at all) | the pilot is you alone, three sessions, reported as a single-person feasibility run |

Do not "top up" a short day with an extra session of your own: the sessions per
person were fixed in STUDY.md, and extra sessions for R01 would make the author
the dominant source of data.

At the end of H5, run the backup routine once more (checksums, encrypted copy),
then nothing is added to `runs/study/` again.

## The audit and the analysis (Day 18 H6)

Everything here was decided before the first session: the checks, the
statistics, the wording. This hour only runs them and writes down what came out.

```
cd runs/study && shasum -a 256 -c SHA256SUMS && cd -          # nothing edited since collection
for f in runs/study/R0*/ride-*.vcraw; do cargo run -q -p vc-cli -- replay "$f"; done   # every session identical
cargo run -q -p vc-report -- runs/study/R0*/*.jsonl --exclude-rider R00 \
  -o runs/study/results.html --audit | tee runs/study/audit.txt
```

All five audit lines must say PASS before any result is read. If one fails, the
Results say so, with the reason, and the affected sessions are reported both
with and without (the pre-registration does not allow quietly dropping them).

Then fill STUDY.md's Results section from `results.html` and `audit.txt` (write
numbers, not adjectives). For M6, copy the ledgers of R01 and of each friend who
ticked the publication line into `data/study/` (the JSONL only; the captures
stay private), render the report from exactly those files and commit both:

```
mkdir -p data/study docs/study
cp runs/study/R01/ledger-*.jsonl data/study/        # plus each consenting friend's
cargo run -q -p vc-report -- data/study/*.jsonl -o docs/study/results.html
```

CI then proves M6 by regenerating `docs/study/results.html` from
`data/study/*.jsonl` and comparing the bytes.

## If something fails

| Symptom | Likely cause | Fix |
|---|---|---|
| soak-check lists rows right after the disconnect as unexplained | reconnect slower than the 20 s grace | look at the capture: if every channel recovered without help, widen the grace for disconnects only and say so; if not, it is a real failure |
| The soak log stops for minutes | iPhone suspended the app (screen locked without background modes) | run with the screen on; record the finding |
| The seed on screen is wrong | rider or session picker not set | the runbook's second iPhone line; a wrong-seed session is kept and logged as a deviation, not rerun |
| `policy_hash` differs between sessions | a rebuild with a different configuration, or Study mode not on | stop; find the cause before the next session; report the sessions separately |
| A participant's first session has many Reversed verdicts | the slider's direction was misunderstood | data, not a bug: report it; the briefing card is not changed mid-study |
| `shasum -c` reports a changed file | something edited a ledger | restore it from the backup; find what touched it |
