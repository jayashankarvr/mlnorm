// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Visual-spoofing + archaic-sequence fixes (native review #25).
//!
//! Users (and old editors) produce sequences that *look* right but are
//! semantically wrong, breaking search/sort/NLP. This pass rewrites them to the
//! single canonical codepoint. It only ever touches the malformed sequences;
//! clean modern text passes through unchanged (so it cannot corrupt good input).
//!
//! Fixes (all native-ratified):
//!   - `െ` + `െ`  (U+0D46 U+0D46) → `ൈ` (U+0D48)   two E signs faking the AI sign
//!   - `ഉ` + `ൗ`  (U+0D09 U+0D57) → `ഊ` (U+0D0A)   indep U + AU mark faking UU
//!   - `എ` + `െ`  (U+0D0E U+0D46) → `ഐ` (U+0D10)   indep E + E sign faking indep AI
//!   - `ു` + `്`  (U+0D41 U+0D4D) → `്`  (U+0D4D)   archaic samvruthokaram → bare virama
//!   - `ൎ`        (U+0D4E)         → `ർ` (U+0D7C)   historical dot-reph → modern chillu RR
//!
//! Deliberately NOT done (native cautions):
//!   - `ൌ` (U+0D4C) → `ൗ`, could be unsafe depending on corpus integrity (skip).
//!   - `ററ` → `റ്റ` (tta digraph), ambiguous with a genuine ര+ര (rara); needs
//!     context, handled elsewhere if ever (see correctness_issues.csv #25).
//!
//! Runs on the NFC'd char buffer before chillu atomization. Idempotent: none of
//! the output characters is the left side of any rule.

pub(crate) fn map(chars: &[char]) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match (c, next) {
            ('\u{0D46}', Some('\u{0D46}')) => { out.push('\u{0D48}'); i += 2; } // ee → ai sign
            ('\u{0D09}', Some('\u{0D57}')) => { out.push('\u{0D0A}'); i += 2; } // u+au → uu
            ('\u{0D0E}', Some('\u{0D46}')) => { out.push('\u{0D10}'); i += 2; } // E+e → AI
            ('\u{0D4E}', _) => { out.push('\u{0D7C}'); i += 1; }                // dot-reph → chillu RR
            ('\u{0D4D}', _) => {
                // archaic samvruthokaram: a vowel-sign-U immediately before a virama
                // is dropped (പാലു് → പാല്). Handled on the virama and looping so a
                // run of u-signs collapses in ONE pass, keeps normalize idempotent.
                while out.last() == Some(&'\u{0D41}') {
                    out.pop();
                }
                out.push('\u{0D4D}');
                i += 1;
            }
            _ => { out.push(c); i += 1; }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::map;

    fn run(s: &str) -> String {
        map(&s.chars().collect::<Vec<_>>()).into_iter().collect()
    }

    #[test]
    fn fixes_visual_spoofs() {
        assert_eq!(run("\u{0D46}\u{0D46}"), "\u{0D48}");           // െെ → ൈ
        assert_eq!(run("\u{0D09}\u{0D57}"), "\u{0D0A}");           // ഉൗ → ഊ
        assert_eq!(run("\u{0D0E}\u{0D46}"), "\u{0D10}");           // എെ → ഐ
    }

    #[test]
    fn fixes_samvruthokaram_and_dot_reph() {
        // കു് (ka + u-sign + virama) → ക് (ka + virama)
        assert_eq!(run("\u{0D15}\u{0D41}\u{0D4D}"), "\u{0D15}\u{0D4D}");
        // തൎക്കം dot-reph → തർക്കം (ർ chillu RR)
        assert_eq!(run("\u{0D24}\u{0D4E}\u{0D15}"), "\u{0D24}\u{0D7C}\u{0D15}");
    }

    #[test]
    fn clean_text_unchanged() {
        let clean = "\u{0D15}\u{0D41}\u{0D24}\u{0D4D}\u{0D24}\u{0D3F}"; // കുത്തി-ish
        assert_eq!(run(clean), clean);
        assert_eq!(run("\u{0D48}"), "\u{0D48}"); // already ൈ
    }

    #[test]
    fn idempotent() {
        let once = run("\u{0D46}\u{0D46}\u{0D09}\u{0D57}");
        assert_eq!(run(&once), once);
    }
}
