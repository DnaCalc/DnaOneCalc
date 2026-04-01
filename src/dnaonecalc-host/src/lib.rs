pub mod artifact;
pub mod capsule;
pub mod conditional_formatting;
pub mod document;
pub mod extension;
pub mod function_surface;
pub mod observation;
pub mod retained;
pub mod runtime;
pub mod shell;
pub mod workspace;

use oxfml_core::{parse_formula, FormulaChannelKind, FormulaSourceRecord, ParseRequest};
use oxfunc_core::functions::sum::eval_sum_surface;
use oxfunc_core::resolver::{RefResolutionError, ReferenceResolver, ResolverCapabilities};
use oxfunc_core::value::{CallArgValue, EvalValue, ReferenceLike};
use oxreplay_abstractions::{LaneId, RegistryRef};
use oxreplay_core::{is_replay_ready, ReplayEvent, ReplayScenario};

pub use artifact::{
    stable_hash, ArtifactAttachmentRef, ArtifactEnvelope, ArtifactKind, ArtifactLineageRef,
    StableArtifactRef,
};
pub use capsule::{
    ImportedScenarioCapsule, PersistedScenarioCapsule, ScenarioCapsuleArtifactEntry,
    ScenarioCapsuleAttachmentEntry, ScenarioCapsuleManifest,
};
pub use conditional_formatting::{
    validate_isolated_conditional_formatting_carrier, ConditionalFormattingCarrierSummary,
    IsolatedConditionalFormattingCarrier,
};
pub use document::{
    read_spreadsheetml_document, write_spreadsheetml_document, DocumentArtifactIndexEntry,
    DocumentViewStateRecord, OneCalcDocumentRecord, PersistedOneCalcDocument,
};
pub use extension::{
    activate_windows_rtd_topic, advance_rtd_topic, admitted_extension_abi,
    extension_root_runtime_truth, invoke_extension_provider, load_extension_root,
    validate_extension_manifest, ActivatedRtdTopicSession, ExtensionAbiContract,
    ExtensionCapabilityTruth, ExtensionInvocationArgument, ExtensionInvocationSummary,
    ExtensionManifestLoadFailure, ExtensionProviderRuntimeTruth,
    ExtensionProviderEntrypoint, ExtensionProviderManifest, ExtensionRootLoadSummary,
    ExtensionRootRuntimeTruthSummary, ExtensionValidationResult, LoadedExtensionProvider,
    LinuxRtdRegistryEntry, LinuxRtdRegistrySummary, RegisteredExtensionBehavior,
    RegisteredExtensionFunction, RegisteredRtdTopic, RtdTopicUpdateSummary,
};
pub use function_surface::{
    AdmissionCategory, FunctionSurfaceCatalog, FunctionSurfaceEntry, SurfaceLabelSummary,
};
pub use observation::{
    invoke_live_windows_capture, load_observation_source_bundle, LoadedObservationSourceBundle,
    ObservationBridgePayload, ObservationCapturePayload, ObservationInterpretation,
    ObservationProvenancePayload, ObservationSurfaceDescriptor, ObservationSurfaceValue,
};
pub use retained::{
    CapabilityLedgerSnapshotRecord, CapabilityModeAvailabilityRecord, ComparisonMismatchRecord,
    ComparisonRecord, HandoffPacketRecord, HandoffReadinessRecord, ObservationRecord,
    PersistedCapabilitySnapshot, PersistedComparison, PersistedHandoffPacket, PersistedObservation,
    PersistedReplayCapture, PersistedScenarioRun, PersistedWitness, ReopenedScenarioRun,
    ReplayCaptureRecord, RetainedProvenanceRecord, RetainedRecalcContextRecord,
    RetainedScenarioStore, ScenarioRecord, ScenarioRunRecord, WitnessRecord,
};
pub use runtime::{
    CapabilitySnapshotDiffSummary, CompletionProposalSummary, DocumentRoundTripInvariantReport,
    DrivenRecalcSummary, DrivenRunComparison, DrivenSingleFormulaHost, FormulaEditPacketSummary,
    FormulaEditorSession, FormulaEvaluationSummary, FunctionHelpSummary, HostPacketKind,
    OneCalcHostProfile, OpenedCapabilitySnapshotSummary, OpenedHandoffPacketSummary,
    OpenedOneCalcWorkspace, OpenedReplayCaptureSummary, OpenedTwinCompareSummary,
    OpenedWitnessSummary, PlatformGate, PromotedScenarioIndex, PromotedScenarioIndexRow,
    RecalcContext, RecalcTriggerKind, ScenarioLibraryFilter, ScenarioLibrarySavedView,
    ReopenedDrivenSingleFormulaRun, ReopenedOneCalcDocument, RetainedRunDiffSummary,
    RetainedRunXRaySummary, RuntimeAdapter,
};
pub use shell::{launch_shell, launch_shell_with_formula, OneCalcShellApp};
pub use workspace::{
    read_workspace_manifest, write_workspace_manifest, OneCalcWorkspaceManifest,
    PersistedOneCalcWorkspace, WorkspaceDocumentEntry,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyProbeReport {
    pub formula_token: String,
    pub parse_token_count: usize,
    pub parse_diagnostic_count: usize,
    pub sum_result: f64,
    pub replay_ready: bool,
    pub replay_registry_ref_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyProbeError {
    SumDidNotReturnNumber,
}

struct NoReferenceResolver;

impl ReferenceResolver for NoReferenceResolver {
    fn capabilities(&self) -> ResolverCapabilities {
        ResolverCapabilities::permissive_local()
    }

    fn resolve_reference(
        &self,
        reference: &ReferenceLike,
    ) -> Result<EvalValue, RefResolutionError> {
        Err(RefResolutionError::UnresolvedReference {
            target: reference.target.clone(),
        })
    }
}

pub fn run_dependency_probe() -> Result<DependencyProbeReport, DependencyProbeError> {
    // This is a bounded seam probe, not ordinary host execution.
    let source = FormulaSourceRecord::new("onecalc.probe", 1, "=SUM(1,2,3)")
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
    let formula_token = source.formula_token().0;

    let parse = parse_formula(ParseRequest { source });
    let parse_token_count = parse.green_tree.full_fidelity_tokens.len();
    let parse_diagnostic_count = parse.green_tree.diagnostics.len();

    let args = [
        CallArgValue::Eval(EvalValue::Number(1.0)),
        CallArgValue::Eval(EvalValue::Number(2.0)),
        CallArgValue::Eval(EvalValue::Number(3.0)),
    ];
    let sum_result = match eval_sum_surface(&args, &NoReferenceResolver) {
        Ok(EvalValue::Number(number)) => number,
        Ok(_) | Err(_) => return Err(DependencyProbeError::SumDidNotReturnNumber),
    };

    let replay = ReplayScenario {
        scenario_id: "onecalc.probe.sum".to_string(),
        lane_id: LaneId("onecalc".to_string()),
        events: vec![ReplayEvent {
            event_id: "event-001".to_string(),
            source_label: "sum_probe".to_string(),
            normalized_family: "evaluation.sum".to_string(),
        }],
        source_metadata: None,
        registry_refs: vec![RegistryRef {
            family: "probe".to_string(),
            version: "v1".to_string(),
        }],
    };

    Ok(DependencyProbeReport {
        formula_token,
        parse_token_count,
        parse_diagnostic_count,
        sum_result,
        replay_ready: is_replay_ready(&replay),
        replay_registry_ref_count: replay.registry_refs.len(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn dependency_probe_uses_real_upstream_libraries() {
        let report = run_dependency_probe().expect("dependency probe should succeed");

        assert!(!report.formula_token.is_empty());
        assert!(report.parse_token_count > 0);
        assert_eq!(report.parse_diagnostic_count, 0);
        assert_eq!(report.sum_result, 6.0);
        assert!(report.replay_ready);
        assert_eq!(report.replay_registry_ref_count, 1);
    }

    #[test]
    fn runtime_adapter_exposes_profile_and_packet_register() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);

        assert_eq!(adapter.host_profile(), OneCalcHostProfile::OcH0);
        assert_eq!(adapter.host_profile().id(), "OC-H0");
        assert_eq!(
            adapter.packet_kinds(),
            &[
                HostPacketKind::FormulaEdit,
                HostPacketKind::EditAcceptRecalc,
                HostPacketKind::ReplayCapture,
            ]
        );
    }

    #[test]
    fn runtime_adapter_evaluates_admitted_formula_through_upstream_host() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
        let summary = adapter
            .evaluate_formula("=SUM(1,2,3)")
            .expect("admitted SUM formula should evaluate");

        assert!(!summary.formula_token.is_empty());
        assert_eq!(summary.worksheet_value_summary, "Number(6)");
        assert_eq!(summary.payload_summary, "Number");
        assert_eq!(summary.returned_presentation_hint_status, "none");
        assert_eq!(summary.host_style_state_status, "none");
        assert_eq!(summary.effective_display_status, "none");
        assert_eq!(summary.commit_decision_kind, "accepted");
    }

    #[test]
    fn extension_abi_contract_and_validation_keep_scope_narrow() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let contract = adapter.extension_abi_contract();
        let admitted_manifest = ExtensionProviderManifest {
            provider_id: "demo.sum.provider".to_string(),
            display_name: "Demo Sum Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "providers/demo_sum".to_string(),
        };
        let blocked_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "providers/demo_rtd".to_string(),
        };

        let admitted = adapter.validate_extension_manifest(&admitted_manifest);
        let blocked = adapter.validate_extension_manifest(&blocked_manifest);

        assert_eq!(contract.abi_id, "dnaonecalc.desktop.extension_abi");
        assert!(contract
            .admitted_capabilities
            .contains(&"host_managed_function_registration".to_string()));
        assert!(contract
            .excluded_capabilities
            .contains(&"rtd_provider".to_string()));
        assert!(admitted.admitted);
        assert_eq!(
            admitted.admitted_capabilities,
            vec!["host_managed_function_registration".to_string()]
        );
        assert!(!blocked.admitted);
        assert!(blocked
            .blocked_reasons
            .iter()
            .any(|reason| reason.contains("rtd_provider")));
    }

    #[test]
    fn extension_root_loading_surfaces_admitted_and_rejected_providers() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-extension-root-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let admitted_manifest = ExtensionProviderManifest {
            provider_id: "demo.sum.provider".to_string(),
            display_name: "Demo Sum Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "providers/demo_sum".to_string(),
        };
        let blocked_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "providers/demo_rtd".to_string(),
        };

        fs::write(
            root.join("demo-sum").join("provider.json"),
            serde_json::to_string_pretty(&admitted_manifest)
                .expect("admitted manifest should serialize"),
        )
        .expect("admitted manifest should write");
        fs::write(
            root.join("demo-sum").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: vec![RegisteredExtensionFunction {
                    function_name: "DEMOADD".to_string(),
                    behavior: RegisteredExtensionBehavior::SumNumbers,
                }],
                rtd_topics: Vec::new(),
            })
            .expect("entrypoint should serialize"),
        )
        .expect("entrypoint should write");
        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&blocked_manifest)
                .expect("blocked manifest should serialize"),
        )
        .expect("blocked manifest should write");

        let loaded = adapter
            .load_extension_root(&root)
            .expect("extension root should load");

        assert_eq!(loaded.discovered_manifest_count, 2);
        assert_eq!(loaded.admitted_providers.len(), 1);
        assert_eq!(loaded.rejected_providers.len(), 1);
        assert!(loaded.malformed_manifests.is_empty());
        assert_eq!(
            loaded.admitted_providers[0].manifest.provider_id,
            "demo.sum.provider"
        );
        assert_eq!(
            loaded.admitted_providers[0].validation.admitted_capabilities,
            vec!["host_managed_function_registration".to_string()]
        );
        assert_eq!(
            loaded.rejected_providers[0].manifest.provider_id,
            "demo.rtd.provider"
        );
        assert!(loaded.rejected_providers[0]
            .validation
            .blocked_reasons
            .iter()
            .any(|reason| reason.contains("rtd_provider")));
    }

    #[test]
    fn extension_provider_invocation_keeps_failures_explicit() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-extension-provider-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
        fs::create_dir_all(root.join("demo-fail")).expect("demo fail dir should create");
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let admitted_manifest = ExtensionProviderManifest {
            provider_id: "demo.sum.provider".to_string(),
            display_name: "Demo Sum Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "functions.json".to_string(),
        };
        let failing_manifest = ExtensionProviderManifest {
            provider_id: "demo.fail.provider".to_string(),
            display_name: "Demo Fail Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "functions.json".to_string(),
        };
        let blocked_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "functions.json".to_string(),
        };

        fs::write(
            root.join("demo-sum").join("provider.json"),
            serde_json::to_string_pretty(&admitted_manifest)
                .expect("admitted manifest should serialize"),
        )
        .expect("admitted manifest should write");
        fs::write(
            root.join("demo-sum").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: vec![RegisteredExtensionFunction {
                    function_name: "DEMOADD".to_string(),
                    behavior: RegisteredExtensionBehavior::SumNumbers,
                }],
                rtd_topics: Vec::new(),
            })
            .expect("sum entrypoint should serialize"),
        )
        .expect("sum entrypoint should write");

        fs::write(
            root.join("demo-fail").join("provider.json"),
            serde_json::to_string_pretty(&failing_manifest)
                .expect("failing manifest should serialize"),
        )
        .expect("failing manifest should write");
        fs::write(
            root.join("demo-fail").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: vec![RegisteredExtensionFunction {
                    function_name: "DEMOFAIL".to_string(),
                    behavior: RegisteredExtensionBehavior::AlwaysError {
                        message: "provider execution failed".to_string(),
                    },
                }],
                rtd_topics: Vec::new(),
            })
            .expect("failing entrypoint should serialize"),
        )
        .expect("failing entrypoint should write");

        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&blocked_manifest)
                .expect("blocked manifest should serialize"),
        )
        .expect("blocked manifest should write");

        let sum = adapter
            .invoke_extension_provider(
                &root,
                "demo.sum.provider",
                "DEMOADD",
                &[
                    ExtensionInvocationArgument::Number(1.0),
                    ExtensionInvocationArgument::Number(2.0),
                    ExtensionInvocationArgument::Number(3.0),
                ],
            )
            .expect("admitted provider should invoke");
        let fail = adapter
            .invoke_extension_provider(&root, "demo.fail.provider", "DEMOFAIL", &[])
            .expect("failing provider should still return explicit state");
        let blocked = adapter
            .invoke_extension_provider(&root, "demo.rtd.provider", "RTDDEMO", &[])
            .expect("blocked provider should still return explicit state");

        assert_eq!(sum.provider_state, "admitted");
        assert_eq!(sum.invocation_state, "returned");
        assert_eq!(sum.value_summary.as_deref(), Some("Number(6)"));
        assert_eq!(sum.failure_reason, None);

        assert_eq!(fail.provider_state, "admitted");
        assert_eq!(fail.invocation_state, "provider_error");
        assert_eq!(
            fail.failure_reason.as_deref(),
            Some("provider execution failed")
        );

        assert_eq!(blocked.provider_state, "rejected");
        assert_eq!(blocked.invocation_state, "blocked");
        assert!(blocked
            .failure_reason
            .as_deref()
            .expect("blocked provider should report a reason")
            .contains("rtd_provider"));
    }

    #[test]
    fn extension_runtime_truth_keeps_declared_rtd_state_visible() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-extension-rtd-truth-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let admitted_manifest = ExtensionProviderManifest {
            provider_id: "demo.sum.provider".to_string(),
            display_name: "Demo Sum Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "functions.json".to_string(),
        };
        let rtd_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "functions.json".to_string(),
        };

        fs::write(
            root.join("demo-sum").join("provider.json"),
            serde_json::to_string_pretty(&admitted_manifest)
                .expect("admitted manifest should serialize"),
        )
        .expect("admitted manifest should write");
        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&rtd_manifest)
                .expect("rtd manifest should serialize"),
        )
        .expect("rtd manifest should write");

        let truth = adapter
            .extension_root_runtime_truth(&root)
            .expect("extension truth should load");
        let sum_provider = truth
            .provider_truths
            .iter()
            .find(|provider| provider.provider_id == "demo.sum.provider")
            .expect("sum provider should be present");
        let rtd_provider = truth
            .provider_truths
            .iter()
            .find(|provider| provider.provider_id == "demo.rtd.provider")
            .expect("rtd provider should be present");

        assert_eq!(sum_provider.provider_state, "admitted");
        assert_eq!(
            sum_provider.capability_truths[0].runtime_state,
            "admitted"
        );
        assert_eq!(rtd_provider.provider_state, "declared_with_blocked_capabilities");
        assert_eq!(rtd_provider.capability_truths[0].capability_id, "rtd_provider");
        if std::env::consts::OS == "windows" {
            assert_eq!(rtd_provider.capability_truths[0].runtime_state, "declared_but_not_yet_admitted");
        } else {
            assert_eq!(
                rtd_provider.capability_truths[0].runtime_state,
                "blocked_by_platform"
            );
        }
    }

    #[test]
    fn windows_rtd_activation_runs_the_admitted_in_process_subset() {
        if std::env::consts::OS != "windows" {
            return;
        }

        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-windows-rtd-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let rtd_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "functions.json".to_string(),
        };

        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&rtd_manifest)
                .expect("rtd manifest should serialize"),
        )
        .expect("rtd manifest should write");
        fs::write(
            root.join("demo-rtd").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: Vec::new(),
                rtd_topics: vec![RegisteredRtdTopic {
                    topic_id: "PRICE".to_string(),
                    initial_value: "100.0".to_string(),
                    updates: vec!["101.5".to_string(), "103.0".to_string()],
                }],
            })
            .expect("rtd entrypoint should serialize"),
        )
        .expect("rtd entrypoint should write");

        let truth = adapter
            .extension_root_runtime_truth(&root)
            .expect("extension truth should load");
        let rtd_provider = truth
            .provider_truths
            .iter()
            .find(|provider| provider.provider_id == "demo.rtd.provider")
            .expect("rtd provider should be present");
        assert_eq!(
            rtd_provider.capability_truths[0].runtime_state,
            "admitted_windows_subset"
        );

        let mut session = adapter
            .activate_windows_rtd_topic(&root, "demo.rtd.provider", "PRICE")
            .expect("windows rtd topic should activate");
        assert_eq!(session.current_value, "100.0");
        assert_eq!(session.lifecycle_state, "active");

        let first = adapter.advance_rtd_topic(&mut session);
        let second = adapter.advance_rtd_topic(&mut session);
        let third = adapter.advance_rtd_topic(&mut session);

        assert_eq!(first.current_value, "101.5");
        assert_eq!(first.remaining_update_count, 1);
        assert_eq!(second.current_value, "103.0");
        assert_eq!(second.remaining_update_count, 0);
        assert_eq!(second.lifecycle_state, "active_final_value");
        assert_eq!(third.current_value, "103.0");
        assert_eq!(third.lifecycle_state, "active_no_pending_updates");
    }

    #[test]
    fn linux_rtd_registry_truth_keeps_cross_platform_gate_explicit() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-linux-rtd-registry-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let rtd_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "functions.json".to_string(),
        };

        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&rtd_manifest)
                .expect("rtd manifest should serialize"),
        )
        .expect("rtd manifest should write");
        fs::write(
            root.join("demo-rtd").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: Vec::new(),
                rtd_topics: vec![RegisteredRtdTopic {
                    topic_id: "PRICE".to_string(),
                    initial_value: "100.0".to_string(),
                    updates: vec!["101.5".to_string()],
                }],
            })
            .expect("rtd entrypoint should serialize"),
        )
        .expect("rtd entrypoint should write");

        let summary = adapter
            .linux_rtd_registry_truth(&root)
            .expect("linux rtd registry truth should load");

        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].provider_id, "demo.rtd.provider");
        assert_eq!(summary.entries[0].topic_ids, vec!["PRICE".to_string()]);
        if std::env::consts::OS == "linux" {
            assert_eq!(summary.gate_state, "admitted_design_floor");
        } else {
            assert_eq!(summary.gate_state, "blocked_on_host_platform");
        }
    }

    #[test]
    fn promoted_scenario_index_links_live_retained_evidence() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.index", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context.clone())
            .expect("edit-and-accept recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-scenario-index-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);
        let persisted = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "index-smoke",
            )
            .expect("retained run should persist");
        let replay = adapter
            .emit_replay_capture_for_run(&store, &persisted.run.scenario_run_id)
            .expect("replay capture should persist");

        let run_ref = persisted.run.envelope.stable_ref();
        let capability_snapshot_ref = persisted.capability_snapshot.snapshot.envelope.stable_ref();
        let witness_ref = StableArtifactRef {
            artifact_kind: ArtifactKind::Witness.id().to_string(),
            logical_id: "witness-index-smoke".to_string(),
            content_hash: None,
        };
        let comparison = ComparisonRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.comparison".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::Comparison.id().to_string(),
                logical_id: "comparison-index-smoke".to_string(),
                content_hash: stable_hash(&("comparison-index-smoke", persisted.run.scenario_run_id.as_str())),
                created_at_unix_ms: 1,
                created_by_build: "dnaonecalc-host@test".to_string(),
                host_profile_id: "OC-H1".to_string(),
                packet_kind: "twin_compare".to_string(),
                seam_pin_set_id: "onecalc:test".to_string(),
                capability_floor: "OC-H1".to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            comparison_id: "comparison-index-smoke".to_string(),
            left_artifact_ref: run_ref.clone(),
            right_artifact_ref: StableArtifactRef {
                artifact_kind: ArtifactKind::Observation.id().to_string(),
                logical_id: "observation-index-smoke".to_string(),
                content_hash: None,
            },
            comparison_envelope: vec!["value_surface".to_string()],
            mismatches: Vec::new(),
            reliability_badge: "narrow".to_string(),
            projection_limitations: vec!["observation_envelope_narrow".to_string()],
            explanation_refs: Vec::new(),
            witness_candidate_refs: vec![witness_ref.clone()],
        };
        let witness = WitnessRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.witness".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::Witness.id().to_string(),
                logical_id: witness_ref.logical_id.clone(),
                content_hash: stable_hash(&("witness-index-smoke", persisted.run.scenario_run_id.as_str())),
                created_at_unix_ms: 2,
                created_by_build: "dnaonecalc-host@test".to_string(),
                host_profile_id: "OC-H1".to_string(),
                packet_kind: "explain".to_string(),
                seam_pin_set_id: "onecalc:test".to_string(),
                capability_floor: "OC-H1".to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            witness_id: witness_ref.logical_id.clone(),
            scenario_id: persisted.scenario.scenario_id.clone(),
            left_run_ref: run_ref.clone(),
            right_run_ref: run_ref.clone(),
            explain_floor: "retained_diff".to_string(),
            explanation_lines: vec!["stable".to_string()],
            blocked_dimensions: vec!["excel_observation".to_string()],
            emitted_at_unix_ms: 2,
        };
        let handoff = HandoffPacketRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.handoff_packet".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::HandoffPacket.id().to_string(),
                logical_id: "handoff-index-smoke".to_string(),
                content_hash: stable_hash(&("handoff-index-smoke", persisted.run.scenario_run_id.as_str())),
                created_at_unix_ms: 3,
                created_by_build: "dnaonecalc-host@test".to_string(),
                host_profile_id: "OC-H1".to_string(),
                packet_kind: "handoff".to_string(),
                seam_pin_set_id: "onecalc:test".to_string(),
                capability_floor: "OC-H1".to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            handoff_id: "handoff-index-smoke".to_string(),
            scenario_id: persisted.scenario.scenario_id.clone(),
            source_run_ref: run_ref.clone(),
            witness_ref: witness_ref.clone(),
            capability_snapshot_ref: capability_snapshot_ref.clone(),
            requested_action_kind: "widen_compare".to_string(),
            target_lane: "OxXlObs".to_string(),
            expected_behavior: "stable".to_string(),
            observed_behavior: "stable".to_string(),
            supporting_artifact_refs: vec![replay.capture.envelope.stable_ref()],
            reliability_state: "narrow".to_string(),
            status: "ready".to_string(),
            readiness: vec![HandoffReadinessRecord {
                item_id: "capability_snapshot".to_string(),
                satisfied: true,
            }],
            emitted_at_unix_ms: 3,
        };

        store
            .persist_comparison(&comparison)
            .expect("comparison should persist");
        store
            .persist_witness(&witness)
            .expect("witness should persist");
        store
            .persist_handoff_packet(&handoff)
            .expect("handoff should persist");

        let index = adapter
            .build_promoted_scenario_index(&store)
            .expect("promoted scenario index should build");

        assert_eq!(index.rows.len(), 1);
        let row = &index.rows[0];
        assert_eq!(
            row.row_id,
            format!("promoted-scenario:{}", persisted.scenario.scenario_id)
        );
        assert_eq!(row.scenario_id, persisted.scenario.scenario_id);
        assert_eq!(row.latest_run_id, persisted.run.scenario_run_id);
        assert_eq!(row.replay_capture_ids, vec![replay.capture.replay_capture_id.clone()]);
        assert_eq!(row.comparison_ids, vec!["comparison-index-smoke".to_string()]);
        assert_eq!(row.witness_ids, vec!["witness-index-smoke".to_string()]);
        assert_eq!(row.handoff_ids, vec!["handoff-index-smoke".to_string()]);
    }

    #[test]
    fn scenario_library_filters_and_saved_views_use_promoted_index_fields() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let index = PromotedScenarioIndex {
            rows: vec![
                PromotedScenarioIndexRow {
                    row_id: "promoted-scenario:one".to_string(),
                    scenario_id: "scenario-one".to_string(),
                    scenario_slug: "one".to_string(),
                    latest_run_id: "run-one".to_string(),
                    host_profile_id: "OC-H1".to_string(),
                    runtime_platform: std::env::consts::OS.to_string(),
                    formula_text: "=SUM(1,2,3)".to_string(),
                    worksheet_value_summary: "Number(6)".to_string(),
                    replay_capture_ids: vec!["replay-one".to_string()],
                    comparison_ids: vec!["comparison-one".to_string()],
                    witness_ids: vec!["witness-one".to_string()],
                    handoff_ids: vec!["handoff-one".to_string()],
                },
                PromotedScenarioIndexRow {
                    row_id: "promoted-scenario:two".to_string(),
                    scenario_id: "scenario-two".to_string(),
                    scenario_slug: "two".to_string(),
                    latest_run_id: "run-two".to_string(),
                    host_profile_id: "OC-H1".to_string(),
                    runtime_platform: std::env::consts::OS.to_string(),
                    formula_text: "=ABS(-3)".to_string(),
                    worksheet_value_summary: "Number(3)".to_string(),
                    replay_capture_ids: vec!["replay-two".to_string()],
                    comparison_ids: Vec::new(),
                    witness_ids: Vec::new(),
                    handoff_ids: Vec::new(),
                },
            ],
        };
        let filter = ScenarioLibraryFilter {
            host_profile_ids: vec!["OC-H1".to_string()],
            runtime_platform: Some(std::env::consts::OS.to_string()),
            replay_required: Some(true),
            comparison_required: Some(true),
            witness_required: Some(true),
            handoff_required: Some(true),
        };
        let filtered = adapter.apply_scenario_library_filter(&index, &filter);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].scenario_id, "scenario-one");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-scenario-library-view-test-{}.json",
            std::process::id()
        ));
        let view = ScenarioLibrarySavedView {
            view_id: "with-evidence".to_string(),
            display_name: "With Evidence".to_string(),
            filter: filter.clone(),
        };
        adapter
            .save_scenario_library_view(&root, &view)
            .expect("saved view should persist");
        let reopened = adapter
            .read_scenario_library_view(&root)
            .expect("saved view should reopen");

        assert_eq!(reopened, view);
    }

    #[test]
    fn h1_driven_host_runs_edit_accept_manual_and_forced_recalc_with_explicit_context() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");

        let edit_summary = adapter
            .edit_accept_recalc(
                &mut host,
                "=SUM(1,2,3)",
                RecalcContext::edit_accept(Some(46_000.0), Some(0.25)),
            )
            .expect("edit-and-accept recalc should succeed");
        assert_eq!(edit_summary.host_profile_id, "OC-H1");
        assert_eq!(edit_summary.trigger_kind, "edit_accept");
        assert_eq!(edit_summary.packet_kind, "edit_accept_recalc");
        assert_eq!(edit_summary.formula_text_version, 2);
        assert_eq!(
            edit_summary.structure_context_version,
            "onecalc:single_formula:h1"
        );
        assert_eq!(edit_summary.evaluation.worksheet_value_summary, "Number(6)");

        let manual_summary = adapter
            .manual_recalc(&mut host, RecalcContext::manual(Some(46_000.0), Some(0.25)))
            .expect("manual recalc should succeed");
        assert_eq!(manual_summary.trigger_kind, "manual");
        assert_eq!(manual_summary.packet_kind, "manual_recalc");
        assert_eq!(manual_summary.formula_text_version, 2);
        assert_eq!(
            manual_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );

        let forced_summary = adapter
            .forced_recalc(&mut host, RecalcContext::forced(Some(46_000.0), Some(0.25)))
            .expect("forced recalc should succeed");
        assert_eq!(forced_summary.trigger_kind, "forced");
        assert_eq!(forced_summary.packet_kind, "forced_recalc");
        assert_eq!(forced_summary.formula_text_version, 2);
        assert_eq!(
            forced_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );
    }

    #[test]
    fn h0_profile_rejects_h1_driven_host_model() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
        let error = adapter
            .new_driven_single_formula_host("onecalc.h1", "=SUM(1,2,3)")
            .expect_err("OC-H0 should reject the driven host model");

        assert!(error.contains("does not admit the driven single-formula host model"));
    }

    #[test]
    fn h1_runs_persist_scenario_and_scenario_run_and_reopen_through_runtime() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context.clone())
            .expect("edit-and-accept recalc should succeed");

        let store_root = std::env::temp_dir().join(format!(
            "dnaonecalc-h1-retained-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&store_root);
        let store = RetainedScenarioStore::new(&store_root);
        let persisted = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM baseline",
            )
            .expect("retained scenario and run should persist");

        assert!(persisted.scenario_path.exists());
        assert!(persisted.run_path.exists());
        assert!(persisted.capability_snapshot.snapshot_path.exists());
        assert_eq!(persisted.scenario.scenario_slug, "sum-baseline");
        assert_eq!(persisted.scenario.host_profile_id, "OC-H1");
        assert_eq!(persisted.run.scenario_id, persisted.scenario.scenario_id);
        assert_eq!(persisted.scenario.envelope.artifact_kind, "scenario");
        assert_eq!(persisted.run.envelope.artifact_kind, "scenario_run");
        assert_eq!(
            persisted
                .capability_snapshot
                .snapshot
                .envelope
                .artifact_kind,
            "capability_ledger_snapshot"
        );
        assert_eq!(
            persisted.run.scenario_ref.logical_id,
            persisted.scenario.scenario_id
        );
        assert_eq!(persisted.run.envelope.lineage_refs.len(), 1);
        assert_eq!(
            persisted.run.envelope.lineage_refs[0]
                .artifact_ref
                .logical_id,
            persisted.scenario.scenario_id
        );
        assert_eq!(
            persisted
                .run
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("run should point to the capability snapshot")
                .logical_id,
            persisted
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );
        assert_eq!(
            persisted
                .scenario
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("scenario should point to the capability snapshot")
                .logical_id,
            persisted
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );

        let mut reopened = adapter
            .reopen_driven_scenario_run(&store, &persisted.run.scenario_run_id)
            .expect("retained run should reopen");
        assert_eq!(
            reopened.retained.scenario.formula_text,
            persisted.scenario.formula_text
        );
        assert_eq!(
            reopened.driven_host.formula_text(),
            persisted.scenario.formula_text
        );
        assert_eq!(
            reopened.driven_host.formula_text_version(),
            recalc_summary.formula_text_version
        );

        let reopened_summary = adapter
            .manual_recalc(
                &mut reopened.driven_host,
                RecalcContext::manual(Some(46_000.0), Some(0.25)),
            )
            .expect("reopened driven host should recalc");
        assert_eq!(reopened_summary.host_profile_id, "OC-H1");
        assert_eq!(
            reopened_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );

        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn retained_runs_emit_replay_capture_outputs_and_open_them_through_oxreplay() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.replay", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context)
            .expect("replay source recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-replay-capture-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);
        let persisted = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM replay",
            )
            .expect("retained run should persist");
        let replay_capture = adapter
            .emit_replay_capture_for_run(&store, &persisted.run.scenario_run_id)
            .expect("replay capture should emit");

        assert!(replay_capture.capture_path.exists());
        assert!(replay_capture.replay_path.exists());
        assert_eq!(
            replay_capture.capture.envelope.artifact_kind,
            "replay_capture"
        );
        assert_eq!(
            replay_capture.capture.scenario_run_ref.logical_id,
            persisted.run.scenario_run_id
        );

        let reopened_run = store
            .read_run(&persisted.run.scenario_run_id)
            .expect("run should be rewritten with replay capture ref");
        assert_eq!(
            reopened_run
                .replay_capture_ref
                .as_ref()
                .expect("replay capture ref should be set")
                .logical_id,
            replay_capture.capture.replay_capture_id
        );

        let opened = adapter
            .open_replay_capture(&store, &replay_capture.capture.replay_capture_id)
            .expect("replay capture should open");
        assert!(opened.replay_ready);
        assert_eq!(
            opened.replay_floor,
            "cap.C1.replay_valid (normalized_replay_open)"
        );
        assert!(opened.event_count >= 2);
        assert_eq!(opened.view_family, "normalized_replay");
        assert_eq!(
            opened.projection_source_artifact_family,
            "runtime_formula_result"
        );
        assert_eq!(
            opened.projection_phase.as_deref(),
            Some("CommittedOrRejected")
        );
        assert!(opened.projection_alias.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retained_run_xray_and_diff_surfaces_open_on_real_retained_data() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.xray", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let store_root =
            std::env::temp_dir().join(format!("dnaonecalc-xray-diff-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&store_root);
        let store = RetainedScenarioStore::new(&store_root);

        let first_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let first_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", first_context)
            .expect("first xray run should succeed");
        let first = adapter
            .persist_driven_scenario_run(&store, &host, &first_context, &first_summary, "SUM xray")
            .expect("first retained run should persist");
        let first_replay = adapter
            .emit_replay_capture_for_run(&store, &first.run.scenario_run_id)
            .expect("first replay capture should emit");

        let second_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
        let second_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,4)", second_context)
            .expect("second xray run should succeed");
        let second = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &second_context,
                &second_summary,
                "SUM xray",
            )
            .expect("second retained run should persist");
        let second_replay = adapter
            .emit_replay_capture_for_run(&store, &second.run.scenario_run_id)
            .expect("second replay capture should emit");

        let xray = adapter
            .open_retained_run_xray(&store, &first.run.scenario_run_id)
            .expect("retained X-Ray should open");
        assert_eq!(xray.scenario_id, first.scenario.scenario_id);
        assert_eq!(xray.scenario_run_id, first.run.scenario_run_id);
        assert_eq!(
            xray.replay_capture_id.as_deref(),
            Some(first_replay.capture.replay_capture_id.as_str())
        );
        assert_eq!(
            xray.replay_floor.as_deref(),
            Some("cap.C1.replay_valid (normalized_replay_open)")
        );
        assert_eq!(
            xray.replay_projection_source_artifact_family.as_deref(),
            Some("runtime_formula_result")
        );
        assert_eq!(
            xray.replay_projection_phase.as_deref(),
            Some("CommittedOrRejected")
        );
        assert!(xray.replay_projection_alias.is_none());
        assert_eq!(
            xray.formatting_truth_plane,
            "returned_presentation_hint+host_style_state=>effective_display"
        );
        assert!(xray
            .conditional_formatting_scope
            .contains("Conditional Formatting: admitted="));
        assert!(xray
            .blocked_dimensions
            .contains(&"conditional_formatting_rules_not_attached_to_retained_run".to_string()));

        let diff = adapter
            .diff_retained_run_xray(
                &store,
                &first.run.scenario_run_id,
                &second.run.scenario_run_id,
            )
            .expect("retained diff should open");
        assert!(diff.same_scenario);
        assert!(diff.formula_text_changed);
        assert!(!diff.worksheet_value_match);
        assert!(diff.capability_snapshot_changed);
        assert!(diff.replay_pair_openable);
        assert_eq!(
            diff.formatting_truth_plane,
            "returned_presentation_hint+host_style_state=>effective_display"
        );
        assert!(diff
            .conditional_formatting_scope
            .contains("Conditional Formatting: admitted="));
        assert!(diff
            .blocked_dimensions
            .contains(&"conditional_formatting_rules_not_attached_to_retained_run".to_string()));
        assert_eq!(diff.diff_floor, "retained_artifact_direct_diff");
        assert!(!first_replay.capture.replay_capture_id.is_empty());
        assert!(!second_replay.capture.replay_capture_id.is_empty());

        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn retained_witness_generation_uses_real_diff_state_and_keeps_blocked_dimensions_explicit() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.witness", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let store_root =
            std::env::temp_dir().join(format!("dnaonecalc-witness-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&store_root);
        let store = RetainedScenarioStore::new(&store_root);

        let left_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let left_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", left_context)
            .expect("left witness run should succeed");
        let left = adapter
            .persist_driven_scenario_run(&store, &host, &left_context, &left_summary, "SUM witness")
            .expect("left retained run should persist");
        adapter
            .emit_replay_capture_for_run(&store, &left.run.scenario_run_id)
            .expect("left replay capture should emit");

        let right_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
        let right_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,4)", right_context)
            .expect("right witness run should succeed");
        let right = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &right_context,
                &right_summary,
                "SUM witness",
            )
            .expect("right retained run should persist");
        adapter
            .emit_replay_capture_for_run(&store, &right.run.scenario_run_id)
            .expect("right replay capture should emit");

        let persisted = adapter
            .generate_retained_witness(
                &store,
                &left.run.scenario_run_id,
                &right.run.scenario_run_id,
            )
            .expect("witness should generate");
        assert!(persisted.witness_path.exists());
        assert_eq!(persisted.witness.envelope.artifact_kind, "witness");
        assert_eq!(
            persisted.witness.left_run_ref.logical_id,
            left.run.scenario_run_id
        );
        assert_eq!(
            persisted.witness.right_run_ref.logical_id,
            right.run.scenario_run_id
        );

        let opened = adapter
            .open_witness(&store, &persisted.witness.witness_id)
            .expect("witness should open");
        assert_eq!(opened.explain_floor, "retained_diff_explain_summary");
        assert!(opened
            .explanation_lines
            .iter()
            .any(|line| line.contains("formula_text_changed=true")));
        assert!(opened
            .blocked_dimensions
            .contains(&"distill_not_integrated".to_string()));
        assert!(opened
            .blocked_dimensions
            .contains(&"no_oxreplay_explain_adapter_invocation_yet".to_string()));
        assert!(opened.replay_projection_aliases.is_empty());

        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn handoff_packets_are_generated_from_retained_evidence_and_gated_by_capability_truth() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.handoff", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let store_root =
            std::env::temp_dir().join(format!("dnaonecalc-handoff-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&store_root);
        let store = RetainedScenarioStore::new(&store_root);

        let left_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let left_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", left_context)
            .expect("left handoff run should succeed");
        let left = adapter
            .persist_driven_scenario_run(&store, &host, &left_context, &left_summary, "SUM handoff")
            .expect("left retained run should persist");
        adapter
            .emit_replay_capture_for_run(&store, &left.run.scenario_run_id)
            .expect("left replay capture should emit");

        let right_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
        let right_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,4)", right_context)
            .expect("right handoff run should succeed");
        let right = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &right_context,
                &right_summary,
                "SUM handoff",
            )
            .expect("right retained run should persist");
        adapter
            .emit_replay_capture_for_run(&store, &right.run.scenario_run_id)
            .expect("right replay capture should emit");

        let witness = adapter
            .generate_retained_witness(
                &store,
                &left.run.scenario_run_id,
                &right.run.scenario_run_id,
            )
            .expect("witness should generate");
        let handoff = adapter
            .generate_handoff_packet(&store, &witness.witness.witness_id)
            .expect("handoff should generate");
        assert!(handoff.handoff_path.exists());
        assert_eq!(handoff.handoff.envelope.artifact_kind, "handoff_packet");
        assert_eq!(handoff.handoff.status, "ready");

        let opened = adapter
            .open_handoff_packet(&store, &handoff.handoff.handoff_id)
            .expect("handoff should open");
        assert_eq!(opened.target_lane, "OxReplay/DnaOneCalc");
        assert_eq!(opened.requested_action_kind, "clarify_contract");
        assert_eq!(opened.status, "ready");
        assert!(opened.readiness.iter().all(|item| item.satisfied));
        assert_eq!(
            opened.capability_snapshot_id,
            left.run
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("left run should have capability snapshot")
                .logical_id
        );
        assert!(opened.replay_projection_alias.is_none());
        assert_eq!(
            opened.replay_projection_phase.as_deref(),
            Some("CommittedOrRejected")
        );

        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn runtime_adapter_emits_capability_snapshot_from_executable_truth() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let snapshot = adapter
            .emit_capability_snapshot("edit_accept_recalc", None)
            .expect("capability snapshot should emit");

        assert_eq!(
            snapshot.envelope.artifact_kind,
            "capability_ledger_snapshot"
        );
        assert_eq!(snapshot.host_kind, "dnaonecalc-host");
        assert_eq!(snapshot.capability_floor, "OC-H1");
        assert!(snapshot
            .packet_kind_register
            .contains(&"edit_accept_recalc".to_string()));
        assert!(snapshot
            .dependency_set
            .contains(&"oxreplay_core".to_string()));
        assert!(snapshot
            .dependency_set
            .contains(&"oxreplay_abstractions".to_string()));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "DNA-only" && mode.state == "available"));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Excel-observed" && mode.state == "available"));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Twin compare" && mode.state == "available"));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Replay" && mode.state == "available"));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Diff" && mode.state == "available"));
        assert!(!snapshot
            .provisional_seams
            .contains(&"observation_path_not_integrated".to_string()));
    }

    #[test]
    fn capability_snapshot_open_and_diff_read_persisted_immutable_truth() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-capability-diff-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);

        let left = adapter
            .persist_capability_snapshot(&store, "formula_edit", None)
            .expect("left snapshot should persist");
        let right = adapter
            .persist_capability_snapshot(
                &store,
                "edit_accept_recalc",
                Some(&left.snapshot.capability_snapshot_id),
            )
            .expect("right snapshot should persist");

        let opened = adapter
            .open_capability_snapshot(&store, &right.snapshot.capability_snapshot_id)
            .expect("right snapshot should open");
        let diff = adapter
            .diff_capability_snapshots(
                &store,
                &left.snapshot.capability_snapshot_id,
                &right.snapshot.capability_snapshot_id,
            )
            .expect("snapshot diff should open");

        assert_eq!(
            opened.diff_base_snapshot_id,
            Some(left.snapshot.capability_snapshot_id.clone())
        );
        assert!(opened
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Replay"));
        assert!(diff.packet_kinds_added.is_empty());
        assert!(!diff.function_surface_policy_changed);
        assert_eq!(diff.diff_floor, "immutable_capability_snapshot_diff");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn observation_artifact_persists_from_upstream_source_bundle() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-observation-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("OxXlObs")
            .join("states/excel/xlobs_capture_values_formulae_001");

        let persisted = adapter
            .persist_observation_from_existing_source(&store, &source_root)
            .expect("observation artifact should persist");

        assert!(persisted.observation_path.exists());
        assert_eq!(persisted.observation.envelope.artifact_kind, "observation");
        assert_eq!(
            persisted.observation.capture.surfaces[0].surface.surface_id,
            "sheet1_a1_value"
        );
        assert!(persisted
            .observation
            .lossiness
            .contains(&"normalized_replay_projection_is_lossy".to_string()));
        assert_eq!(
            persisted.observation.provenance.bridge.bridge_kind,
            "external_process"
        );

        let reopened = store
            .read_observation(&persisted.observation.observation_id)
            .expect("observation artifact should reopen");
        assert_eq!(reopened.scenario_id, "xlobs_capture_values_formulae_001");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn twin_compare_artifact_persists_and_opens_on_real_run_and_observation() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-twin-compare-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.compare.obs", "=SUM(10,20,12)")
            .expect("compare host should initialize");
        let context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(10,20,12)", context)
            .expect("compare recalc should succeed");
        let retained = adapter
            .persist_driven_scenario_run(&store, &host, &context, &summary, "Twin compare")
            .expect("retained run should persist");
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("OxXlObs")
            .join("states/excel/xlobs_capture_values_formulae_001");
        let observation = adapter
            .persist_observation_from_existing_source(&store, &source_root)
            .expect("observation should persist");

        let comparison = adapter
            .compare_run_with_observation(
                &store,
                &retained.run.scenario_run_id,
                &observation.observation.observation_id,
            )
            .expect("comparison should persist");
        let opened = adapter
            .open_twin_compare(&store, &comparison.comparison.comparison_id)
            .expect("compare view should open");

        assert_eq!(opened.reliability_badge, "direct");
        assert_eq!(
            opened.comparison_envelope,
            vec!["worksheet_value".to_string(), "formula_text".to_string()]
        );
        assert!(opened
            .mismatch_lines
            .iter()
            .any(|line| line.contains("worksheet_value:match")));
        assert!(opened
            .mismatch_lines
            .iter()
            .any(|line| line.contains("formula_text:mismatch")));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn widening_request_handoff_emits_from_real_compare_state() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root =
            std::env::temp_dir().join(format!("dnaonecalc-widening-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.widening", "=SUM(10,20,12)")
            .expect("widening host should initialize");
        let context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(10,20,12)", context)
            .expect("widening recalc should succeed");
        let retained = adapter
            .persist_driven_scenario_run(&store, &host, &context, &summary, "Widening request")
            .expect("retained run should persist");
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("OxXlObs")
            .join("states/excel/xlobs_capture_values_formulae_001");
        let observation = adapter
            .persist_observation_from_existing_source(&store, &source_root)
            .expect("observation should persist");
        let comparison = adapter
            .compare_run_with_observation(
                &store,
                &retained.run.scenario_run_id,
                &observation.observation.observation_id,
            )
            .expect("comparison should persist");

        let handoff = adapter
            .generate_observation_widening_handoff(&store, &comparison.comparison.comparison_id)
            .expect("widening handoff should persist");
        let opened = adapter
            .open_handoff_packet(&store, &handoff.handoff.handoff_id)
            .expect("handoff should open");

        assert_eq!(opened.target_lane, "OxXlObs/DnaOneCalc");
        assert_eq!(opened.requested_action_kind, "widen_observation_envelope");
        assert_eq!(opened.status, "ready");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retained_h1_runs_compare_version_to_version_in_code() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.compare", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let store_root =
            std::env::temp_dir().join(format!("dnaonecalc-h1-compare-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&store_root);
        let store = RetainedScenarioStore::new(&store_root);

        let first_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let first_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", first_context)
            .expect("first run should succeed");
        let first = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &first_context,
                &first_summary,
                "SUM compare",
            )
            .expect("first run should persist");

        let second_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
        let second_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,4)", second_context)
            .expect("second run should succeed");
        let second = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &second_context,
                &second_summary,
                "SUM compare",
            )
            .expect("second run should persist");

        let comparison = adapter
            .compare_retained_driven_runs(
                &store,
                &first.run.scenario_run_id,
                &second.run.scenario_run_id,
            )
            .expect("retained driven runs should compare");

        assert!(comparison.same_scenario);
        assert!(comparison.formula_version_changed);
        assert!(comparison.formula_text_changed);
        assert!(!comparison.worksheet_value_match);
        assert_eq!(comparison.reliability_badge, "direct");

        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn spreadsheetml_document_round_trip_reopens_into_the_h1_host() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.document", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context)
            .expect("document recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-document-roundtrip-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(root.join("retained"));
        let persisted_run = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM document",
            )
            .expect("retained run should persist");
        let document_path = root.join("sum-document.xml");
        let persisted_document = adapter
            .persist_isolated_document(
                &document_path,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM document",
                Some(&persisted_run),
            )
            .expect("isolated document should persist");

        assert!(persisted_document.document_path.exists());
        assert_eq!(
            persisted_document.document.document_scope,
            "isolated_single_formula_instance"
        );
        assert_eq!(
            persisted_document.document.persistence_format_id,
            "spreadsheetml2003.onecalc.single_instance.v1"
        );
        assert_eq!(persisted_document.document.artifact_index.len(), 3);

        let mut reopened = adapter
            .reopen_isolated_document(&persisted_document.document_path)
            .expect("isolated document should reopen");
        assert_eq!(
            reopened.document.formula_stable_id,
            persisted_document.document.formula_stable_id
        );
        assert_eq!(
            reopened.driven_host.formula_text(),
            persisted_document.document.formula_text
        );
        assert_eq!(
            reopened.driven_host.formula_text_version(),
            persisted_document.document.formula_text_version
        );

        let reopened_summary = adapter
            .manual_recalc(
                &mut reopened.driven_host,
                RecalcContext::manual(Some(46_000.0), Some(0.25)),
            )
            .expect("reopened document should recalc");
        assert_eq!(reopened_summary.host_profile_id, "OC-H1");
        assert_eq!(
            reopened_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn spreadsheetml_document_round_trip_preserves_identity_and_current_formatting_invariants() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.document.invariants", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context)
            .expect("document recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-document-invariants-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(root.join("retained"));
        let persisted_run = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM document invariant",
            )
            .expect("retained run should persist");
        let persisted_document = adapter
            .persist_isolated_document(
                root.join("sum-document-invariants.xml"),
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM document invariant",
                Some(&persisted_run),
            )
            .expect("isolated document should persist");

        let invariants = adapter
            .verify_isolated_document_roundtrip_invariants(&persisted_document)
            .expect("document invariants should survive round-trip");

        assert!(invariants.document_id_preserved);
        assert!(invariants.formula_identity_preserved);
        assert!(invariants.structure_context_preserved);
        assert!(invariants.library_context_snapshot_ref_preserved);
        assert!(invariants.artifact_index_preserved);
        assert!(invariants.effective_display_status_preserved);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scenario_capsule_export_and_intake_preserve_lineage_and_capability_refs() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.capsule", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context)
            .expect("capsule source recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-scenario-capsule-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let export_store = RetainedScenarioStore::new(root.join("retained-export"));
        let import_store = RetainedScenarioStore::new(root.join("retained-import"));
        let persisted_run = adapter
            .persist_driven_scenario_run(
                &export_store,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM capsule",
            )
            .expect("retained run should persist");
        let exported = adapter
            .export_scenario_capsule(
                &export_store,
                root.join("capsule"),
                &[&persisted_run.run.scenario_run_id],
            )
            .expect("ScenarioCapsule export should succeed");

        assert_eq!(
            exported.manifest.root_scenario_id,
            persisted_run.scenario.scenario_id
        );
        assert_eq!(exported.manifest.lineage_roots.len(), 1);
        assert_eq!(exported.manifest.capability_snapshot_refs.len(), 1);
        assert_eq!(
            exported.manifest.capability_snapshot_refs[0].logical_id,
            persisted_run
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );

        let imported = adapter
            .import_scenario_capsule(&import_store, &exported.capsule_root)
            .expect("ScenarioCapsule intake should succeed");

        assert_eq!(imported.imported_paths.len(), 3);
        assert!(imported.deduped_paths.is_empty());
        assert!(imported.conflict_paths.is_empty());
        assert!(imported.manifest_copy_path.exists());

        let imported_run_body = fs::read_to_string(
            import_store
                .root()
                .join("imports")
                .join("scenario-runs")
                .join(format!("{}.json", persisted_run.run.scenario_run_id)),
        )
        .expect("imported run should exist");
        let imported_run: ScenarioRunRecord =
            serde_json::from_str(&imported_run_body).expect("imported run should deserialize");
        assert_eq!(
            imported_run
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("imported run should preserve capability snapshot ref")
                .logical_id,
            persisted_run
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );
        assert_eq!(imported_run.envelope.lineage_refs.len(), 1);
        assert_eq!(
            imported_run.envelope.lineage_refs[0]
                .artifact_ref
                .logical_id,
            persisted_run.scenario.scenario_id
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn workspace_manifest_groups_multiple_isolated_documents_without_merging_them() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root =
            std::env::temp_dir().join(format!("dnaonecalc-workspace-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(root.join("retained"));

        let mut first_host = adapter
            .new_driven_single_formula_host("onecalc.h1.workspace.left", "=SUM(1,2,3)")
            .expect("first workspace host should initialize");
        let first_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let first_summary = adapter
            .edit_accept_recalc(&mut first_host, "=SUM(1,2,3)", first_context)
            .expect("first workspace recalc should succeed");
        let first_retained = adapter
            .persist_driven_scenario_run(
                &store,
                &first_host,
                &first_context,
                &first_summary,
                "SUM workspace left",
            )
            .expect("first retained run should persist");
        let first_document = adapter
            .persist_isolated_document(
                root.join("left.xml"),
                &first_host,
                &first_context,
                &first_summary,
                "SUM workspace left",
                Some(&first_retained),
            )
            .expect("first document should persist");

        let mut second_host = adapter
            .new_driven_single_formula_host("onecalc.h1.workspace.right", "=SUM(1,2,4)")
            .expect("second workspace host should initialize");
        let second_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
        let second_summary = adapter
            .edit_accept_recalc(&mut second_host, "=SUM(1,2,4)", second_context)
            .expect("second workspace recalc should succeed");
        let second_retained = adapter
            .persist_driven_scenario_run(
                &store,
                &second_host,
                &second_context,
                &second_summary,
                "SUM workspace right",
            )
            .expect("second retained run should persist");
        let second_document = adapter
            .persist_isolated_document(
                root.join("right.xml"),
                &second_host,
                &second_context,
                &second_summary,
                "SUM workspace right",
                Some(&second_retained),
            )
            .expect("second document should persist");

        let persisted = adapter
            .persist_workspace_manifest(
                root.join("workspace.onecalc.json"),
                "Workspace Test",
                &[
                    &first_document.document_path,
                    &second_document.document_path,
                ],
            )
            .expect("workspace manifest should persist");
        let opened = adapter
            .open_workspace(&persisted.manifest_path)
            .expect("workspace should reopen");

        assert_eq!(opened.reopened_documents.len(), 2);
        assert_eq!(
            opened.manifest.active_document_id,
            first_document.document.document_id
        );
        assert_ne!(
            opened.reopened_documents[0].document.document_id,
            opened.reopened_documents[1].document.document_id
        );
        assert_eq!(
            opened.reopened_documents[0].document.formula_text,
            "=SUM(1,2,3)"
        );
        assert_eq!(
            opened.reopened_documents[1].document.formula_text,
            "=SUM(1,2,4)"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
