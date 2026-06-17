// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Step 1, mojibake repair (0.2.0). Runs **first**, before any other rule.
//!
//! "Mojibake" here is the classic double-encoding: text that was valid UTF-8,
//! decoded *as if* it were Latin-1 (ISO-8859-1) or Windows-1252, then re-encoded
//! as UTF-8. Each original byte `b` becomes the UTF-8 encoding of the codepoint
//! `U+00b` (Latin-1), so a 3-byte Malayalam character turns into three
//! Latin-1-range characters. Pre-2012 Malayalam web scrapes are full of this.
//!
//! ## Why it must run first
//!
//! Every later rule assumes valid, singly-encoded UTF-8. If we ran NFC or chillu
//! atomization over mojibake, we'd canonicalize garbage. Repairing here means the
//! rest of the pipeline only ever sees clean Malayalam codepoints.
//!
//! ## Conservative by construction
//!
//! A false repair corrupts good text, which is worse than leaving mojibake in, so
//! the bar to fire is deliberately high:
//!
//! 1. We only attempt repair on a contiguous run whose characters are **all** in
//!    the Latin-1 / Windows-1252 range that a mis-decode produces (`U+0080..U+00FF`
//!    plus the eight printable Windows-1252 punctuation codepoints in `U+2000`+).
//! 2. We map each such char back to its single source byte, decode the byte run
//!    as UTF-8, and **only** accept the result if it (a) decodes cleanly with no
//!    replacement chars and (b) decodes to at least one Malayalam codepoint
//!    (`U+0D00..U+0D7F`). A run that round-trips to more Latin text is left alone, 
//!    it was probably real Latin text, not mojibake.
//!
//! ASCII and well-formed Malayalam are untouched: neither contains a maximal run
//! of `U+0080..U+00FF` chars, so the detector never fires.

/// The eight printable Windows-1252 codepoints that live in `0x80..=0x9F` and map
/// to `U+2000`-block characters instead of C1 controls. Their inverse (char →
/// source byte) is needed to reconstruct cp1252 mojibake.
#[inline]
fn cp1252_high_byte(c: char) -> Option<u8> {
    Some(match c {
        '\u{20AC}' => 0x80, // €
        '\u{201A}' => 0x82, // ‚
        '\u{0192}' => 0x83, // ƒ
        '\u{201E}' => 0x84, // „
        '\u{2026}' => 0x85, // …
        '\u{2020}' => 0x86, // †
        '\u{2021}' => 0x87, // ‡
        '\u{02C6}' => 0x88, // ˆ
        '\u{2030}' => 0x89, // ‰
        '\u{0160}' => 0x8A, // Š
        '\u{2039}' => 0x8B, // ‹
        '\u{0152}' => 0x8C, // Œ
        '\u{017D}' => 0x8E, // Ž
        '\u{2018}' => 0x91, // ‘
        '\u{2019}' => 0x92, // ’
        '\u{201C}' => 0x93, // “
        '\u{201D}' => 0x94, // ”
        '\u{2022}' => 0x95, // •
        '\u{2013}' => 0x96, // en dash
        '\u{2014}' => 0x97, // em dash
        '\u{02DC}' => 0x98, // ˜
        '\u{2122}' => 0x99, // ™
        '\u{0161}' => 0x9A, // š
        '\u{203A}' => 0x9B, // ›
        '\u{0153}' => 0x9C, // œ
        '\u{017E}' => 0x9E, // ž
        '\u{0178}' => 0x9F, // Ÿ
        _ => return None,
    })
}

/// Map a single mojibake char back to the source byte it was mis-decoded from, if
/// it is in the recoverable range. `None` means "not a mojibake-shaped char".
#[inline]
fn source_byte(c: char) -> Option<u8> {
    match c as u32 {
        // Latin-1 high range (incl. the C1 controls U+0080..U+009F that a pure
        // Latin-1 mis-decode produces) maps 1:1 to its byte value.
        0x0080..=0x00FF => Some(c as u8),
        // The cp1252 printable exceptions: a Windows-1252 mis-decode turns the C1
        // byte into a U+2000-block char instead of the raw control.
        _ => cp1252_high_byte(c),
    }
}

