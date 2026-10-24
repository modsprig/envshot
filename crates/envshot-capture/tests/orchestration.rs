use envshot_capture::{
    CaptureContext, Collector, CollectorDescriptor, CollectorError, CollectorRegistry,
    SectionContribution, capture_with_registry,
};
use envshot_model::{CollectorIssue, IssueCode, SectionData, SectionStatus, SourceKind};

struct FakeCollector {
    id: &'static str,
    section_id: &'static str,
    behavior: Behavior,
}

enum Behavior {
    Contribute {
        status: SectionStatus,
        data: Option<SectionData>,
        issues: Vec<CollectorIssue>,
    },
    Error(String),
    Panic,
}

impl FakeCollector {
    fn contribute(
        id: &'static str,
        section_id: &'static str,
        status: SectionStatus,
        data: Option<SectionData>,
    ) -> Self {
        Self {
            id,
            section_id,
            behavior: Behavior::Contribute {
                status,
                data,
                issues: Vec::new(),
            },
        }
    }

    fn with_issue(mut self, code: &str) -> Self {
        if let Behavior::Contribute { issues, .. } = &mut self.behavior {
            issues.push(CollectorIssue::new(
                IssueCode::new(code).expect("non-empty"),
            ));
        }
        self
    }

    fn failing(id: &'static str, section_id: &'static str, message: &str) -> Self {
        Self {
            id,
            section_id,
            behavior: Behavior::Error(message.to_owned()),
        }
    }

    fn panicking(id: &'static str, section_id: &'static str) -> Self {
        Self {
            id,
            section_id,
            behavior: Behavior::Panic,
        }
    }
}

impl Collector for FakeCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(self.id, self.section_id, 1, SourceKind::NativeApi)
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        match &self.behavior {
            Behavior::Contribute {
                status,
                data,
                issues,
            } => Ok(SectionContribution::new(
                self.section_id,
                *status,
                data.clone(),
                issues.clone(),
            )),
            Behavior::Error(message) => Err(CollectorError::new(message)),
            Behavior::Panic => panic!("deliberate test panic with secret payload"),
        }
    }
}

fn os_available() -> FakeCollector {
    FakeCollector::contribute(
        "collector.os",
        "os",
        SectionStatus::Available,
        Some(SectionData::Os(envshot_model::OsData {
            family: envshot_model::OsFamily::Windows,
            version: envshot_model::VersionValue::new("10.0.1"),
            build: None,
        })),
    )
}

fn memory_denied() -> FakeCollector {
    FakeCollector::contribute(
        "collector.memory",
        "memory",
        SectionStatus::PermissionDenied,
        None,
    )
    .with_issue("access_denied")
}

fn registry(
    collectors: Vec<FakeCollector>,
) -> Result<CollectorRegistry, envshot_capture::CaptureError> {
    let mut builder = CollectorRegistry::builder();
    for collector in collectors {
        builder = builder.register(collector)?;
    }
    Ok(builder.build())
}

#[test]
fn registration_order_does_not_affect_output() {
    let forward = registry(vec![
        os_available(),
        FakeCollector::contribute("collector.cpu", "cpu", SectionStatus::Available, None),
        FakeCollector::contribute(
            "collector.locale",
            "locale",
            SectionStatus::Unavailable,
            None,
        ),
    ])
    .expect("valid");
    let backward = registry(vec![
        FakeCollector::contribute(
            "collector.locale",
            "locale",
            SectionStatus::Unavailable,
            None,
        ),
        FakeCollector::contribute("collector.cpu", "cpu", SectionStatus::Available, None),
        os_available(),
    ])
    .expect("valid");

    let forward_snapshot = capture_with_registry(&forward, envshot_model::CaptureProfile::Standard);
    let backward_snapshot =
        capture_with_registry(&backward, envshot_model::CaptureProfile::Standard);

    let forward_json = serde_json::to_string(&forward_snapshot).expect("must serialize");
    let backward_json = serde_json::to_string(&backward_snapshot).expect("must serialize");
    assert_eq!(forward_json, backward_json);
}

#[test]
fn duplicate_collector_id_is_rejected() {
    let error = registry(vec![os_available(), os_available()])
        .expect_err("duplicate collector IDs must be rejected");

    assert_eq!(error.code(), "capture.duplicate_collector");
}

