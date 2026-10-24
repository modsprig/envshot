use envshot_model::{CaptureProfile, PrivacyProfile, SectionData, SectionStatus, read_snapshot};

const SNAPSHOTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/snapshots/v0.1");
const MALFORMED_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/malformed");

type SnapshotAssertions = dyn Fn(&envshot_model::Snapshot);
type FixtureSpec = (&'static str, bool, &'static SnapshotAssertions);

const STABLE: bool = true;
const TOLERANCE: bool = false;
const SNAPSHOT_FIXTURES: &[FixtureSpec] = &[
    ("empty-test-envelope.json", STABLE, &|snapshot| {
        assert_eq!(snapshot.capture.profile, CaptureProfile::EmptyTest);
        assert_eq!(snapshot.capture.privacy, PrivacyProfile::Standard);
        assert!(snapshot.sections.is_empty());
    }),
    ("complete-basic.json", STABLE, &|snapshot| {
        assert_eq!(snapshot.capture.profile, CaptureProfile::Standard);
        let names: Vec<_> = snapshot.sections.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            [
                "architecture",
                "cpu",
                "locale",
                "memory",
                "os",
                "virtualization"
            ]
        );
        for section in snapshot.sections.values() {
            assert_eq!(section.status(), SectionStatus::Available);
        }
        let os = snapshot.sections.get("os").expect("os section");
        let Some(SectionData::Os(os_data)) = os.data() else {
            panic!("os must carry OsData");
        };
        assert_eq!(os_data.version.raw(), "10.0.26100");
        assert_eq!(os.provenance()[0].collector_id(), "os.version");
    }),
    ("partial-snapshot.json", STABLE, &|snapshot| {
        let cpu = snapshot.sections.get("cpu").expect("cpu section");
        assert_eq!(cpu.status(), SectionStatus::Partial);
        assert!(cpu.data().is_some(), "partial requires data");
        assert_eq!(cpu.issues()[0].code.as_str(), "parse_failed");

        let memory = snapshot.sections.get("memory").expect("memory section");
        assert_eq!(memory.status(), SectionStatus::PermissionDenied);
        assert!(memory.data().is_none());
    }),
    ("unsupported-collector.json", STABLE, &|snapshot| {
        let virtualization = snapshot.sections.get("virtualization").expect("section");
        assert_eq!(virtualization.status(), SectionStatus::Unsupported);
        let locale = snapshot.sections.get("locale").expect("section");
        assert_eq!(locale.status(), SectionStatus::Unavailable);
    }),
    ("future-optional-field.json", TOLERANCE, &|snapshot| {
        assert_eq!(snapshot.sections.len(), 1);
        assert!(snapshot.sections.contains_key("os"));
    }),
    ("complete-basic-noise.json", TOLERANCE, &|snapshot| {
        assert!(snapshot.sections.contains_key("cpu"));
        assert!(snapshot.sections.contains_key("os"));
    }),
    ("complete-basic-memory-bump.json", STABLE, &|snapshot| {
        let memory = snapshot.sections.get("memory").expect("memory section");
        assert!(memory.data().is_some());
    }),
];

const MALFORMED_FIXTURES: &[(&str, &str)] = &[
    ("available-without-data.json", "snapshot.read.malformed"),
    ("partial-without-issue.json", "snapshot.read.malformed"),
    ("unknown-status.json", "snapshot.read.malformed"),
    ("foreign-payload.json", "snapshot.read.malformed"),
    ("empty-issue-code.json", "snapshot.read.malformed"),
    ("non-object-sections.json", "snapshot.read.malformed"),
    ("truncated.json", "snapshot.read.malformed"),
    (
        "future-major.json",
        "snapshot.read.unsupported_schema_major",
    ),
];

fn read_fixture(dir: &str, name: &str) -> String {
    std::fs::read_to_string(format!("{dir}/{name}"))
        .unwrap_or_else(|error| panic!("fixture {name} must be readable: {error}"))
}

#[test]
fn all_snapshot_fixtures_read_and_round_trip_byte_stably() {
    for (name, byte_stable, assertions) in SNAPSHOT_FIXTURES.iter().copied() {
        let content = read_fixture(SNAPSHOTS_DIR, name);
        let snapshot = read_snapshot(&content)
            .unwrap_or_else(|error| panic!("fixture {name} must read: {error}"));

        assertions(&snapshot);

        let reserialized = serde_json::to_string(&snapshot).expect("must serialize");
        if byte_stable {
            assert_eq!(reserialized, content, "fixture {name} is not byte-stable");
        } else {
            let reparsed = read_snapshot(&reserialized)
                .unwrap_or_else(|error| panic!("fixture {name} must re-read: {error}"));
            assert_eq!(reparsed, snapshot, "fixture {name} round-trip changed");
        }
    }
}

#[test]
fn all_malformed_fixtures_fail_with_recorded_codes() {
    for (name, expected_code) in MALFORMED_FIXTURES {
        let content = read_fixture(MALFORMED_DIR, name);
        let error = read_snapshot(&content).expect_err("malformed fixture must be rejected");
        assert_eq!(
            error.code(),
            *expected_code,
            "fixture {name} produced the wrong code"
        );
    }
}

#[test]
fn fixture_registries_cover_every_committed_file() {
    for (dir, registry) in [
        (SNAPSHOTS_DIR, SNAPSHOT_FIXTURES.len()),
        (MALFORMED_DIR, MALFORMED_FIXTURES.len()),
    ] {
        let mut committed: Vec<String> = std::fs::read_dir(dir)
            .expect("fixture directory must exist")
            .map(|entry| {
                entry
                    .expect("entry readable")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        committed.sort();
        assert_eq!(
            committed.len(),
            registry,
            "directory {dir} has files not covered by the registry: {committed:?}"
        );
    }
}
