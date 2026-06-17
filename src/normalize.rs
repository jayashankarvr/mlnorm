// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Core normalization logic for Malayalam text, the ordered transform pipeline.
//!
//! This is a **sealed, concrete** unit by deliberate design (see
//! DESIGN_PRINCIPLES.md "The normalizer, concrete, sealed, no trait"). There is
//! no trait, no config object, and no extension point: the entire system's
//! correctness rests on there being exactly one canonical byte output per input,
//! and polymorphism here is a footgun. Internally the rules are an ordered list
//! of small functions (DRY: each rule written once, composed in sequence). The
//! 0.2.0 transforms live in the [`crate::transforms`] submodules.
//!
//! ## Pipeline (full pipeline, steps 1 through 9 of `docs/components/mlnorm.md`)
//!
//! ```text
//! 1. mojibake repair         fix double-encoded UTF-8 (runs first, conservative)
//! 2. NFC                     canonical composition
//! 3. chillu atomization      consonant + virama + ZWJ -> atomic chillu (U+0D7A..U+0D7F)
//! 4. ZWJ strip               remove all remaining ZWJ
//! 5. ZWNJ positional filter  keep iff virama + ZWNJ + consonant, else strip
//! 6. nta canonicalization    pick one ന്റ encoding (base NA + virama + RRA), map the other
//! 7. archaic codepoint map   ൟ->ഈ, archaic numeric/date signs dropped
//! 8. digit normalization     Malayalam digits -> ASCII (one direction)
//! 9. punctuation normalize   danda, smart quotes, ellipsis -> ASCII
//! + final NFC recompose      re-compose any pair exposed by joiner removal
//! ```
//!
//! ## Two documented ordering decisions
//!
//! 1. **nta after joiner strip (not before, as the doc's numbering implies).** A
//!    stray ZWJ wedged inside an nta cluster (`ൻ + ് + ZWJ + റ`) would otherwise
//!    hide the cluster from nta matching on pass 1 and reveal it on pass 2,
//!    breaking idempotency. Per the doc, every ZWJ after chillu atomization is
//!    noise, so stripping before nta matching changes no clean-input output and
//!    closes the hole. Regression: golden `nta_chillu_with_stray_zwj` + unit
//!    `nta_with_stray_zwj_is_idempotent`.
//!
//! 2. **A final NFC recompose pass.** Removing a joiner can place two codepoints
//!    adjacent that NFC composes (e.g. `െ U+0D46 + ZWNJ + U+0D57` -> after strip ->
//!    `U+0D46 U+0D57`, which composes to `ൌ U+0D4C`). The initial NFC ran *before*
//!    the strip, so without a second pass `normalize` would not be idempotent
//!    (pass 2 would compose what pass 1 left apart). Regression: golden
//!    `recompose_after_zwnj_strip` + proptest `idempotent_*`.

use unicode_normalization::UnicodeNormalization;

use crate::transforms::{archaic, digits, mojibake, punct, spoof};

// Relevant codepoints, named once and reused.

/// U+0D4D MALAYALAM SIGN VIRAMA (chandrakkala).
const VIRAMA: char = '\u{0D4D}';
/// U+200D ZERO WIDTH JOINER.
const ZWJ: char = '\u{200D}';
/// U+200C ZERO WIDTH NON-JOINER.
const ZWNJ: char = '\u{200C}';
/// U+0D31 MALAYALAM LETTER RRA, the second consonant of the /nṯa/ cluster.
const RRA: char = '\u{0D31}';
/// U+0D7B MALAYALAM LETTER CHILLU N, atomic chillu for ന, the non-canonical
/// first element of the /nṯa/ cluster's alternate encoding.
const CHILLU_N: char = '\u{0D7B}';
/// U+0D28 MALAYALAM LETTER NA, the canonical first element of /nṯa/.
const NA: char = '\u{0D28}';

