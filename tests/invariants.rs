// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Property-based invariant tests.
//!
//! A poor-man's stand-in for the `cargo-fuzz` target the doc requires for the
//! 1.0 freeze (`fuzz/`): proptest throws adversarial input at `normalize` and
//! asserts the contract invariants. Malayalam from the wild is adversarial by
//! accident, so the generators include the joiners, virama, chillus, and the
//! nta consonants specifically.

use proptest::prelude::*;

use mlnormalize::{normalize, stripped_key};

const ZWJ: char = '\u{200D}';
const ZWNJ: char = '\u{200C}';
const VIRAMA: char = '\u{0D4D}';

/// A character set biased toward the codepoints the normalizer reasons about,
/// mixed with arbitrary chars so we also hit the pass-through path.
fn norm_char() -> impl Strategy<Value = char> {
    prop_oneof![
        // joiners + virama + nta + a spread of chillus and base consonants
        Just('\u{200C}'), // ZWNJ
        Just('\u{200D}'), // ZWJ
        Just('\u{0D4D}'), // virama
        Just('\u{0D31}'), // RRA
        Just('\u{0D28}'), // NA
        Just('\u{0D7B}'), // CHILLU N
        Just('\u{0D23}'),
        Just('\u{0D30}'),
        Just('\u{0D32}'),
        Just('\u{0D33}'),
        Just('\u{0D15}'),
        // archaic / vowel-sign neighbourhood
        (0x0D00u32..=0x0D7Fu32).prop_filter_map("valid char", char::from_u32),
        any::<char>(),
    ]
}

fn norm_string() -> impl Strategy<Value = String> {
    prop::collection::vec(norm_char(), 0..40).prop_map(|v| v.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    /// normalize never panics on any input.
    #[test]
    fn never_panics(s in any::<String>()) {
        let _ = normalize(&s);
    }

    /// Idempotent: normalize(normalize(x)) == normalize(x).
    #[test]
    fn idempotent_arbitrary(s in any::<String>()) {
        let once = normalize(&s);
        prop_assert_eq!(normalize(&once), once);
    }

    /// Idempotent on the joiner-heavy generator (the interesting cases).
    #[test]
    fn idempotent_targeted(s in norm_string()) {
        let once = normalize(&s);
        prop_assert_eq!(normalize(&once), once);
    }

    /// Output never contains a ZWJ, step 4 strips them all, unconditionally.
    #[test]
    fn no_zwj_in_output(s in norm_string()) {
        let out = normalize(&s);
        prop_assert!(!out.contains(ZWJ));
    }

    /// Every surviving ZWNJ sits in one of the four load-bearing slots (native
    /// review #24): virama+ZWNJ+consonant, consonant+ZWNJ+virama,
    /// chillu+ZWNJ+consonant, or consonant+ZWNJ+vowel-sign. Never word-final,
    /// doubled, or between vowels.
    #[test]
    fn surviving_zwnj_in_a_keep_slot(s in norm_string()) {
        let is_cons = |c: char| matches!(c, '\u{0D15}'..='\u{0D39}' | '\u{0D7A}'..='\u{0D7F}');
        let is_chillu = |c: char| matches!(c, '\u{0D7A}'..='\u{0D7F}');
        let is_vsign = |c: char| matches!(c, '\u{0D3E}'..='\u{0D4C}' | '\u{0D57}' | '\u{0D62}'..='\u{0D63}');
        let out = normalize(&s);
        let chars: Vec<char> = out.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            if c == ZWNJ {
                prop_assert!(i > 0 && i + 1 < chars.len(), "ZWNJ at boundary: {out:?}");
                let p = chars[i - 1];
                let n = chars[i + 1];
                let ok = (p == VIRAMA && is_cons(n))
                    || (is_cons(p) && n == VIRAMA)
                    || (is_chillu(p) && is_cons(n))
                    || (is_cons(p) && is_vsign(n));
                prop_assert!(ok, "ZWNJ not in a keep slot: prev={:?} next={:?} in {out:?}", p, n);
            }
        }
    }

    /// Deterministic within a run.
    #[test]
    fn deterministic(s in norm_string()) {
        prop_assert_eq!(normalize(&s), normalize(&s));
    }

    /// stripped_key never panics.
    #[test]
    fn stripped_key_never_panics(s in any::<String>()) {
        let _ = stripped_key(&s);
    }

    /// stripped_key is idempotent.
    #[test]
    fn stripped_key_idempotent(s in norm_string()) {
        let once = stripped_key(&s);
        prop_assert_eq!(stripped_key(&once), once);
    }

    /// stripped_key contains no joiners at all (that's the whole point).
    #[test]
    fn stripped_key_has_no_joiners(s in norm_string()) {
        let out = stripped_key(&s);
        prop_assert!(!out.contains(ZWJ) && !out.contains(ZWNJ));
    }
}
