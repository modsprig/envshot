# Fingerprint specification for `repro-v1`

Normative rules for fingerprint computation. Source of truth is
`envshot-core::fingerprint`; this document mirrors it and freezes the
`repro-v1` fact set.

## Envelope

```text
repro-v1:sha256:<64 lowercase hex characters>
```

Algorithm: SHA-256 over domain-separated canonical bytes:

```text
"envshot-fingerprint\0" || profile_id || "\0" || schema_major || "\0" || canonical_projection_bytes
```

## Canonical projection bytes

The projection is a JSON object serialized compactly (no whitespace) in UTF-8:

```json
{"facts":{...sorted...},"profile":"repro-v1","schema":{"major":0,"minor":1,"name":"envshot.snapshot"}}
```

- Object keys are sorted (serde_json's default B-tree map; no
  `preserve_order`).
- Sets are serialized sorted; ordered lists keep their semantic order.
- Absent optional facts are **omitted** (no nulls); present facts are always
  emitted.
- Integers only; no floats anywhere in a projection.
- Version facts use normalized components (`{"major":..,"minor":..,"patch":..}`)
  when the raw text parses; unparsable versions project their raw text.
- Closed vocabularies project as their stable snake_case codes.

## `repro-v1` fact review

| Fact | Why it affects compatibility | Stable between captures | Identifying? | When unavailable |
|---|---|---|---|---|
| `os.family` | OS-specific APIs/behavior | yes | no | key omitted |
| `os.version` | API surface, ABI | yes | no | key omitted (whole section absent) |
| `os.build` | Patch-level behavior differences | yes | no | key omitted when absent |
| `arch.process` / `arch.os` | Instruction set, ABI | yes | no | key omitted |
| `cpu.features` | ISA extensions (AVX, AES, ...) | yes | no | key omitted |
| `memory.installed_bytes` | Memory-scale reproduction | yes | no | key omitted |
| `virtualization.category` | Container/VM behavior differences | yes (category is coarse) | no | key omitted |

**Deliberately excluded** (candidates recorded for `repro-v2`):

- `cpu.model`: brand strings vary across equivalent SKUs, and the feature
  set already captures the ISA surface;
- CPU topology counts: they describe machine shape rather than ISA, which
  makes them noisy for the compatibility question;
- locale/timezone: a weak compatibility signal with a higher correlation
  risk, and the strict privacy policy omits them entirely;
- GPU/driver and runtime facts: their collectors landed after the profile
  was frozen, so they wait for a `repro-v2` review instead of joining
  retroactively.

The schema identifier and major are part of the hashed bytes; the minor is
informational. No identity-class field (username, hostname, paths, MAC/IP,
serials, machine UUIDs, account/tenant IDs) is projected in any mode.

## Compatibility rule

After any public release, `repro-v1` semantics are immutable: any change that
would alter a committed golden vector's digest requires a new profile ID
(`repro-v2`). Golden vectors under `fixtures/snapshots/v0.1/` run in CI for
as long as `repro-v1` is supported.
