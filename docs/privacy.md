# Privacy behavior, field by field (schema 0.1)

The source of truth is the `PrivacyPolicy` projection in
`envshot-core::privacy`; this document mirrors it. Standard is the default
at the library and CLI layers.

## Policy modes

| Mode | Intent | Issue details | Behavior |
|---|---|---|---|
| `standard` (default) | Shareable bug reports | Stripped (debug channel) | Canonical facts kept |
| `strict` | Minimal correlation | Stripped | Plus: locale identifiers omitted |
| `off` | Debug-only, explicit | Kept verbatim | Everything as captured; CLI warns |

The applied mode is recorded in the document (`capture.privacy`).

## Field-by-field classification

| Section | Field | Standard | Strict | Rationale |
|---|---|---|---|---|
| os | family/version/build | keep | keep | System compatibility facts, not user-identifying |
| architecture | process/os | keep | keep | Hardware class facts |
| cpu | model | keep | keep | Hardware brand; not user-configurable |
| cpu | counts | keep | keep | Class facts |
| cpu | features | keep | keep | Capability flags (set, sorted) |
| memory | installed_bytes | keep | keep | Class fact (category-relevant, integer) |
| locale | languages | **keep** | **omit** | Potentially identifying configuration (region/language) |
| locale | timezone | **keep** | **omit** | Location-correlating identifier |
| virtualization | category | keep | keep | Coarse class fact |
| any | issue `code` | keep | keep | Stable machine-readable identity |
| any | issue `detail` | **strip** | **strip** | Debug channel; free-form text |
| any | provenance collector id/revision/source kind | keep | keep | Non-sensitive metadata |
| any | provenance notes | keep | keep | Non-sensitive capability notes |

- Fields not present in schema 0.1 (paths, environment values) are governed by
  the primitives that will police them: `symbolic_path` (home prefixes become
  `%USERPROFILE%`/`$HOME`) and `EnvironmentPolicy` (empty allowlist default,
  secret keys never collectible, bounded values).
- The capture envelope (`schema`, `producer`, `capture`) carries no
  identity-bearing data; `producer.version` is the tool version.

## What never exists

- **No stable pseudonymous host identifier**: no hash, UUID, or machine ID of
  the host is computed or emitted in any mode (a stable hash would enable
  cross-report correlation, the exact failure mode this project forbids).
- **No direct identifiers**: username, hostname, home path, IP/MAC, hardware
  serials, machine UUIDs, account/tenant IDs are not collected by any built-in
  collector.
- **No secret values**: the secret-key classifier rejects TOKEN/SECRET/
  PASSWORD/CREDENTIAL/...-class keys even if a future policy allowlists them.

## Defensive leak scan

`envshot_core::testing::canaries()` provides deterministic canary strings
(`S3CR3T_CANARY_*` covering username/hostname/home path/IP/MAC/serial/UUID/
token classes) and `assert_no_canaries` scans serialized output. Tests use
them against hostile fake collectors and host-derived values. This is
defense-in-depth only: the typed schema and policy projection are the primary
design.
