# Contributing

## Building and testing

```text
cargo build --release
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
```

The .NET wrapper builds and tests with `dotnet test csharp/EnvShot.Tests`
(the project builds the native library automatically).

## Ground rules

- No network access during capture. No telemetry. No arbitrary command
  execution.
- Standard privacy must not intentionally collect direct identifiers or
  secrets. See `docs/privacy.md` for the field-by-field contract.
- New runtime dependencies need a recorded rationale: purpose, license,
  MSRV and platform impact, and why std or native APIs are insufficient.
- Released fingerprint profiles are immutable. A semantic change requires a
  new profile ID.
- Schema changes follow `docs/schema.md`: additive within a minor version,
  breaking changes bump the major.
- `unsafe` is confined to the two documented boundaries (Windows platform
  adapters and the C ABI) with a safety comment at every use site.

## License

MIT. See [LICENSE](LICENSE).
