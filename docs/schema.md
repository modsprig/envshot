# Snapshot schema, version 0.1

This document describes the JSON representation implemented in
`crates/envshot-model`. The schema follows a simple compatibility policy:
changes are additive within a minor version, and breaking changes require a
major bump.

## Document shape

```json
{
  "schema":   { "name": "envshot.snapshot", "major": 0, "minor": 1 },
  "producer": { "name": "envshot", "version": "1.0.0" },
  "capture":  { "profile": "empty_test", "privacy": "standard" },
  "sections": { "<section-id>": { "...section wrapper..." } }
}
```

- `schema.name` is a stable contract identifier, independent of package
  versions. The major version is the reader-compatibility gate: documents
  with a different major are rejected with
  `snapshot.read.unsupported_schema_major`.
- `capture.profile` is one of `empty_test`, `minimal`, `standard`. The
  `minimal` profile runs the built-in collector set; `standard` grows as
  more collectors land.
- `capture.privacy` records which privacy profile produced the document:
  `standard`, `strict`, or `off`. Readers default absent values to
  `standard` (tolerance for older documents); writers always emit it.
- Key order in serialized output follows struct declaration order; sections
  are a `BTreeMap` serialized sorted by ID. Equal semantic input always
  produces equal output.
- Unknown top-level fields are ignored by readers (forward tolerance).

## Section wrapper

```json
"<section-id>": {
  "status": "available",
  "data": { "...payload..." },
  "provenance": [
    { "collector_id": "os.version", "revision": 1,
      "source_kind": "native_api", "notes": null }
  ],
  "issues": [ { "code": "parse_failed", "detail": null } ]
}
```

- `status` is one of `available`, `partial`, `unsupported`,
  `permission_denied`, `unavailable`, `failed` (a closed vocabulary with
  stable snake_case codes).
- Invariants, enforced on construction and on deserialization: `available`
  requires `data`; `partial` requires `data` plus at least one issue; the
  non-collecting states carry no data through their constructors.
- `provenance` is normalized on every write path: sorted by
  `(collector_id, revision)` with exact duplicates dropped. Collection
  order can never leak into output.
- `source_kind` is one of `native_api`, `system_file`, `known_command`,
  `library_adapter`, `derived`.
- Issue `code` is a stable machine-readable identity (an open, non-empty
  vocabulary). Issue `detail` and provenance `notes` are human-oriented,
  not semantically stable, and excluded from fingerprints.

## Section IDs and payload types (schema 0.1)

| Section ID | Payload | Notes |
|---|---|---|
| `os` | `OsData` | `family` (`windows`\|`linux`\|`macos`\|`unknown`), `version`, optional `build` |
| `architecture` | `ArchitectureData` | `process`/`os` ∈ `x86`\|`x64`\|`arm64`\|`unknown` |
| `cpu` | `CpuData` | `model`, `logical_count`, `physical_count`, `features` (set, serialized sorted) |
| `memory` | `MemoryData` | `installed_bytes` (u64, integer canonical units) |
| `locale` | `LocaleData` | `languages` (ordered by preference), `timezone` |
| `virtualization` | `VirtualizationData` | `category` ∈ `physical`\|`virtual_machine`\|`container`\|`unknown` |
| `gpu` | `Vec<GpuData>` | keyed list; identity is the adapter index; `name`, `vendor_id`, `device_id`, optional `driver_version` |
| `windows` | `WindowsData` | Windows only: `edition`, `display_version`, `ubr`, all optional |
| `distro` | `DistroData` | Linux only: `name`, `version` from os-release |

### Version values

```json
{ "raw": "10.0.26100", "normalized": { "major": 10, "minor": 0, "patch": 26100 } }
```

`raw` is preserved verbatim. `normalized` is derived from `raw` by a
conservative rule: split on `.`; the first three components contribute their
leading ASCII digit run (which must fit in `u64`) as major, minor, and
patch; a component without such a digit run stops the parse; text beyond
the first three components is ignored. Readers recompute `normalized` from
`raw` and ignore stored forms, so derived data can never disagree with the
raw text.

## Reader tolerance policy (schema 0.1)

- Unknown top-level fields: ignored.
- Unknown section IDs: skipped (a section added in a later minor is
  invisible to older readers).
- Unknown fields inside section payloads: ignored.
- Unknown enum codes (status, source kind, family, and so on): rejected.
  Enum changes are schema events, not silently tolerated variations.
- A known section ID with a foreign payload shape: rejected.

## Reader error codes

| Code | Meaning |
|---|---|
| `snapshot.read.malformed` | invalid JSON, missing or invalid schema identifier, shape mismatch, violated section invariant |
| `snapshot.read.unsupported_schema_major` | document declares a schema major the reader does not support |

## Fixtures

- `fixtures/snapshots/v0.1/` holds readable documents pinned by
  compatibility tests: `complete-basic`, `partial-snapshot`,
  `unsupported-collector`, `future-optional-field`,
  `empty-test-envelope`.
- `fixtures/malformed/` holds rejected documents, each pinned to its
  expected error code.
