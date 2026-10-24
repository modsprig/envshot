# Platform capability matrix

Parity means the same user questions are answerable, not identical fields.
"source" names the collector; statuses follow the section contract
(available/partial/unsupported/unavailable/permission_denied/failed).

| Question | Windows | Linux | macOS |
|---|---|---|---|
| OS family/version/build | `os` (sysinfo) + `windows` (registry: edition, display version, UBR) | `os` (sysinfo) + `distro` (os-release: name, version) | `os` (sysinfo product version) |
| Process/OS architecture + emulation | `architecture` (IsWow64Process2) | `architecture` (std + sysinfo) | `architecture` (std + sysinfo) |
| CPU model/features/counts | `cpu` (sysinfo + CPUID features) | `cpu` (sysinfo; features via CPUID where x86) | `cpu` (sysinfo) |
| Installed memory | `memory` | `memory` | `memory` |
| Locale/timezone | `locale` (sys-locale + iana-time-zone) | same | same |
| Virtualization/container category | `virtualization` (CPUID hypervisor bit) | `virtualization` (CPUID bit + container markers) | `virtualization` (unknown unless detectable) |
| GPU adapters + driver | `gpu` (DXGI; Windows only) | not yet | not yet |
| Distinctly Windows facts | `windows` section | n/a | n/a |
| Distinctly Linux facts | n/a | `distro` section | n/a |

Unsupported sections appear with the `unsupported` status and a stable issue
code rather than disappearing, so consumers can distinguish "not asked" from
"answer is no".

## Deliberate exclusions (all platforms)

No username, hostname, home path, IP/MAC, hardware serials, machine UUIDs,
account/tenant IDs, secret environment variables; no GPU serial/UUID/LUID;
no stable pseudonymous host identifier. See `docs/privacy.md`.
