// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Step 8, digit normalization (0.2.0). **One direction:** Malayalam -> ASCII.
//!
//! Malayalam digits (U+0D66..U+0D6F, ൦..൯) are vanishingly rare in modern text;
//! ASCII digits dominate even in otherwise pure-Malayalam writing. Collapsing to
//! one form stops the tokenizer from learning two encodings of "2024". We pick
//! ASCII as the single direction and document it as frozen for the 1.x line, 
//! reversing the direction would change output bytes and is a major bump.

/// Map a Malayalam digit (U+0D66..U+0D6F) to its ASCII equivalent, else `None`.
#[inline]
fn ascii_digit(c: char) -> Option<char> {
    match c as u32 {
        0x0D66..=0x0D6F => char::from_u32(b'0' as u32 + (c as u32 - 0x0D66)),
        _ => None,
    }
}

/// Normalize Malayalam digits to ASCII over the char buffer.
pub(crate) fn normalize_digits(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    for &c in chars {
        out.push(ascii_digit(c).unwrap_or(c));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ten_digits() {
        let mal: Vec<char> = "൦൧൨൩൪൫൬൭൮൯".chars().collect();
        let got: String = normalize_digits(&mal).into_iter().collect();
        assert_eq!(got, "0123456789");
    }

    #[test]
    fn ascii_digits_untouched() {
        let s: Vec<char> = "2024".chars().collect();
        assert_eq!(normalize_digits(&s), s);
    }

    #[test]
    fn mixed_in_text() {
        let s: Vec<char> = "വർഷം ൨൦൨൪".chars().collect();
        let got: String = normalize_digits(&s).into_iter().collect();
        assert_eq!(got, "വർഷം 2024");
    }
}