#[test]
fn duplicate_section_id_is_rejected() {
    let memory_other = FakeCollector::contribute(
        "collector.memory.other",
        "memory",
        SectionStatus::Available,
        None,
    );

    let error = registry(vec![memory_denied(), memory_other])
        .expect_err("duplicate section IDs must be rejected");

    assert_eq!(error.code(), "capture.duplicate_section");
}

#[test]
fn environmental_failure_degrades_only_its_section() {
    let snapshot = capture_with_registry(
        &registry(vec![os_available(), memory_denied()]).expect("valid"),
        envshot_model::CaptureProfile::Standard,
    );

    let memory = snapshot.sections.get("memory").expect("memory section");
    assert_eq!(memory.status(), SectionStatus::PermissionDenied);
    assert_eq!(memory.issues()[0].code.as_str(), "access_denied");

    let os = snapshot.sections.get("os").expect("os section");
    assert_eq!(os.status(), SectionStatus::Available);

    assert_eq!(memory.provenance()[0].collector_id(), "collector.memory");
    assert_eq!(memory.provenance()[0].source_kind(), SourceKind::NativeApi);
}

#[test]
fn unexpected_error_degrades_to_failed_without_raw_text() {
    let failing = FakeCollector::failing(
        "collector.hostile",
        "os",
        "raw io error: C:\\Users\\secret\\path",
    );

    let snapshot = capture_with_registry(
        &registry(vec![failing]).expect("valid"),
        envshot_model::CaptureProfile::Standard,
    );

    let os = snapshot.sections.get("os").expect("os section");
    assert_eq!(os.status(), SectionStatus::Failed);
    assert_eq!(os.issues()[0].code.as_str(), "collector_failed");
    assert!(
        os.issues()[0].detail.is_none(),
        "raw error text must not leak"
    );

    let json = serde_json::to_string(&snapshot).expect("must serialize");
    assert!(!json.contains("secret"), "raw error text leaked: {json}");
}

#[test]
fn panicking_collector_degrades_to_failed() {
    let snapshot = capture_with_registry(
        &registry(vec![
            os_available(),
            FakeCollector::panicking("collector.panic", "cpu"),
        ])
        .expect("valid"),
        envshot_model::CaptureProfile::Standard,
    );

    let cpu = snapshot.sections.get("cpu").expect("cpu section");
    assert_eq!(cpu.status(), SectionStatus::Failed);
    assert_eq!(cpu.issues()[0].code.as_str(), "collector_panicked");
    assert_eq!(cpu.provenance()[0].collector_id(), "collector.panic");

    let os = snapshot.sections.get("os").expect("os section");
    assert_eq!(os.status(), SectionStatus::Available, "others unaffected");
}

#[test]
fn invalid_contribution_degrades_to_failed() {
    let invalid =
        FakeCollector::contribute("collector.bad", "memory", SectionStatus::Partial, None);

    let snapshot = capture_with_registry(
        &registry(vec![invalid]).expect("valid"),
        envshot_model::CaptureProfile::Standard,
    );

    let memory = snapshot.sections.get("memory").expect("memory section");
    assert_eq!(memory.status(), SectionStatus::Failed);
    assert_eq!(
        memory.issues()[0].code.as_str(),
        "collector_invalid_contribution"
    );
}

#[test]
fn empty_registry_yields_envelope() {
    let empty = CollectorRegistry::empty();
    let snapshot = capture_with_registry(&empty, envshot_model::CaptureProfile::EmptyTest);

    assert!(snapshot.sections.is_empty());
    assert_eq!(snapshot.schema, envshot_model::SchemaIdentifier::current());
}

#[test]
fn empty_descriptor_ids_are_rejected() {
    struct EmptyId;
    impl Collector for EmptyId {
        fn descriptor(&self) -> CollectorDescriptor {
            CollectorDescriptor::new("", "os", 1, SourceKind::SystemFile)
        }
        fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
            unreachable!("registration fails before collection")
        }
    }
    struct EmptySection;
    impl Collector for EmptySection {
        fn descriptor(&self) -> CollectorDescriptor {
            CollectorDescriptor::new("collector.ok", "", 1, SourceKind::SystemFile)
        }
        fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
            unreachable!("registration fails before collection")
        }
    }

    let error = CollectorRegistry::builder()
        .register(EmptyId)
        .expect_err("empty collector id must be rejected");
    assert_eq!(error.code(), "capture.invalid_descriptor");

    let error = CollectorRegistry::builder()
        .register(EmptySection)
        .expect_err("empty section id must be rejected");
    assert_eq!(error.code(), "capture.invalid_descriptor");
}
