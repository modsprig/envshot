use envshot_capture::capture;
use envshot_model::{CaptureProfile, Snapshot};
use proptest::prelude::*;

fn serialize(snapshot: &Snapshot) -> String {
    serde_json::to_string(snapshot).expect("snapshot serialization must not fail")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn repeated_captures_are_byte_identical(repeat_count in 1u8..=32) {
        let reference = serialize(&capture(CaptureProfile::EmptyTest));
        for _ in 0..repeat_count {
            prop_assert_eq!(&serialize(&capture(CaptureProfile::EmptyTest)), &reference);
        }
    }

    #[test]
    fn serialize_parse_serialize_is_a_fixpoint(round_trips in 1u8..=8) {
        let mut current = serialize(&capture(CaptureProfile::EmptyTest));
        for _ in 0..round_trips {
            let parsed: Snapshot =
                serde_json::from_str(&current).expect("own serialization must parse");
            let next = serialize(&parsed);
            prop_assert_eq!(&next, &current);
            current = next;
        }
    }




    #[test]
    fn writer_reader_writer_is_a_fixpoint(round_trips in 1u8..=8) {
        let mut current = serialize(&capture(CaptureProfile::EmptyTest));
        for _ in 0..round_trips {
            let parsed = envshot_model::read_snapshot(&current)
                .expect("own serialization must read");
            let next = serialize(&parsed);
            prop_assert_eq!(&next, &current);
            current = next;
        }
    }
}
