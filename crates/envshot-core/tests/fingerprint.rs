use envshot_core::{FingerprintProfile, fingerprint};

const COMPLETE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/snapshots/v0.1/complete-basic.json"
));
const NOISE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/snapshots/v0.1/complete-basic-noise.json"
));
const MEMORY_BUMP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/snapshots/v0.1/complete-basic-memory-bump.json"
));

fn read(content: &str) -> envshot_model::Snapshot {
    envshot_model::read_snapshot(content).expect("fixture must read")
}

fn repro_v1(content: &str) -> envshot_core::Fingerprint {
    fingerprint(&read(content), &FingerprintProfile::ReproV1).expect("supported major")
}

#[test]
fn envelope_shape_matches_the_specification() {
    let fingerprint = repro_v1(COMPLETE);
    let segments: Vec<&str> = fingerprint.value.splitn(3, ':').collect();
    let [profile, algorithm, digest] = segments.as_slice() else {
        panic!("value must have three segments: {}", fingerprint.value);
    };
    assert_eq!(*profile, "repro-v1");
    assert_eq!(*algorithm, "sha256");
    assert_eq!(digest.len(), 64);
    assert!(
        digest
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

#[test]
fn projection_is_canonical_and_explains_the_fingerprint() {
    let fingerprint = repro_v1(COMPLETE);
    let parsed: serde_json::Value =
        serde_json::from_str(&fingerprint.projection).expect("projection must be JSON");

    assert_eq!(parsed["profile"], "repro-v1");
    assert_eq!(parsed["schema"]["major"], 0);
    assert_eq!(parsed["facts"]["os.family"], "windows");
    assert_eq!(parsed["facts"]["os.version"]["major"], 10);
    assert_eq!(
        parsed["facts"]["cpu.features"],
        serde_json::json!(["avx2", "sse4_2"])
    );
    assert_eq!(
        parsed["facts"]["memory.installed_bytes"],
        serde_json::json!(17_179_869_184_u64)
    );

    let serialized = fingerprint.projection.clone();
    for excluded in [
        "locale",
        "timezone",
        "model",
        "logical_count",
        "detail",
        "provenance",
    ] {
        assert!(
            !serialized.contains(excluded),
            "excluded fact {excluded} appeared"
        );
    }
}

#[test]
fn noise_variants_produce_identical_fingerprints() {
    assert_eq!(repro_v1(COMPLETE).value, repro_v1(NOISE).value);
    assert_eq!(repro_v1(COMPLETE).projection, repro_v1(NOISE).projection);
}

#[test]
fn one_fact_change_changes_the_digest() {
    assert_ne!(repro_v1(COMPLETE).value, repro_v1(MEMORY_BUMP).value);
}

#[test]
fn fingerprint_is_independent_of_standard_vs_strict_privacy() {
    use envshot_core::{PrivacyPolicy, apply_privacy};

    let standard = apply_privacy(&read(COMPLETE), &PrivacyPolicy::standard());
    let strict = apply_privacy(&read(COMPLETE), &PrivacyPolicy::strict());

    let standard_fp = fingerprint(&standard, &FingerprintProfile::ReproV1)
        .expect("supported major")
        .value;
    let strict_fp = fingerprint(&strict, &FingerprintProfile::ReproV1)
        .expect("supported major")
        .value;
    assert_eq!(standard_fp, strict_fp);
}

#[test]
fn round_trip_preserves_the_fingerprint() {
    let direct = repro_v1(COMPLETE);
    let round_tripped = repro_v1(&serde_json::to_string(&read(COMPLETE)).expect("must serialize"));
    assert_eq!(direct.value, round_tripped.value);
    assert_eq!(direct.projection, round_tripped.projection);
}

#[test]
fn unsupported_schema_major_is_rejected() {
    let mut future = read(COMPLETE);
    future.schema.major = 1;

    let error = fingerprint(&future, &FingerprintProfile::ReproV1)
        .expect_err("future major must be rejected");
    assert!(matches!(
        error,
        envshot_core::FingerprintError::UnsupportedSchemaMajor {
            found: 1,
            supported: 0
        }
    ));
}

#[test]
fn empty_envelope_fingerprints_deterministically() {
    let envelope = envshot_model::read_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/snapshots/v0.1/empty-test-envelope.json"
    )))
    .expect("fixture reads");
    let first = fingerprint(&envelope, &FingerprintProfile::ReproV1).expect("supported major");
    let second = fingerprint(&envelope, &FingerprintProfile::ReproV1).expect("supported major");
    assert_eq!(first.value, second.value);
    assert!(first.projection.contains(r#""facts":{}"#));
}

#[test]
fn golden_vector_complete_basic_is_frozen() {
    assert_eq!(
        repro_v1(COMPLETE).value,
        "repro-v1:sha256:4119291c8cf461350058f0cedf1de7ff7d5fdb75d5ee6bc0f3045599b32608a7"
    );
}

#[test]
fn golden_vector_noise_variant_is_frozen() {
    assert_eq!(
        repro_v1(NOISE).value,
        "repro-v1:sha256:4119291c8cf461350058f0cedf1de7ff7d5fdb75d5ee6bc0f3045599b32608a7"
    );
}

#[test]
fn golden_vector_memory_bump_is_frozen_and_distinct() {
    assert_eq!(
        repro_v1(MEMORY_BUMP).value,
        "repro-v1:sha256:56d80e37ea9ade53154c4ea30220580d01882e854a935d824380a96df84a1e68"
    );
}
