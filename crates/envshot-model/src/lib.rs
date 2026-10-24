use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaIdentifier {
    pub name: String,

    pub major: u32,

    pub minor: u32,
}

impl SchemaIdentifier {
    #[must_use]
    pub fn current() -> Self {
        Self {
            name: "envshot.snapshot".to_owned(),
            major: 0,
            minor: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProducerInfo {
    pub name: String,

    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureProfile {
    EmptyTest,

    Minimal,

    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureMetadata {
    pub profile: CaptureProfile,

    #[serde(default)]
    pub privacy: PrivacyProfile,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyProfile {
    #[default]
    Standard,
    Strict,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionStatus {
    Available,

    Partial,

    Unsupported,

    PermissionDenied,

    Unavailable,

    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct IssueCode(String);

impl IssueCode {
    pub fn new(code: impl Into<String>) -> Result<Self, InvalidIssueCode> {
        let code = code.into();
        if code.is_empty() {
            return Err(InvalidIssueCode);
        }
        Ok(Self(code))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IssueCode {
    type Error = InvalidIssueCode;

    fn try_from(code: String) -> Result<Self, Self::Error> {
        IssueCode::new(code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid issue code: code must not be empty")]
pub struct InvalidIssueCode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectorIssue {
    pub code: IssueCode,

    pub detail: Option<String>,
}

impl CollectorIssue {
    #[must_use]
    pub fn new(code: IssueCode) -> Self {
        Self { code, detail: None }
    }

    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "WireSection<T>")]
pub struct Section<T> {
    status: SectionStatus,
    data: Option<T>,
    provenance: Vec<Provenance>,
    issues: Vec<CollectorIssue>,
}

#[derive(Debug, Deserialize)]
struct WireSection<T> {
    status: SectionStatus,
    data: Option<T>,
    #[serde(default)]
    provenance: Vec<Provenance>,
    #[serde(default)]
    issues: Vec<CollectorIssue>,
}

impl<T> TryFrom<WireSection<T>> for Section<T> {
    type Error = SectionError;

    fn try_from(wire: WireSection<T>) -> Result<Self, Self::Error> {
        Ok(Section::new(wire.status, wire.data, wire.issues)?.with_provenance(wire.provenance))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SectionError {
    #[error("section status `{status:?}` requires data")]
    DataRequired { status: SectionStatus },

    #[error("partial section requires at least one issue")]
    IssueRequired,
}

impl<T> Section<T> {
    #[must_use]
    pub fn available(data: T) -> Self {
        Self {
            status: SectionStatus::Available,
            data: Some(data),
            provenance: Vec::new(),
            issues: Vec::new(),
        }
    }

    pub fn partial(data: T, issues: Vec<CollectorIssue>) -> Result<Self, SectionError> {
        Self::new(SectionStatus::Partial, Some(data), issues)
    }

    #[must_use]
    pub fn unsupported(issues: Vec<CollectorIssue>) -> Self {
        Self {
            status: SectionStatus::Unsupported,
            data: None,
            provenance: Vec::new(),
            issues,
        }
    }

    #[must_use]
    pub fn permission_denied(issues: Vec<CollectorIssue>) -> Self {
        Self {
            status: SectionStatus::PermissionDenied,
            data: None,
            provenance: Vec::new(),
            issues,
        }
    }

    #[must_use]
    pub fn unavailable(issues: Vec<CollectorIssue>) -> Self {
        Self {
            status: SectionStatus::Unavailable,
            data: None,
            provenance: Vec::new(),
            issues,
        }
    }

    #[must_use]
    pub fn failed(issues: Vec<CollectorIssue>) -> Self {
        Self {
            status: SectionStatus::Failed,
            data: None,
            provenance: Vec::new(),
            issues,
        }
    }

    pub fn new(
        status: SectionStatus,
        data: Option<T>,
        issues: Vec<CollectorIssue>,
    ) -> Result<Self, SectionError> {
        match status {
            SectionStatus::Available | SectionStatus::Partial if data.is_none() => {
                return Err(SectionError::DataRequired { status });
            }
            SectionStatus::Partial if issues.is_empty() => {
                return Err(SectionError::IssueRequired);
            }
            _ => {}
        }
        Ok(Self {
            status,
            data,
            provenance: Vec::new(),
            issues,
        })
    }

    #[must_use]
    pub fn status(&self) -> SectionStatus {
        self.status
    }

    #[must_use]
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    #[must_use]
    pub fn issues(&self) -> &[CollectorIssue] {
        &self.issues
    }

    #[must_use]
    pub fn provenance(&self) -> &[Provenance] {
        &self.provenance
    }

    #[must_use]
    pub fn with_provenance(mut self, provenance: Vec<Provenance>) -> Self {
        self.provenance = normalize_provenance(provenance);
        self
    }
}

fn normalize_provenance(mut provenance: Vec<Provenance>) -> Vec<Provenance> {
    provenance.sort();
    provenance.dedup();
    provenance
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    NativeApi,

    SystemFile,

    KnownCommand,

    LibraryAdapter,

    Derived,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Provenance {
    collector_id: String,
    revision: u32,
    source_kind: SourceKind,
    notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid provenance: collector id must not be empty")]
pub struct InvalidProvenance;

impl Provenance {
    pub fn new(
        collector_id: impl Into<String>,
        revision: u32,
        source_kind: SourceKind,
    ) -> Result<Self, InvalidProvenance> {
        let collector_id = collector_id.into();
        if collector_id.is_empty() {
            return Err(InvalidProvenance);
        }
        Ok(Self {
            collector_id,
            revision,
            source_kind,
            notes: None,
        })
    }

    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    #[must_use]
    pub fn collector_id(&self) -> &str {
        &self.collector_id
    }

    #[must_use]
    pub fn revision(&self) -> u32 {
        self.revision
    }

    #[must_use]
    pub fn source_kind(&self) -> SourceKind {
        self.source_kind
    }

    #[must_use]
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema: SchemaIdentifier,

    pub producer: ProducerInfo,

    pub capture: CaptureMetadata,

    #[serde(deserialize_with = "deserialize_sections")]
    pub sections: BTreeMap<String, Section<SectionData>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SectionData {
    Os(OsData),

    Architecture(ArchitectureData),

    Cpu(CpuData),

    Memory(MemoryData),

    Locale(LocaleData),

    Virtualization(VirtualizationData),

    Windows(WindowsData),

    Gpu(Vec<GpuData>),

    Distro(DistroData),
}

fn deserialize_sections<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, Section<SectionData>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;

    let value = serde_json::Value::deserialize(deserializer)?;
    let map = value
        .as_object()
        .ok_or_else(|| D::Error::custom("sections must be a JSON object"))?;

    let mut sections = BTreeMap::new();
    for (key, section_value) in map {
        let Some(wrap) = known_sections().iter().find(|(id, _)| id == key) else {
            continue;
        };
        let section = (wrap.1)(section_value).map_err(D::Error::custom)?;
        sections.insert(key.clone(), section);
    }
    Ok(sections)
}

type SectionWrapper = fn(&serde_json::Value) -> Result<Section<SectionData>, String>;

fn known_sections() -> &'static [(&'static str, SectionWrapper)] {
    fn wrap_os(value: &serde_json::Value) -> Result<Section<SectionData>, String> {
        typed_section::<OsData>(value, SectionData::Os)
    }
    fn wrap_architecture(value: &serde_json::Value) -> Result<Section<SectionData>, String> {
        typed_section::<ArchitectureData>(value, SectionData::Architecture)
    }
    fn wrap_cpu(value: &serde_json::Value) -> Result<Section<SectionData>, String> {
        typed_section::<CpuData>(value, SectionData::Cpu)
    }
    fn wrap_memory(value: &serde_json::Value) -> Result<Section<SectionData>, String> {
        typed_section::<MemoryData>(value, SectionData::Memory)
    }
    fn wrap_locale(value: &serde_json::Value) -> Result<Section<SectionData>, String> {
        typed_section::<LocaleData>(value, SectionData::Locale)
    }
    fn wrap_virtualization(value: &serde_json::Value) -> Result<Section<SectionData>, String> {
        typed_section::<VirtualizationData>(value, SectionData::Virtualization)
    }

    &[
        ("os", wrap_os),
        ("architecture", wrap_architecture),
        ("cpu", wrap_cpu),
        ("memory", wrap_memory),
        ("locale", wrap_locale),
        ("virtualization", wrap_virtualization),
    ]
}

fn typed_section<T>(
    section_value: &serde_json::Value,
    wrap: fn(T) -> SectionData,
) -> Result<Section<SectionData>, String>
where
    T: serde::de::DeserializeOwned,
{
    let parsed: Section<T> =
        serde_json::from_value(section_value.clone()).map_err(|error| error.to_string())?;
    Ok(Section {
        status: parsed.status,
        data: parsed.data.map(wrap),
        provenance: parsed.provenance,
        issues: parsed.issues,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "WireVersionValue")]
pub struct VersionValue {
    raw: String,
    normalized: Option<NormalizedVersion>,
}

#[derive(Debug, Deserialize)]
struct WireVersionValue {
    raw: String,
}

impl TryFrom<WireVersionValue> for VersionValue {
    type Error = String;

    fn try_from(wire: WireVersionValue) -> Result<Self, Self::Error> {
        Ok(Self::new(wire.raw))
    }
}

impl VersionValue {
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        Self {
            normalized: NormalizedVersion::parse(&raw),
            raw,
        }
    }

    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub fn normalized(&self) -> Option<&NormalizedVersion> {
        self.normalized.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedVersion {
    pub major: u64,

    pub minor: Option<u64>,

    pub patch: Option<u64>,
}

impl NormalizedVersion {
    fn parse(raw: &str) -> Option<Self> {
        let mut components = raw.split('.');
        let major = numeric_prefix(components.next()?)?;
        let minor = components.next().and_then(numeric_prefix);
        let patch = if minor.is_some() {
            components.next().and_then(numeric_prefix)
        } else {
            None
        };
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

fn numeric_prefix(component: &str) -> Option<u64> {
    let digits = component
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit())
        .count();
    if digits == 0 {
        None
    } else {
        component[..digits].parse().ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsData {
    pub family: OsFamily,

    pub version: VersionValue,

    pub build: Option<VersionValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsFamily {
    Windows,

    Linux,

    #[serde(rename = "macos")]
    MacOs,

    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureData {
    pub process: Architecture,

    pub os: Architecture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86,

    X64,

    Arm64,

    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuData {
    pub model: Option<String>,

    pub logical_count: Option<u32>,

    pub physical_count: Option<u32>,

    pub features: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryData {
    pub installed_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocaleData {
    pub languages: Vec<String>,

    pub timezone: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualizationData {
    pub category: VirtualizationCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuData {
    pub index: u32,

    pub name: String,

    pub vendor_id: u32,

    pub device_id: u32,

    pub driver_version: Option<VersionValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistroData {
    pub name: Option<String>,

    pub version: Option<VersionValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsData {
    pub edition: Option<String>,

    pub display_version: Option<String>,

    pub ubr: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VirtualizationCategory {
    Physical,

    VirtualMachine,

    Container,

    Unknown,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotReadError {
    #[error("malformed snapshot document: {message}")]
    Malformed { message: String },

    #[error("unsupported schema major {found} (reader supports major {supported})")]
    UnsupportedSchemaMajor { found: u32, supported: u32 },
}

impl SnapshotReadError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "snapshot.read.malformed",
            Self::UnsupportedSchemaMajor { .. } => "snapshot.read.unsupported_schema_major",
        }
    }
}

pub const DEFAULT_MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadLimits {
    pub max_input_bytes: usize,
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
        }
    }
}

pub fn read_snapshot(input: &str) -> Result<Snapshot, SnapshotReadError> {
    read_snapshot_with_limits(input, &ReadLimits::default())
}

pub fn read_snapshot_with_limits(
    input: &str,
    limits: &ReadLimits,
) -> Result<Snapshot, SnapshotReadError> {
    if input.len() > limits.max_input_bytes {
        return Err(SnapshotReadError::Malformed {
            message: format!(
                "input size {} exceeds the configured limit of {} bytes",
                input.len(),
                limits.max_input_bytes
            ),
        });
    }
    read_snapshot_limited(input)
}

fn read_snapshot_limited(input: &str) -> Result<Snapshot, SnapshotReadError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| SnapshotReadError::Malformed {
            message: error.to_string(),
        })?;

    let schema_value =
        value
            .get("schema")
            .cloned()
            .ok_or_else(|| SnapshotReadError::Malformed {
                message: "missing schema identifier".to_owned(),
            })?;
    let schema: SchemaIdentifier =
        serde_json::from_value(schema_value).map_err(|error| SnapshotReadError::Malformed {
            message: format!("invalid schema identifier: {error}"),
        })?;

    let supported = SchemaIdentifier::current().major;
    if schema.major != supported {
        return Err(SnapshotReadError::UnsupportedSchemaMajor {
            found: schema.major,
            supported,
        });
    }

    serde_json::from_value(value).map_err(|error| SnapshotReadError::Malformed {
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_schema_identifier_matches_the_accepted_contract() {
        let schema = SchemaIdentifier::current();
        assert_eq!(schema.name, "envshot.snapshot");
        assert_eq!(schema.major, 0);
        assert_eq!(schema.minor, 1);
    }

    #[test]
    fn section_status_serializes_as_stable_snake_case_codes() {
        let cases = [
            (SectionStatus::Available, "\"available\""),
            (SectionStatus::Partial, "\"partial\""),
            (SectionStatus::Unsupported, "\"unsupported\""),
            (SectionStatus::PermissionDenied, "\"permission_denied\""),
            (SectionStatus::Unavailable, "\"unavailable\""),
            (SectionStatus::Failed, "\"failed\""),
        ];
        for (status, json) in cases {
            assert_eq!(serde_json::to_string(&status).unwrap(), json);
            assert_eq!(serde_json::from_str::<SectionStatus>(json).unwrap(), status);
        }
    }

    #[test]
    fn deserialization_ignores_unknown_object_fields() {
        let json = r#"{
            "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
            "producer": {"name": "envshot", "version": "0.0.0", "build": "future"},
            "capture": {"profile": "empty_test"},
            "future_top_level": true,
            "sections": {}
        }"#;
        let parsed: Snapshot = serde_json::from_str(json).expect("unknown fields ignored");
        assert_eq!(parsed.schema, SchemaIdentifier::current());
        assert_eq!(parsed.producer.version, "0.0.0");
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn unknown_capture_profile_code_is_rejected() {
        let parsed = serde_json::from_str::<CaptureProfile>("\"turbo\"");
        assert!(
            parsed.is_err(),
            "future profile codes must not parse as a known variant"
        );
    }

    #[test]
    fn unknown_section_status_code_is_rejected() {
        let parsed = serde_json::from_str::<SectionStatus>("\"ok\"");
        assert!(
            parsed.is_err(),
            "unknown status codes must not parse as a known variant"
        );
    }

    #[test]
    fn malformed_json_is_rejected_as_an_input_error() {
        let parsed = serde_json::from_str::<Snapshot>("{ not json");
        assert!(parsed.is_err());
    }
}

#[cfg(test)]
mod limit_tests {
    use super::*;

    #[test]
    fn default_limit_is_four_mebibytes() {
        assert_eq!(DEFAULT_MAX_INPUT_BYTES, 4 * 1024 * 1024);
        assert_eq!(DEFAULT_MAX_INPUT_BYTES, 4_194_304);
    }

    #[test]
    fn exact_limit_boundary_is_accepted() {
        let doc = format!(
            "{{\"schema\":{{\"name\":\"envshot.snapshot\",\"major\":0,\"minor\":1}},              \"producer\":{{\"name\":\"envshot\",\"version\":\"0.0.0\"}},              \"capture\":{{\"profile\":\"standard\",\"privacy\":\"standard\"}},              \"sections\":{{}},\"pad\":\"{}\"}}",
            "x".repeat(4096)
        );
        let exact = ReadLimits {
            max_input_bytes: doc.len(),
        };
        assert!(read_snapshot_with_limits(&doc, &exact).is_ok());
        let over = ReadLimits {
            max_input_bytes: doc.len() - 1,
        };
        assert!(read_snapshot_with_limits(&doc, &over).is_err());
    }

    #[test]
    fn default_read_accepts_documents_above_lower_bound_candidates() {
        let big = format!(
            "{{\"schema\":{{\"name\":\"envshot.snapshot\",\"major\":0,\"minor\":1}},\
              \"producer\":{{\"name\":\"envshot\",\"version\":\"0.0.0\"}},\
              \"capture\":{{\"profile\":\"standard\",\"privacy\":\"standard\"}},\
              \"sections\":{{}},\"pad\":\"{}\"}}",
            "x".repeat(4096)
        );
        assert!(read_snapshot(&big).is_ok());
    }
}
