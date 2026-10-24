use envshot_model::VersionValue;
use proptest::prelude::*;

fn version_like() -> impl Strategy<Value = String> {
    let numeric = r"[0-9]{1,10}";
    let component = prop_oneof![
        Just(String::new()),
        numeric.prop_map(|s| s),
        "[a-z]+",
        "[0-9]*[a-z][0-9a-z]*",
        r"[0-9]+\-[a-z]+\.[0-9]+"
    ];
    prop_oneof![
        8 => prop::collection::vec(component, 0..=5).prop_map(|parts| parts.join(".")),
        1 => Just(String::from(" 10 .0")),
        1 => Just(String::from("\u{0}1.2")),
    ]
}

fn rule(raw: &str) -> bool {
    raw.split('.')
        .next()
        .and_then(|first| {
            let digits = first
                .char_indices()
                .take_while(|(_, c)| c.is_ascii_digit())
                .count();
            if digits == 0 {
                None
            } else {
                first[..digits].parse::<u64>().ok()
            }
        })
        .is_some()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn raw_is_preserved_and_normalized_follows_the_rule(raw in version_like()) {
        let value = VersionValue::new(&raw);
        prop_assert_eq!(value.raw(), raw.as_str());
        prop_assert_eq!(value.normalized().is_some(), rule(&raw));
    }

    #[test]
    fn rederiving_from_raw_is_identity(raw in version_like()) {
        let value = VersionValue::new(&raw);
        let rederived = VersionValue::new(value.raw());
        prop_assert_eq!(&rederived, &value);
        let serialized = serde_json::to_string(&value).expect("must serialize");
        let parsed: VersionValue = serde_json::from_str(&serialized).expect("must parse");
        prop_assert_eq!(parsed, value);
    }
}
