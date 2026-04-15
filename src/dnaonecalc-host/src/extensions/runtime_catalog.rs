use std::collections::BTreeMap;

use super::{
    AbiEntryPoint, ExcelLifecycleEntryPoint, ExtensionPackagingKind, ExtensionPlatform,
    ExtensionPlatformSupport, FrozenExtensionHostContract, RtdHostContract,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionProviderLifecycleState {
    Discovered,
    Admitted,
    Enabled,
    Active,
    BlockedByPlatform,
    RejectedByContract,
}

impl ExtensionProviderLifecycleState {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Admitted => "admitted",
            Self::Enabled => "enabled",
            Self::Active => "active",
            Self::BlockedByPlatform => "blocked-by-platform",
            Self::RejectedByContract => "rejected-by-contract",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionProviderManifest {
    pub provider_id: String,
    pub display_name: String,
    pub declared_abi_version_slug: String,
    pub target_platform: ExtensionPlatform,
    pub packaging: ExtensionPackagingKind,
    pub requested_entry_points: Vec<AbiEntryPoint>,
    pub requested_excel_entry_points: Vec<ExcelLifecycleEntryPoint>,
    pub requires_rtd_contract: bool,
}

impl ExtensionProviderManifest {
    pub fn native(
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
        target_platform: ExtensionPlatform,
        packaging: ExtensionPackagingKind,
        contract: &FrozenExtensionHostContract,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            declared_abi_version_slug: contract.abi_version.slug().to_string(),
            target_platform,
            packaging,
            requested_entry_points: Vec::new(),
            requested_excel_entry_points: Vec::new(),
            requires_rtd_contract: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionProviderRecord {
    pub manifest: ExtensionProviderManifest,
    pub lifecycle_state: ExtensionProviderLifecycleState,
    pub status_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionProviderCatalog {
    pub runtime_platform: ExtensionPlatform,
    pub runtime_gate: ExtensionPlatformSupport,
    pub providers: BTreeMap<String, ExtensionProviderRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionLoadReport {
    pub runtime_platform: ExtensionPlatform,
    pub discovered_count: usize,
    pub admitted_count: usize,
    pub enabled_count: usize,
    pub active_count: usize,
    pub blocked_count: usize,
    pub rejected_count: usize,
}

impl ExtensionLoadReport {
    pub fn summary(&self) -> String {
        format!(
            "{} · {} discovered · {} admitted · {} enabled · {} active · {} blocked",
            self.runtime_platform.slug(),
            self.discovered_count,
            self.admitted_count,
            self.enabled_count,
            self.active_count,
            self.blocked_count + self.rejected_count,
        )
    }
}

impl ExtensionProviderCatalog {
    pub fn new(
        runtime_platform: ExtensionPlatform,
        contract: &FrozenExtensionHostContract,
    ) -> Self {
        let runtime_gate = platform_support_for_extension_platform(contract, runtime_platform)
            .expect("frozen extension contract must include the runtime platform");
        Self {
            runtime_platform,
            runtime_gate,
            providers: BTreeMap::new(),
        }
    }

    pub fn discover_provider(&mut self, manifest: ExtensionProviderManifest) -> bool {
        let provider_id = manifest.provider_id.clone();
        let next_record = ExtensionProviderRecord {
            status_summary: format!(
                "Discovered manifest for {} on {}",
                manifest.packaging.slug(),
                manifest.target_platform.slug()
            ),
            manifest,
            lifecycle_state: ExtensionProviderLifecycleState::Discovered,
        };
        match self.providers.get(&provider_id) {
            Some(existing) if existing == &next_record => false,
            _ => {
                self.providers.insert(provider_id, next_record);
                true
            }
        }
    }

    pub fn validate_provider(
        &mut self,
        contract: &FrozenExtensionHostContract,
        provider_id: &str,
    ) -> bool {
        let Some(record) = self.providers.get_mut(provider_id) else {
            return false;
        };
        let (lifecycle_state, status_summary) =
            validate_provider_manifest(contract, self.runtime_gate, &record.manifest);
        if record.lifecycle_state == lifecycle_state && record.status_summary == status_summary {
            return false;
        }
        record.lifecycle_state = lifecycle_state;
        record.status_summary = status_summary;
        true
    }

    pub fn set_provider_enabled(&mut self, provider_id: &str, enabled: bool) -> bool {
        let Some(record) = self.providers.get_mut(provider_id) else {
            return false;
        };
        match (enabled, record.lifecycle_state) {
            (true, ExtensionProviderLifecycleState::Admitted) => {
                record.lifecycle_state = ExtensionProviderLifecycleState::Enabled;
                record.status_summary = format!(
                    "Enabled for {} on {}",
                    self.runtime_gate.packaging.slug(),
                    self.runtime_platform.slug()
                );
                true
            }
            (false, ExtensionProviderLifecycleState::Enabled)
            | (false, ExtensionProviderLifecycleState::Active) => {
                record.lifecycle_state = ExtensionProviderLifecycleState::Admitted;
                record.status_summary = format!(
                    "Admitted by {} gate and waiting for enablement",
                    self.runtime_platform.slug()
                );
                true
            }
            _ => false,
        }
    }

    pub fn set_provider_active(&mut self, provider_id: &str, active: bool) -> bool {
        let Some(record) = self.providers.get_mut(provider_id) else {
            return false;
        };
        match (active, record.lifecycle_state) {
            (true, ExtensionProviderLifecycleState::Enabled) => {
                record.lifecycle_state = ExtensionProviderLifecycleState::Active;
                record.status_summary = format!(
                    "Active on {} with {} packaging",
                    self.runtime_platform.slug(),
                    self.runtime_gate.packaging.slug()
                );
                true
            }
            (false, ExtensionProviderLifecycleState::Active) => {
                record.lifecycle_state = ExtensionProviderLifecycleState::Enabled;
                record.status_summary = format!(
                    "Enabled for {} on {}",
                    self.runtime_gate.packaging.slug(),
                    self.runtime_platform.slug()
                );
                true
            }
            _ => false,
        }
    }

    pub fn load_report(&self) -> ExtensionLoadReport {
        let mut report = ExtensionLoadReport {
            runtime_platform: self.runtime_platform,
            discovered_count: 0,
            admitted_count: 0,
            enabled_count: 0,
            active_count: 0,
            blocked_count: 0,
            rejected_count: 0,
        };

        for record in self.providers.values() {
            match record.lifecycle_state {
                ExtensionProviderLifecycleState::Discovered => report.discovered_count += 1,
                ExtensionProviderLifecycleState::Admitted => report.admitted_count += 1,
                ExtensionProviderLifecycleState::Enabled => report.enabled_count += 1,
                ExtensionProviderLifecycleState::Active => report.active_count += 1,
                ExtensionProviderLifecycleState::BlockedByPlatform => report.blocked_count += 1,
                ExtensionProviderLifecycleState::RejectedByContract => report.rejected_count += 1,
            }
        }

        report
    }
}

pub fn current_extension_runtime_platform() -> ExtensionPlatform {
    if cfg!(target_arch = "wasm32") {
        ExtensionPlatform::BrowserWasm
    } else if cfg!(windows) {
        ExtensionPlatform::WindowsDesktop
    } else {
        ExtensionPlatform::LinuxDesktop
    }
}

pub fn default_extension_provider_catalog() -> ExtensionProviderCatalog {
    let contract = super::frozen_extension_host_contract();
    ExtensionProviderCatalog::new(current_extension_runtime_platform(), &contract)
}

pub fn platform_support_for_extension_platform(
    contract: &FrozenExtensionHostContract,
    platform: ExtensionPlatform,
) -> Option<ExtensionPlatformSupport> {
    contract
        .platforms
        .iter()
        .copied()
        .find(|support| support.platform == platform)
}

fn validate_provider_manifest(
    contract: &FrozenExtensionHostContract,
    runtime_gate: ExtensionPlatformSupport,
    manifest: &ExtensionProviderManifest,
) -> (ExtensionProviderLifecycleState, String) {
    if manifest.declared_abi_version_slug != contract.abi_version.slug() {
        return (
            ExtensionProviderLifecycleState::RejectedByContract,
            format!(
                "Rejected by contract: declared ABI {} is not admitted by {}",
                manifest.declared_abi_version_slug,
                contract.abi_version.slug()
            ),
        );
    }
    if !runtime_gate.native_loading_admitted {
        return (
            ExtensionProviderLifecycleState::BlockedByPlatform,
            format!(
                "Blocked by platform: {} does not admit native extension loading",
                runtime_gate.platform.slug()
            ),
        );
    }
    if manifest.target_platform != runtime_gate.platform {
        return (
            ExtensionProviderLifecycleState::BlockedByPlatform,
            format!(
                "Blocked by platform: declared for {} but runtime is {}",
                manifest.target_platform.slug(),
                runtime_gate.platform.slug()
            ),
        );
    }
    if manifest.packaging != runtime_gate.packaging {
        return (
            ExtensionProviderLifecycleState::BlockedByPlatform,
            format!(
                "Blocked by platform: {} packaging does not match {} runtime gate",
                manifest.packaging.slug(),
                runtime_gate.packaging.slug()
            ),
        );
    }
    if manifest.requires_rtd_contract && runtime_gate.rtd_contract == RtdHostContract::Unsupported {
        return (
            ExtensionProviderLifecycleState::BlockedByPlatform,
            format!(
                "Blocked by platform: {} does not admit RTD activation",
                runtime_gate.platform.slug()
            ),
        );
    }
    if manifest
        .requested_entry_points
        .iter()
        .any(|entry| !contract.abi_entry_points.contains(entry))
    {
        return (
            ExtensionProviderLifecycleState::RejectedByContract,
            format!(
                "Rejected by contract: {} requests ABI entry points outside the admitted subset",
                manifest.display_name
            ),
        );
    }
    if manifest
        .requested_excel_entry_points
        .iter()
        .any(|entry| !contract.admitted_excel_entry_points.contains(entry))
    {
        return (
            ExtensionProviderLifecycleState::RejectedByContract,
            format!(
                "Rejected by contract: {} requests Excel lifecycle hooks outside xlAutoOpen/xlAutoClose",
                manifest.display_name
            ),
        );
    }
    (
        ExtensionProviderLifecycleState::Admitted,
        format!(
            "Admitted by {} gate for {} packaging",
            runtime_gate.platform.slug(),
            runtime_gate.packaging.slug()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::{frozen_extension_host_contract, ExtensionPlatform};

    #[test]
    fn default_catalog_tracks_the_current_runtime_platform_gate() {
        let contract = frozen_extension_host_contract();
        let catalog = default_extension_provider_catalog();

        assert_eq!(
            catalog.runtime_platform,
            current_extension_runtime_platform()
        );
        assert_eq!(
            catalog.runtime_gate,
            platform_support_for_extension_platform(&contract, catalog.runtime_platform)
                .expect("runtime platform support")
        );
        assert!(catalog.providers.is_empty());
    }

    #[test]
    fn lifecycle_transitions_preserve_discovered_admitted_enabled_and_active_states() {
        let contract = frozen_extension_host_contract();
        let mut catalog =
            ExtensionProviderCatalog::new(ExtensionPlatform::WindowsDesktop, &contract);

        assert!(catalog.discover_provider(ExtensionProviderManifest::native(
            "discovered",
            "Discovered Provider",
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            &contract,
        )));
        assert!(catalog.discover_provider(ExtensionProviderManifest::native(
            "admitted",
            "Admitted Provider",
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            &contract,
        )));
        assert!(catalog.discover_provider(ExtensionProviderManifest::native(
            "enabled",
            "Enabled Provider",
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            &contract,
        )));
        assert!(catalog.discover_provider(ExtensionProviderManifest::native(
            "active",
            "Active Provider",
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            &contract,
        )));
        assert!(catalog.discover_provider(ExtensionProviderManifest::native(
            "blocked",
            "Blocked Provider",
            ExtensionPlatform::LinuxDesktop,
            ExtensionPackagingKind::SharedObject,
            &contract,
        )));

        assert!(catalog.validate_provider(&contract, "admitted"));
        assert!(catalog.validate_provider(&contract, "enabled"));
        assert!(catalog.set_provider_enabled("enabled", true));
        assert!(catalog.validate_provider(&contract, "active"));
        assert!(catalog.set_provider_enabled("active", true));
        assert!(catalog.set_provider_active("active", true));
        assert!(catalog.validate_provider(&contract, "blocked"));

        let report = catalog.load_report();
        assert_eq!(report.discovered_count, 1);
        assert_eq!(report.admitted_count, 1);
        assert_eq!(report.enabled_count, 1);
        assert_eq!(report.active_count, 1);
        assert_eq!(report.blocked_count, 1);
        assert_eq!(report.rejected_count, 0);
        assert_eq!(
            report.summary(),
            "windows-desktop · 1 discovered · 1 admitted · 1 enabled · 1 active · 1 blocked"
        );
    }

    #[test]
    fn validation_rejects_non_admitted_abi_claims_before_enablement() {
        let contract = frozen_extension_host_contract();
        let mut catalog =
            ExtensionProviderCatalog::new(ExtensionPlatform::WindowsDesktop, &contract);
        let mut manifest = ExtensionProviderManifest::native(
            "rejected",
            "Rejected Provider",
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            &contract,
        );
        manifest.declared_abi_version_slug = "onecalc-native-extension-abi-v1".to_string();

        assert!(catalog.discover_provider(manifest));
        assert!(catalog.validate_provider(&contract, "rejected"));
        assert!(!catalog.set_provider_enabled("rejected", true));
        assert_eq!(
            catalog
                .providers
                .get("rejected")
                .expect("rejected record")
                .lifecycle_state,
            ExtensionProviderLifecycleState::RejectedByContract
        );
    }
}
