# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| 1.0.x | yes |

## Reporting a vulnerability

Open a minimal public issue describing the vulnerability class and request a
private contact; do not include exploit details or secrets in the public
issue.

## What the software is and is not

This software is a local diagnostics library and CLI. It captures a snapshot
of the local execution environment and produces a document intended for
sharing in bug reports and support tickets.

- Capture performs **no network access** and emits **no telemetry**.
- Standard privacy omits direct identifiers (username, hostname, home
  paths, IP/MAC addresses, hardware serials, machine UUIDs, account/tenant
  IDs) and secret-bearing values (tokens, passwords, API keys). See
  `docs/privacy.md` for the field-by-field contract.
- The explicit `privacy = off` mode is debug-only and can expose raw values,
  including user-configured and potentially identifying content. The CLI
  requires an explicit flag and warns on stderr before producing such output.
- Standard mode minimizes direct identifiers but cannot guarantee that every
  third-party or runtime-provided string is non-identifying; documented
  collector privacy classifications are the authority for what is collected.

## Handling snapshot files

Snapshot files are untrusted input. When diffing, fingerprinting, or
inspecting snapshot files from unknown sources, the CLI applies conservative
read limits (document size, depth, list lengths) and treats malformed input as
an input error, not a crash.
