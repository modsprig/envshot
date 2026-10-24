use std::collections::BTreeMap;

use envshot_model::{
    Architecture, ArchitectureData, CpuData, LocaleData, MemoryData, NormalizedVersion, OsData,
    OsFamily, ProducerInfo, SchemaIdentifier, Section, SectionData, Snapshot,
    VirtualizationCategory, VirtualizationData, read_snapshot,
};

fn envelope(sections: BTreeMap<String, Section<SectionData>>) -> Snapshot {
    Snapshot {
        schema: SchemaIdentifier::current(),
        producer: ProducerInfo {
            name: "envshot".to_owned(),
            version: "0.0.0".to_owned(),
        },
        capture: envshot_model::CaptureMetadata {
            profile: envshot_model::CaptureProfile::EmptyTest,
            privacy: envshot_model::PrivacyProfile::Standard,
        },
        sections,
    }
}

fn os_data() -> SectionData {
    SectionData::Os(OsData {
        family: OsFamily::Windows,
        version: envshot_model::VersionValue::new("10.0.26100"),
        build: Some(envshot_model::VersionValue::new("26100.1742")),
    })
}

#[test]
fn version_value_normalizes_and_keeps_raw() {
    let version = envshot_model::VersionValue::new("10.0.26100");

    assert_eq!(version.raw(), "10.0.26100");
    assert_eq!(
        version.normalized(),
        Some(&NormalizedVersion {
            major: 10,
            minor: Some(0),
            patch: Some(26100),
        })
    );
}

#[test]
fn version_value_parses_suffix_components_conservatively() {
    let version = envshot_model::VersionValue::new("1.2.3-beta.1");

    assert_eq!(version.raw(), "1.2.3-beta.1");
    assert_eq!(
        version.normalized(),
        Some(&NormalizedVersion {
            major: 1,
            minor: Some(2),
            patch: Some(3),
        })
    );

    let short = envshot_model::VersionValue::new("5");
    assert_eq!(
        short.normalized(),
        Some(&NormalizedVersion {
            major: 5,
            minor: None,
            patch: None,
        })
    );
}

#[test]
fn version_value_preserves_unparsable_raw() {
    let version = envshot_model::VersionValue::new("unknown");

    assert_eq!(version.raw(), "unknown");
    assert_eq!(version.normalized(), None);
}

#[test]
fn all_typed_payloads_round_trip_through_documents() {
    let mut sections = BTreeMap::new();
    sections.insert("os".to_owned(), Section::available(os_data()));
    sections.insert(
        "architecture".to_owned(),
        Section::available(SectionData::Architecture(ArchitectureData {
            process: Architecture::X64,
            os: Architecture::Arm64,
        })),
    );
    sections.insert(
        "cpu".to_owned(),
        Section::available(SectionData::Cpu(CpuData {
            model: Some("Test CPU".to_owned()),
            logical_count: Some(8),
            physical_count: Some(4),
            features: ["avx2", "sse4_2"].into_iter().map(String::from).collect(),
        })),
    );
    sections.insert(
        "memory".to_owned(),
        Section::available(SectionData::Memory(MemoryData {
            installed_bytes: 34_359_738_368,
        })),
    );
    sections.insert(
        "locale".to_owned(),
        Section::available(SectionData::Locale(LocaleData {
            languages: vec!["en-US".to_owned(), "de-DE".to_owned()],
            timezone: Some("Europe/Berlin".to_owned()),
        })),
    );
    sections.insert(
        "virtualization".to_owned(),
        Section::available(SectionData::Virtualization(VirtualizationData {
            category: VirtualizationCategory::VirtualMachine,
        })),
    );

    let snapshot = envelope(sections);
    let json = serde_json::to_string(&snapshot).expect("serialization must not fail");
    let parsed = read_snapshot(&json).expect("own serialization must read");

    assert_eq!(parsed, snapshot);

    let os = parsed.sections.get("os").expect("os section");
    let SectionData::Os(os_data) = os.data().expect("available has data") else {
        panic!("os section must carry OsData");
    };
    assert_eq!(os_data.version.raw(), "10.0.26100");
    assert_eq!(
        os_data.build.as_ref().expect("build").normalized(),
        Some(&NormalizedVersion {
            major: 26100,
            minor: Some(1742),
            patch: None
        })
    );

    let virtualization = parsed.sections.get("virtualization").expect("section");
    let SectionData::Virtualization(virtualization_data) =
        virtualization.data().expect("available has data")
    else {
        panic!("virtualization section must carry VirtualizationData");
    };
    assert_eq!(
        virtualization_data.category,
        VirtualizationCategory::VirtualMachine
    );
}

#[test]
fn cpu_features_serialize_sorted() {
    let cpu = CpuData {
        model: None,
        logical_count: None,
        physical_count: None,
        features: ["sse4_2", "avx2", "avx512f"]
            .into_iter()
            .map(String::from)
            .collect(),
    };
    let section = Section::available(SectionData::Cpu(cpu));

    let json = serde_json::to_string(&section).expect("serialization must not fail");
    assert!(
        json.contains(r#""features":["avx2","avx512f","sse4_2"]"#),
        "features must serialize sorted by name: {json}"
    );
}

#[test]
fn known_section_rejects_foreign_payload_shape() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {
            "memory": {"status": "available", "data": {"family": "windows", "version": {"raw": "10"}}}
        }
    }"#;

    let error = read_snapshot(document).expect_err("foreign payload must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn unknown_sections_are_tolerated_and_skipped() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {
            "os": {"status": "available", "data": {"family": "linux", "version": {"raw": "6.9"}}},
            "future_thing": {"status": "available", "data": {"whatever": true}}
        }
    }"#;

    let snapshot = read_snapshot(document).expect("unknown sections must be tolerated");

    assert!(snapshot.sections.contains_key("os"));
    assert!(
        !snapshot.sections.contains_key("future_thing"),
        "unknown sections are skipped by this schema minor"
    );
}

#[test]
fn reader_rejects_non_object_sections() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": []
    }"#;

    let error = read_snapshot(document).expect_err("non-object sections must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");
}
