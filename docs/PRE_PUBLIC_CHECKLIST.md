# Before making the repository public

Run `scripts/pre-public-check.sh` on the day (it is read-only and needs
`brew install gitleaks`). Then tick the items below by hand. Back to the
[README](../README.md).

Last automated run: 2026-10-16, all PASS. gitleaks 8.21.2 found no leaks in 39 commits (full
history). No signing files, keystores, `local.properties`, DerivedData or xcuserdata are
tracked. No email addresses or phone numbers are in tracked files. Both LICENSE files,
THIRD_PARTY.md and VOICE_LICENSE.md are present. No `.onnx` file is committed. The overclaim
scan is clean.

- [ ] `scripts/pre-public-check.sh` passes.
- [ ] Commit author emails are public git metadata (the script prints them). If you'd rather
      not publish an address, set GitHub's noreply address for future commits; history keeps
      the old ones.
- [ ] No consent sheet, participant name or phone screenshot with notifications in `results/`
      or the video.
- [ ] The README status table reflects the latest `results/<stamp>/SUMMARY.md`: every phone
      number either measured-on-hardware with its source, or "pending".
- [ ] Re-check Aki's Knowledge Hub for new posts and update the README's Credits if needed
      ([FACT_CHECK.md](FACT_CHECK.md)).
- [ ] The free-team signing profile is not committed (Xcode keeps it in ~/Library, which is
      outside the repository).
- [ ] Settings > General > Change visibility > Public. Then merge the PR and tag `v0.1.0` /
      `v0.2.0` per [RELEASE_NOTES.md](RELEASE_NOTES.md).
