# Changelog

All notable changes to this project are documented in this file.

## [2.0.0] - 2026-04-26

### Added
- Scheduler-friendly parallel execution with adaptive thresholds for encode/decode.
- Extended SIMD coverage (SSE2/AVX2/NEON) in colorspace/dequant hot paths.
- Installable shared libraries with ABI symlinks:
  - `libbitgrain.so.<ABI_MAJOR>`
  - `libbitgrain-simd.so.<ABI_MAJOR>`
- `pkg-config` metadata: `bitgrain.pc`.
- CMake consumer config: `BitgrainConfig.cmake`.
- Environment-based thread controls:
  - `BITGRAIN_THREADS`
  - `BITGRAIN_THREADS_CAP`

### Changed
- Improved library installation layout for external consumers.
- Expanded API contract docs for error-message lifetime and decode failure semantics.
- Updated Rust crate metadata repository URL.

### Notes
- The public C API remains in `includes/encoder.h`.
- Semantic versioning policy:
  - Patch/minor: source-compatible C API changes
  - Major: potential ABI break
