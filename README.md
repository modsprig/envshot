# envshot

A privacy-safe, deterministic Rust library and CLI for capturing typed
diagnostic snapshots of the local execution environment. It normalizes the
facts, compares two snapshots semantically, and computes stable versioned
fingerprints over explicitly selected compatibility facts.

Use it when you need answers to questions like these:

- Why does this bug reproduce on machine A but not on machine B?
- Can a bug report carry useful machine context without leaking personal
  data?
- Can two reports be compared by tools instead of reading walls of text?
- Can other Rust and .NET applications embed the same behavior through a
  stable API?

This is not a reproducible-build environment manager, a system monitor, an
inventory agent, a cloud service, or a telemetry product.

## Usage

### Capture

```text
envshot capture --profile standard
{"schema":{"name":"envshot.snapshot","major":0,"minor":1},"producer":{"name":"envshot","version":"1.0.0"},"capture":{"profile":"standard","privacy":"standard"},"sections":{"architecture":{"status":"available",...},"cpu":{...},...}}
```

Pass `-o snapshot.json` to write to a file. Profiles: `minimal`, `standard`,
and `empty_test` (a collector-free envelope).

### Diff

```text
envshot diff before.json after.json
architecture:
  section added

cpu:
  status: partial -> available
```

`--format json` emits the versioned diff envelope
(`{"diff_schema":{"name":"envshot.diff","version":1},...}`). `--policy
diagnostic` adds producer and capture metadata changes.

### Fingerprint

```text
envshot fingerprint snapshot.json
repro-v1:sha256:4119291c8cf461350058f0cedf1de7ff7d5fdb75d5ee6bc0f3045599b32608a7
```

`--format json` adds the canonical projection, which explains what the hash
covers. See [docs/fingerprint.md](docs/fingerprint.md) for the frozen
`repro-v1` fact set.

## Platform support matrix

| Capability | Windows | Linux | macOS |
|---|---|---|---|
| OS identity + version/build | full (sysinfo + registry) | full (sysinfo + os-release) | full (sysinfo) |
| Architecture + emulation | full (IsWow64Process2) | core | core |
| CPU features/counts | full (CPUID) | x86 CPUID | sysinfo |
| Memory | full | full | full |
| Locale/timezone | full | full | full |
| Virtualization category | CPUID hypervisor bit | CPUID + container markers | limited |
| GPU adapters/driver | full (DXGI) | not yet | not yet |
| Distro identity | n/a | full (os-release) | n/a |

## Privacy

Standard privacy (the default everywhere) exists so output can go straight
into a public bug report:

- No username, hostname, home path, IP/MAC, hardware serials, machine UUIDs,
  or account/tenant IDs.
- No secret environment variables and no stable pseudonymous host
  identifier.
- No network access during capture and no telemetry.

The field-by-field contract lives in
[docs/privacy.md](docs/privacy.md). The security policy is in
[SECURITY.md](SECURITY.md).

## Documentation

| Document | Contents |
|---|---|
| [docs/schema.md](docs/schema.md) | snapshot schema v0.1: JSON shape, section contract, tolerance policy |
| [docs/privacy.md](docs/privacy.md) | field-by-field privacy behavior |
| [docs/diff.md](docs/diff.md) | semantic diff semantics and output examples |
| [docs/fingerprint.md](docs/fingerprint.md) | normative fingerprint spec and frozen vectors |
| [docs/platforms.md](docs/platforms.md) | per-OS capability matrix |
| [docs/performance.md](docs/performance.md) | performance baseline |
| [docs/unsafe-inventory.md](docs/unsafe-inventory.md) | unsafe-code inventory and safety notes |

## Embedding

- **Rust**: depend on the `envshot` facade crate. `capture`, `diff`,
  `fingerprint`, and `read_snapshot` are the whole surface. Custom
  collectors implement the `Collector` trait and register through
  `capture_with_registry`.
- **C# / .NET**: the `envshot_ffi` C ABI speaks UTF-8 JSON in and out, is
  versioned, and never lets a panic cross the boundary. The thin `EnvShot`
  wrapper shows the pattern; no semantics are duplicated outside the Rust
  core.

## Building

```text
cargo build --release
cargo test --workspace --all-features
```

Minimum supported Rust is 1.95 (see `rust-toolchain.toml`). The .NET wrapper
targets `net10.0` and builds the native library automatically.

## License

MIT. See [LICENSE](LICENSE).
