# References

`mlnormalize` implements canonicalization **facts** about the Malayalam script and its
Unicode encoding, restated in our own transform pipeline. No text, tables, code, or
datasets from any source below are reproduced or redistributed. These citations are
scholarly credit; they imply no endorsement and create no license obligation.

## Sources

- **The Unicode Standard**, Malayalam block (U+0D00..U+0D7F): codepoint identities,
  the atomic chillu letters (U+0D7A..U+0D7F), the virama (U+0D4D), and the dependent
  vowel signs. Normalization forms (NFC) follow Unicode Standard Annex #15.
- **Unicode chart for the Malayalam block** (`U0D00.pdf`): the base-consonant to
  atomic-chillu mapping and the archaic codepoints retired by the script.
- **Native-reviewer rulings** (project correctness log, items #18/#24/#25): which
  sequences are load-bearing in modern Malayalam and must be preserved rather than
  folded. These rulings fixed the NNNA/TTTA preservation, the four ZWNJ keep-slots,
  and the visual-spoofing / samvruthokaram / dot-reph cases (see `transforms/spoof.rs`).

The /nṯa/ canonical form (base NA + virama + RRA) and the one-direction digit and
punctuation maps are frozen for the 0.x line; changing them is a major version bump.
