use envshot_core::normalize;
use envshot_model::{
    CaptureMetadata, CaptureProfile, CpuData, LocaleData, NormalizedVersion, OsData, OsFamily,
    PrivacyProfile, ProducerInfo, SchemaIdentifier, Section, SectionData, Snapshot, VersionValue,
};
use proptest::prelude::*;

fn envelope(sections: Vec<(&'static str, SectionData)>) -> Snapshot {
    let mut map = std::collections::BTreeMap::new();
    for (id, data) in sections {
        map.insert(id.to_owned(), Section::available(data));
    }
    Snapshot {
        schema: SchemaIdentifier::current(),
        producer: ProducerInfo {
            name: "envshot".to_owned(),
            version: "0.0.0".to_owned(),
        },
        capture: CaptureMetadata {
            profile: CaptureProfile::Standard,
            privacy: PrivacyProfile::Standard,
        },
        sections: map,
    }
}

#[test]
fn formatting_only_differences_normalize_identically() {
    let messy = envelope(vec![
        (
            "cpu",
            SectionData::Cpu(CpuData {
                model: Some("Fixture   CPU\tX3".to_owned()),
                logical_count: Some(8),
                physical_count: Some(4),
                features: ["AVX2", "Sse4_2"].into_iter().map(String::from).collect(),
            }),
        ),
        (
            "os",
            SectionData::Os(OsData {
                family: OsFamily::Windows,
                version: VersionValue::new("10.0.26100"),
                build: None,
            }),
        ),
        (
            "locale",
            SectionData::Locale(LocaleData {
                languages: vec!["  en-US ".to_owned()],
                timezone: Some(" Europe/Berlin ".to_owned()),
            }),
        ),
    ]);
    let clean = envelope(vec![
        (
            "cpu",
            SectionData::Cpu(CpuData {
                model: Some("Fixture CPU X3".to_owned()),
                logical_count: Some(8),
                physical_count: Some(4),
                features: ["avx2", "sse4_2"].into_iter().map(String::from).collect(),
            }),
        ),
        (
            "os",
            SectionData::Os(OsData {
                family: OsFamily::Windows,
                version: VersionValue::new("10.0.26100"),
                build: None,
            }),
        ),
        (
            "locale",
            SectionData::Locale(LocaleData {
                languages: vec!["en-US".to_owned()],
                timezone: Some("Europe/Berlin".to_owned()),
            }),
        ),
    ]);

    let messy_normalized = normalize(&messy);
    let clean_normalized = normalize(&clean);

    assert_eq!(
        serde_json::to_string(&messy_normalized).expect("must serialize"),
        serde_json::to_string(&clean_normalized).expect("must serialize")
    );
}

#[test]
fn raw_version_text_is_preserved() {
    let snapshot = envelope(vec![(
        "os",
        SectionData::Os(OsData {
            family: OsFamily::Windows,
            version: VersionValue::new("10.0.26100"),
            build: None,
        }),
    )]);

    let normalized = normalize(&snapshot);
    let Some(SectionData::Os(os)) = normalized.sections["os"].data() else {
        panic!("os section must remain");
    };
    assert_eq!(os.version.raw(), "10.0.26100");
    assert_eq!(
        os.version.normalized(),
        Some(&NormalizedVersion {
            major: 10,
            minor: Some(0),
            patch: Some(26100),
        })
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]


    #[test]
    fn normalization_is_idempotent(
        cpu_model in ".*",
        locale_value in "[a-z]{0,4}-[A-Z]{0,4} {0,3}",
        timezone in " *Area/ *Place_* ",
    ) {
        let mut features = std::collections::BTreeSet::new();
        features.insert("AVX2".to_owned());
        features.insert("sse4_2 ".to_owned());
        let snapshot = envelope(vec![
            (
                "cpu",
                SectionData::Cpu(CpuData {
                    model: Some(cpu_model),
                    logical_count: Some(2),
                    physical_count: None,
                    features,
                }),
            ),
            (
                "locale",
                SectionData::Locale(LocaleData {
                    languages: vec![locale_value],
                    timezone: Some(timezone),
                }),
            ),
        ]);

        let once = normalize(&snapshot);
        let twice = normalize(&once);
        prop_assert_eq!(&once, &twice);

        let once_json = serde_json::to_string(&once).expect("must serialize");
        let twice_json = serde_json::to_string(&twice).expect("must serialize");
        prop_assert_eq!(once_json, twice_json);
    }
}
