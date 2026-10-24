use envshot_capture::{
    CaptureContext, Collector, CollectorDescriptor, CollectorError, CollectorRegistry,
    SectionContribution, capture_with_registry,
};
use envshot_model::{CaptureProfile, SectionStatus, SourceKind};
use proptest::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Spec {
    id: &'static str,
    section: &'static str,
    status: SectionStatus,
}

fn pool() -> Vec<Spec> {
    vec![
        Spec {
            id: "c.a",
            section: "os",
            status: SectionStatus::Available,
        },
        Spec {
            id: "c.b",
            section: "cpu",
            status: SectionStatus::Unsupported,
        },
        Spec {
            id: "c.c",
            section: "memory",
            status: SectionStatus::PermissionDenied,
        },
        Spec {
            id: "c.d",
            section: "locale",
            status: SectionStatus::Unavailable,
        },
    ]
}

struct FixedCollector(Spec);

impl Collector for FixedCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(self.0.id, self.0.section, 1, SourceKind::NativeApi)
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        Ok(SectionContribution::new(
            self.0.section,
            self.0.status,
            None,
            Vec::new(),
        ))
    }
}

fn capture_in_order(specs: &[Spec]) -> String {
    let mut builder = CollectorRegistry::builder();
    for spec in specs {
        builder = builder
            .register(FixedCollector(spec.clone()))
            .expect("unique specs");
    }
    let snapshot = capture_with_registry(&builder.build(), CaptureProfile::Standard);
    serde_json::to_string(&snapshot).expect("must serialize")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn output_is_independent_of_registration_order(
        specs in proptest::collection::vec(proptest::sample::select(pool()), 0usize..=4),
    ) {

        let mut chosen: Vec<Spec> = Vec::new();
        for spec in specs {
            if !chosen.iter().any(|s| s.id == spec.id) {
                chosen.push(spec);
            }
        }


        let mut canonical = chosen.clone();
        canonical.sort_by_key(|s| s.id);

        let generated = capture_in_order(&chosen);
        let canonical_json = capture_in_order(&canonical);
        prop_assert_eq!(generated, canonical_json);
    }

    #[test]
    fn repeated_captures_are_byte_identical(
        specs in proptest::collection::vec(proptest::sample::select(pool()), 1usize..=4),
    ) {
        let registry = {
            let mut builder = CollectorRegistry::builder();
            let mut seen: Vec<&'static str> = Vec::new();
            for spec in specs {
                if seen.contains(&spec.id) {
                    continue;
                }
                seen.push(spec.id);
                builder = builder.register(FixedCollector(spec)).expect("unique specs");
            }
            builder.build()
        };

        let first = serde_json::to_string(&capture_with_registry(&registry, CaptureProfile::Standard))
            .expect("must serialize");
        let second = serde_json::to_string(&capture_with_registry(&registry, CaptureProfile::Standard))
            .expect("must serialize");
        prop_assert_eq!(first, second);
    }
}
