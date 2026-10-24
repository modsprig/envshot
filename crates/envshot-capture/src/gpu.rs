use envshot_model::{CollectorIssue, GpuData, IssueCode, SectionStatus, VersionValue};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdapterInfo {
    pub index: u32,
    pub name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub driver_version: Option<u64>,
}

pub(crate) fn gpu_contribution(
    adapters: Result<Vec<AdapterInfo>, CollectorError>,
) -> SectionContribution {
    match adapters {
        Ok(list) => SectionContribution::new(
            "gpu",
            SectionStatus::Available,
            Some(envshot_model::SectionData::Gpu(
                list.into_iter()
                    .map(|adapter| GpuData {
                        index: adapter.index,
                        name: adapter.name,
                        vendor_id: adapter.vendor_id,
                        device_id: adapter.device_id,
                        driver_version: adapter
                            .driver_version
                            .map(|packed| VersionValue::new(decode_driver_version(packed))),
                    })
                    .collect(),
            )),
            Vec::new(),
        ),
        Err(_) => SectionContribution::new(
            "gpu",
            SectionStatus::Unavailable,
            None,
            vec![CollectorIssue::new(
                IssueCode::new("gpu_unavailable").expect("static code"),
            )],
        ),
    }
}

pub(crate) fn decode_driver_version(packed: u64) -> String {
    let a = (packed >> 48) & 0xFFFF;
    let b = (packed >> 32) & 0xFFFF;
    let c = (packed >> 16) & 0xFFFF;
    let d = packed & 0xFFFF;
    format!("{a}.{b}.{c}.{d}")
}

pub(crate) struct GpuCollector;

impl Collector for GpuCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "gpu.adapters",
            "gpu",
            1,
            envshot_model::SourceKind::LibraryAdapter,
        )
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        Ok(gpu_contribution(enumerate_adapters()))
    }
}

#[cfg(windows)]
fn enumerate_adapters() -> Result<Vec<AdapterInfo>, CollectorError> {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, DXGI_ADAPTER_DESC1, IDXGIDevice, IDXGIFactory1,
    };
    use windows::core::Interface as _;

    let factory: IDXGIFactory1 =
        unsafe { CreateDXGIFactory1() }.map_err(|_| CollectorError::new("dxgi factory"))?;

    let mut adapters = Vec::new();
    let mut index = 0u32;
    loop {
        #[allow(clippy::manual_let_else)]
        let adapter = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(_) => break,
        };
        let desc: DXGI_ADAPTER_DESC1 = unsafe { adapter.GetDesc1() }
            .map_err(|_| CollectorError::new("dxgi adapter description"))?;
        let name = String::from_utf16_lossy(
            desc.Description
                .iter()
                .take_while(|unit| **unit != 0)
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
        );

        let driver_version = unsafe { adapter.CheckInterfaceSupport(&IDXGIDevice::IID) }
            .ok()
            .map(i64::cast_unsigned);

        adapters.push(AdapterInfo {
            index,
            name: name.trim_end().to_owned(),
            vendor_id: desc.VendorId,
            device_id: desc.DeviceId,
            driver_version,
        });
        index += 1;
    }
    Ok(adapters)
}

#[cfg(not(windows))]
fn enumerate_adapters() -> Result<Vec<AdapterInfo>, CollectorError> {
    Err(CollectorError::new("dxgi is windows-only"))
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn adapters_map_verbatim() {
        let contribution = gpu_contribution(Ok(vec![AdapterInfo {
            index: 0,
            name: "Fixture GPU".to_owned(),
            vendor_id: 0x10DE,
            device_id: 0x1234,
            driver_version: Some(9_007_199_255_784_243),
        }]));
        assert_eq!(contribution.status, SectionStatus::Available);
        let Some(envshot_model::SectionData::Gpu(adapters)) = contribution.data else {
            panic!("must carry Gpu list");
        };
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].name, "Fixture GPU");
        assert_eq!(adapters[0].vendor_id, 0x10DE);
        assert_eq!(
            adapters[0]
                .driver_version
                .as_ref()
                .map(|v| v.raw().to_string()),
            Some("32.0.15.60211".to_owned())
        );
    }

    #[test]
    fn empty_adapter_list_is_available() {
        let contribution = gpu_contribution(Ok(Vec::new()));
        assert_eq!(contribution.status, SectionStatus::Available);
        let Some(envshot_model::SectionData::Gpu(adapters)) = contribution.data else {
            panic!("must carry Gpu list");
        };
        assert!(adapters.is_empty());
    }

    #[test]
    fn enumeration_failure_degrades_to_unavailable() {
        let contribution = gpu_contribution(Err(CollectorError::new("hostile")));
        assert_eq!(contribution.status, SectionStatus::Unavailable);
        assert_eq!(contribution.issues[0].code.as_str(), "gpu_unavailable");
    }

    #[test]
    fn driver_version_decode_matches_windows_display_format() {
        assert_eq!(decode_driver_version(0), "0.0.0.0");
        let packed = (32u64 << 48) | (15u64 << 16) | 0xEB33;
        assert_eq!(decode_driver_version(packed), "32.0.15.60211");
    }
}
