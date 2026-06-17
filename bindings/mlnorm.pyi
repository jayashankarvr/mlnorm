"""Type stub for the mlnorm PyO3 wheel.

The implementation is the compiled Rust core (see crates/mlnorm). Output is
byte-identical to the Rust `mlnorm::normalize` / `mlnorm::stripped_key`.
"""

from typing import Tuple

__version__: str

def normalize(text: str) -> str:
    """Canonical normalization. Idempotent, deterministic, no locale dependence.

    Byte-identical to the Rust core. This is the single source of truth for
    Malayalam byte canonicalization across training and inference.
    """
    ...

def stripped_key(text: str) -> str:
    """Joiner-stripped projection for dedup/retrieval matching only.

    NOT training text. Train on ``normalize``, match on ``stripped_key``: two
    documents differing only in joiner placement collapse to the same key.
    """
    ...

def version() -> Tuple[int, int, int]:
    """The (major, minor, patch) normalizer contract version, e.g. (0, 4, 0).

    A major-version difference means the output bytes changed and every
    downstream artifact must be rebuilt.
    """
    ...