/// Is `c` the first char of a potential mojibake run? A mojibake-encoded
/// Malayalam character always begins with a lead byte `0xE0` (Malayalam is in the
/// 3-byte UTF-8 range `U+0800..U+FFFF`, lead byte `0xE0..0xEF`, and the
/// Malayalam block specifically uses `0xE0`), which mis-decodes to `U+00E0` (à).
/// Anchoring the run to that lead byte keeps us from chewing on incidental
/// accented Latin.
#[inline]
fn is_run_anchor(c: char) -> bool {
    matches!(c as u32, 0xE0..=0xEF)
        || cp1252_high_byte(c)
            .map(|b| (0xE0..=0xEF).contains(&b))
            .unwrap_or(false)
}

/// True if the decoded run contains at least one Malayalam codepoint and no
/// U+FFFD replacement char, our accept gate.
#[inline]
fn looks_repaired(s: &str) -> bool {
    let mut saw_malayalam = false;
    for c in s.chars() {
        if c == '\u{FFFD}' {
            return false;
        }
        if matches!(c as u32, 0x0D00..=0x0D7F) {
            saw_malayalam = true;
        }
    }
    saw_malayalam
}

/// Repair double-encoded UTF-8 mojibake. Conservative: leaves anything it is not
/// confident about untouched.
pub(crate) fn repair(input: &str) -> String {
    // Cheap pre-check: no mojibake-range chars at all -> nothing to do. This keeps
    // the hot path (ASCII + clean Malayalam) allocation-free.
    if !input.chars().any(|c| source_byte(c).is_some()) {
        return input.to_string();
    }

    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        // Try to start a run only at a plausible 3-byte lead-byte anchor; this is
        // what makes the detector conservative about lone accented Latin chars.
        if is_run_anchor(chars[i]) {
            // Extend the run over every consecutive mojibake-range char.
            let start = i;
            let mut bytes: Vec<u8> = Vec::new();
            let mut j = i;
            while j < chars.len() {
                match source_byte(chars[j]) {
                    Some(b) => {
                        bytes.push(b);
                        j += 1;
                    }
                    None => break,
                }
            }
            // Need at least 3 bytes for one Malayalam char.
            if bytes.len() >= 3 {
                if let Ok(decoded) = std::str::from_utf8(&bytes) {
                    if looks_repaired(decoded) {
                        out.push_str(decoded);
                        i = j;
                        continue;
                    }
                }
            }
            // Not a confident repair: emit the anchor char verbatim and advance by
            // one so a later char can still anchor its own run.
            out.push(chars[start]);
            i = start + 1;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Produce the double-encoded (Latin-1 mojibake) form of a clean string, so
    /// tests can assert repair() inverts it.
    fn to_mojibake(clean: &str) -> String {
        clean.as_bytes().iter().map(|&b| b as char).collect()
    }

    #[test]
    fn repairs_malayalam_word() {
        let clean = "മലയാളം";
        let broken = to_mojibake(clean);
        assert_ne!(broken, clean);
        assert_eq!(repair(&broken), clean);
    }

    #[test]
    fn ascii_untouched() {
        assert_eq!(repair("Hello, World! 123"), "Hello, World! 123");
    }

    #[test]
    fn clean_malayalam_untouched() {
        let clean = "ന്റ മലയാളം";
        assert_eq!(repair(clean), clean);
    }

    #[test]
    fn conservative_on_latin_accents() {
        // A lone "café" is real Latin-1 text, not mojibake; the é does not start a
        // run that decodes to Malayalam, so it must be left alone.
        let s = "café";
        assert_eq!(repair(s), s);
    }

    #[test]
    fn mixed_repairs_only_the_broken_run() {
        let clean = "മല";
        let broken = format!("OS: {} ok", to_mojibake(clean));
        assert_eq!(repair(&broken), "OS: മല ok");
    }

    #[test]
    fn idempotent() {
        let clean = "മലയാളം";
        let broken = to_mojibake(clean);
        let once = repair(&broken);
        assert_eq!(repair(&once), once);
    }
}
