use std::collections::BTreeSet;

use envshot_core::{DiffPolicy, diff};
use envshot_model::{
    CaptureMetadata, CaptureProfile, CpuData, MemoryData, PrivacyProfile, ProducerInfo,
    SchemaIdentifier, Section, SectionData, Snapshot,
};
use proptest::prelude::*;

fn envelope(memory_bytes: u64, logical: u32, features: Vec<String>) -> Snapshot {
    let mut sections = std::collections::BTreeMap::new();
    sections.insert(
        "memory".to_owned(),
        Section::available(SectionData::Memory(MemoryData {
            installed_bytes: memory_bytes,
        })),
    );
    sections.insert(
        "cpu".to_owned(),
        Section::available(SectionData::Cpu(CpuData {
            model: Some("Fixture CPU".to_owned()),
            logical_count: Some(logical),
            physical_count: None,
            features: features.into_iter().collect::<BTreeSet<_>>(),
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

fn generated_snapshot() -> impl Strategy<Value = Snapshot> {
    (
        1u64..=65_536,
        1u32..=64,
        proptest::collection::vec(
            proptest::sample::select(vec!["sse2", "sse4_2", "avx", "avx2", "avx512f"]),
            0usize..=4,
        ),
    )
        .prop_map(|(bytes, logical, features)| {
            envelope(
                bytes,
                logical,
                features.into_iter().map(String::from).collect(),
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn diff_x_with_x_is_empty(x in generated_snapshot()) {
        prop_assert!(diff(&x, &x, &DiffPolicy::compatibility()).is_empty());
        prop_assert!(diff(&x, &x, &DiffPolicy::diagnostic()).is_empty());
    }

    #[test]
    fn changes_are_sorted_by_path(x in generated_snapshot(), y in generated_snapshot()) {
        for policy in [DiffPolicy::compatibility(), DiffPolicy::diagnostic()] {
            let diff = diff(&x, &y, &policy);
            let paths: Vec<&str> = diff.changes().iter().map(|c| c.path.as_str()).collect();
            let mut sorted = paths.clone();
            sorted.sort_unstable();
            prop_assert_eq!(paths, sorted);
        }
    }

    #[test]
    fn round_trip_does_not_create_a_diff(x in generated_snapshot()) {
        let serialized = serde_json::to_string(&x).expect("must serialize");
        let parsed = envshot_model::read_snapshot(&serialized).expect("must read");
        prop_assert!(diff(&x, &parsed, &DiffPolicy::compatibility()).is_empty());
        prop_assert!(diff(&x, &parsed, &DiffPolicy::diagnostic()).is_empty());
    }
}

#[test]
fn metadata_only_changes_produce_no_compatibility_diff() {
    let mut a = envelope(1024, 8, vec!["avx2".to_owned()]);
    let mut b = a.clone();
    a.capture.privacy = PrivacyProfile::Off;
    a.capture.profile = CaptureProfile::Minimal;
    a.producer.version = "9.9.9".to_owned();
    let _ = &mut b;

    assert!(diff(&a, &b, &DiffPolicy::compatibility()).is_empty());
}
