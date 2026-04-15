mod packaging_conformance;
mod rtd_runtime;
mod runtime_catalog;
mod upstream_pressure;

pub use packaging_conformance::{
    build_packaging_conformance_harnesses, verify_packaging_parity,
    ExtensionPackagingConformanceHarness, ExtensionPackagingParityReport,
};
pub use rtd_runtime::{
    default_extension_rtd_host, ExtensionRtdHostState, RtdHostReport, RtdTopicLifecycleState,
    RtdTopicRecord, RtdTopicRequest, RtdTopicRequestKey, RtdTopicRequestOutcome,
};
pub use runtime_catalog::{
    current_extension_runtime_platform, default_extension_provider_catalog,
    platform_support_for_extension_platform, ExtensionLoadReport, ExtensionProviderCatalog,
    ExtensionProviderLifecycleState, ExtensionProviderManifest, ExtensionProviderRecord,
};
pub use upstream_pressure::{
    default_extension_upstream_pressure_register, ExtensionUpstreamPressureOwner,
    ExtensionUpstreamPressureRecord, ExtensionUpstreamPressureRegister,
    ExtensionUpstreamPressureReport, ExtensionUpstreamPressureStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionAbiVersion {
    V0,
}

impl ExtensionAbiVersion {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::V0 => "onecalc-native-extension-abi-v0",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionPlatform {
    WindowsDesktop,
    LinuxDesktop,
    BrowserWasm,
}

impl ExtensionPlatform {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::WindowsDesktop => "windows-desktop",
            Self::LinuxDesktop => "linux-desktop",
            Self::BrowserWasm => "browser-wasm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionPackagingKind {
    Xll,
    SharedObject,
    None,
}

impl ExtensionPackagingKind {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Xll => "xll",
            Self::SharedObject => "shared-object",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtdHostContract {
    InProcessCom,
    MinimalComLikeRegistry,
    Unsupported,
}

impl RtdHostContract {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::InProcessCom => "in-process-com",
            Self::MinimalComLikeRegistry => "minimal-com-like-registry",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionPlatformSupport {
    pub platform: ExtensionPlatform,
    pub packaging: ExtensionPackagingKind,
    pub native_loading_admitted: bool,
    pub rtd_contract: RtdHostContract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiEntryPoint {
    QueryIdentityAndVersion,
    ExportFunctionCatalog,
    InvokeFunction,
    RegisterProvider,
    UnregisterProvider,
    QueryCapabilityFlags,
    ProviderOutcomeTransport,
    Shutdown,
}

impl AbiEntryPoint {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::QueryIdentityAndVersion => "query-identity-and-version",
            Self::ExportFunctionCatalog => "export-function-catalog",
            Self::InvokeFunction => "invoke-function",
            Self::RegisterProvider => "register-provider",
            Self::UnregisterProvider => "unregister-provider",
            Self::QueryCapabilityFlags => "query-capability-flags",
            Self::ProviderOutcomeTransport => "provider-outcome-transport",
            Self::Shutdown => "shutdown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExcelLifecycleEntryPoint {
    XlAutoOpen,
    XlAutoClose,
}

impl ExcelLifecycleEntryPoint {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::XlAutoOpen => "xlAutoOpen",
            Self::XlAutoClose => "xlAutoClose",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExcelHostCallSurface {
    Excel12,
    Xloper12Only,
    XlfRegisterForm1,
    XlfEvaluate,
    XlUdf,
    XlfRtd,
    RegistrationFlags,
}

impl ExcelHostCallSurface {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Excel12 => "Excel12",
            Self::Xloper12Only => "XLOPER12-only",
            Self::XlfRegisterForm1 => "xlfRegister-form-1",
            Self::XlfEvaluate => "xlfEvaluate",
            Self::XlUdf => "xlUDF",
            Self::XlfRtd => "xlfRtd",
            Self::RegistrationFlags => "registration-flags",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionLoadPhase {
    Discovery,
    AbiValidation,
    Enablement,
    Registration,
    Invocation,
    Teardown,
    RtdActivation,
}

impl ExtensionLoadPhase {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::AbiValidation => "abi-validation",
            Self::Enablement => "enablement",
            Self::Registration => "registration",
            Self::Invocation => "invocation",
            Self::Teardown => "teardown",
            Self::RtdActivation => "rtd-activation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionVisibilityState {
    Declared,
    Admitted,
    Promoted,
}

impl ExtensionVisibilityState {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Admitted => "admitted",
            Self::Promoted => "promoted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionVisibilitySurface {
    ExtensionCenter,
    FunctionHelpAndCompletion,
    ScenarioMetadata,
    ComparisonAndHandoffArtifacts,
}

impl ExtensionVisibilitySurface {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::ExtensionCenter => "extension-center",
            Self::FunctionHelpAndCompletion => "function-help-and-completion",
            Self::ScenarioMetadata => "scenario-metadata",
            Self::ComparisonAndHandoffArtifacts => "comparison-and-handoff-artifacts",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionNonClaim {
    BrowserNativeLoading,
    LegacyXloper,
    WorksheetRegisterIdAndCallSemantics,
    OxVbaPackagingParity,
}

impl ExtensionNonClaim {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::BrowserNativeLoading => "browser-native-loading",
            Self::LegacyXloper => "legacy-xloper",
            Self::WorksheetRegisterIdAndCallSemantics => "worksheet-register-id-call-semantics",
            Self::OxVbaPackagingParity => "oxvba-packaging-parity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OxVbaStatus {
    DesignInputOnly,
    EmbeddedHostRuntimePressure,
}

impl OxVbaStatus {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::DesignInputOnly => "design-input-only",
            Self::EmbeddedHostRuntimePressure => "embedded-host-runtime-pressure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSafetyModel {
    pub visibility_states: &'static [ExtensionVisibilityState],
    pub visible_surfaces: &'static [ExtensionVisibilitySurface],
    pub non_claims: &'static [ExtensionNonClaim],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenExtensionHostContract {
    pub abi_version: ExtensionAbiVersion,
    pub platforms: &'static [ExtensionPlatformSupport],
    pub abi_entry_points: &'static [AbiEntryPoint],
    pub admitted_excel_entry_points: &'static [ExcelLifecycleEntryPoint],
    pub host_call_subset: &'static [ExcelHostCallSurface],
    pub load_phases: &'static [ExtensionLoadPhase],
    pub safety_model: ExtensionSafetyModel,
    pub oxvba_status: OxVbaStatus,
}

const PLATFORM_SUPPORT: &[ExtensionPlatformSupport] = &[
    ExtensionPlatformSupport {
        platform: ExtensionPlatform::WindowsDesktop,
        packaging: ExtensionPackagingKind::Xll,
        native_loading_admitted: true,
        rtd_contract: RtdHostContract::InProcessCom,
    },
    ExtensionPlatformSupport {
        platform: ExtensionPlatform::LinuxDesktop,
        packaging: ExtensionPackagingKind::SharedObject,
        native_loading_admitted: true,
        rtd_contract: RtdHostContract::MinimalComLikeRegistry,
    },
    ExtensionPlatformSupport {
        platform: ExtensionPlatform::BrowserWasm,
        packaging: ExtensionPackagingKind::None,
        native_loading_admitted: false,
        rtd_contract: RtdHostContract::Unsupported,
    },
];

const ABI_ENTRY_POINTS: &[AbiEntryPoint] = &[
    AbiEntryPoint::QueryIdentityAndVersion,
    AbiEntryPoint::ExportFunctionCatalog,
    AbiEntryPoint::InvokeFunction,
    AbiEntryPoint::RegisterProvider,
    AbiEntryPoint::UnregisterProvider,
    AbiEntryPoint::QueryCapabilityFlags,
    AbiEntryPoint::ProviderOutcomeTransport,
    AbiEntryPoint::Shutdown,
];

const EXCEL_ENTRY_POINTS: &[ExcelLifecycleEntryPoint] = &[
    ExcelLifecycleEntryPoint::XlAutoOpen,
    ExcelLifecycleEntryPoint::XlAutoClose,
];

const HOST_CALL_SUBSET: &[ExcelHostCallSurface] = &[
    ExcelHostCallSurface::Excel12,
    ExcelHostCallSurface::Xloper12Only,
    ExcelHostCallSurface::XlfRegisterForm1,
    ExcelHostCallSurface::XlfEvaluate,
    ExcelHostCallSurface::XlUdf,
    ExcelHostCallSurface::XlfRtd,
    ExcelHostCallSurface::RegistrationFlags,
];

const LOAD_PHASES: &[ExtensionLoadPhase] = &[
    ExtensionLoadPhase::Discovery,
    ExtensionLoadPhase::AbiValidation,
    ExtensionLoadPhase::Enablement,
    ExtensionLoadPhase::Registration,
    ExtensionLoadPhase::Invocation,
    ExtensionLoadPhase::Teardown,
    ExtensionLoadPhase::RtdActivation,
];

const VISIBILITY_STATES: &[ExtensionVisibilityState] = &[
    ExtensionVisibilityState::Declared,
    ExtensionVisibilityState::Admitted,
    ExtensionVisibilityState::Promoted,
];

const VISIBILITY_SURFACES: &[ExtensionVisibilitySurface] = &[
    ExtensionVisibilitySurface::ExtensionCenter,
    ExtensionVisibilitySurface::FunctionHelpAndCompletion,
    ExtensionVisibilitySurface::ScenarioMetadata,
    ExtensionVisibilitySurface::ComparisonAndHandoffArtifacts,
];

const NON_CLAIMS: &[ExtensionNonClaim] = &[
    ExtensionNonClaim::BrowserNativeLoading,
    ExtensionNonClaim::LegacyXloper,
    ExtensionNonClaim::WorksheetRegisterIdAndCallSemantics,
    ExtensionNonClaim::OxVbaPackagingParity,
];

pub fn frozen_extension_host_contract() -> FrozenExtensionHostContract {
    FrozenExtensionHostContract {
        abi_version: ExtensionAbiVersion::V0,
        platforms: PLATFORM_SUPPORT,
        abi_entry_points: ABI_ENTRY_POINTS,
        admitted_excel_entry_points: EXCEL_ENTRY_POINTS,
        host_call_subset: HOST_CALL_SUBSET,
        load_phases: LOAD_PHASES,
        safety_model: ExtensionSafetyModel {
            visibility_states: VISIBILITY_STATES,
            visible_surfaces: VISIBILITY_SURFACES,
            non_claims: NON_CLAIMS,
        },
        oxvba_status: OxVbaStatus::DesignInputOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_extension_contract_preserves_platform_honesty_by_host() {
        let contract = frozen_extension_host_contract();

        assert_eq!(
            contract.abi_version.slug(),
            "onecalc-native-extension-abi-v0"
        );
        assert_eq!(contract.platforms.len(), 3);
        assert_eq!(
            contract.platforms[0],
            ExtensionPlatformSupport {
                platform: ExtensionPlatform::WindowsDesktop,
                packaging: ExtensionPackagingKind::Xll,
                native_loading_admitted: true,
                rtd_contract: RtdHostContract::InProcessCom,
            }
        );
        assert_eq!(
            contract.platforms[1],
            ExtensionPlatformSupport {
                platform: ExtensionPlatform::LinuxDesktop,
                packaging: ExtensionPackagingKind::SharedObject,
                native_loading_admitted: true,
                rtd_contract: RtdHostContract::MinimalComLikeRegistry,
            }
        );
        assert_eq!(
            contract.platforms[2],
            ExtensionPlatformSupport {
                platform: ExtensionPlatform::BrowserWasm,
                packaging: ExtensionPackagingKind::None,
                native_loading_admitted: false,
                rtd_contract: RtdHostContract::Unsupported,
            }
        );
    }

    #[test]
    fn frozen_extension_contract_preserves_admitted_excel_sdk_subset() {
        let contract = frozen_extension_host_contract();

        assert_eq!(
            contract
                .abi_entry_points
                .iter()
                .map(|entry| entry.slug())
                .collect::<Vec<_>>(),
            vec![
                "query-identity-and-version",
                "export-function-catalog",
                "invoke-function",
                "register-provider",
                "unregister-provider",
                "query-capability-flags",
                "provider-outcome-transport",
                "shutdown",
            ]
        );
        assert_eq!(
            contract
                .admitted_excel_entry_points
                .iter()
                .map(|entry| entry.slug())
                .collect::<Vec<_>>(),
            vec!["xlAutoOpen", "xlAutoClose"]
        );
        assert_eq!(
            contract
                .host_call_subset
                .iter()
                .map(|surface| surface.slug())
                .collect::<Vec<_>>(),
            vec![
                "Excel12",
                "XLOPER12-only",
                "xlfRegister-form-1",
                "xlfEvaluate",
                "xlUDF",
                "xlfRtd",
                "registration-flags",
            ]
        );
        assert!(contract
            .safety_model
            .non_claims
            .contains(&ExtensionNonClaim::BrowserNativeLoading));
    }

    #[test]
    fn frozen_extension_contract_keeps_visibility_and_non_claim_rules_explicit() {
        let contract = frozen_extension_host_contract();

        assert_eq!(
            contract
                .load_phases
                .iter()
                .map(|phase| phase.slug())
                .collect::<Vec<_>>(),
            vec![
                "discovery",
                "abi-validation",
                "enablement",
                "registration",
                "invocation",
                "teardown",
                "rtd-activation",
            ]
        );
        assert_eq!(
            contract
                .safety_model
                .visibility_states
                .iter()
                .map(|state| state.slug())
                .collect::<Vec<_>>(),
            vec!["declared", "admitted", "promoted"]
        );
        assert_eq!(
            contract
                .safety_model
                .visible_surfaces
                .iter()
                .map(|surface| surface.slug())
                .collect::<Vec<_>>(),
            vec![
                "extension-center",
                "function-help-and-completion",
                "scenario-metadata",
                "comparison-and-handoff-artifacts",
            ]
        );
        assert_eq!(
            contract
                .safety_model
                .non_claims
                .iter()
                .map(|claim| claim.slug())
                .collect::<Vec<_>>(),
            vec![
                "browser-native-loading",
                "legacy-xloper",
                "worksheet-register-id-call-semantics",
                "oxvba-packaging-parity",
            ]
        );
        assert_eq!(contract.oxvba_status.slug(), "design-input-only");
    }
}
