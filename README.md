# mlnorm

A Malayalam Unicode normalizer. It maps Malayalam text to one canonical byte
sequence per input, so the same text is identical at training time and inference
time. The normalization is deterministic (same input, same bytes, every platform,
every run, no locale and no randomness) and idempotent
(`normalize(normalize(x)) == normalize(x)`).

The core is a small pure-Rust crate with one runtime dependency
(`unicode-normalization`). A PyO3 wheel exposes the same functions to Python with
byte-identical output.

## What it does

An ordered pipeline of small transforms:

1. Mojibake repair (conservative fix for double-encoded UTF-8; runs first)
2. NFC (Unicode canonical composition)
3. Chillu atomization (`consonant + virama + ZWJ` to the atomic chillu letters U+0D7A..U+0D7F)
4. ZWJ stripping (all remaining ZWJ removed)
5. ZWNJ positional filter (kept only in the four load-bearing slots, stripped elsewhere)
6. nta canonicalization (both ന്റ encodings map to base NA + virama + RRA)
7. Archaic codepoint map (for example ൟ to ഈ; archaic numeric and date signs dropped)
8. Digit normalization (Malayalam digits to ASCII, one direction)
9. Punctuation normalization (danda, smart quotes, ellipsis to ASCII)
   plus a visual-spoofing fixup pass and a final NFC recompose

A second output, `stripped_key`, removes all joiners (including the ZWNJs that
`normalize` keeps) to produce a dedup and retrieval matching key. It is not training
text: train on `normalize`, match on `stripped_key`.

## Rust

```rust
use mlnorm::normalize;

let canonical = normalize("ൻ്റ"); // "ന്റ"
```

The public API:

```rust
mlnorm::normalize(&str) -> String       // canonical text
mlnorm::stripped_key(&str) -> String    // dedup/matching key (not training text)
mlnorm::version() -> mlnorm::NormVersion // contract version
mlnorm::version_string() -> String       // "0.4.0"
```

Run the tests (golden file plus property-based invariants):

```sh
cargo test
```

There is also a small CLI that normalizes stdin:

```sh
echo "ൻ്റ" | cargo run --bin mlnorm-cli
```

## Python

```python
import mlnorm

mlnorm.normalize("ൻ്റ")    # "ന്റ"
mlnorm.stripped_key(text)   # dedup/matching key (not training text)
mlnorm.version()            # (0, 4, 0)
mlnorm.__version__          # "0.4.0"
```

Build the wheel with [maturin](https://github.com/PyO3/maturin) from the `bindings/`
directory:

```sh
cd bindings
maturin build --release     # wheel in target/wheels/
maturin develop             # install into the active venv
```

## Versioning

The version is a load-bearing contract, not just a release tag. Every downstream
artifact records the `mlnorm` version it was normalized with. A change in output
bytes is a version bump, and a major bump means downstream artifacts must be
rebuilt. The Rust crate, the Python package, and `version()` all report the same
version (0.4.0).

## License

Apache-2.0. See `LICENSE` and `NOTICE`. Contributions are accepted under Apache-2.0
§5 (inbound = outbound); no separate CLA is required.

Sources for the normalization facts are credited in `REFERENCES.md`.
