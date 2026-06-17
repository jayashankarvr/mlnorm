// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! The ordered transform rules behind the sealed [`crate::normalize`] pipeline.
//!
//! Each rule is a small, single-purpose function written once (DRY) and composed
//! in sequence by `crate::normalize::normalize`. There is no trait and no
//! registry, the normalizer is concrete and sealed by design (see
//! DESIGN_PRINCIPLES.md). These modules are `pub(crate)` implementation detail,
//! not a public extension surface.
//!
//! Pipeline order (matching `docs/components/mlnormalize.md`, with one documented
//! deviation around nta, see `crate::normalize`):
//!
//! 1. [`mojibake`], repair double-encoded UTF-8 (runs first, conservative)
//! 2. NFC (in `crate::normalize`)
//! 3. chillu atomization (in `crate::normalize`)
//! 4. ZWJ strip + 5. ZWNJ positional filter (in `crate::normalize`)
//! 6. nta canonicalization (in `crate::normalize`)
//! 7. [`archaic`], archaic codepoint map (ൟ->ഈ, etc.)
//! 8. [`digits`], Malayalam digits -> ASCII (one direction)
//! 9. [`punct`], danda, smart quotes, ellipsis -> canonical ASCII

pub(crate) mod archaic;
pub(crate) mod digits;
pub(crate) mod mojibake;
pub(crate) mod punct;
pub(crate) mod spoof;
