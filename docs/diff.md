# Semantic diff output examples

The diff model, policies, and semantics live in `envshot-core::diff`. The
examples below are the actual output of

```text
envshot diff fixtures/snapshots/v0.1/partial-snapshot.json fixtures/snapshots/v0.1/complete-basic.json
```

## Text rendering (default)

Changes are grouped by section, in the diff's deterministic path order:

```text
architecture:
  section added

cpu:
  status: partial -> available

locale:
  section added

memory:
  status: permission_denied -> available

os:
  build: <absent> -> 26100.1742
  family: linux -> windows
  version: 6.9 -> 10.0.26100

virtualization:
  section added
```

- `section added` / `section removed` mark whole-section appearance;
- `<absent>` marks a field that exists only on one side;
- `features: + avx512f, sse4_2` would be a `SetChanged` entry: added members
  after `+`, removed members after `-`;
- `status:` lines are `StatusChanged` entries, reported independently of
  data changes (a differing status suppresses field-level noise);
- the compatibility view (default) excludes diagnostic metadata such as
  producer version and capture profile/privacy; `--policy diagnostic`
  includes them;
- closed-vocabulary values (statuses, families, categories) render as their
  stable snake_case codes.

## JSON rendering (`envshot diff --format json`)

```json
{"diff_schema":{"name":"envshot.diff","version":1},"changes":[{"path":"cpu.status","kind":"status_changed","before":"partial","after":"available","relevance":"compatibility"}]}
```

The envelope name and version are independent of the snapshot schema version;
consumers of the JSON envelope check `diff_schema` before reading `changes`.
