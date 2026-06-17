// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! PyO3 extension module for `mlnorm`, the Python wheel entry point.
//!
//! This compiles to a cdylib named `mlnorm` (the Python import name) and is
//! packaged into a wheel by maturin (`bindings/pyproject.toml`). It is a *thin*
//! wrapper: every function forwards directly to the `mlnorm` core crate, so Python
//! gets **byte-identical** output to the Rust core (the whole point of the
//! normalizer contract, one implementation, two callers; see
//! `docs/ARCHITECTURE.md`).
//!
//! The mirrored API (matching `docs/components/mlnorm.md`):
//!
//! ```python
//! import mlnorm
//! mlnorm.normalize(text)      # -> str, byte-identical to the Rust core
//! mlnorm.stripped_key(text)   # -> str, dedup/matching projection (not training text)
//! mlnorm.version()            # -> (0, 4, 0)
//! mlnorm.__version__          # -> "0.4.0"
//! ```

use pyo3::prelude::*;

/// `mlnorm.normalize(text) -> str`, byte-identical to the Rust core.
#[pyfunction]
fn normalize(text: &str) -> PyResult<String> {
    Ok(mlnorm_core::normalize(text))
}

/// `mlnorm.stripped_key(text) -> str`, the dedup/matching projection. Not training text.
#[pyfunction]
fn stripped_key(text: &str) -> PyResult<String> {
    Ok(mlnorm_core::stripped_key(text))
}

/// `mlnorm.version() -> (major, minor, patch)`.
#[pyfunction]
fn version() -> PyResult<(u32, u32, u32)> {
    Ok(mlnorm_core::version().as_tuple())
}

#[pymodule]
fn mlnorm(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(stripped_key, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    // Version stamp as a module attribute for provenance recording.
    m.add("__version__", mlnorm_core::version_string())?;
    Ok(())
}
