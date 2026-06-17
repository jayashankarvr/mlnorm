// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Fuzz target: `normalize` never panics on any input.
//!
//! Required clean for the 1.0.0 freeze (see `docs/components/mlnorm.md`,
//! "Testing"). Malayalam from the wild is adversarial by accident; this finds the
//! combining-mark edge cases a human won't enumerate. Run with:
//!
//! ```sh
//! cargo +nightly fuzz run normalize_no_panic
//! ```
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // The contract: never panics, output is valid UTF-8 (guaranteed by the
        // String return type), no isolated-combining-mark corruption.
        let _ = mlnorm::normalize(s);
    }
});
