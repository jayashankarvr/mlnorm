// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Fuzz target: `stripped_key` never panics, is idempotent, and emits no joiners.
//!
//! The dedup/matching projection has the same idempotency obligation as
//! `normalize` (stripping a kept ZWNJ can expose an nta cluster), so it gets its
//! own freeze-blocking fuzz target. Run with:
//!
//! ```sh
//! cargo +nightly fuzz run stripped_key
//! ```
#![no_main]

use libfuzzer_sys::fuzz_target;

const ZWJ: char = '\u{200D}';
const ZWNJ: char = '\u{200C}';

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let once = mlnorm::stripped_key(s);
        let twice = mlnorm::stripped_key(&once);
        assert_eq!(once, twice, "stripped_key not idempotent for input {s:?}");
        assert!(
            !once.contains(ZWJ) && !once.contains(ZWNJ),
            "stripped_key left a joiner for input {s:?}"
        );
    }
});
