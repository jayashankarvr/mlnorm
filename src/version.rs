// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Version tracking for the normalizer.
//!
//! The contract (see ARCHITECTURE.md "The normalizer contract"): the version is
//! a tripwire. Every downstream artifact records the `mlnormalize` version it was
//! built against, and a **major** bump means the output bytes changed and every
//! downstream artifact must be rebuilt. Treat the constants below as load-bearing.

/// Semantic version of the normalizer's output contract.
///
/// Stamped onto every artifact that consumes normalized text so provenance is
/// always traceable. Equality compares all three fields; a difference in `major`
/// is what the tooling treats as "retrain everything".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NormVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl NormVersion {
    /// The current normalizer version.
    pub const CURRENT: NormVersion = NormVersion {
        major: VERSION_MAJOR,
        minor: VERSION_MINOR,
        patch: VERSION_PATCH,
    };

    /// `(major, minor, patch)` tuple, mirroring the Python binding's return shape.
    pub fn as_tuple(&self) -> (u32, u32, u32) {
        (self.major, self.minor, self.patch)
    }
}

impl core::fmt::Display for NormVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// 0.4.0, all rule content through the stripped_key milestone is implemented:
/// steps 1 through 9 of `docs/components/mlnormalize.md` (mojibake repair, NFC, chillu
/// atomization, ZWJ strip, ZWNJ positional filter, nta canonicalization, archaic
/// map, digit normalization, punctuation normalization) plus the
/// [`crate::stripped_key`] second output.
///
/// NOT yet 1.0.0: per `docs/DEVELOPMENT_PLAN.md`, 1.0.0 means the contract is
/// frozen, `cargo-fuzz` is clean, and the first CPT builds against it. The fuzz
/// targets exist (`fuzz/`) but have not been run clean here (cargo-fuzz needs
/// rustc ≥1.91; this box has 1.89), and no CPT has run. Stamp 1.0.0 only after a
/// clean fuzz run and the freeze. The major number is the tripwire the tooling
/// enforces; see ARCHITECTURE.md "The normalizer contract".
pub const VERSION_MAJOR: u32 = 0;
// 0.4.0, native-review normalization additions (correctness_issues.csv #18/#24/#25):
// NNNA/TTTA no longer folded; ZWNJ keep-rule expanded to 4 load-bearing slots;
// visual-spoofing + samvruthokaram + dot-reph fixes (transforms/spoof.rs). These
// change output bytes for some inputs, so the minor bumps (clean modern text is
// unchanged; major stays 0 until the 1.0 fuzz-freeze).
pub const VERSION_MINOR: u32 = 4;
pub const VERSION_PATCH: u32 = 0;

/// The version stamp every downstream artifact records.
pub fn version() -> NormVersion {
    NormVersion::CURRENT
}

/// Returns the version as a `major.minor.patch` string for logging/manifests.
pub fn version_string() -> String {
    NormVersion::CURRENT.to_string()
}
