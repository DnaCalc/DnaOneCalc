use super::{
    AbiEntryPoint, ExcelHostCallSurface, ExcelLifecycleEntryPoint, ExtensionLoadPhase,
    ExtensionPackagingKind, ExtensionPlatform, ExtensionProviderManifest,
    FrozenExtensionHostContract,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionPackagingConformanceHarness {
    pub platform: ExtensionPlatform,
    pub packaging: ExtensionPackagingKind,
    pub artifact_name: String,
    pub admitted_abi_version_slug: String,
    pub admitted_entry_points: Vec<AbiEntryPoint>,
    pub admitted_excel_entry_points: Vec<ExcelLifecycleEntryPoint>,
    pub admitted_host_call_subset: Vec<ExcelHostCallSurface>,
    pub admitted_load_phases: Vec<ExtensionLoadPhase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionPackagingParityReport {
    pub windows_artifact_name: String,
    pub linux_artifact_name: String,
    pub shared_abi_version_slug: String,
    pub parity_gaps: Vec<String>,
}

impl ExtensionPackagingParityReport {
    pub fn is_conformant(&self) -> bool {
        self.parity_gaps.is_empty()
    }
}

pub fn build_packaging_conformance_harnesses(
    contract: &FrozenExtensionHostContract,
    provider_id: &str,
) -> [ExtensionPackagingConformanceHarness; 2] {
    [
        build_platform_harness(
            contract,
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            format!("{provider_id}.xll"),
        ),
        build_platform_harness(
            contract,
            ExtensionPlatform::LinuxDesktop,
            ExtensionPackagingKind::SharedObject,
            format!("lib{provider_id}.so"),
        ),
    ]
}

pub fn verify_packaging_parity(
    contract: &FrozenExtensionHostContract,
    windows_manifest: &ExtensionProviderManifest,
    linux_manifest: &ExtensionProviderManifest,
) -> ExtensionPackagingParityReport {
    let [windows_harness, linux_harness] =
        build_packaging_conformance_harnesses(contract, &windows_manifest.provider_id);
    let mut parity_gaps = Vec::new();

    parity_gaps.extend(validate_manifest_against_harness(
        windows_manifest,
        &windows_harness,
    ));
    parity_gaps.extend(validate_manifest_against_harness(
        linux_manifest,
        &linux_harness,
    ));

    if windows_manifest.declared_abi_version_slug != linux_manifest.declared_abi_version_slug {
        parity_gaps.push("Windows and Linux packaging declare different ABI versions".to_string());
    }
    if windows_manifest.requested_entry_points != linux_manifest.requested_entry_points {
        parity_gaps
            .push("Windows and Linux packaging request different ABI entry point sets".to_string());
    }
    if windows_manifest.requested_excel_entry_points != linux_manifest.requested_excel_entry_points
    {
        parity_gaps.push(
            "Windows and Linux packaging request different Excel lifecycle hook sets".to_string(),
        );
    }
    if windows_manifest.requires_rtd_contract != linux_manifest.requires_rtd_contract {
        parity_gaps.push(
            "Windows and Linux packaging disagree on whether RTD is part of the admitted claim"
                .to_string(),
        );
    }

    ExtensionPackagingParityReport {
        windows_artifact_name: windows_harness.artifact_name,
        linux_artifact_name: linux_harness.artifact_name,
        shared_abi_version_slug: contract.abi_version.slug().to_string(),
        parity_gaps,
    }
}

fn build_platform_harness(
    contract: &FrozenExtensionHostContract,
    platform: ExtensionPlatform,
    packaging: ExtensionPackagingKind,
    artifact_name: String,
) -> ExtensionPackagingConformanceHarness {
    ExtensionPackagingConformanceHarness {
        platform,
        packaging,
        artifact_name,
        admitted_abi_version_slug: contract.abi_version.slug().to_string(),
        admitted_entry_points: contract.abi_entry_points.to_vec(),
        admitted_excel_entry_points: contract.admitted_excel_entry_points.to_vec(),
        admitted_host_call_subset: contract.host_call_subset.to_vec(),
        admitted_load_phases: contract.load_phases.to_vec(),
    }
}

fn validate_manifest_against_harness(
    manifest: &ExtensionProviderManifest,
    harness: &ExtensionPackagingConformanceHarness,
) -> Vec<String> {
    let mut gaps = Vec::new();

    if manifest.target_platform != harness.platform {
        gaps.push(format!(
            "{} declares {} but the harness expects {}",
            manifest.display_name,
            manifest.target_platform.slug(),
            harness.platform.slug()
        ));
    }
    if manifest.packaging != harness.packaging {
        gaps.push(format!(
            "{} declares {} packaging but the harness expects {}",
            manifest.display_name,
            manifest.packaging.slug(),
            harness.packaging.slug()
        ));
    }
    if manifest.declared_abi_version_slug != harness.admitted_abi_version_slug {
        gaps.push(format!(
            "{} declares ABI {} but the harness expects {}",
            manifest.display_name,
            manifest.declared_abi_version_slug,
            harness.admitted_abi_version_slug
        ));
    }
    if manifest
        .requested_entry_points
        .iter()
        .any(|entry| !harness.admitted_entry_points.contains(entry))
    {
        gaps.push(format!(
            "{} requests ABI entry points outside the admitted subset",
            manifest.display_name
        ));
    }
    if manifest
        .requested_excel_entry_points
        .iter()
        .any(|entry| !harness.admitted_excel_entry_points.contains(entry))
    {
        gaps.push(format!(
            "{} requests Excel lifecycle hooks outside the admitted subset",
            manifest.display_name
        ));
    }

    gaps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::{
        frozen_extension_host_contract, ExtensionPackagingKind, ExtensionPlatform,
    };

    #[test]
    fn harnesses_preserve_shared_abi_claims_across_xll_and_so_packaging() {
        let contract = frozen_extension_host_contract();
        let [windows, linux] = build_packaging_conformance_harnesses(&contract, "quote-feed");

        assert_eq!(windows.artifact_name, "quote-feed.xll");
        assert_eq!(linux.artifact_name, "libquote-feed.so");
        assert_eq!(
            windows.admitted_abi_version_slug,
            linux.admitted_abi_version_slug
        );
        assert_eq!(windows.admitted_entry_points, linux.admitted_entry_points);
        assert_eq!(
            windows.admitted_excel_entry_points,
            linux.admitted_excel_entry_points
        );
        assert_eq!(
            windows.admitted_host_call_subset,
            linux.admitted_host_call_subset
        );
        assert_eq!(windows.admitted_load_phases, linux.admitted_load_phases);
        assert_eq!(windows.packaging, ExtensionPackagingKind::Xll);
        assert_eq!(linux.packaging, ExtensionPackagingKind::SharedObject);
    }

    #[test]
    fn parity_report_accepts_windows_and_linux_manifests_with_same_claims() {
        let contract = frozen_extension_host_contract();
        let mut windows = ExtensionProviderManifest::native(
            "quote-feed",
            "Quote Feed",
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            &contract,
        );
        windows.requested_entry_points = vec![AbiEntryPoint::QueryIdentityAndVersion];
        windows.requested_excel_entry_points = vec![ExcelLifecycleEntryPoint::XlAutoOpen];
        windows.requires_rtd_contract = true;

        let mut linux = ExtensionProviderManifest::native(
            "quote-feed",
            "Quote Feed",
            ExtensionPlatform::LinuxDesktop,
            ExtensionPackagingKind::SharedObject,
            &contract,
        );
        linux.requested_entry_points = windows.requested_entry_points.clone();
        linux.requested_excel_entry_points = windows.requested_excel_entry_points.clone();
        linux.requires_rtd_contract = true;

        let report = verify_packaging_parity(&contract, &windows, &linux);
        assert!(report.is_conformant());
        assert_eq!(report.windows_artifact_name, "quote-feed.xll");
        assert_eq!(report.linux_artifact_name, "libquote-feed.so");
        assert_eq!(
            report.shared_abi_version_slug,
            "onecalc-native-extension-abi-v0"
        );
    }

    #[test]
    fn parity_report_flags_claim_drift_between_windows_and_linux_packaging() {
        let contract = frozen_extension_host_contract();
        let mut windows = ExtensionProviderManifest::native(
            "quote-feed",
            "Quote Feed",
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            &contract,
        );
        windows.requested_entry_points = vec![AbiEntryPoint::RegisterProvider];
        windows.requires_rtd_contract = true;

        let mut linux = ExtensionProviderManifest::native(
            "quote-feed",
            "Quote Feed",
            ExtensionPlatform::LinuxDesktop,
            ExtensionPackagingKind::SharedObject,
            &contract,
        );
        linux.requested_entry_points = vec![AbiEntryPoint::Shutdown];
        linux.requires_rtd_contract = false;

        let report = verify_packaging_parity(&contract, &windows, &linux);
        assert!(!report.is_conformant());
        assert!(report
            .parity_gaps
            .iter()
            .any(|gap| gap.contains("different ABI entry point sets")));
        assert!(report
            .parity_gaps
            .iter()
            .any(|gap| gap.contains("disagree on whether RTD")));
    }
}