/// Map a base consonant to its atomic chillu, for the
/// `consonant + virama + ZWJ` legacy sequence. Returns `None` for consonants
/// that have no atomic chillu form.
///
/// Table is exactly the one in `docs/components/mlnorm.md`:
///
/// | base | atomic |
/// |------|--------|
/// | ണ U+0D23 | ൺ U+0D7A |
/// | ന U+0D28 | ൻ U+0D7B |
/// | ര U+0D30 | ർ U+0D7C |
/// | ല U+0D32 | ൽ U+0D7D |
/// | ള U+0D33 | ൾ U+0D7E |
/// | ക U+0D15 | ൿ U+0D7F |
#[inline]
fn chillu_for(base: char) -> Option<char> {
    Some(match base {
        '\u{0D23}' => '\u{0D7A}', // ണ -> ൺ
        '\u{0D28}' => '\u{0D7B}', // ന -> ൻ
        '\u{0D30}' => '\u{0D7C}', // ര -> ർ
        '\u{0D32}' => '\u{0D7D}', // ല -> ൽ
        '\u{0D33}' => '\u{0D7E}', // ള -> ൾ
        '\u{0D15}' => '\u{0D7F}', // ക -> ൿ
        _ => return None,
    })
}

/// Is `c` a Malayalam consonant letter (the base-consonant block U+0D15..U+0D39,
/// plus the atomic chillu letters U+0D7A..U+0D7F)?
///
/// Used by the ZWNJ positional filter: a ZWNJ is kept only when it sits between
/// a virama and a following consonant (the legitimate conjunct-prevention slot).
#[inline]
fn is_consonant(c: char) -> bool {
    matches!(c, '\u{0D15}'..='\u{0D39}' | '\u{0D7A}'..='\u{0D7F}')
}

/// Is `c` an atomic chillu letter (U+0D7A..U+0D7F)? Used by the ZWNJ filter's
/// "chillu + ZWNJ + consonant" keep rule (block archaic stacking).
#[inline]
fn is_chillu(c: char) -> bool {
    matches!(c, '\u{0D7A}'..='\u{0D7F}')
}

/// Is `c` a Malayalam dependent vowel sign (matra)? The matra block
/// U+0D3E..U+0D4C (AA..AU, NOT the virama U+0D4D), the AU length mark U+0D57,
/// and the vocalic-l signs U+0D62/U+0D63. Used by the ZWNJ filter's
/// "consonant + ZWNJ + vowel-sign" keep rule (force a non-conjoining vowel).
#[inline]
fn is_vowel_sign(c: char) -> bool {
    matches!(c, '\u{0D3E}'..='\u{0D4C}' | '\u{0D57}' | '\u{0D62}'..='\u{0D63}')
}

