// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! PyO3 extension module for `mlnormalize`, the Python wheel entry point.
//!
//! This compiles to a cdylib named `mlnormalize` (the Python import name) and is
//! packaged into a wheel by maturin (`bindings/pyproject.toml`). It is a *thin*
//! wrapper: every function forwards directly to the `mlnormalize` core crate, so Python
//! gets **byte-identical** output to the Rust core (the whole point of the
//! normalizer contract, one implementation, two callers; see
//! `docs/ARCHITECTURE.md`).
//!
//! The mirrored API (matching `docs/components/mlnormalize.md`):
//!
//! ```python
//! import mlnormalize
//! mlnormalize.normalize(text)      # -> str, byte-identical to the Rust core
//! mlnormalize.stripped_key(text)   # -> str, dedup/matching projection (not training text)
//! mlnormalize.version()            # -> (0, 4, 0)
//! mlnormalize.__version__          # -> "0.4.0"
//! ```

use pyo3::prelude::*;

/// `mlnormalize.normalize(text) -> str`, byte-identical to the Rust core.
#[pyfunction]
fn normalize(text: &str) -> PyResult<String> {
    Ok(mlnormalize_core::normalize(text))
}

/// `mlnormalize.stripped_key(text) -> str`, the dedup/matching projection. Not training text.
#[pyfunction]
fn stripped_key(text: &str) -> PyResult<String> {
    Ok(mlnormalize_core::stripped_key(text))
}

/// `mlnormalize.version() -> (major, minor, patch)`.
#[pyfunction]
fn version() -> PyResult<(u32, u32, u32)> {
    Ok(mlnormalize_core::version().as_tuple())
}

#[pymodule]
fn mlnormalize(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(stripped_key, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    // Version stamp as a module attribute for provenance recording.
    m.add("__version__", mlnormalize_core::version_string())?;
    Ok(())
}
