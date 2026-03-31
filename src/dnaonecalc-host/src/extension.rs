use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionProviderManifest {
    pub provider_id: String,
    pub display_name: String,
    pub abi_version: String,
    pub host_profile_ids: Vec<String>,
    pub platform_gate_ids: Vec<String>,
    pub declared_capabilities: Vec<String>,
    pub entrypoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionAbiContract {
    pub abi_id: String,
    pub abi_version: String,
    pub host_profile_id: String,
    pub platform_gate_id: String,
    pub admitted_capabilities: Vec<String>,
    pub excluded_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionValidationResult {
    pub provider_id: String,
    pub admitted: bool,
    pub admitted_capabilities: Vec<String>,
    pub blocked_capabilities: Vec<String>,
    pub blocked_reasons: Vec<String>,
}

pub fn admitted_extension_abi(
    host_profile_id: &str,
    platform_gate_id: &str,
) -> ExtensionAbiContract {
    ExtensionAbiContract {
        abi_id: "dnaonecalc.desktop.extension_abi".to_string(),
        abi_version: "v1".to_string(),
        host_profile_id: host_profile_id.to_string(),
        platform_gate_id: platform_gate_id.to_string(),
        admitted_capabilities: vec!["host_managed_function_registration".to_string()],
        excluded_capabilities: vec![
            "worksheet_register_call_semantics".to_string(),
            "rtd_provider".to_string(),
            "vba_bridge".to_string(),
            "browser_host".to_string(),
        ],
    }
}

pub fn validate_extension_manifest(
    manifest: &ExtensionProviderManifest,
    host_profile_id: &str,
    platform_gate_id: &str,
) -> ExtensionValidationResult {
    let contract = admitted_extension_abi(host_profile_id, platform_gate_id);
    let mut blocked_reasons = Vec::new();
    let mut admitted_capabilities = Vec::new();
    let mut blocked_capabilities = Vec::new();

    if manifest.abi_version != contract.abi_version {
        blocked_reasons.push(format!(
            "abi_version {} does not match admitted {}",
            manifest.abi_version, contract.abi_version
        ));
    }
    if !manifest.host_profile_ids.iter().any(|item| item == host_profile_id) {
        blocked_reasons.push(format!(
            "host profile {} is not declared by provider",
            host_profile_id
        ));
    }
    if !manifest
        .platform_gate_ids
        .iter()
        .any(|item| item == platform_gate_id)
    {
        blocked_reasons.push(format!(
            "platform gate {} is not declared by provider",
            platform_gate_id
        ));
    }

    for capability in &manifest.declared_capabilities {
        if contract
            .admitted_capabilities
            .iter()
            .any(|admitted| admitted == capability)
        {
            admitted_capabilities.push(capability.clone());
        } else {
            blocked_capabilities.push(capability.clone());
            blocked_reasons.push(format!("capability {} is not admitted in OneCalc v1", capability));
        }
    }

    ExtensionValidationResult {
        provider_id: manifest.provider_id.clone(),
        admitted: blocked_reasons.is_empty() && !admitted_capabilities.is_empty(),
        admitted_capabilities,
        blocked_capabilities,
        blocked_reasons,
    }
}