/// Canonical normalization. Idempotent, deterministic, no locale dependence.
///
/// See the module docs for the ordered pipeline. This is the public entry point;
/// `crate::normalize` simply forwards to it.
pub fn normalize(input: &str) -> String {
    // Step 1: mojibake repair, before anything else (every later rule assumes
    // valid singly-encoded UTF-8). Conservative: a no-op on ASCII and clean text.
    let repaired = mojibake::repair(input);

    // Fast exit for the common ASCII case: NFC is a no-op on ASCII and none of
    // the Malayalam rules can fire, so the input is already canonical. (Run after
    // mojibake repair, which can only *introduce* non-ASCII, never the reverse.)
    if repaired.is_ascii() {
        return repaired;
    }

    // Step 2: NFC. Collect once into a Vec<char> so later passes can look ahead
    // and behind by index without re-decoding UTF-8 in the inner loop.
    let chars: Vec<char> = repaired.nfc().collect();

    // Step 3: chillu atomization. Consumes the chillu-forming ZWJs, so it must
    // run while they are still present (before any ZWJ stripping).
    let chars = atomize_chillus(&chars);

    // Steps 4 + 5: strip all remaining ZWJs, and apply the ZWNJ positional
    // filter, in one pass. (Run before nta, see module docs, ordering decision 1.)
    let chars = strip_joiners_to_chars(&chars);

    // Step 7: archaic codepoint map.
    let chars = archaic::map(&chars);

    // Step 8: Malayalam digits -> ASCII.
    let chars = digits::normalize_digits(&chars);

    // Step 9: punctuation -> canonical ASCII.
    let chars = punct::normalize_punct(&chars);

    // Step 9.5 (post-deletion fixups): visual-spoofing + archaic-sequence fixes (native review #25):
    // െെ->ൈ, ഉൗ->ഊ, എെ->ഐ, samvruthokaram ു්->්, dot-reph->chillu RR. Ordering is
    // load-bearing for idempotency: run it AFTER every deletion pass (ZWJ/ZWNJ
    // strip, archaic, digits, punct) so a stripped/deleted char that newly
    // juxtaposes ു and ് is still cleaned (proptest: "ു ZWNJ ്"), but BEFORE nta
    // so the cluster spoof can create by dropping ു (e.g. ൻ ു ് റ -> ൻ ് റ) is
    // then canonicalized to form A by nta (proptest: "ൻ ു ് റ"). A no-op on clean text.
    let chars = spoof::map(&chars);

    // Step 6, deferred: nta canonicalization, run *after* the archaic map. This is
    // a second instance of ordering decision 1 (see module docs): the archaic map
    // can *delete* a codepoint wedged inside an nta cluster (e.g. an archaic
    // number `ൻ ൰ ് റ`) or *rewrite* a char into an nta consonant, either of which
    // would hide/reveal the cluster across passes and break idempotency if nta ran
    // first. Running nta last over the fully-mapped buffer closes that hole; it
    // changes no clean-input output. Regression: golden `nta_hidden_by_archaic`.
    let chars = canonicalize_nta(&chars);

    // Final recompose: removing joiners can expose an NFC-composable pair (see
    // module docs, ordering decision 2). A second NFC pass closes the idempotency
    // hole; it is a no-op when nothing was exposed.
    chars.into_iter().collect::<String>().nfc().collect()
}

/// Joiner-stripped projection for dedup/retrieval matching only, **not** training
/// text. (0.3.0 in the doc; the second output of the crate.)
///
/// `stripped_key(x)` is `normalize(x)` with *all* ZWJ **and** ZWNJ removed,
/// including the conjunct-prevention ZWNJs that [`normalize`] deliberately keeps
/// (step 5). The point is that two documents differing only in joiner placement, 
/// `സോഫ്റ്റ്‌വെയർ` with and without the internal ZWNJ, collapse to the same key,
/// so `mldata`'s dedup treats them as duplicates. **Train on `normalize`, match on
/// `stripped_key`** (see `docs/components/mlnorm.md`).
///
/// Implemented as `normalize` followed by an unconditional joiner strip, then a
/// recompose, so it inherits every canonicalization `normalize` does and stays
/// idempotent and deterministic. It is a strict projection: `stripped_key` is
/// always a fixpoint of itself and of `normalize` modulo the kept ZWNJs.
pub fn stripped_key(input: &str) -> String {
    let canonical = normalize(input);
    // The only joiner that can survive `normalize` is a kept ZWNJ (ZWJ is already
    // gone). Strip it. But removing that ZWNJ can place two codepoints adjacent
    // that a later rule cares about, e.g. `ൻ ് ZWNJ റ` (normalize keeps the ZWNJ,
    // so nta did not fire) becomes `ൻ ് റ`, an nta cluster, so we must re-run the
    // full pipeline over the stripped text rather than just recompose. Feeding the
    // stripped string back through `normalize` re-applies nta canonicalization and
    // NFC recompose, keeping `stripped_key` idempotent and canonical.
    let stripped: String = canonical
        .chars()
        .filter(|&c| c != ZWJ && c != ZWNJ)
        .collect();
    normalize(&stripped)
}

