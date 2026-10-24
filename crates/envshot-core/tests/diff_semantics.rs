use std::collections::BTreeSet;

use envshot_core::{DiffPolicy, diff};
use envshot_model::{
    Architecture, ArchitectureData, CaptureMetadata, CaptureProfile, CpuData, LocaleData,
    MemoryData, OsData, OsFamily, PrivacyProfile, ProducerInfo, SchemaIdentifier, Section,
    SectionData, Snapshot, VersionValue, VirtualizationCategory, VirtualizationData,
};

fn snapshot(sections: Vec<(&'static str, SectionData)>) -> Snapshot {
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

fn memory_data(bytes: u64) -> SectionData {
    SectionData::Memory(MemoryData {
        installed_bytes: bytes,
    })
}

#[test]
fn identical_snapshots_diff_to_empty() {
    let a = snapshot(vec![("memory", memory_data(1024))]);
    let b = a.clone();
    assert!(diff(&a, &b, &DiffPolicy::compatibility()).is_empty());
}

#[test]
fn scalar_change_yields_one_entry() {
    let a = snapshot(vec![("memory", memory_data(1024))]);
    let b = snapshot(vec![("memory", memory_data(2048))]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(diff.changes().len(), 1);
    let change = &diff.changes()[0];
    assert_eq!(change.path, "memory.installed_bytes");
    assert!(change.is_changed());
    assert_eq!(change.before.as_deref(), Some("1024"));
    assert_eq!(change.after.as_deref(), Some("2048"));
}

#[test]
fn status_transition_is_explicit() {
    let mut a = snapshot(vec![("cpu", cpu_data())]);
    let mut b = a.clone();
    b.sections.insert(
        "cpu".to_owned(),
        Section::<SectionData>::unavailable(vec![]),
    );
    let _ = &mut a;

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(
        diff.changes().len(),
        1,
        "exactly the status change: {diff:?}"
    );
    assert_eq!(diff.changes()[0].path, "cpu.status");
    assert!(diff.changes()[0].is_status_changed());
}

fn cpu_data() -> SectionData {
    SectionData::Cpu(CpuData {
        model: Some("Fixture CPU".to_owned()),
        logical_count: Some(8),
        physical_count: Some(4),
        features: BTreeSet::from(["avx2".to_owned(), "sse2".to_owned()]),
    })
}

#[test]
fn set_permutation_is_not_a_change() {
    let a = snapshot(vec![("cpu", cpu_data())]);
    let mut b = snapshot(vec![("cpu", cpu_data())]);

    b.sections.insert(
        "cpu".to_owned(),
        Section::available(SectionData::Cpu(CpuData {
            model: Some("Fixture CPU".to_owned()),
            logical_count: Some(8),
            physical_count: Some(4),
            features: BTreeSet::from(["sse2".to_owned(), "avx2".to_owned()]),
        })),
    );

    assert!(diff(&a, &b, &DiffPolicy::compatibility()).is_empty());
}

#[test]
fn set_membership_changes_are_reported_sorted() {
    let a = snapshot(vec![("cpu", cpu_data())]);
    let b = snapshot(vec![(
        "cpu",
        SectionData::Cpu(CpuData {
            model: Some("Fixture CPU".to_owned()),
            logical_count: Some(8),
            physical_count: Some(4),
            features: BTreeSet::from([
                "avx2".to_owned(),
                "avx512f".to_owned(),
                "sse4_2".to_owned(),
            ]),
        }),
    )]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(diff.changes().len(), 1);
    let change = &diff.changes()[0];
    assert_eq!(change.path, "cpu.features");
    assert!(change.is_set_changed());
    assert_eq!(change.before.as_deref(), Some("sse2"));
    assert_eq!(change.after.as_deref(), Some("avx512f, sse4_2"));
}

#[test]
fn equal_normalized_versions_are_not_a_change() {
    let a = snapshot(vec![("os", os_with_version("10.0.1"))]);
    let b = snapshot(vec![("os", os_with_version("10.0.1+hotfix"))]);

    assert!(diff(&a, &b, &DiffPolicy::compatibility()).is_empty());
}

#[test]
fn version_change_displays_raw_texts() {
    let a = snapshot(vec![("os", os_with_version("10.0.1"))]);
    let b = snapshot(vec![("os", os_with_version("10.0.2"))]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(diff.changes().len(), 1);
    assert_eq!(diff.changes()[0].path, "os.version");
    assert_eq!(diff.changes()[0].before.as_deref(), Some("10.0.1"));
    assert_eq!(diff.changes()[0].after.as_deref(), Some("10.0.2"));
}

#[test]
fn unparsable_versions_fall_back_to_raw() {
    let a = snapshot(vec![("os", os_with_version("unknown-a"))]);
    let b = snapshot(vec![("os", os_with_version("unknown-b"))]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(diff.changes().len(), 1);
    assert_eq!(diff.changes()[0].path, "os.version");
}

fn os_with_version(raw: &str) -> SectionData {
    SectionData::Os(OsData {
        family: OsFamily::Windows,
        version: VersionValue::new(raw),
        build: None,
    })
}

#[test]
fn added_and_removed_sections_are_reported() {
    let a = snapshot(vec![]);
    let b = snapshot(vec![("memory", memory_data(1024))]);

    let added = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(added.changes().len(), 1);
    assert_eq!(added.changes()[0].path, "memory");
    assert!(added.changes()[0].is_added());

    let removed = diff(&b, &a, &DiffPolicy::compatibility());
    assert_eq!(removed.changes().len(), 1);
    assert!(removed.changes()[0].is_removed());
}

#[test]
fn changes_are_sorted_by_path() {
    let a = snapshot(vec![("memory", memory_data(1))]);
    let b = snapshot(vec![
        ("memory", memory_data(2)),
        ("os", os_with_version("10.0.1")),
        ("cpu", cpu_data()),
    ]);

    let diff = diff(&a, &b, &DiffPolicy::diagnostic());
    let paths: Vec<&str> = diff.changes().iter().map(|c| c.path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(paths, sorted);
    assert!(paths.contains(&"os"));
    assert!(paths.contains(&"cpu"));
}

#[test]
fn metadata_changes_follow_the_policy() {
    let a = snapshot(vec![]);
    let mut b = snapshot(vec![]);
    b.producer.version = "0.0.1".to_owned();
    b.capture.profile = CaptureProfile::Minimal;
    b.capture.privacy = PrivacyProfile::Strict;

    assert!(
        diff(&a, &b, &DiffPolicy::compatibility()).is_empty(),
        "metadata-only changes produce no compatibility diff"
    );

    let diagnostic = diff(&a, &b, &DiffPolicy::diagnostic());
    assert!(diagnostic.changes().len() >= 3);
}

#[test]
fn partial_and_unsupported_states_diff_without_panics() {
    let a = snapshot(vec![(
        "virtualization",
        SectionData::Virtualization(VirtualizationData {
            category: VirtualizationCategory::Physical,
        }),
    )]);
    let mut b = a.clone();
    b.sections.insert(
        "virtualization".to_owned(),
        Section::<SectionData>::unsupported(Vec::new()),
    );
    b.sections.insert(
        "architecture".to_owned(),
        Section::available(SectionData::Architecture(ArchitectureData {
            process: Architecture::X64,
            os: Architecture::Arm64,
        })),
    );

    let diff = diff(&a, &b, &DiffPolicy::diagnostic());
    assert!(!diff.is_empty());
}

#[test]
fn same_status_data_changes_are_reported_for_every_section_type() {
    let a = snapshot(vec![
        (
            "architecture",
            SectionData::Architecture(ArchitectureData {
                process: Architecture::X86,
                os: Architecture::X64,
            }),
        ),
        (
            "locale",
            SectionData::Locale(LocaleData {
                languages: vec!["en-US".to_owned()],
                timezone: Some("Etc/UTC".to_owned()),
            }),
        ),
        (
            "virtualization",
            SectionData::Virtualization(VirtualizationData {
                category: VirtualizationCategory::Physical,
            }),
        ),
    ]);
    let b = snapshot(vec![
        (
            "architecture",
            SectionData::Architecture(ArchitectureData {
                process: Architecture::X64,
                os: Architecture::X64,
            }),
        ),
        (
            "locale",
            SectionData::Locale(LocaleData {
                languages: vec!["de-DE".to_owned()],
                timezone: Some("Europe/Berlin".to_owned()),
            }),
        ),
        (
            "virtualization",
            SectionData::Virtualization(VirtualizationData {
                category: VirtualizationCategory::VirtualMachine,
            }),
        ),
    ]);

    let compatibility = diff(&a, &b, &DiffPolicy::compatibility());
    let compat_paths: Vec<&str> = compatibility
        .changes()
        .iter()
        .map(|c| c.path.as_str())
        .collect();
    assert_eq!(
        compat_paths,
        ["architecture.process", "virtualization.category"]
    );

    let diagnostic = diff(&a, &b, &DiffPolicy::diagnostic());
    let paths: Vec<&str> = diagnostic
        .changes()
        .iter()
        .map(|c| c.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "architecture.process",
            "locale.languages",
            "locale.timezone",
            "virtualization.category"
        ]
    );
}

#[test]
fn change_kind_predicates_are_exclusive() {
    let a = snapshot(vec![("memory", memory_data(1))]);
    let b = snapshot(vec![("memory", memory_data(2))]);
    let empty = snapshot(vec![]);
    let removed = diff(&b, &empty, &DiffPolicy::compatibility());
    let added = diff(&empty, &b, &DiffPolicy::compatibility());
    let status_base = snapshot(vec![("memory", memory_data(1)), ("cpu", cpu_data())]);
    let mut with_status = status_base.clone();
    with_status.sections.insert(
        "cpu".to_owned(),
        Section::<SectionData>::unavailable(Vec::new()),
    );
    let status = diff(&status_base, &with_status, &DiffPolicy::compatibility());

    let changed_diff = diff(&a, &b, &DiffPolicy::compatibility());
    let changed = &changed_diff.changes()[0];
    assert!(changed.is_changed());
    assert!(
        !changed.is_added()
            && !changed.is_removed()
            && !changed.is_status_changed()
            && !changed.is_set_changed()
    );

    assert!(added.changes()[0].is_added());
    assert!(removed.changes()[0].is_removed());
    assert!(status.changes()[0].is_status_changed());

    let cpu_base = snapshot(vec![("cpu", cpu_data())]);
    let mut c = cpu_base.clone();
    c.sections.insert(
        "cpu".to_owned(),
        Section::available(SectionData::Cpu(CpuData {
            model: None,
            logical_count: None,
            physical_count: None,
            features: BTreeSet::from(["avx512f".to_owned()]),
        })),
    );
    let set_diff = diff(&cpu_base, &c, &DiffPolicy::compatibility());
    let set_change = set_diff
        .changes()
        .iter()
        .find(|change| change.path == "cpu.features")
        .expect("features change");
    assert!(set_change.is_set_changed());
    assert!(!set_change.is_changed());
}

#[test]
fn option_scalar_changes_are_reported() {
    let a = snapshot(vec![("cpu", cpu_data())]);
    let b = snapshot(vec![(
        "cpu",
        SectionData::Cpu(CpuData {
            model: Some("Fixture CPU".to_owned()),
            logical_count: Some(16),
            physical_count: None,
            features: BTreeSet::from(["avx2".to_owned(), "sse2".to_owned()]),
        }),
    )]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    let paths: Vec<&str> = diff.changes().iter().map(|c| c.path.as_str()).collect();
    assert_eq!(paths, ["cpu.logical_count", "cpu.physical_count"]);
    let logical = diff
        .changes()
        .iter()
        .find(|c| c.path == "cpu.logical_count")
        .expect("logical change");
    assert_eq!(logical.before.as_deref(), Some("8"));
    assert_eq!(logical.after.as_deref(), Some("16"));
    let physical = diff
        .changes()
        .iter()
        .find(|c| c.path == "cpu.physical_count")
        .expect("physical change");
    assert_eq!(physical.before.as_deref(), Some("4"));
    assert_eq!(physical.after, None, "removed values render as absent");
}

#[test]
fn optional_build_field_uses_version_aware_comparison() {
    let a = snapshot(vec![("os", os_with_version("10.0.1"))]);
    let b = snapshot(vec![(
        "os",
        SectionData::Os(OsData {
            family: OsFamily::Windows,
            version: VersionValue::new("10.0.1"),
            build: Some(VersionValue::new("26100")),
        }),
    )]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(diff.changes().len(), 1);
    assert_eq!(diff.changes()[0].path, "os.build");
    assert_eq!(diff.changes()[0].after.as_deref(), Some("26100"));
}

#[test]
fn pure_addition_set_changes_are_reported() {
    let a = snapshot(vec![("cpu", cpu_data())]);
    let b = snapshot(vec![(
        "cpu",
        SectionData::Cpu(CpuData {
            model: Some("Fixture CPU".to_owned()),
            logical_count: Some(8),
            physical_count: Some(4),
            features: BTreeSet::from(["avx2".to_owned(), "sse2".to_owned(), "avx512f".to_owned()]),
        }),
    )]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(diff.changes().len(), 1);
    assert_eq!(diff.changes()[0].path, "cpu.features");
    assert_eq!(diff.changes()[0].after.as_deref(), Some("avx512f"));
    assert_eq!(diff.changes()[0].before.as_deref(), Some(""));
}

#[test]
fn present_build_values_compare_via_components() {
    let a = snapshot(vec![(
        "os",
        SectionData::Os(OsData {
            family: OsFamily::Windows,
            version: VersionValue::new("10.0.1"),
            build: Some(VersionValue::new("26100")),
        }),
    )]);
    let b = snapshot(vec![(
        "os",
        SectionData::Os(OsData {
            family: OsFamily::Windows,
            version: VersionValue::new("10.0.1"),
            build: Some(VersionValue::new("26101")),
        }),
    )]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    assert_eq!(diff.changes().len(), 1);
    assert_eq!(diff.changes()[0].path, "os.build");
    assert_eq!(diff.changes()[0].before.as_deref(), Some("26100"));
    assert_eq!(diff.changes()[0].after.as_deref(), Some("26101"));
}

#[test]
fn windows_data_changes_are_reported() {
    let a = snapshot(vec![(
        "windows",
        SectionData::Windows(envshot_model::WindowsData {
            edition: Some("Professional".to_owned()),
            display_version: Some("23H2".to_owned()),
            ubr: Some(6199),
        }),
    )]);
    let b = snapshot(vec![(
        "windows",
        SectionData::Windows(envshot_model::WindowsData {
            edition: Some("Professional".to_owned()),
            display_version: Some("24H2".to_owned()),
            ubr: Some(7100),
        }),
    )]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    let paths: Vec<&str> = diff.changes().iter().map(|c| c.path.as_str()).collect();
    assert_eq!(paths, ["windows.display_version", "windows.ubr"]);
}

#[test]
fn gpu_keyed_list_diff_joins_by_index_and_reports_membership() {
    let a = snapshot(vec![(
        "gpu",
        SectionData::Gpu(vec![gpu_adapter(
            0,
            "Old GPU",
            0x10DE,
            0x1234,
            Some("32.0.15.60211"),
        )]),
    )]);
    let b = snapshot(vec![(
        "gpu",
        SectionData::Gpu(vec![
            gpu_adapter(0, "Old GPU", 0x10DE, 0x1234, Some("32.0.16.1088")),
            gpu_adapter(1, "New GPU", 0x8086, 0x5678, None),
        ]),
    )]);

    let forward = diff(&a, &b, &DiffPolicy::compatibility());
    let paths: Vec<&str> = forward.changes().iter().map(|c| c.path.as_str()).collect();
    assert_eq!(paths, ["gpu.adapters[0].driver_version", "gpu.adapters[1]"]);
    assert!(forward.changes()[0].is_changed());
    assert!(forward.changes()[1].is_added());

    let reversed = diff(&b, &a, &DiffPolicy::compatibility());
    assert!(
        reversed
            .changes()
            .iter()
            .any(|c| c.path == "gpu.adapters[1]" && c.is_removed())
    );
}

fn gpu_adapter(
    index: u32,
    name: &str,
    vendor_id: u32,
    device_id: u32,
    driver: Option<&str>,
) -> envshot_model::GpuData {
    envshot_model::GpuData {
        index,
        name: name.to_owned(),
        vendor_id,
        device_id,
        driver_version: driver.map(VersionValue::new),
    }
}

#[test]
fn distro_data_changes_are_reported() {
    let a = snapshot(vec![(
        "distro",
        SectionData::Distro(envshot_model::DistroData {
            name: Some("Fedora Linux".to_owned()),
            version: Some(VersionValue::new("42")),
        }),
    )]);
    let b = snapshot(vec![(
        "distro",
        SectionData::Distro(envshot_model::DistroData {
            name: Some("Fedora Linux".to_owned()),
            version: Some(VersionValue::new("43")),
        }),
    )]);

    let diff = diff(&a, &b, &DiffPolicy::compatibility());
    let paths: Vec<&str> = diff.changes().iter().map(|c| c.path.as_str()).collect();
    assert_eq!(paths, ["distro.version"]);
}

#[test]
fn equal_normalized_build_versions_are_not_a_change() {
    let a = snapshot(vec![(
        "os",
        SectionData::Os(OsData {
            family: OsFamily::Windows,
            version: VersionValue::new("10.0.1"),
            build: Some(VersionValue::new("26100")),
        }),
    )]);
    let b = snapshot(vec![(
        "os",
        SectionData::Os(OsData {
            family: OsFamily::Windows,
            version: VersionValue::new("10.0.1"),
            build: Some(VersionValue::new("26100+x")),
        }),
    )]);

    assert!(diff(&a, &b, &DiffPolicy::compatibility()).is_empty());
}
