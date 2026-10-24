use std::sync::Arc;

use envshot_capture::{
    CaptureContext, Collector, CollectorDescriptor, CollectorError, CollectorRegistry,
    SectionContribution,
};
use envshot_model::{CaptureProfile, SectionData, SectionStatus, SourceKind};

#[test]
fn real_host_windows_version_satisfies_invariants() {
    if !cfg!(windows) {
        return;
    }
    let snapshot = envshot_capture::capture(CaptureProfile::Standard);
    let section = snapshot.sections.get("windows").expect("windows section");
    assert!(matches!(
        section.status(),
        SectionStatus::Available | SectionStatus::Partial
    ));
    let Some(SectionData::Windows(data)) = section.data() else {
        panic!("windows section must carry WindowsData");
    };
    let facts = u8::from(data.edition.is_some())
        + u8::from(data.display_version.is_some())
        + u8::from(data.ubr.is_some());
    assert!(
        facts > 0,
        "a real Windows install exposes at least one fact"
    );
    assert_eq!(section.provenance()[0].collector_id(), "windows.version");
}

#[cfg(not(windows))]
#[test]
fn non_windows_hosts_report_unsupported() {
    let snapshot = envshot_capture::capture(CaptureProfile::Standard);
    let section = snapshot.sections.get("windows").expect("windows section");
    assert_eq!(section.status(), SectionStatus::Unsupported);
}

struct FakeReader {
    edition: Option<String>,
    display_version: Option<String>,
    ubr: Option<u32>,
}

impl envshot_capture::RegistryReader for FakeReader {
    fn read_string(&self, _key: &str, name: &str) -> Result<Option<String>, CollectorError> {
        match name {
            "EditionID" => Ok(self.edition.clone()),
            "DisplayVersion" => Ok(self.display_version.clone()),
            _ => Ok(None),
        }
    }

    fn read_u32(&self, _key: &str, name: &str) -> Result<Option<u32>, CollectorError> {
        match name {
            "UBR" => Ok(self.ubr),
            _ => Ok(None),
        }
    }
}

struct WindowsViaReader(Arc<dyn envshot_capture::RegistryReader>);

impl Collector for WindowsViaReader {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new("windows.version", "windows", 1, SourceKind::SystemFile)
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        let reader = self.0.as_ref();
        let edition = reader.read_string("k", "EditionID")?;
        let display = reader.read_string("k", "DisplayVersion")?;
        let ubr = reader.read_u32("k", "UBR")?;
        Ok(SectionContribution::new(
            "windows",
            SectionStatus::Available,
            Some(SectionData::Windows(envshot_model::WindowsData {
                edition,
                display_version: display,
                ubr,
            })),
            Vec::new(),
        ))
    }
}

fn capture_with_reader(
    reader: Arc<dyn envshot_capture::RegistryReader>,
) -> envshot_model::Snapshot {
    let registry = CollectorRegistry::builder()
        .register(WindowsViaReader(reader))
        .expect("unique")
        .build();
    envshot_capture::capture_with_registry(&registry, CaptureProfile::Standard)
}

#[test]
fn synthetic_registry_values_map_verbatim() {
    let reader: Arc<dyn envshot_capture::RegistryReader> = Arc::new(FakeReader {
        edition: Some("Professional".to_owned()),
        display_version: Some("24H2".to_owned()),
        ubr: Some(3031),
    });
    let snapshot = capture_with_reader(reader);

    let Some(SectionData::Windows(data)) =
        snapshot.sections.get("windows").expect("section").data()
    else {
        panic!("windows section must carry WindowsData");
    };
    assert_eq!(data.edition.as_deref(), Some("Professional"));
    assert_eq!(data.display_version.as_deref(), Some("24H2"));
    assert_eq!(data.ubr, Some(3031));
}

#[test]
fn default_reader_treats_unknown_values_as_absent() {
    let reader = envshot_capture::default_registry_reader();

    assert_eq!(
        reader
            .read_string(
                r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                "ENVSHOT_NOPE"
            )
            .expect("unreadable is absent, not an error"),
        None
    );
}