/// Step 3, chillu atomization.
///
/// Replace every `base + VIRAMA + ZWJ` chillu-forming sequence with its atomic
/// codepoint. After this pass the only ZWJs left are non-chillu noise (handled
/// in steps 4/5). A `base + VIRAMA + ZWJ` whose base has no atomic chillu is left
/// untouched here; its ZWJ is removed later by the unconditional ZWJ strip.
fn atomize_chillus(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == VIRAMA && chars[i + 2] == ZWJ {
            if let Some(atomic) = chillu_for(chars[i]) {
                out.push(atomic);
                i += 3;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Step 6, nta (/nṯa/) canonicalization.
///
/// The /nṯa/ cluster ന്റ has two live encodings:
///   A. `ന (U+0D28) + ് (U+0D4D) + റ (U+0D31)`, base NA + virama + RRA
///   B. `ൻ (U+0D7B) + ് (U+0D4D) + റ (U+0D31)`, atomic CHILLU-N + virama + RRA
///
/// **Canonical form chosen: A**, base NA + virama + RRA. Documented and frozen
/// for the 1.x major line (changing it is a major version bump).
fn canonicalize_nta(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if i + 2 < chars.len()
            && chars[i] == CHILLU_N
            && chars[i + 1] == VIRAMA
            && chars[i + 2] == RRA
        {
            // B (ൻ്റ) -> A (ന്റ): emit base NA, keep virama + RRA.
            out.push(NA);
            out.push(VIRAMA);
            out.push(RRA);
            i += 3;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Steps 4 + 5, ZWJ strip and ZWNJ positional filter, fused into one pass.
///
/// - **ZWJ (step 4):** stripped unconditionally. After chillu atomization the
///   only remaining ZWJs are rendering-preference noise.
/// - **ZWNJ (step 5):** kept in the four positional slots where it is structurally
///   load-bearing in Malayalam (native review #24); stripped everywhere else
///   (word-final, doubled, between vowels):
///     1. virama + ZWNJ + consonant, conjunct prevention (e.g. സോഫ്റ്റ്‌വെയർ)
///     2. consonant + ZWNJ + virama, modern *disconnected* conjunct (e.g. k‌ra)
///     3. chillu + ZWNJ + consonant, block archaic chillu+consonant stacking
///     4. consonant + ZWNJ + vowelsign, force a non-conjoining (separate) vowel
///   Why preserve, not just tolerate: if the model never sees these ZWNJs it never
///   learns to emit them, and generated technical/modern text renders wrong forever.
fn strip_joiners_to_chars(chars: &[char]) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    for (i, &c) in chars.iter().enumerate() {
        match c {
            ZWJ => {
                // Step 4: drop unconditionally.
            }
            ZWNJ => {
                // Step 5: keep in any of the four load-bearing slots, else strip.
                let prev = out.last().copied();
                let next = chars.get(i + 1).copied();
                let prev_consonant = prev.map(is_consonant).unwrap_or(false);
                let prev_virama = prev == Some(VIRAMA);
                let prev_chillu = prev.map(is_chillu).unwrap_or(false);
                let next_consonant = next.map(is_consonant).unwrap_or(false);
                let next_virama = next == Some(VIRAMA);
                let next_vowel_sign = next.map(is_vowel_sign).unwrap_or(false);
                let keep = (prev_virama && next_consonant)        // 1
                    || (prev_consonant && next_virama)             // 2
                    || (prev_chillu && next_consonant)             // 3
                    || (prev_consonant && next_vowel_sign);        // 4
                if keep {
                    out.push(ZWNJ);
                }
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chillu_atomization_all_six() {
        let cases = [
            ("\u{0D23}\u{0D4D}\u{200D}", "\u{0D7A}"), // ണ -> ൺ
            ("\u{0D28}\u{0D4D}\u{200D}", "\u{0D7B}"), // ന -> ൻ
            ("\u{0D30}\u{0D4D}\u{200D}", "\u{0D7C}"), // ര -> ർ
            ("\u{0D32}\u{0D4D}\u{200D}", "\u{0D7D}"), // ല -> ൽ
            ("\u{0D33}\u{0D4D}\u{200D}", "\u{0D7E}"), // ള -> ൾ
            ("\u{0D15}\u{0D4D}\u{200D}", "\u{0D7F}"), // ക -> ൿ
        ];
        for (legacy, atomic) in cases {
            assert_eq!(normalize(legacy), atomic, "legacy {legacy:?}");
            assert_eq!(normalize(atomic), atomic);
        }
    }

    #[test]
    fn nta_both_encodings_collapse_to_na_form() {
        let form_a = "\u{0D28}\u{0D4D}\u{0D31}"; // ന + ് + റ  (canonical)
        let form_b = "\u{0D7B}\u{0D4D}\u{0D31}"; // ൻ + ് + റ
        assert_eq!(normalize(form_a), form_a);
        assert_eq!(normalize(form_b), form_a);
    }

    #[test]
    fn legacy_nta_via_atomization() {
        let legacy = "\u{0D28}\u{0D4D}\u{200D}\u{0D4D}\u{0D31}";
        let canonical = "\u{0D28}\u{0D4D}\u{0D31}";
        assert_eq!(normalize(legacy), canonical);
    }

    #[test]
    fn nta_with_stray_zwj_is_idempotent() {
        let input = "\u{0D7B}\u{0D4D}\u{200D}\u{0D31}";
        let canonical = "\u{0D28}\u{0D4D}\u{0D31}"; // ന + ് + റ
        let once = normalize(input);
        assert_eq!(once, canonical);
        assert_eq!(normalize(&once), once);
    }

    #[test]
    fn zwj_non_chillu_stripped() {
        let input = "\u{0D15}\u{200D}\u{0D15}"; // ക ZWJ ക
        assert_eq!(normalize(input), "\u{0D15}\u{0D15}");
    }

    #[test]
    fn zwnj_kept_in_virama_consonant_slot() {
        let input = "\u{0D4D}\u{200C}\u{0D15}";
        assert_eq!(normalize(input), input);
    }

    #[test]
    fn zwnj_stripped_word_final_and_doubled() {
        assert_eq!(normalize("\u{0D15}\u{200C}"), "\u{0D15}");
        let input = "\u{0D4D}\u{200C}\u{200C}\u{0D15}";
        assert_eq!(normalize(input), "\u{0D4D}\u{200C}\u{0D15}");
    }

    #[test]
    fn recompose_after_zwnj_strip_is_idempotent() {
        // െ (U+0D46) + ZWNJ + U+0D57: the ZWNJ is stripped (no virama before), and
        // the now-adjacent pair composes under NFC to ൌ (U+0D4C). The final
        // recompose pass must make this a fixpoint on the first call.
        let input = "\u{0D46}\u{200C}\u{0D57}";
        let once = normalize(input);
        assert_eq!(once, "\u{0D4C}");
        assert_eq!(normalize(&once), once);
    }

    // --- native review #18: NNNA/TTTA preserved (not folded) ---

    #[test]
    fn nnna_ttta_preserved() {
        assert_eq!(normalize("\u{0D29}"), "\u{0D29}"); // ഩ NNNA stays
        assert_eq!(normalize("\u{0D3A}"), "\u{0D3A}"); // ഺ TTTA stays
        // and inside a word
        let w = "\u{0D15}\u{0D29}"; // ക + NNNA
        assert_eq!(normalize(w), w);
    }

    // --- native review #24: the three new ZWNJ keep-slots ---

    #[test]
    fn zwnj_kept_disconnected_conjunct() {
        // consonant + ZWNJ + virama + consonant: ക ZWNJ ് ര (disconnected k‌ra)
        let input = "\u{0D15}\u{200C}\u{0D4D}\u{0D30}";
        let out = normalize(input);
        assert!(out.contains('\u{200C}'), "ZWNJ should be kept: {out:?}");
        assert_eq!(normalize(&out), out); // idempotent
    }

    #[test]
    fn zwnj_kept_chillu_consonant() {
        // chillu + ZWNJ + consonant: ൾ ZWNJ മ (block archaic stacking)
        let input = "\u{0D7E}\u{200C}\u{0D2E}";
        assert!(normalize(input).contains('\u{200C}'));
    }

    #[test]
    fn zwnj_kept_consonant_vowel_sign() {
        // consonant + ZWNJ + vowel-sign: സ ZWNJ ു (non-conjoining vowel)
        let input = "\u{0D38}\u{200C}\u{0D41}";
        let out = normalize(input);
        assert!(out.contains('\u{200C}'), "ZWNJ should be kept: {out:?}");
        assert_eq!(normalize(&out), out);
    }

    #[test]
    fn zwnj_still_stripped_between_independent_vowels() {
        // independent vowel + ZWNJ + independent vowel: not a keep slot
        let input = "\u{0D05}\u{200C}\u{0D06}"; // അ ZWNJ ആ
        assert_eq!(normalize(input), "\u{0D05}\u{0D06}");
    }

    // --- native review #25: visual-spoofing fixes through the full pipeline ---

    #[test]
    fn spoof_fixes_through_pipeline() {
        // ക + െ + െ  (ka + two E signs)  ->  ക + ൈ
        assert_eq!(normalize("\u{0D15}\u{0D46}\u{0D46}"), "\u{0D15}\u{0D48}");
        // കു്  (ka + u-sign + virama, archaic samvruthokaram)  ->  ക്
        assert_eq!(normalize("\u{0D15}\u{0D41}\u{0D4D}"), "\u{0D15}\u{0D4D}");
        // തൎക്കം dot-reph -> തർക്കം
        assert_eq!(
            normalize("\u{0D24}\u{0D4E}\u{0D15}\u{0D4D}\u{0D15}\u{0D02}"),
            "\u{0D24}\u{0D7C}\u{0D15}\u{0D4D}\u{0D15}\u{0D02}"
        );
    }

    #[test]
    fn archaic_digits_punct_applied() {
        // ൟ -> ഈ, Malayalam digit -> ASCII, smart quote -> ASCII, danda -> period.
        assert_eq!(normalize("\u{0D5F}"), "\u{0D08}");
        assert_eq!(normalize("\u{0D68}\u{0D66}\u{0D68}\u{0D6A}"), "2024");
        assert_eq!(normalize("\u{201C}x\u{201D}"), "\"x\"");
        assert_eq!(normalize("a\u{0964}"), "a.");
    }

    #[test]
    fn idempotent_on_mixed() {
        let input = "സോഫ്റ്റ്\u{200C}വെയർ ന്റ ൻ\u{0D4D}റ Hello \u{0D68}\u{0D66}";
        let once = normalize(input);
        assert_eq!(normalize(&once), once);
    }

    // stripped_key

    #[test]
    fn stripped_key_removes_kept_zwnj() {
        // software: normalize keeps the internal ZWNJ; stripped_key removes it.
        let with = "സോഫ്റ്റ്\u{200C}വെയർ";
        let without = "സോഫ്റ്റ്വെയർ";
        assert_eq!(normalize(with), with); // normalize keeps it
        assert_eq!(stripped_key(with), stripped_key(without)); // dedup collapses
        assert!(!stripped_key(with).contains(ZWNJ));
    }

    #[test]
    fn stripped_key_idempotent_and_no_joiners() {
        let input = "സോഫ്റ്റ്\u{200C}വെയർ ന്റ Hello";
        let once = stripped_key(input);
        assert_eq!(stripped_key(&once), once);
        assert!(!once.contains(ZWJ) && !once.contains(ZWNJ));
    }
}
