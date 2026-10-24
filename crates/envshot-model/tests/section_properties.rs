use envshot_model::{CollectorIssue, IssueCode, Provenance, Section, SectionStatus};
use proptest::prelude::*;

fn all_statuses() -> Vec<SectionStatus> {
    vec![
        SectionStatus::Available,
        SectionStatus::Partial,
        SectionStatus::Unsupported,
        SectionStatus::PermissionDenied,
        SectionStatus::Unavailable,
        SectionStatus::Failed,
    ]
}

fn issues(count: usize) -> Vec<CollectorIssue> {
    (0..count)
        .map(|i| {
            let code = IssueCode::new(format!("code_{i}")).expect("non-empty");
            CollectorIssue::new(code)
        })
        .collect()
}

fn is_valid(status: SectionStatus, has_data: bool, issue_count: usize) -> bool {
    match status {
        SectionStatus::Available => has_data,
        SectionStatus::Partial => has_data && issue_count >= 1,
        _ => true,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn new_accepts_exactly_the_valid_combinations(
        status in proptest::sample::select(all_statuses()),
        has_data in proptest::bool::ANY,
        issue_count in 0usize..=3,
    ) {
        let result = Section::<()>::new(status, has_data.then_some(()), issues(issue_count));
        prop_assert_eq!(result.is_ok(), is_valid(status, has_data, issue_count));
    }

    #[test]
    fn valid_sections_round_trip_semantically(
        status in proptest::sample::select(all_statuses()),
        issue_count in 0usize..=3,
    ) {


        let has_data = matches!(status, SectionStatus::Available | SectionStatus::Partial);
        let issue_count = if status == SectionStatus::Partial {
            issue_count.max(1)
        } else {
            issue_count
        };
        let section =
            Section::<u32>::new(status, has_data.then_some(7), issues(issue_count))
                .expect("generated combination is valid");

        let json = serde_json::to_string(&section).expect("serialization must not fail");
        let parsed: Section<u32> = serde_json::from_str(&json).expect("must parse");
        prop_assert_eq!(&parsed, &section);
        prop_assert_eq!(parsed.issues().len(), section.issues().len());
    }

    #[test]
    fn issue_order_is_preserved_through_round_trip(issue_count in 1usize..=6) {
        let section =
            Section::<()>::new(SectionStatus::Failed, None, issues(issue_count))
                .expect("failed with issues is valid");

        let json = serde_json::to_string(&section).expect("serialization must not fail");
        let parsed: Section<()> = serde_json::from_str(&json).expect("must parse");

        let original: Vec<String> =
            section.issues().iter().map(|i| i.code.as_str().to_owned()).collect();
        let round_tripped: Vec<String> =
            parsed.issues().iter().map(|i| i.code.as_str().to_owned()).collect();
        prop_assert_eq!(round_tripped, original);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]




    #[test]
    fn provenance_is_order_independent(
        entries in proptest::collection::vec(
            (
                proptest::sample::select(vec!["collector.a", "collector.b", "collector.c", "collector.d"]),
                0u32..=3,
            ),
            0usize..=6,
        ),
    ) {
        use envshot_model::SourceKind;

        let make = |pairs: &[(&str, u32)]| -> Section<()> {
            let records: Vec<Provenance> = pairs
                .iter()
                .map(|(id, revision)| {
                    Provenance::new(*id, *revision, SourceKind::SystemFile)
                        .expect("test ids are non-empty")
                })
                .collect();
            Section::<()>::failed(Vec::new()).with_provenance(records)
        };

        let forward = make(&entries);
        let mut reversed = entries.clone();
        reversed.reverse();
        let backward = make(&reversed);

        prop_assert_eq!(&forward, &backward);
        let forward_json = serde_json::to_string(&forward).expect("must serialize");
        let backward_json = serde_json::to_string(&backward).expect("must serialize");
        prop_assert_eq!(forward_json, backward_json);
    }
}
