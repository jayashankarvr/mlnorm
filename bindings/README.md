# mlnormalize (Python wheel)

PyO3 bindings for the [`mlnormalize`](https://github.com/jayashankarvr/mlnormalize) Malayalam
Unicode normalizer. The wheel is a thin wrapper around the Rust core, so Python gets
byte-identical output. That identity is the point of the normalizer contract: one
implementation, two callers.

## Install

```sh
pip install mlnormalize
```

Also available as a Rust crate on crates.io (same byte-identical output): `cargo add mlnormalize`.

```python
import mlnormalize
mlnormalize.normalize("ൻ്റ")     # -> "ന്റ"  (byte-identical to the Rust core)
mlnormalize.stripped_key(text)    # -> str    (dedup/matching key; NOT training text)
mlnormalize.version()             # -> (0, 4, 0)
mlnormalize.__version__           # -> "0.4.0"
```

## Build

Built with [maturin](https://github.com/PyO3/maturin) from this directory:

```sh
maturin build --release     # produces a wheel in target/wheels/
maturin develop             # install into the active venv for development
```
