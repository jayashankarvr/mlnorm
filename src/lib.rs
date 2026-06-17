// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Malayalam Unicode normalizer: bit-identical normalization for training and inference.
//!
//! This is the kernel of the Malayalam LM system. The normalizer contract
//! (see `docs/ARCHITECTURE.md`):
//! - **One implementation.** Rust core here, exposed to Python via a PyO3 wheel.
//!   Python never reimplements it; it calls it.
//! - **Versioned**, and the version is a tripwire ([`NormVersion`]).
//! - **Idempotent:** `normalize(normalize(x)) == normalize(x)`.
//! - **Deterministic:** same input, same bytes, every platform, every run. No
//!   locale, no randomness, no thread-order dependence.
//! - Used **identically** at train-time and inference-time.
//!
//! The implementation is a sealed, concrete unit with no trait and no extension
//! point, deliberately (see `docs/DESIGN_PRINCIPLES.md`). Internally it is an
//! ordered list of small transforms; see [`normalize`] and the `normalize`
//! module.
//!
//! # Pipeline (pre-freeze; 1.0.0 awaits a clean cargo-fuzz run)
//! 1. Mojibake repair (double-encoded UTF-8, conservative; runs first)
//! 2. NFC normalization (Unicode canonical composition)
//! 3. Chillu atomization (`consonant + virama + ZWJ` -> atomic chillu U+0D7A..U+0D7F)
//! 4. ZWJ stripping (all remaining ZWJ removed)
//! 5. ZWNJ positional filter (keep iff `virama + ZWNJ + consonant`, else strip)
//! 6. NTA canonicalization (the two ന്റ encodings -> base NA + virama + RRA)
//! 7. Archaic codepoint map (ൟ->ഈ, archaic numeric/date signs dropped)
//! 8. Digit normalization (Malayalam digits -> ASCII, one direction)
//! 9. Punctuation normalization (danda, smart quotes, ellipsis -> ASCII)
//! + final NFC recompose (re-compose pairs exposed by joiner removal)
//!
//! A **second output**, [`stripped_key`], produces a fully joiner-stripped
//! projection for dedup/retrieval matching only, never training text.

pub mod normalize;
pub mod transforms;
pub mod version;

pub use version::{NormVersion, VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH};

/// Canonical normalization. Idempotent, deterministic, no locale dependence.
///
/// This is the single source of truth for Malayalam byte canonicalization across
/// the entire system. See the module docs for the ordered pipeline.
pub fn normalize(input: &str) -> String {
    normalize::normalize(input)
}

/// Joiner-stripped projection for dedup/retrieval matching only. **Not** training
/// text, train on [`normalize`], match on `stripped_key`.
///
/// `stripped_key(x)` is [`normalize`] with *all* joiners (ZWJ and ZWNJ) removed,
/// including the conjunct-prevention ZWNJs `normalize` deliberately keeps, so two
/// documents differing only in joiner placement collapse to one dedup key. See
/// the `normalize` module and `docs/components/mlnorm.md`.
pub fn stripped_key(input: &str) -> String {
    normalize::stripped_key(input)
}

/// The version stamp every downstream artifact records.
pub fn version() -> NormVersion {
    version::version()
}

/// Returns the version as a `major.minor.patch` string for logging/manifests.
pub fn version_string() -> String {
    version::version_string()
}

// The PyO3 extension module (the Python wheel) lives in `bindings/` and is built
// by maturin. It is a thin wrapper that forwards to the public functions above,
// guaranteeing byte-identical output between the Rust core and Python. Keeping it
// out of this crate keeps the core pure-Rust and dependency-light (the stated
// reuse-surface goal) and publishable to crates.io on its own. See
// `bindings/src/lib.rs` and `bindings/pyproject.toml`.

#[cfg(test)]
mod tests {
    use crate::{normalize, version};

    #[test]
    fn test_idempotent() {
        let input = "നമ്മളാണ്";
        let once = normalize(input);
        let twice = normalize(&once);
        assert_eq!(once, twice, "normalize must be idempotent");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn test_whitespace_only() {
        assert_eq!(normalize("   \t\n"), "   \t\n");
    }

    #[test]
    fn test_ascii_unchanged() {
        let ascii = "Hello, World!";
        assert_eq!(normalize(ascii), ascii);
    }

    #[test]
    fn test_version() {
        let v = version();
        assert_eq!(v.as_tuple(), (0, 4, 0));
        assert_eq!(v.to_string(), "0.4.0");
    }

    #[test]
    fn test_stripped_key_collapses_joiner_variants() {
        let with = "സോഫ്റ്റ്\u{200C}വെയർ";
        let without = "സോഫ്റ്റ്വെയർ";
        assert_eq!(crate::stripped_key(with), crate::stripped_key(without));
    }
}
