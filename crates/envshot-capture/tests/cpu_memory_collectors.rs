use envshot_capture::capture;
use envshot_model::{CaptureProfile, SectionData, SectionStatus};

#[test]
fn real_host_cpu_satisfies_invariants() {
    let snapshot = capture(CaptureProfile::Standard);

    let section = snapshot.sections.get("cpu").expect("built-in cpu section");
    assert!(matches!(
        section.status(),
        SectionStatus::Available | SectionStatus::Partial
    ));
    let Some(SectionData::Cpu(cpu)) = section.data() else {
        panic!("cpu section must carry CpuData");
    };
    assert!(
        cpu.logical_count.unwrap_or(0) > 0,
        "logical count must be positive"
    );
    if let Some(model) = &cpu.model {
        assert!(!model.is_empty(), "model must be non-empty when present");
    }
    assert_eq!(section.provenance()[0].collector_id(), "cpu.host");
}

#[test]
fn real_host_memory_satisfies_invariants() {
    let snapshot = capture(CaptureProfile::Standard);

    let section = snapshot
        .sections
        .get("memory")
        .expect("built-in memory section");
    assert_eq!(section.status(), SectionStatus::Available);
    let Some(SectionData::Memory(memory)) = section.data() else {
        panic!("memory section must carry MemoryData");
    };
    assert!(
        memory.installed_bytes > 0,
        "installed bytes must be positive"
    );
}

#[test]
fn empty_test_profile_still_yields_envelope() {
    let snapshot = capture(CaptureProfile::EmptyTest);
    assert!(snapshot.sections.is_empty());
}

#[test]
fn cpu_features_are_within_the_curated_vocabulary() {
    const VOCABULARY: [&str; 8] = [
        "sse", "sse2", "sse4_1", "sse4_2", "aes", "avx", "avx2", "avx512f",
    ];
    let snapshot = capture(CaptureProfile::Standard);
    let Some(SectionData::Cpu(cpu)) = snapshot.sections.get("cpu").expect("cpu section").data()
    else {
        panic!("cpu section must carry CpuData");
    };
    for feature in &cpu.features {
        assert!(
            VOCABULARY.contains(&feature.as_str()),
            "non-vocabulary feature {feature:?}"
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[test]
fn x86_64_hosts_report_a_non_empty_feature_set() {
    let snapshot = capture(CaptureProfile::Standard);
    let Some(SectionData::Cpu(cpu)) = snapshot.sections.get("cpu").expect("cpu section").data()
    else {
        panic!("cpu section must carry CpuData");
    };
    assert!(
        !cpu.features.is_empty(),
        "x86_64 hosts must report features"
    );
}
