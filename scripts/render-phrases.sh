#!/usr/bin/env bash
# Kept for the Day 10 instructions: the cue bank is now made by render_cues.sh.
exec "$(dirname "$0")/render_cues.sh" "$@"
