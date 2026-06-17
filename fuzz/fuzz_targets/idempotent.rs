// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Fuzz target: `normalize` is idempotent, `normalize(normalize(x)) == normalize(x)`.
//!
//! This is the contract invariant that two real bugs already hid behind (a stray
//! ZWJ inside an nta cluster; an NFC-composable pair exposed by joiner removal;
//! an archaic codepoint wedged inside an nta cluster). Required clean for the
//! 1.0.0 freeze. Run with:
//!
//! ```sh
//! cargo +nightly fuzz run idempotent
//! ```
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let once = mlnorm::normalize(s);
        let twice = mlnorm::normalize(&once);
        assert_eq!(once, twice, "normalize not idempotent for input {s:?}");
    }
});
