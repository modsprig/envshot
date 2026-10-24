use std::collections::BTreeSet;

use envshot_core::{
    DiffPolicy, FingerprintProfile, PrivacyPolicy, apply_privacy, diff, fingerprint, normalize,
    symbolic_path,
};
use envshot_model::{
    CaptureMetadata, CaptureProfile, CpuData, LocaleData, PrivacyProfile, ReadLimits, Section,
    SectionData, SectionStatus, Snapshot, read_snapshot_with_limits,
};
use proptest::prelude::*;

fn envelope(features: Vec<String>, timezone: String) -> Snapshot {
    let mut sections = std::collections::BTreeMap::new();
    sections.insert(
        "cpu".to_owned(),
        Section::available(SectionData::Cpu(CpuData {
            model: Some("Fixture CPU".to_owned()),
            logical_count: Some(8),
            physical_count: None,
            features: features.into_iter().collect::<BTreeSet<_>>(),
        })),
    );
    sections.insert(
        "locale".to_owned(),
        Section::available(SectionData::Locale(LocaleData {
            languages: vec!["en-US".to_owned()],
            timezone: Some(timezone),
        })),
    );
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
        sections,
    }
}

use envshot_model::{ProducerInfo, SchemaIdentifier};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]



    #[test]
    fn large_feature_sets_survive_the_pipeline(
        count in 4_000usize..=5_000usize,
        seed in any::<u64>(),
    ) {
        let features: Vec<String> = (0..count)
            .map(|i| format!("feature_{i}_{seed}"))
            .collect();
        let snapshot = envelope(features.clone(), "Etc/UTC".to_owned());

        let normalized = normalize(&snapshot);
        prop_assert_eq!(normalized.sections["cpu"].data().is_some(), true);
        let projected = apply_privacy(&normalized, &PrivacyPolicy::standard());
        let computed = fingerprint(&projected, &FingerprintProfile::ReproV1)
            .expect("supported major");
        let again = fingerprint(&apply_privacy(&normalized, &PrivacyPolicy::standard()),
            &FingerprintProfile::ReproV1).expect("supported major");
        prop_assert_eq!(computed.value, again.value);



        let _ = diff(&normalized, &projected, &DiffPolicy::diagnostic());
    }



    #[test]
    fn unicode_extremes_survive_the_pipeline(
        marker in "[\u{1F600}\u{05D0}\u{0301}\u{200B}]{0,200}",
    ) {
        let snapshot = envelope(
            Vec::new(),
            format!("{marker}Etc/UTC"),
        );
        let normalized = normalize(&snapshot);
        let projected = apply_privacy(&normalized, &PrivacyPolicy::off());
        let reserialized = serde_json::to_string(&projected).expect("must serialize");
        let reparsed = envshot_model::read_snapshot(&reserialized).expect("must read");
        prop_assert_eq!(reparsed, projected);
    }


    #[test]
    fn generated_canaries_do_not_leak(
        secret in "[A-Za-z0-9_]{8,32}",
    ) {
        use envshot_model::CollectorIssue;
        let mut snapshot = envelope(Vec::new(), "Etc/UTC".to_owned());
        let issue = CollectorIssue::new(
            envshot_model::IssueCode::new("parse_failed").expect("static"),
        )
        .with_detail(format!("S3CR3T_CANARY_TOKEN_{secret}"));
        snapshot.sections.insert(
            "hostile".to_owned(),
            Section::<SectionData>::new(SectionStatus::Failed, None, vec![issue])
                .expect("valid"),
        );

        for policy in [PrivacyPolicy::standard(), PrivacyPolicy::strict()] {
            let json = serde_json::to_string(&apply_privacy(&snapshot, &policy))
                .expect("must serialize");
            prop_assert!(!json.contains("S3CR3T_CANARY_TOKEN"), "canary leaked under {policy:?}");
        }
    }
}

#[test]
fn over_limit_input_is_rejected() {
    let big = format!(
        "{{\"schema\":{{\"name\":\"envshot.snapshot\",\"major\":0,\"minor\":1}},\"producer\":{{\"name\":\"envshot\",\"version\":\"0.0.0\"}},\"capture\":{{\"profile\":\"standard\",\"privacy\":\"standard\"}},\"sections\":{{}},\"pad\":\"{}\"}}",
        "x".repeat(4096)
    );
    let limits = ReadLimits {
        max_input_bytes: 1024,
    };
    let error =
        read_snapshot_with_limits(&big, &limits).expect_err("over-limit input must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");

    assert!(envshot_model::read_snapshot(&big).is_ok());
    let exact = ReadLimits {
        max_input_bytes: big.len(),
    };
    assert!(read_snapshot_with_limits(&big, &exact).is_ok());
}

#[test]
fn every_status_combination_survives_the_pipeline() {
    let statuses = [
        SectionStatus::Available,
        SectionStatus::Partial,
        SectionStatus::Unsupported,
        SectionStatus::PermissionDenied,
        SectionStatus::Unavailable,
        SectionStatus::Failed,
    ];
    for status in statuses {
        let (data, issues) = match status {
            SectionStatus::Available | SectionStatus::Partial => {
                let issue = envshot_model::CollectorIssue::new(
                    envshot_model::IssueCode::new("parse_failed").expect("static"),
                );
                (
                    Some(SectionData::Memory(envshot_model::MemoryData {
                        installed_bytes: 1,
                    })),
                    vec![issue],
                )
            }
            _ => (None, Vec::new()),
        };
        let mut snapshot = envelope(Vec::new(), "Etc/UTC".to_owned());
        snapshot.sections.insert(
            "probed".to_owned(),
            Section::<SectionData>::new(status, data, issues).expect("valid construction"),
        );
        for policy in [
            PrivacyPolicy::standard(),
            PrivacyPolicy::strict(),
            PrivacyPolicy::off(),
        ] {
            let json =
                serde_json::to_string(&apply_privacy(&snapshot, &policy)).expect("must serialize");
            assert!(!json.contains("S3CR3T"));
        }
    }
}

#[test]
fn path_privacy_edge_cases() {
    assert_eq!(
        symbolic_path(r"C:\Users\Alice\", Some(r"C:\Users\Alice")),
        "%USERPROFILE%"
    );
    assert_eq!(
        symbolic_path(r"\server\share\doc.txt", Some(r"C:\Users\Alice")),
        r"\server\share\doc.txt"
    );
    assert_eq!(
        symbolic_path(r"C:\users\ALICE\Deep\Path", Some(r"C:\Users\Alice")),
        r"%USERPROFILE%\Deep\Path"
    );
}
