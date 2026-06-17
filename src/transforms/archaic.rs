// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Step 7, archaic codepoint map (0.2.0).
//!
//! Digitized literature and Wikisource carry a long tail of archaic Malayalam
//! codepoints that no modern text uses. Map the ones with a clean modern
//! equivalent, and drop the archaic numeric/date signs that carry no information a
//! modern reader (or model) needs.
//!
//! The map is deliberately small and explicit, a per-codepoint table, written
//! once. It is **not** a frequency-audited long tail of ligatures; that audit
//! needs the corpus (a downstream task / blocker) and any additions are a *minor*
//! version bump within the 1.x line as long as they only touch codepoints that do
//! not appear in clean modern text (so existing artifacts are unaffected).

/// Map an archaic codepoint to its modern replacement, or `None` to delete it
/// (archaic signs with no modern equivalent), or, if the char is not archaic, 
/// the caller keeps it as-is.
///
/// `Some(Some(c))` = replace with `c`; `Some(None)` = delete; `None` = not archaic.
#[inline]
fn archaic_map(c: char) -> Option<Option<char>> {
    Some(match c {
        // U+0D5F MALAYALAM LETTER ARCHAIC II -> ഈ (U+0D08). The canonical example.
        '\u{0D5F}' => Some('\u{0D08}'),
        // NOTE: U+0D29 NNNA (alveolar nasal ṉ) and U+0D3A TTTA (alveolar ṯ) are
        // deliberately NOT folded (native review #18). They encode a real alveolar
        // distinction still used in scholarly/phonetic transcription (e.g. Kerala
        // Panineeyam); folding them into ന/റ would destroy that semantic content.
        // They pass through unchanged.
        // Archaic numeric / fraction / date signs: no modern equivalent, drop.
        '\u{0D70}'        // MALAYALAM NUMBER TEN (archaic)
        | '\u{0D71}'      // MALAYALAM NUMBER ONE HUNDRED
        | '\u{0D72}'      // MALAYALAM NUMBER ONE THOUSAND
        | '\u{0D73}'      // MALAYALAM FRACTION ONE QUARTER
        | '\u{0D74}'      // MALAYALAM FRACTION ONE HALF
        | '\u{0D75}'      // MALAYALAM FRACTION THREE QUARTERS
        | '\u{0D76}'      // MALAYALAM FRACTION ONE SIXTEENTH
        | '\u{0D77}'      // MALAYALAM FRACTION ONE EIGHTH
        | '\u{0D78}'      // MALAYALAM FRACTION THREE SIXTEENTHS
        | '\u{0D79}' => None, // MALAYALAM DATE MARK
        _ => return None,
    })
}

/// Apply the archaic map over the char buffer. Operates on the already-NFC,
/// joiner-filtered char slice from the core pipeline.
pub(crate) fn map(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    for &c in chars {
        match archaic_map(c) {
            Some(Some(replacement)) => out.push(replacement),
            Some(None) => {} // delete
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archaic_ii_to_modern_ii() {
        // ൟ (U+0D5F) -> ഈ (U+0D08)
        assert_eq!(map(&['\u{0D5F}']), vec!['\u{0D08}']);
    }

    #[test]
    fn archaic_date_mark_deleted() {
        assert_eq!(
            map(&['\u{0D15}', '\u{0D79}', '\u{0D15}']),
            vec!['\u{0D15}', '\u{0D15}']
        );
    }

    #[test]
    fn modern_text_untouched() {
        let s: Vec<char> = "ന്റ".chars().collect();
        assert_eq!(map(&s), s);
    }
}
