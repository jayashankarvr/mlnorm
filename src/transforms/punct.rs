// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Step 9, punctuation normalization (0.2.0).
//!
//! Collapse the many Unicode encodings of the same punctuation mark to one ASCII
//! form so the tokenizer doesn't fragment on typographic variation:
//!
//! - smart quotes (single + double, plus the low-9 variants) → ASCII `'` / `"`
//! - en/em dash, horizontal bar → ASCII `-`
//! - the danda and double danda (U+0964/U+0965) → ASCII `.`, Malayalam prose uses
//!   the ASCII period as its sentence terminator; the Devanagari-derived danda
//!   shows up only in transliterated/imported text and means the same stop.
//! - the ellipsis character (U+2026) → three ASCII dots
//! - NBSP and the narrow/figure spaces → a plain ASCII space
//!
//! One direction, frozen for the 1.x line. Note ellipsis maps to a *string*, so
//! this rule is the one transform that can change char count; it runs last and the
//! final recompose pass in `crate::normalize` is a no-op over ASCII, so it stays
//! idempotent.

/// Map a single punctuation char to its canonical ASCII string, or `None` to keep
/// it unchanged. Most map to a single char; ellipsis is the one expansion.
#[inline]
pub(crate) fn canonical(c: char) -> Option<&'static str> {
    Some(match c {
        // Single quotes / apostrophes.
        '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{2032}' => "'",
        // Double quotes.
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{2033}' => "\"",
        // Dashes.
        '\u{2013}' | '\u{2014}' | '\u{2015}' | '\u{2212}' => "-",
        // Danda / double danda -> period.
        '\u{0964}' | '\u{0965}' => ".",
        // Ellipsis -> three dots.
        '\u{2026}' => "...",
        // Various spaces -> ASCII space.
        '\u{00A0}' | '\u{2007}' | '\u{2009}' | '\u{202F}' => " ",
        _ => return None,
    })
}

/// Normalize punctuation over the char buffer.
pub(crate) fn normalize_punct(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    for &c in chars {
        match canonical(c) {
            Some(s) => out.extend(s.chars()),
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_quotes() {
        let s: Vec<char> = "\u{201C}hi\u{201D} \u{2018}x\u{2019}".chars().collect();
        let got: String = normalize_punct(&s).into_iter().collect();
        assert_eq!(got, "\"hi\" 'x'");
    }

    #[test]
    fn dashes_and_ellipsis() {
        let s: Vec<char> = "a\u{2014}b\u{2026}".chars().collect();
        let got: String = normalize_punct(&s).into_iter().collect();
        assert_eq!(got, "a-b...");
    }

    #[test]
    fn danda_to_period() {
        let s: Vec<char> = "വാക്യം\u{0964}".chars().collect();
        let got: String = normalize_punct(&s).into_iter().collect();
        assert_eq!(got, "വാക്യം.");
    }

    #[test]
    fn nbsp_to_space() {
        let s: Vec<char> = "a\u{00A0}b".chars().collect();
        let got: String = normalize_punct(&s).into_iter().collect();
        assert_eq!(got, "a b");
    }

    #[test]
    fn ascii_punct_untouched() {
        let s: Vec<char> = "a-b. \"c\" 'd'".chars().collect();
        let got: String = normalize_punct(&s).into_iter().collect();
        assert_eq!(got, "a-b. \"c\" 'd'");
    }
}
