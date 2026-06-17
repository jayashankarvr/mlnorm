// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Golden-file tests for the normalizer, the release tripwire.
//!
//! Each entry in `tests/golden/cases.json` is an `input -> expected` pair. A diff
//! against `expected` blocks the release (see `docs/components/mlnorm.md`,
//! "Testing"). The cases cover, per the doc's checklist:
//!
//! - every chillu variant, atomic **and** legacy `consonant + virama + ZWJ`
//! - both nta encodings (and the legacy-via-ZWJ spelling)
//! - ZWJ in chillu position and non-chillu position
//! - ZWNJ in keep-position (e.g. സോഫ്റ്റ്‌വെയർ) and strip-positions
//!   (word-final, doubled, between vowels, no-virama-before, no-consonant-after)
//! - mixed Malayalam / Latin
//! - empty / whitespace edge cases
//!
//! Beyond exact-bytes checks, every case must also satisfy the global invariants:
//! idempotency, valid UTF-8, and determinism (stable across repeated runs).

use serde::Deserialize;

use mlnorm::normalize;

#[derive(Debug, Deserialize)]
struct GoldenFile {
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    input: String,
    expected: String,
}

fn load() -> GoldenFile {
    let raw = include_str!("golden/cases.json");
    serde_json::from_str(raw).expect("golden/cases.json must be valid JSON")
}

/// Hex dump of a string's codepoints, for readable failure diffs.
fn cp(s: &str) -> String {
    s.chars()
        .map(|c| format!("U+{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn golden_exact_bytes() {
    let golden = load();
    assert!(!golden.cases.is_empty(), "golden file is empty");

    let mut failures = Vec::new();
    for c in &golden.cases {
        let got = normalize(&c.input);
        if got != c.expected {
            failures.push(format!(
                "  [{}]\n    input:    {}\n    expected: {}\n    got:      {}",
                c.name,
                cp(&c.input),
                cp(&c.expected),
                cp(&got)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "golden mismatches ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn golden_idempotent() {
    for c in load().cases {
        let once = normalize(&c.input);
        let twice = normalize(&once);
        assert_eq!(
            once,
            twice,
            "non-idempotent for [{}]: {} -> {}",
            c.name,
            cp(&once),
            cp(&twice)
        );
    }
}

#[test]
fn golden_deterministic() {
    // Same input -> same bytes across repeated calls. (Single-process proxy for
    // the cross-platform/cross-run determinism the contract requires; the kernel
    // has no locale, randomness, or thread-order dependence.)
    for c in load().cases {
        let a = normalize(&c.input);
        let b = normalize(&c.input);
        assert_eq!(a, b, "non-deterministic for [{}]", c.name);
    }
}

#[test]
fn golden_expected_is_canonical_fixpoint() {
    // The expected output must itself be a fixpoint: normalize(expected) ==
    // expected. Guards the golden file against a stale/wrong expected value.
    for c in load().cases {
        assert_eq!(
            normalize(&c.expected),
            c.expected,
            "golden expected is not a fixpoint for [{}]: {}",
            c.name,
            cp(&c.expected)
        );
    }
}
