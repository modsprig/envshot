# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Snapshot schema, fingerprint profile, and C ABI versions are versioned
independently of the package version; release notes call out each changed
contract separately.

## 1.0.0 (2026-10-24)

Initial release.

### Added

- **Snapshot capture**: deterministic, privacy-safe capture of the local
  execution environment. Built-in collectors: OS identity/version/build,
  process & OS architecture (emulation-aware on Windows), CPU
  model/features/counts (CPUID-derived feature set), installed memory,
  locale/timezone, virtualization/container category, GPU adapters +
  driver versions (Windows via DXGI), Linux distribution identity
  (os-release), Windows edition/display-version/UBR (registry). Profiles:
  `minimal`, `standard`, `empty_test` (collector-free envelope).
- **Schema v0.1** (`envshot.snapshot` 0/1): typed sections with explicit
  status vocabulary (`available`/`partial`/`unsupported`/
  `permission_denied`/`unavailable`/`failed`), provenance records, stable
  issue codes, unknown-field tolerance, deterministic byte-stable JSON.
- **Privacy**: `standard` (default, shareable), `strict` (omits locale
  identifiers and all debug details), `off` (debug-only, explicit). No
  username/hostname/paths/IP/MAC/serials/UUIDs in standard output; no
  stable pseudonymous host identifier; secret-key classifier for
  environment variables; symbolic home-path normalization.
- **Semantic diff**: schema-aware typed comparison. Scalar changes,
  explicit status transitions, sorted-set membership, and version-aware
  comparison with raw display; keyed-list join for GPU adapters;
  `compatibility` and `diagnostic` views; path-sorted deterministic
  output; text and versioned JSON rendering.
- **Fingerprints**: `repro-v1`, a SHA-256 hash over an explicit and
  privacy-safe projection with domain-separated canonical bytes; frozen
  golden vectors; an explain surface that shows the projection.
- **CLI**: `envshot capture`, `envshot diff`, `envshot fingerprint` with
  documented exit codes (0/2/3/5) and machine-readable stdout.
- **Embedding**: small Rust facade; custom collector extension point;
  versioned C ABI over UTF-8 JSON buffers (panic-safe, no unwinding, exact
  buffer ownership); thin .NET wrapper (`net10.0`).
- **Engineering**: deterministic quality gates: fmt/clippy (`-D warnings`),
  a full test matrix (257 tests including property suites), coverage,
  CRAP <= 6, a mutation-tested core (model+core campaign: 264 mutants, 178
  caught, 84 unviable, 1 documented equivalent, 1 documented timeout),
  dependency and license checks (`cargo deny`, `cargo audit`), stress
  suites, a performance baseline, and an unsafe inventory.
