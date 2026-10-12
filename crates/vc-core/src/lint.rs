//! Aki's cue rules as code. Strict: verb first, at most one number,
//! 8–12 syllables. Terse: verb first, at most one number, at most 8.
//! Syllables come only from the dictionary below; an unknown word fails.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintProfile {
    Strict,
    Terse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintError {
    Empty,
    Digits,
    NotVerbFirst,
    TooManyNumbers(u8),
    UnknownWord(u8),
    Syllables(u8),
}

pub const VERBS: &[&str] = &[
    "back",
    "begin",
    "check",
    "drop",
    "ease",
    "finish",
    "go",
    "hold",
    "keep",
    "lower",
    "push",
    "raise",
    "save",
    "soft-pedal",
    "spin",
    "start",
    "stop",
    "surge",
];

pub const NUMBER_WORDS: &[&str] = &[
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "twenty",
    "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety", "hundred",
];

pub const SYLLABLES: &[(&str, u8)] = &[
    ("at", 1),
    ("back", 1),
    ("by", 1),
    ("cadence", 2),
    ("check", 1),
    ("choose", 1),
    ("connection", 3),
    ("drifting", 2),
    ("drop", 1),
    ("ease", 1),
    ("for", 1),
    ("go", 1),
    ("heart", 1),
    ("high", 1),
    ("hold", 1),
    ("if", 1),
    ("in", 1),
    ("into", 2),
    ("is", 1),
    ("it", 1),
    ("keep", 1),
    ("later", 2),
    ("like", 1),
    ("meter", 2),
    ("now", 1),
    ("off", 1),
    ("or", 1),
    ("pace", 1),
    ("plan", 1),
    ("power", 2),
    ("raise", 1),
    ("rate", 1),
    ("reserve", 2),
    ("right", 1),
    ("save", 1),
    ("seconds", 2),
    ("soft-pedal", 3),
    ("surge", 1),
    ("target", 2),
    ("tempo", 2),
    ("the", 1),
    ("too", 1),
    ("watts", 1),
    ("you", 1),
    ("high", 1),
    ("you're", 1),
    ("your", 1),
    ("one", 1),
    ("two", 1),
    ("three", 1),
    ("four", 1),
    ("five", 1),
    ("six", 1),
    ("seven", 2),
    ("eight", 1),
    ("nine", 1),
    ("ten", 1),
    ("twenty", 2),
    ("thirty", 2),
    ("forty", 2),
    ("fifty", 2),
    ("sixty", 2),
    ("seventy", 3),
    ("eighty", 2),
    ("ninety", 2),
    ("hundred", 2),
];

fn clean(w: &str) -> &str {
    w.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '\'' || c == '-'))
}

fn in_list(list: &[&str], w: &str) -> bool {
    list.iter().any(|v| v.eq_ignore_ascii_case(w))
}

fn syllables(w: &str) -> Option<u8> {
    SYLLABLES
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(w))
        .map(|(_, n)| *n)
}

/// Lint one sentence. Returns its syllable count.
pub fn lint(text: &str, profile: LintProfile) -> Result<u8, LintError> {
    if text.chars().any(|c| c.is_ascii_digit()) {
        return Err(LintError::Digits);
    }

    let (mut total, mut runs, mut in_run, mut words) = (0u8, 0u8, false, 0u8);

    for (i, w) in text
        .split_whitespace()
        .map(clean)
        .filter(|w| !w.is_empty())
        .enumerate()
    {
        if i == 0 && !in_list(VERBS, w) {
            return Err(LintError::NotVerbFirst);
        }

        let is_number = in_list(NUMBER_WORDS, w);
        if is_number && !in_run {
            runs += 1;
        }
        in_run = is_number;

        let n = syllables(w).ok_or(LintError::UnknownWord(i.min(255) as u8))?;
        total = total.saturating_add(n);
        words = words.saturating_add(1);
    }

    if words == 0 {
        return Err(LintError::Empty);
    }
    if runs > 1 {
        return Err(LintError::TooManyNumbers(runs));
    }

    let ok = match profile {
        LintProfile::Strict => (8..=12).contains(&total),
        LintProfile::Terse => total <= 8,
    };

    if ok {
        Ok(total)
    } else {
        Err(LintError::Syllables(total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LintError::*;
    use LintProfile::*;

    #[test]
    fn lint_counts_syllables_from_the_dictionary() {
        assert_eq!(
            lint("Ease right off now, heart rate is too high", Strict),
            Ok(9)
        );
        assert_eq!(lint("Surge in five", Terse), Ok(3));
        assert_eq!(
            lint("Soft-pedal now, or keep it if you choose?", Strict),
            Ok(10)
        );
    }

    #[test]
    fn lint_catches_each_kind_of_violation() {
        // Made-up phrases only: nothing here grades anyone's published examples.
        assert_eq!(lint("Hold 300 watts", Strict), Err(Digits));
        assert_eq!(
            lint("Cadence is too low, raise it now", Strict),
            Err(NotVerbFirst)
        );
        assert_eq!(
            lint("Hold two hundred watts for ten seconds now", Strict),
            Err(TooManyNumbers(2))
        );
        assert_eq!(lint("Ease off", Strict), Err(Syllables(2)));
        assert_eq!(
            lint("Ease off now, your reserve is at twenty", Terse),
            Err(Syllables(10))
        );
        assert_eq!(
            lint("Ease off now, magnificent rider", Strict),
            Err(UnknownWord(3))
        );
        assert_eq!(lint(" ?! ", Strict), Err(Empty));
    }
}
