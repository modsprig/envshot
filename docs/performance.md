# Performance baseline

Numbers from `cargo run --release -p envshot --example perf_baseline` on the
development host (Windows 10 x64, Rust 1.97.1, release profile). Timings are
wall-clock and host-dependent. Treat them as baselines for catching gross
regressions, not as CI gates.

## Pure pipeline (typical snapshot, 10k iterations)

| Operation | Per iteration |
|---|---|
| normalize | ~1.4 µs |
| privacy projection (standard) | ~2.3 µs |
| diff (identical inputs, compatibility) | ~0.2 µs |
| fingerprint (repro-v1) | ~4.9 µs |
| total pipeline (normalize → privacy) | ~4.7 µs |

## Document size

A typical standard-profile snapshot serializes to **~900 bytes** of compact
JSON (9 sections, no inventory bloat by design).

## Real capture (includes collector I/O)

| Step | Duration |
|---|---|
| Total standard capture (collectors + normalize + privacy) | ~230 ms |

The cost is dominated by collector I/O (DXGI adapter enumeration, registry
reads, sysinfo refreshes); the pure pipeline above is noise by comparison.
The default untrusted-input limit (`ReadLimits`, 4 MiB) bounds hostile input;
no legitimate snapshot approaches it.

## Guidance

- Rerun the example locally before/after touching normalize/privacy/diff/
  fingerprint hot paths.
- Wall-clock CI gates on shared runners are deliberately avoided by design;
  gross regressions (order-of-magnitude) are the signal to watch.
