use std::collections::BTreeMap;

use super::{
    default_extension_provider_catalog as default_provider_catalog,
    platform_support_for_extension_platform, ExtensionProviderCatalog,
    ExtensionProviderLifecycleState, FrozenExtensionHostContract, RtdHostContract,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtdTopicLifecycleState {
    Declared,
    ActivationPending,
    AwaitingValue,
    ValueReady,
    UpdatePending,
    CapabilityDenied,
    ConnectionFailed,
    ProviderError,
    ProviderNotReady,
    BlockedByPlatform,
    Disconnected,
}

impl RtdTopicLifecycleState {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::ActivationPending => "activation-pending",
            Self::AwaitingValue => "awaiting-value",
            Self::ValueReady => "value-ready",
            Self::UpdatePending => "update-pending",
            Self::CapabilityDenied => "capability-denied",
            Self::ConnectionFailed => "connection-failed",
            Self::ProviderError => "provider-error",
            Self::ProviderNotReady => "provider-not-ready",
            Self::BlockedByPlatform => "blocked-by-platform",
            Self::Disconnected => "disconnected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RtdTopicRequestKey(pub String);

impl RtdTopicRequestKey {
    pub fn from_request(request: &RtdTopicRequest) -> Self {
        Self(format!(
            "{}|{}|{}|{}",
            request.provider_id,
            request.prog_id,
            request.server_name,
            request.topic_strings.join("|")
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtdTopicRequest {
    pub provider_id: String,
    pub prog_id: String,
    pub server_name: String,
    pub topic_strings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtdTopicRequestOutcome {
    Value(String),
    NoValueYet,
    CapabilityDenied,
    ConnectionFailed,
    ProviderError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtdTopicRecord {
    pub request: RtdTopicRequest,
    pub lifecycle_state: RtdTopicLifecycleState,
    pub current_value_summary: Option<String>,
    pub status_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionRtdHostState {
    pub host_contract: RtdHostContract,
    pub topics: BTreeMap<RtdTopicRequestKey, RtdTopicRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtdHostReport {
    pub host_contract: RtdHostContract,
    pub activation_pending_count: usize,
    pub awaiting_value_count: usize,
    pub active_count: usize,
    pub update_pending_count: usize,
    pub blocked_count: usize,
    pub disconnected_count: usize,
}

impl RtdHostReport {
    pub fn summary(&self) -> String {
        format!(
            "{} · {} pending · {} waiting · {} active · {} blocked",
            self.host_contract.slug(),
            self.activation_pending_count + self.update_pending_count,
            self.awaiting_value_count,
            self.active_count,
            self.blocked_count,
        )
    }
}

impl ExtensionRtdHostState {
    pub fn new(contract: &FrozenExtensionHostContract, catalog: &ExtensionProviderCatalog) -> Self {
        let host_contract =
            platform_support_for_extension_platform(contract, catalog.runtime_platform)
                .expect("runtime platform must exist in frozen extension contract")
                .rtd_contract;
        Self {
            host_contract,
            topics: BTreeMap::new(),
        }
    }

    pub fn declare_topic(
        &mut self,
        provider_catalog: &ExtensionProviderCatalog,
        request: RtdTopicRequest,
    ) -> bool {
        let key = RtdTopicRequestKey::from_request(&request);
        let next_record = if self.host_contract == RtdHostContract::Unsupported {
            RtdTopicRecord {
                request,
                lifecycle_state: RtdTopicLifecycleState::BlockedByPlatform,
                current_value_summary: None,
                status_summary: "Blocked by platform: this host does not admit RTD activation"
                    .to_string(),
            }
        } else if !provider_is_rtd_ready(provider_catalog, &request.provider_id) {
            RtdTopicRecord {
                request,
                lifecycle_state: RtdTopicLifecycleState::ProviderNotReady,
                current_value_summary: None,
                status_summary:
                    "Provider not ready: discover, admit, and enable the RTD provider first"
                        .to_string(),
            }
        } else {
            RtdTopicRecord {
                request,
                lifecycle_state: RtdTopicLifecycleState::ActivationPending,
                current_value_summary: None,
                status_summary: format!("Activation pending through {}", self.host_contract.slug()),
            }
        };

        match self.topics.get(&key) {
            Some(existing) if existing == &next_record => false,
            _ => {
                self.topics.insert(key, next_record);
                true
            }
        }
    }

    pub fn resolve_topic(
        &mut self,
        key: &RtdTopicRequestKey,
        outcome: RtdTopicRequestOutcome,
    ) -> bool {
        let Some(record) = self.topics.get_mut(key) else {
            return false;
        };
        let (lifecycle_state, current_value_summary, status_summary) = match outcome {
            RtdTopicRequestOutcome::Value(summary) => (
                RtdTopicLifecycleState::ValueReady,
                Some(summary.clone()),
                format!(
                    "Active value projected through {}",
                    self.host_contract.slug()
                ),
            ),
            RtdTopicRequestOutcome::NoValueYet => (
                RtdTopicLifecycleState::AwaitingValue,
                None,
                "Topic connected; waiting for the first RTD value".to_string(),
            ),
            RtdTopicRequestOutcome::CapabilityDenied => (
                RtdTopicLifecycleState::CapabilityDenied,
                None,
                "Capability denied during RTD resolution".to_string(),
            ),
            RtdTopicRequestOutcome::ConnectionFailed => (
                RtdTopicLifecycleState::ConnectionFailed,
                None,
                "Connection failed during RTD activation".to_string(),
            ),
            RtdTopicRequestOutcome::ProviderError(detail) => (
                RtdTopicLifecycleState::ProviderError,
                None,
                format!("Provider error: {detail}"),
            ),
        };
        if record.lifecycle_state == lifecycle_state
            && record.current_value_summary == current_value_summary
            && record.status_summary == status_summary
        {
            return false;
        }
        record.lifecycle_state = lifecycle_state;
        record.current_value_summary = current_value_summary;
        record.status_summary = status_summary;
        true
    }

    pub fn mark_topic_update_pending(&mut self, key: &RtdTopicRequestKey) -> bool {
        let Some(record) = self.topics.get_mut(key) else {
            return false;
        };
        if record.lifecycle_state != RtdTopicLifecycleState::ValueReady {
            return false;
        }
        record.lifecycle_state = RtdTopicLifecycleState::UpdatePending;
        record.status_summary = "External RTD update pending recalc".to_string();
        true
    }

    pub fn disconnect_topic(&mut self, key: &RtdTopicRequestKey) -> bool {
        let Some(record) = self.topics.get_mut(key) else {
            return false;
        };
        if record.lifecycle_state == RtdTopicLifecycleState::Disconnected {
            return false;
        }
        record.lifecycle_state = RtdTopicLifecycleState::Disconnected;
        record.status_summary = "Topic disconnected from the RTD host".to_string();
        true
    }

    pub fn host_report(&self) -> RtdHostReport {
        let mut report = RtdHostReport {
            host_contract: self.host_contract,
            activation_pending_count: 0,
            awaiting_value_count: 0,
            active_count: 0,
            update_pending_count: 0,
            blocked_count: 0,
            disconnected_count: 0,
        };

        for record in self.topics.values() {
            match record.lifecycle_state {
                RtdTopicLifecycleState::ActivationPending => report.activation_pending_count += 1,
                RtdTopicLifecycleState::AwaitingValue => report.awaiting_value_count += 1,
                RtdTopicLifecycleState::ValueReady => report.active_count += 1,
                RtdTopicLifecycleState::UpdatePending => report.update_pending_count += 1,
                RtdTopicLifecycleState::CapabilityDenied
                | RtdTopicLifecycleState::ConnectionFailed
                | RtdTopicLifecycleState::ProviderError
                | RtdTopicLifecycleState::ProviderNotReady
                | RtdTopicLifecycleState::BlockedByPlatform => report.blocked_count += 1,
                RtdTopicLifecycleState::Disconnected => report.disconnected_count += 1,
                RtdTopicLifecycleState::Declared => {}
            }
        }

        report
    }
}

pub fn default_extension_rtd_host() -> ExtensionRtdHostState {
    let contract = super::frozen_extension_host_contract();
    let catalog = default_provider_catalog();
    ExtensionRtdHostState::new(&contract, &catalog)
}

fn provider_is_rtd_ready(catalog: &ExtensionProviderCatalog, provider_id: &str) -> bool {
    catalog.providers.get(provider_id).is_some_and(|record| {
        matches!(
            record.lifecycle_state,
            ExtensionProviderLifecycleState::Enabled | ExtensionProviderLifecycleState::Active
        ) && record.manifest.requires_rtd_contract
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::{
        frozen_extension_host_contract, ExtensionPackagingKind, ExtensionPlatform,
        ExtensionProviderManifest,
    };

    #[test]
    fn supported_rtd_hosts_preserve_activation_value_update_and_disconnect_states() {
        let contract = frozen_extension_host_contract();
        let mut catalog =
            ExtensionProviderCatalog::new(ExtensionPlatform::WindowsDesktop, &contract);
        let mut manifest = ExtensionProviderManifest::native(
            "rtd-provider",
            "RTD Provider",
            ExtensionPlatform::WindowsDesktop,
            ExtensionPackagingKind::Xll,
            &contract,
        );
        manifest.requires_rtd_contract = true;
        assert!(catalog.discover_provider(manifest));
        assert!(catalog.validate_provider(&contract, "rtd-provider"));
        assert!(catalog.set_provider_enabled("rtd-provider", true));

        let mut host = ExtensionRtdHostState::new(&contract, &catalog);
        let request = RtdTopicRequest {
            provider_id: "rtd-provider".to_string(),
            prog_id: "Vendor.Rtd".to_string(),
            server_name: "".to_string(),
            topic_strings: vec!["ticker".to_string(), "MSFT".to_string()],
        };
        let key = RtdTopicRequestKey::from_request(&request);

        assert!(host.declare_topic(&catalog, request));
        assert_eq!(
            host.topics.get(&key).expect("topic").lifecycle_state,
            RtdTopicLifecycleState::ActivationPending
        );
        assert!(host.resolve_topic(&key, RtdTopicRequestOutcome::NoValueYet));
        assert_eq!(
            host.topics.get(&key).expect("topic").lifecycle_state,
            RtdTopicLifecycleState::AwaitingValue
        );
        assert!(host.resolve_topic(&key, RtdTopicRequestOutcome::Value("42".to_string())));
        assert_eq!(
            host.topics.get(&key).expect("topic").lifecycle_state,
            RtdTopicLifecycleState::ValueReady
        );
        assert!(host.mark_topic_update_pending(&key));
        assert_eq!(
            host.topics.get(&key).expect("topic").lifecycle_state,
            RtdTopicLifecycleState::UpdatePending
        );
        assert!(host.disconnect_topic(&key));
        assert_eq!(
            host.topics.get(&key).expect("topic").lifecycle_state,
            RtdTopicLifecycleState::Disconnected
        );
    }

    #[test]
    fn unsupported_hosts_block_rtd_activation_explicitly() {
        let contract = frozen_extension_host_contract();
        let catalog = ExtensionProviderCatalog::new(ExtensionPlatform::BrowserWasm, &contract);
        let mut host = ExtensionRtdHostState::new(&contract, &catalog);
        let request = RtdTopicRequest {
            provider_id: "browser-provider".to_string(),
            prog_id: "Vendor.Rtd".to_string(),
            server_name: "".to_string(),
            topic_strings: vec!["ticker".to_string(), "MSFT".to_string()],
        };
        let key = RtdTopicRequestKey::from_request(&request);

        assert!(host.declare_topic(&catalog, request));
        assert_eq!(
            host.topics.get(&key).expect("topic").lifecycle_state,
            RtdTopicLifecycleState::BlockedByPlatform
        );
        assert_eq!(
            host.host_report().summary(),
            "unsupported · 0 pending · 0 waiting · 0 active · 1 blocked"
        );
    }

    #[test]
    fn supported_hosts_keep_provider_not_ready_visible() {
        let contract = frozen_extension_host_contract();
        let catalog = ExtensionProviderCatalog::new(ExtensionPlatform::WindowsDesktop, &contract);
        let mut host = ExtensionRtdHostState::new(&contract, &catalog);
        let request = RtdTopicRequest {
            provider_id: "missing-provider".to_string(),
            prog_id: "Vendor.Rtd".to_string(),
            server_name: "".to_string(),
            topic_strings: vec!["ticker".to_string(), "MSFT".to_string()],
        };
        let key = RtdTopicRequestKey::from_request(&request);

        assert!(host.declare_topic(&catalog, request));
        assert_eq!(
            host.topics.get(&key).expect("topic").lifecycle_state,
            RtdTopicLifecycleState::ProviderNotReady
        );
    }
}
