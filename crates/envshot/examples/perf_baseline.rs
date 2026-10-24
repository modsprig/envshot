use std::collections::BTreeSet;
use std::time::Instant;

use envshot_core::{DiffPolicy, PrivacyPolicy, apply_privacy, diff, fingerprint, normalize};
use envshot_model::{
    CaptureMetadata, CaptureProfile, CpuData, MemoryData, OsData, OsFamily, PrivacyProfile,
    ProducerInfo, SchemaIdentifier, Section, SectionData, Snapshot, VersionValue,
    VirtualizationCategory, VirtualizationData,
};

fn typical_snapshot() -> Snapshot {
    let mut sections = std::collections::BTreeMap::new();
    sections.insert(
        "os".to_owned(),
        Section::available(SectionData::Os(OsData {
            family: OsFamily::Windows,
            version: VersionValue::new("10.0.26100"),
            build: Some(VersionValue::new("26100.1742")),
        })),
    );
    sections.insert(
        "architecture".to_owned(),
        Section::available(SectionData::Architecture(envshot_model::ArchitectureData {
            process: envshot_model::Architecture::X64,
            os: envshot_model::Architecture::X64,
        })),
    );
    sections.insert(
        "cpu".to_owned(),
        Section::available(SectionData::Cpu(CpuData {
            model: Some("Fixture CPU X3".to_owned()),
            logical_count: Some(16),
            physical_count: Some(8),
            features: ["avx2", "sse4_2", "aes", "avx512f"]
                .into_iter()
                .map(String::from)
                .collect::<BTreeSet<_>>(),
        })),
    );
    sections.insert(
        "memory".to_owned(),
        Section::available(SectionData::Memory(MemoryData {
            installed_bytes: 17_179_869_184,
        })),
    );
    sections.insert(
        "virtualization".to_owned(),
        Section::available(SectionData::Virtualization(VirtualizationData {
            category: VirtualizationCategory::Physical,
        })),
    );
    Snapshot {
        schema: SchemaIdentifier::current(),
        producer: ProducerInfo {
            name: "envshot".to_owned(),
            version: "0.1.0".to_owned(),
        },
        capture: CaptureMetadata {
            profile: CaptureProfile::Standard,
            privacy: PrivacyProfile::Standard,
        },
        sections,
    }
}

fn measure(name: &str, iterations: u32, run: impl Fn()) -> std::time::Duration {
    #[allow(clippy::let_unit_value)]
    let warm = run();
    std::hint::black_box(warm);
    let start = Instant::now();
    for _ in 0..iterations {
        #[allow(clippy::unit_arg)]
        std::hint::black_box(run());
    }
    let per_iter = start.elapsed() / iterations;
    println!("{name:<28} {iterations:>5} iters  {per_iter:>10.2?} / iter");
    per_iter
}

fn main() {
    println!("envshot performance baseline (host-dependent, not a CI gate)\n");
    let snapshot = typical_snapshot();
    let normalized = normalize(&snapshot);
    let projected = apply_privacy(&normalized, &PrivacyPolicy::standard());

    measure("normalize", 10_000, || {
        let normalized = normalize(&snapshot);
        std::hint::black_box(normalized);
    });
    measure("privacy (standard)", 10_000, || {
        let projected = apply_privacy(&normalized, &PrivacyPolicy::standard());
        std::hint::black_box(projected);
    });
    measure("diff (self)", 10_000, || {
        let differences = diff(&normalized, &projected, &DiffPolicy::compatibility());
        std::hint::black_box(differences);
    });
    measure("fingerprint", 10_000, || {
        let fp = fingerprint(&projected, &envshot_core::FingerprintProfile::ReproV1)
            .expect("supported major");
        std::hint::black_box(fp);
    });
    measure("total pipeline", 10_000, || {
        let raw = typical_snapshot();
        let result = apply_privacy(&normalize(&raw), &PrivacyPolicy::standard());
        std::hint::black_box(result);
    });

    let serialized = serde_json::to_string(&projected).expect("must serialize");
    println!(
        "\ntypical snapshot size: {} bytes (compact JSON)",
        serialized.len()
    );

    let start = Instant::now();
    let captured = envshot::capture(envshot_model::CaptureProfile::Standard);
    let captured = apply_privacy(&normalize(&captured), &PrivacyPolicy::standard());
    println!(
        "total standard capture (real host): {:?} ({} sections)",
        start.elapsed(),
        captured.sections.len()
    );
}
