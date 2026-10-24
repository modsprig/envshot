use envshot_model::{SectionStatus, VirtualizationCategory};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

pub(crate) fn category_from(
    hypervisor: Option<bool>,
    container_markers: bool,
) -> VirtualizationCategory {
    if container_markers {
        return VirtualizationCategory::Container;
    }
    match hypervisor {
        Some(true) => VirtualizationCategory::VirtualMachine,
        Some(false) => VirtualizationCategory::Physical,
        None => VirtualizationCategory::Unknown,
    }
}

#[cfg(target_os = "linux")]
fn container_markers() -> bool {
    std::path::Path::new("/.dockerenv").exists()
        || std::path::Path::new("/run/.containerenv").exists()
}

#[cfg(not(target_os = "linux"))]
fn container_markers() -> bool {
    false
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn hypervisor_bit() -> Option<bool> {
    raw_cpuid::CpuId::new()
        .get_feature_info()
        .map(|feature_info| feature_info.has_hypervisor())
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn hypervisor_bit() -> Option<bool> {
    None
}

pub(crate) struct VirtualizationCollector;

impl Collector for VirtualizationCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "virtualization.host",
            "virtualization",
            1,
            if cfg!(target_os = "linux") {
                envshot_model::SourceKind::SystemFile
            } else {
                envshot_model::SourceKind::LibraryAdapter
            },
        )
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        let category = category_from(hypervisor_bit(), container_markers());
        Ok(SectionContribution::new(
            "virtualization",
            SectionStatus::Available,
            Some(envshot_model::SectionData::Virtualization(
                envshot_model::VirtualizationData { category },
            )),
            Vec::new(),
        ))
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn container_markers_win() {
        assert_eq!(
            category_from(Some(true), true),
            VirtualizationCategory::Container
        );
        assert_eq!(
            category_from(Some(false), true),
            VirtualizationCategory::Container
        );
    }

    #[test]
    fn hypervisor_bit_maps_to_vm_or_physical() {
        assert_eq!(
            category_from(Some(true), false),
            VirtualizationCategory::VirtualMachine
        );
        assert_eq!(
            category_from(Some(false), false),
            VirtualizationCategory::Physical
        );
    }

    #[test]
    fn missing_source_maps_to_unknown() {
        assert_eq!(category_from(None, false), VirtualizationCategory::Unknown);
    }
}
