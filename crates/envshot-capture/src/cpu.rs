use std::collections::BTreeSet;

use envshot_model::{CollectorIssue, CpuData, SectionStatus};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

#[cfg(any(target_arch = "x86", target_arch = "x86_64", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CpuFeatureBits(pub(crate) u32);

#[cfg(any(target_arch = "x86", target_arch = "x86_64", test))]
impl CpuFeatureBits {
    pub(crate) const SSE: Self = Self(1);

    pub(crate) const SSE2: Self = Self(1 << 1);

    pub(crate) const SSE4_1: Self = Self(1 << 2);

    pub(crate) const SSE4_2: Self = Self(1 << 3);

    pub(crate) const AES: Self = Self(1 << 4);

    pub(crate) const AVX: Self = Self(1 << 5);

    pub(crate) const AVX2: Self = Self(1 << 6);

    pub(crate) const AVX512F: Self = Self(1 << 7);

    #[must_use]
    pub(crate) fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub(crate) fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64", test))]
pub(crate) fn cpu_feature_names(bits: CpuFeatureBits) -> BTreeSet<String> {
    let pairs = [
        ("sse", CpuFeatureBits::SSE),
        ("sse2", CpuFeatureBits::SSE2),
        ("sse4_1", CpuFeatureBits::SSE4_1),
        ("sse4_2", CpuFeatureBits::SSE4_2),
        ("aes", CpuFeatureBits::AES),
        ("avx", CpuFeatureBits::AVX),
        ("avx2", CpuFeatureBits::AVX2),
        ("avx512f", CpuFeatureBits::AVX512F),
    ];
    pairs
        .into_iter()
        .filter(|&(_, flag)| bits.contains(flag))
        .map(|(name, _)| name.to_owned())
        .collect()
}

pub(crate) fn cpu_data_from(
    brands: &[&str],
    logical: Option<usize>,
    physical: Option<usize>,
    features: BTreeSet<String>,
) -> CpuData {
    let model = brands
        .iter()
        .map(|brand| brand.trim())
        .find(|brand| !brand.is_empty())
        .map(str::to_owned);
    let count = |value: Option<usize>| value.and_then(|v| u32::try_from(v).ok());
    CpuData {
        model,
        logical_count: count(logical),
        physical_count: count(physical),
        features,
    }
}

pub(crate) struct CpuCollector;

impl Collector for CpuCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "cpu.host",
            "cpu",
            1,
            envshot_model::SourceKind::LibraryAdapter,
        )
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        let mut system = sysinfo::System::new();
        system.refresh_cpu_all();

        let cpus = system.cpus();
        let logical = Some(cpus.len());
        let brands: Vec<&str> = cpus.iter().map(sysinfo::Cpu::brand).collect();
        let physical = sysinfo::System::physical_core_count();

        let (features, status, issues) = collect_features();
        let data = cpu_data_from(&brands, logical, physical, features);

        let status = if status == SectionStatus::Available {
            SectionStatus::Available
        } else {
            SectionStatus::Partial
        };
        Ok(SectionContribution::new(
            "cpu",
            status,
            Some(envshot_model::SectionData::Cpu(data)),
            issues,
        ))
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn collect_features() -> (BTreeSet<String>, SectionStatus, Vec<CollectorIssue>) {
    let cpuid = raw_cpuid::CpuId::new();
    let leaf1 = cpuid.get_feature_info();
    let leaf7 = cpuid.get_extended_feature_info();
    let mut acc = CpuFeatureBits(0);
    let mut set_if = |cond: bool, flag: CpuFeatureBits| {
        if cond {
            acc = acc.union(flag);
        }
    };
    set_if(
        leaf1.as_ref().is_some_and(raw_cpuid::FeatureInfo::has_sse),
        CpuFeatureBits::SSE,
    );
    set_if(
        leaf1.as_ref().is_some_and(raw_cpuid::FeatureInfo::has_sse2),
        CpuFeatureBits::SSE2,
    );
    set_if(
        leaf1
            .as_ref()
            .is_some_and(raw_cpuid::FeatureInfo::has_sse41),
        CpuFeatureBits::SSE4_1,
    );
    set_if(
        leaf1
            .as_ref()
            .is_some_and(raw_cpuid::FeatureInfo::has_sse42),
        CpuFeatureBits::SSE4_2,
    );
    set_if(
        leaf1
            .as_ref()
            .is_some_and(raw_cpuid::FeatureInfo::has_aesni),
        CpuFeatureBits::AES,
    );
    set_if(
        leaf1.as_ref().is_some_and(raw_cpuid::FeatureInfo::has_avx),
        CpuFeatureBits::AVX,
    );
    set_if(
        leaf7
            .as_ref()
            .is_some_and(raw_cpuid::ExtendedFeatures::has_avx2),
        CpuFeatureBits::AVX2,
    );
    set_if(
        leaf7
            .as_ref()
            .is_some_and(raw_cpuid::ExtendedFeatures::has_avx512f),
        CpuFeatureBits::AVX512F,
    );
    (cpu_feature_names(acc), SectionStatus::Available, Vec::new())
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn collect_features() -> (BTreeSet<String>, SectionStatus, Vec<CollectorIssue>) {
    use envshot_model::IssueCode;

    let issue =
        CollectorIssue::new(IssueCode::new("cpu_features_unsupported").expect("static code"));
    (BTreeSet::new(), SectionStatus::Partial, vec![issue])
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    fn bits(sse2: bool, sse4_2: bool, avx2: bool) -> CpuFeatureBits {
        let mut acc = CpuFeatureBits(0);
        if sse2 {
            acc = acc.union(CpuFeatureBits::SSE2);
        }
        if sse4_2 {
            acc = acc.union(CpuFeatureBits::SSE4_2);
        }
        if avx2 {
            acc = acc.union(CpuFeatureBits::AVX2);
        }
        acc
    }

    #[test]
    fn feature_names_contain_exactly_the_set_bits() {
        let names = cpu_feature_names(bits(true, true, true));
        assert_eq!(
            names,
            ["avx2", "sse2", "sse4_2"]
                .into_iter()
                .map(String::from)
                .collect()
        );
    }

    #[test]
    fn no_bits_yield_an_empty_set() {
        assert!(cpu_feature_names(CpuFeatureBits(0)).is_empty());
    }

    #[test]
    fn every_flag_maps_to_its_own_distinct_name() {
        let all = [
            CpuFeatureBits::SSE,
            CpuFeatureBits::SSE2,
            CpuFeatureBits::SSE4_1,
            CpuFeatureBits::SSE4_2,
            CpuFeatureBits::AES,
            CpuFeatureBits::AVX,
            CpuFeatureBits::AVX2,
            CpuFeatureBits::AVX512F,
        ]
        .into_iter()
        .fold(CpuFeatureBits(0), CpuFeatureBits::union);
        assert_eq!(
            cpu_feature_names(all),
            [
                "sse", "sse2", "sse4_1", "sse4_2", "aes", "avx", "avx2", "avx512f"
            ]
            .into_iter()
            .map(String::from)
            .collect()
        );
    }

    #[test]
    fn union_of_a_flag_with_itself_stays_set() {
        assert!(
            CpuFeatureBits::SSE2
                .union(CpuFeatureBits::SSE2)
                .contains(CpuFeatureBits::SSE2)
        );
    }

    #[test]
    fn cpu_data_uses_first_non_empty_trimmed_brand() {
        let data = cpu_data_from(
            &["  ", "Fixture CPU X3 ", ""],
            Some(8),
            Some(4),
            BTreeSet::new(),
        );
        assert_eq!(data.model.as_deref(), Some("Fixture CPU X3"));
        assert_eq!(data.logical_count, Some(8));
        assert_eq!(data.physical_count, Some(4));
    }

    #[test]
    fn absent_counts_stay_null() {
        let data = cpu_data_from(&["CPU"], None, None, BTreeSet::new());
        assert_eq!(data.model.as_deref(), Some("CPU"));
        assert_eq!(data.logical_count, None);
        assert_eq!(data.physical_count, None);
    }
}
