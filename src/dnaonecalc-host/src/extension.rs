use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedExtensionProvider {
    pub manifest_path: String,
    pub manifest: ExtensionProviderManifest,
    pub validation: ExtensionValidationResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionManifestLoadFailure {
    pub manifest_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionRootLoadSummary {
    pub extension_root: String,
    pub discovered_manifest_count: usize,
    pub admitted_providers: Vec<LoadedExtensionProvider>,
    pub rejected_providers: Vec<LoadedExtensionProvider>,
    pub malformed_manifests: Vec<ExtensionManifestLoadFailure>,
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

pub fn load_extension_root(
    extension_root: impl AsRef<Path>,
    host_profile_id: &str,
    platform_gate_id: &str,
) -> Result<ExtensionRootLoadSummary, String> {
    let extension_root = extension_root.as_ref();
    if !extension_root.exists() {
        return Err(format!(
            "extension root {} does not exist",
            extension_root.display()
        ));
    }
    if !extension_root.is_dir() {
        return Err(format!(
            "extension root {} is not a directory",
            extension_root.display()
        ));
    }

    let manifest_paths = discover_manifest_paths(extension_root)?;
    let mut admitted_providers = Vec::new();
    let mut rejected_providers = Vec::new();
    let mut malformed_manifests = Vec::new();

    for manifest_path in &manifest_paths {
        let body = match fs::read_to_string(manifest_path) {
            Ok(body) => body,
            Err(error) => {
                malformed_manifests.push(ExtensionManifestLoadFailure {
                    manifest_path: manifest_path.display().to_string(),
                    reason: format!("read failed: {error}"),
                });
                continue;
            }
        };

        let manifest = match serde_json::from_str::<ExtensionProviderManifest>(&body) {
            Ok(manifest) => manifest,
            Err(error) => {
                malformed_manifests.push(ExtensionManifestLoadFailure {
                    manifest_path: manifest_path.display().to_string(),
                    reason: format!("json parse failed: {error}"),
                });
                continue;
            }
        };

        let validation = validate_extension_manifest(&manifest, host_profile_id, platform_gate_id);
        let loaded = LoadedExtensionProvider {
            manifest_path: manifest_path.display().to_string(),
            manifest,
            validation,
        };

        if loaded.validation.admitted {
            admitted_providers.push(loaded);
        } else {
            rejected_providers.push(loaded);
        }
    }

    Ok(ExtensionRootLoadSummary {
        extension_root: extension_root.display().to_string(),
        discovered_manifest_count: manifest_paths.len(),
        admitted_providers,
        rejected_providers,
        malformed_manifests,
    })
}

fn discover_manifest_paths(extension_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut manifest_paths = Vec::new();
    let root_manifest = extension_root.join("provider.json");
    if root_manifest.is_file() {
        manifest_paths.push(root_manifest);
    }

    let mut entries = fs::read_dir(extension_root)
        .map_err(|error| format!("failed to enumerate extension root: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to enumerate extension root: {error}"))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("provider.json");
        if manifest_path.is_file() {
            manifest_paths.push(manifest_path);
        }
    }

    Ok(manifest_paths)
}
