use std::collections::BTreeSet;

use serde::Serialize;

use envshot_model::{
    ArchitectureData, CpuData, LocaleData, MemoryData, OsData, Section, SectionData, Snapshot,
    VersionValue, VirtualizationData,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,

    Removed,

    Changed,

    SetChanged,

    StatusChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Relevance {
    Compatibility,

    Diagnostic,

    Volatile,

    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Change {
    pub path: String,

    pub kind: ChangeKind,

    pub before: Option<String>,

    pub after: Option<String>,

    pub relevance: Relevance,
}

impl Change {
    #[must_use]
    pub fn is_changed(&self) -> bool {
        self.kind == ChangeKind::Changed
    }

    #[must_use]
    pub fn is_status_changed(&self) -> bool {
        self.kind == ChangeKind::StatusChanged
    }

    #[must_use]
    pub fn is_set_changed(&self) -> bool {
        self.kind == ChangeKind::SetChanged
    }

    #[must_use]
    pub fn is_added(&self) -> bool {
        self.kind == ChangeKind::Added
    }

    #[must_use]
    pub fn is_removed(&self) -> bool {
        self.kind == ChangeKind::Removed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotDiff {
    changes: Vec<Change>,
}

impl SnapshotDiff {
    #[must_use]
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffPolicy {
    include_diagnostic: bool,
}

impl DiffPolicy {
    #[must_use]
    pub fn compatibility() -> Self {
        Self {
            include_diagnostic: false,
        }
    }

    #[must_use]
    pub fn diagnostic() -> Self {
        Self {
            include_diagnostic: true,
        }
    }

    fn keeps(self, relevance: Relevance) -> bool {
        match relevance {
            Relevance::Compatibility => true,
            Relevance::Diagnostic => self.include_diagnostic,
            Relevance::Volatile | Relevance::Unknown => false,
        }
    }
}

#[must_use]
pub fn diff(a: &Snapshot, b: &Snapshot, policy: &DiffPolicy) -> SnapshotDiff {
    let mut changes: Vec<Change> = Vec::new();

    metadata_changes(a, b, &mut changes);
    section_changes(a, b, &mut changes);

    changes.retain(|change| policy.keeps(change.relevance));
    changes.sort();
    SnapshotDiff { changes }
}

fn metadata_changes(a: &Snapshot, b: &Snapshot, out: &mut Vec<Change>) {
    if a.producer.version != b.producer.version {
        out.push(Change {
            path: "producer.version".to_owned(),
            kind: ChangeKind::Changed,
            before: Some(a.producer.version.clone()),
            after: Some(b.producer.version.clone()),
            relevance: Relevance::Diagnostic,
        });
    }
    if a.capture.profile != b.capture.profile {
        out.push(Change {
            path: "capture.profile".to_owned(),
            kind: ChangeKind::Changed,
            before: Some(serde_display(&a.capture.profile)),
            after: Some(serde_display(&b.capture.profile)),
            relevance: Relevance::Diagnostic,
        });
    }
    if a.capture.privacy != b.capture.privacy {
        out.push(Change {
            path: "capture.privacy".to_owned(),
            kind: ChangeKind::Changed,
            before: Some(serde_display(&a.capture.privacy)),
            after: Some(serde_display(&b.capture.privacy)),
            relevance: Relevance::Diagnostic,
        });
    }
}

fn section_changes(a: &Snapshot, b: &Snapshot, out: &mut Vec<Change>) {
    for (id, section_a) in &a.sections {
        match b.sections.get(id) {
            None => out.push(Change {
                path: id.clone(),
                kind: ChangeKind::Removed,
                before: None,
                after: None,
                relevance: Relevance::Compatibility,
            }),
            Some(section_b) => diff_section(id, section_a, section_b, out),
        }
    }
    for id in b.sections.keys() {
        if !a.sections.contains_key(id) {
            out.push(Change {
                path: id.clone(),
                kind: ChangeKind::Added,
                before: None,
                after: None,
                relevance: Relevance::Compatibility,
            });
        }
    }
}

fn diff_section(
    id: &str,
    a: &Section<SectionData>,
    b: &Section<SectionData>,
    out: &mut Vec<Change>,
) {
    if a.status() != b.status() {
        out.push(Change {
            path: format!("{id}.status"),
            kind: ChangeKind::StatusChanged,
            before: Some(serde_display(&a.status())),
            after: Some(serde_display(&b.status())),
            relevance: Relevance::Compatibility,
        });

        return;
    }
    if let (Some(data_a), Some(data_b)) = (a.data(), b.data()) {
        diff_data(id, data_a, data_b, out);
    }
}

fn diff_data(id: &str, a: &SectionData, b: &SectionData, out: &mut Vec<Change>) {
    match (a, b) {
        (SectionData::Os(a), SectionData::Os(b)) => diff_os(id, a, b, out),
        (SectionData::Architecture(a), SectionData::Architecture(b)) => {
            diff_architecture(id, a, b, out);
        }
        (SectionData::Cpu(a), SectionData::Cpu(b)) => diff_cpu(id, a, b, out),
        (SectionData::Memory(a), SectionData::Memory(b)) => diff_memory(id, a, b, out),
        (SectionData::Locale(a), SectionData::Locale(b)) => diff_locale(id, a, b, out),
        (SectionData::Virtualization(a), SectionData::Virtualization(b)) => {
            diff_virtualization(id, a, b, out);
        }
        (SectionData::Windows(a), SectionData::Windows(b)) => {
            diff_windows(id, a, b, out);
        }
        (SectionData::Gpu(a), SectionData::Gpu(b)) => {
            diff_gpu(id, a, b, out);
        }
        (SectionData::Distro(a), SectionData::Distro(b)) => {
            diff_distro(id, a, b, out);
        }

        _ => {}
    }
}

fn diff_os(id: &str, a: &OsData, b: &OsData, out: &mut Vec<Change>) {
    scalar(id, "family", &a.family, &b.family, out);
    version(id, "version", &a.version, &b.version, out);
    version_option(id, "build", a.build.as_ref(), b.build.as_ref(), out);
}

fn diff_architecture(id: &str, a: &ArchitectureData, b: &ArchitectureData, out: &mut Vec<Change>) {
    scalar(id, "process", &a.process, &b.process, out);
    scalar(id, "os", &a.os, &b.os, out);
}

fn diff_cpu(id: &str, a: &CpuData, b: &CpuData, out: &mut Vec<Change>) {
    scalar_option(id, "model", a.model.as_ref(), b.model.as_ref(), out);
    scalar_option(
        id,
        "logical_count",
        a.logical_count.as_ref(),
        b.logical_count.as_ref(),
        out,
    );
    scalar_option(
        id,
        "physical_count",
        a.physical_count.as_ref(),
        b.physical_count.as_ref(),
        out,
    );
    set_changes(id, "features", &a.features, &b.features, out);
}

fn diff_memory(id: &str, a: &MemoryData, b: &MemoryData, out: &mut Vec<Change>) {
    scalar(
        id,
        "installed_bytes",
        &a.installed_bytes,
        &b.installed_bytes,
        out,
    );
}

fn diff_locale(id: &str, a: &LocaleData, b: &LocaleData, out: &mut Vec<Change>) {
    if a.languages != b.languages {
        out.push(Change {
            path: format!("{id}.languages"),
            kind: ChangeKind::Changed,
            before: Some(a.languages.join(", ")),
            after: Some(b.languages.join(", ")),
            relevance: Relevance::Diagnostic,
        });
    }

    scalar_option_with(
        id,
        "timezone",
        a.timezone.as_ref(),
        b.timezone.as_ref(),
        Relevance::Diagnostic,
        out,
    );
}

pub(crate) fn keyed_list_diff<T>(
    id: &str,
    field: &str,
    a: &[T],
    b: &[T],
    key: impl Fn(&T) -> String,
    compare: impl Fn(&T, &T, &mut Vec<Change>),
    out: &mut Vec<Change>,
) {
    for member_a in a {
        let key_a = key(member_a);
        match b.iter().find(|member_b| key(member_b) == key_a) {
            None => out.push(Change {
                path: format!("{id}.{field}[{key_a}]"),
                kind: ChangeKind::Removed,
                before: None,
                after: None,
                relevance: Relevance::Compatibility,
            }),
            Some(member_b) => compare(member_a, member_b, out),
        }
    }
    for member_b in b {
        let key_b = key(member_b);
        if !a.iter().any(|member_a| key(member_a) == key_b) {
            out.push(Change {
                path: format!("{id}.{field}[{key_b}]"),
                kind: ChangeKind::Added,
                before: None,
                after: None,
                relevance: Relevance::Compatibility,
            });
        }
    }
}

fn diff_gpu(
    id: &str,
    a: &[envshot_model::GpuData],
    b: &[envshot_model::GpuData],
    out: &mut Vec<Change>,
) {
    keyed_list_diff(
        id,
        "adapters",
        a,
        b,
        |adapter| adapter.index.to_string(),
        |a, b, out| {
            let member = format!("{id}.adapters[{}]", a.index);
            scalar(&member, "name", &a.name, &b.name, out);
            scalar(&member, "vendor_id", &a.vendor_id, &b.vendor_id, out);
            scalar(&member, "device_id", &a.device_id, &b.device_id, out);
            version_option(
                &member,
                "driver_version",
                a.driver_version.as_ref(),
                b.driver_version.as_ref(),
                out,
            );
        },
        out,
    );
}

fn diff_distro(
    id: &str,
    a: &envshot_model::DistroData,
    b: &envshot_model::DistroData,
    out: &mut Vec<Change>,
) {
    scalar_option(id, "name", a.name.as_ref(), b.name.as_ref(), out);
    version_option(id, "version", a.version.as_ref(), b.version.as_ref(), out);
}

fn diff_windows(
    id: &str,
    a: &envshot_model::WindowsData,
    b: &envshot_model::WindowsData,
    out: &mut Vec<Change>,
) {
    scalar_option(id, "edition", a.edition.as_ref(), b.edition.as_ref(), out);
    scalar_option(
        id,
        "display_version",
        a.display_version.as_ref(),
        b.display_version.as_ref(),
        out,
    );
    scalar_option(id, "ubr", a.ubr.as_ref(), b.ubr.as_ref(), out);
}

fn diff_virtualization(
    id: &str,
    a: &VirtualizationData,
    b: &VirtualizationData,
    out: &mut Vec<Change>,
) {
    scalar(id, "category", &a.category, &b.category, out);
}

fn scalar<T: PartialEq + serde::Serialize>(
    id: &str,
    field: &str,
    a: &T,
    b: &T,
    out: &mut Vec<Change>,
) {
    if a != b {
        out.push(Change {
            path: format!("{id}.{field}"),
            kind: ChangeKind::Changed,
            before: Some(serde_display(a)),
            after: Some(serde_display(b)),
            relevance: Relevance::Compatibility,
        });
    }
}

fn serde_display<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("closed-vocabulary serialization cannot fail")
        .trim_matches('"')
        .to_owned()
}

fn scalar_option<T: PartialEq + serde::Serialize>(
    id: &str,
    field: &str,
    a: Option<&T>,
    b: Option<&T>,
    out: &mut Vec<Change>,
) {
    scalar_option_with(id, field, a, b, Relevance::Compatibility, out);
}

fn scalar_option_with<T: PartialEq + serde::Serialize>(
    id: &str,
    field: &str,
    a: Option<&T>,
    b: Option<&T>,
    relevance: Relevance,
    out: &mut Vec<Change>,
) {
    if a != b {
        out.push(Change {
            path: format!("{id}.{field}"),
            kind: ChangeKind::Changed,
            before: a.map(serde_display),
            after: b.map(serde_display),
            relevance,
        });
    }
}

fn version(id: &str, field: &str, a: &VersionValue, b: &VersionValue, out: &mut Vec<Change>) {
    if version_changed(a, b) {
        out.push(Change {
            path: format!("{id}.{field}"),
            kind: ChangeKind::Changed,
            before: Some(a.raw().to_owned()),
            after: Some(b.raw().to_owned()),
            relevance: Relevance::Compatibility,
        });
    }
}

fn version_option(
    id: &str,
    field: &str,
    a: Option<&VersionValue>,
    b: Option<&VersionValue>,
    out: &mut Vec<Change>,
) {
    match (a, b) {
        (Some(a), Some(b)) => version(id, field, a, b, out),
        (a, b) if a != b => out.push(Change {
            path: format!("{id}.{field}"),
            kind: ChangeKind::Changed,
            before: a.map(|value| value.raw().to_owned()),
            after: b.map(|value| value.raw().to_owned()),
            relevance: Relevance::Compatibility,
        }),
        _ => {}
    }
}

fn version_changed(a: &VersionValue, b: &VersionValue) -> bool {
    match (a.normalized(), b.normalized()) {
        (Some(x), Some(y)) => x != y,
        _ => a.raw() != b.raw(),
    }
}

fn set_changes(
    id: &str,
    field: &str,
    a: &BTreeSet<String>,
    b: &BTreeSet<String>,
    out: &mut Vec<Change>,
) {
    let added: Vec<&String> = b.difference(a).collect();
    let removed: Vec<&String> = a.difference(b).collect();
    if added.is_empty() && removed.is_empty() {
        return;
    }
    out.push(Change {
        path: format!("{id}.{field}"),
        kind: ChangeKind::SetChanged,
        before: Some(removed.into_iter().cloned().collect::<Vec<_>>().join(", ")),
        after: Some(added.into_iter().cloned().collect::<Vec<_>>().join(", ")),
        relevance: Relevance::Compatibility,
    });
}
