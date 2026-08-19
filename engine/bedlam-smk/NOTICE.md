# Vendored fork of smk 0.1.0

Upstream: https://github.com/roarc0/smk (crates.io `smk` 0.1.0)
License: LGPL-2.1-or-later (per upstream Cargo.toml; see upstream repo for text)
Vendored: 2026-08-19, byte-identical to the crates.io artifact
(registry checksum verified against the local cargo cache extraction).
Reason: docs/RESEARCH.md "NEW+unproven -> vendor/fork" policy; see
docs/DECISIONS.md D30. Local patches, if any, MUST be listed here.

## Local patches
- src/audio.rs `render_dpcm` (2026-08-19, bedlam-re item-1): clamp the
  chunk-declared unpacked size to the track buffer and return a typed error
  when the buffer is smaller than the initial-sample bytes. Upstream indexed
  out of bounds on malformed streams (panic) and left buffer_size >
  buffer.len(), which also made Smk::audio_data slice-panic. Behavior on
  well-formed streams is unchanged. Two regression tests added in the same
  test module (dpcm_unpack_size_clamped_to_buffer, dpcm_buffer_too_small_is_error).
- Test-module clippy cleanups (2026-08-19, bedlam-re item-1): behavior-
  identical rewrites in src/huff.rs and src/smk.rs test helpers (vec!
  initializers, iter_mut, resize) to satisfy cargo clippy -D warnings.
