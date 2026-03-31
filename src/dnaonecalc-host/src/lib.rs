pub mod artifact;
pub mod capsule;
pub mod document;
pub mod function_surface;
pub mod retained;
pub mod runtime;
pub mod shell;

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
pub use document::{
    read_spreadsheetml_document, write_spreadsheetml_document, DocumentArtifactIndexEntry,
    DocumentViewStateRecord, OneCalcDocumentRecord, PersistedOneCalcDocument,
};
pub use function_surface::{
    AdmissionCategory, FunctionSurfaceCatalog, FunctionSurfaceEntry, SurfaceLabelSummary,
};
pub use retained::{
    CapabilityLedgerSnapshotRecord, CapabilityModeAvailabilityRecord, PersistedCapabilitySnapshot,
    PersistedScenarioRun, ReopenedScenarioRun, RetainedProvenanceRecord,
    RetainedRecalcContextRecord, RetainedScenarioStore, ScenarioRecord, ScenarioRunRecord,
};
pub use runtime::{
    CompletionProposalSummary, DocumentRoundTripInvariantReport, DrivenRecalcSummary,
    DrivenRunComparison, DrivenSingleFormulaHost, FormulaEditPacketSummary, FormulaEditorSession,
    FormulaEvaluationSummary, FunctionHelpSummary, HostPacketKind, OneCalcHostProfile,
    ParseSnapshot, PlatformGate, RecalcContext, RecalcTriggerKind, ReopenedDrivenSingleFormulaRun,
    ReopenedOneCalcDocument, RuntimeAdapter,
};
pub use shell::{launch_shell, launch_shell_with_formula, OneCalcShellApp};

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
        assert_eq!(summary.effective_display_status, "none");
        assert_eq!(summary.commit_decision_kind, "accepted");
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
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "DNA-only" && mode.state == "available"));
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
}
