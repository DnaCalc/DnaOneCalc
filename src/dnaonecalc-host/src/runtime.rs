use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use oxfml_core::{
    apply_formula_edit, build_function_help_lookup_request, collect_completion_proposals,
    parse_formula, BindContext, CompletionRequest, EditFollowOnStage, EvaluationBackend,
    FormulaChannelKind, FormulaEditRequest, FormulaEditResult, FormulaSourceRecord,
    FormulaTextChangeRange, InMemoryLibraryContextProvider, ParseRequest, SingleFormulaHost,
    StructureContextVersion, TypedContextQueryBundle,
};
use oxfunc_core::value::EvalValue;
use oxfunc_core::xll_export_specs::lookup_function_meta_by_surface_name;
use oxreplay_abstractions::{CapabilityLevel, LaneId, RegistryRef};
use oxreplay_core::{
    is_replay_ready, load_replay_scenario_from_path, ReplayEvent, ReplayScenario, ReplayView,
};

use crate::artifact::{
    stable_hash, ArtifactAttachmentRef, ArtifactEnvelope, ArtifactKind, ArtifactLineageRef,
    StableArtifactRef,
};
use crate::capsule::{ImportedScenarioCapsule, PersistedScenarioCapsule};
use crate::document::{
    read_spreadsheetml_document, write_spreadsheetml_document, DocumentArtifactIndexEntry,
    DocumentViewStateRecord, OneCalcDocumentRecord, PersistedOneCalcDocument,
};
use crate::retained::{
    CapabilityLedgerSnapshotRecord, CapabilityModeAvailabilityRecord, HandoffPacketRecord,
    HandoffReadinessRecord, PersistedHandoffPacket, PersistedReplayCapture, PersistedScenarioRun,
    PersistedWitness, ReplayCaptureRecord, RetainedProvenanceRecord, RetainedRecalcContextRecord,
    RetainedScenarioStore, ScenarioRecord, ScenarioRunRecord, WitnessRecord,
};
use crate::{run_dependency_probe, DependencyProbeError, DependencyProbeReport};
use crate::{FunctionSurfaceCatalog, SurfaceLabelSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneCalcHostProfile {
    OcH0,
    OcH1,
    OcH2,
}

impl OneCalcHostProfile {
    pub const ALL: [Self; 3] = [Self::OcH0, Self::OcH1, Self::OcH2];

    pub const fn id(self) -> &'static str {
        match self {
            Self::OcH0 => "OC-H0",
            Self::OcH1 => "OC-H1",
            Self::OcH2 => "OC-H2",
        }
    }

    pub const fn supports_driven_host(self) -> bool {
        matches!(self, Self::OcH1 | Self::OcH2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPacketKind {
    FormulaEdit,
    EditAcceptRecalc,
    ManualRecalc,
    ForcedRecalc,
    ReplayCapture,
    ExtensionRegistration,
    RtdUpdate,
}

impl HostPacketKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::FormulaEdit => "formula_edit",
            Self::EditAcceptRecalc => "edit_accept_recalc",
            Self::ManualRecalc => "manual_recalc",
            Self::ForcedRecalc => "forced_recalc",
            Self::ReplayCapture => "replay_capture",
            Self::ExtensionRegistration => "extension_registration",
            Self::RtdUpdate => "rtd_update",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformGate {
    DesktopNativeOnly,
}

impl PlatformGate {
    pub const fn id(self) -> &'static str {
        match self {
            Self::DesktopNativeOnly => "desktop_native_only",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::DesktopNativeOnly => {
                "Desktop native host only; browser and secondary hosts are not admitted yet."
            }
        }
    }
}

const OC_H0_PACKET_KINDS: &[HostPacketKind] = &[
    HostPacketKind::FormulaEdit,
    HostPacketKind::EditAcceptRecalc,
    HostPacketKind::ReplayCapture,
];

const OC_H1_PACKET_KINDS: &[HostPacketKind] = &[
    HostPacketKind::FormulaEdit,
    HostPacketKind::EditAcceptRecalc,
    HostPacketKind::ManualRecalc,
    HostPacketKind::ForcedRecalc,
    HostPacketKind::ReplayCapture,
];

const OC_H2_PACKET_KINDS: &[HostPacketKind] = &[
    HostPacketKind::FormulaEdit,
    HostPacketKind::EditAcceptRecalc,
    HostPacketKind::ManualRecalc,
    HostPacketKind::ForcedRecalc,
    HostPacketKind::ReplayCapture,
    HostPacketKind::ExtensionRegistration,
    HostPacketKind::RtdUpdate,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSnapshot {
    pub formula_token: String,
    pub token_count: usize,
    pub diagnostic_count: usize,
}

#[derive(Debug, Clone)]
pub struct FormulaEditorSession {
    formula_stable_id: String,
    formula_text_version: u64,
    latest_result: Option<FormulaEditResult>,
}

impl FormulaEditorSession {
    pub fn new(formula_stable_id: impl Into<String>) -> Self {
        Self {
            formula_stable_id: formula_stable_id.into(),
            formula_text_version: 0,
            latest_result: None,
        }
    }

    pub fn latest_result(&self) -> Option<&FormulaEditResult> {
        self.latest_result.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditPacketSummary {
    pub formula_token: String,
    pub diagnostic_count: usize,
    pub text_change_range: Option<FormulaTextChangeRange>,
    pub reused_green_tree: bool,
    pub reused_red_projection: bool,
    pub reused_bound_formula: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEvaluationSummary {
    pub formula_token: String,
    pub worksheet_value_summary: String,
    pub payload_summary: String,
    pub returned_value_surface_kind: String,
    pub returned_presentation_hint_status: String,
    pub host_style_state_status: String,
    pub effective_display_status: String,
    pub commit_decision_kind: String,
    pub trace_event_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecalcTriggerKind {
    EditAccept,
    Manual,
    Forced,
}

impl RecalcTriggerKind {
    pub const fn packet_kind(self) -> HostPacketKind {
        match self {
            Self::EditAccept => HostPacketKind::EditAcceptRecalc,
            Self::Manual => HostPacketKind::ManualRecalc,
            Self::Forced => HostPacketKind::ForcedRecalc,
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::EditAccept => "edit_accept",
            Self::Manual => "manual",
            Self::Forced => "forced",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecalcContext {
    pub trigger_kind: RecalcTriggerKind,
    pub now_serial: Option<f64>,
    pub random_value: Option<f64>,
}

impl RecalcContext {
    pub const fn edit_accept(now_serial: Option<f64>, random_value: Option<f64>) -> Self {
        Self {
            trigger_kind: RecalcTriggerKind::EditAccept,
            now_serial,
            random_value,
        }
    }

    pub const fn manual(now_serial: Option<f64>, random_value: Option<f64>) -> Self {
        Self {
            trigger_kind: RecalcTriggerKind::Manual,
            now_serial,
            random_value,
        }
    }

    pub const fn forced(now_serial: Option<f64>, random_value: Option<f64>) -> Self {
        Self {
            trigger_kind: RecalcTriggerKind::Forced,
            now_serial,
            random_value,
        }
    }

    pub const fn packet_kind(self) -> HostPacketKind {
        self.trigger_kind.packet_kind()
    }
}

#[derive(Debug)]
pub struct DrivenSingleFormulaHost {
    host: SingleFormulaHost,
}

impl DrivenSingleFormulaHost {
    pub fn formula_stable_id(&self) -> &str {
        &self.host.formula_stable_id
    }

    pub fn formula_text(&self) -> &str {
        &self.host.formula_text
    }

    pub const fn formula_text_version(&self) -> u64 {
        self.host.formula_text_version
    }

    pub const fn formula_channel_kind(&self) -> FormulaChannelKind {
        self.host.formula_channel_kind
    }

    pub fn structure_context_version(&self) -> &str {
        &self.host.structure_context_version
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivenRecalcSummary {
    pub host_profile_id: String,
    pub trigger_kind: String,
    pub packet_kind: String,
    pub formula_text_version: u64,
    pub structure_context_version: String,
    pub evaluation: FormulaEvaluationSummary,
}

#[derive(Debug)]
pub struct ReopenedDrivenSingleFormulaRun {
    pub retained: crate::ReopenedScenarioRun,
    pub driven_host: DrivenSingleFormulaHost,
}

#[derive(Debug)]
pub struct ReopenedOneCalcDocument {
    pub document: OneCalcDocumentRecord,
    pub driven_host: DrivenSingleFormulaHost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRoundTripInvariantReport {
    pub document_id_preserved: bool,
    pub formula_identity_preserved: bool,
    pub structure_context_preserved: bool,
    pub library_context_snapshot_ref_preserved: bool,
    pub artifact_index_preserved: bool,
    pub effective_display_status_preserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedReplayCaptureSummary {
    pub replay_capture_id: String,
    pub scenario_id: String,
    pub replay_floor: String,
    pub replay_ready: bool,
    pub event_count: usize,
    pub registry_ref_count: usize,
    pub view_family: String,
    pub artifact_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedRunXRaySummary {
    pub scenario_id: String,
    pub scenario_run_id: String,
    pub formula_text: String,
    pub formula_text_version: u64,
    pub host_profile_id: String,
    pub packet_kind: String,
    pub worksheet_value_summary: String,
    pub payload_summary: String,
    pub effective_display_status: String,
    pub capability_snapshot_id: String,
    pub replay_capture_id: Option<String>,
    pub replay_floor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedRunDiffSummary {
    pub left_run_id: String,
    pub right_run_id: String,
    pub same_scenario: bool,
    pub formula_text_changed: bool,
    pub worksheet_value_match: bool,
    pub payload_match: bool,
    pub capability_snapshot_changed: bool,
    pub replay_pair_openable: bool,
    pub diff_floor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedWitnessSummary {
    pub witness_id: String,
    pub scenario_id: String,
    pub explain_floor: String,
    pub explanation_lines: Vec<String>,
    pub blocked_dimensions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedHandoffPacketSummary {
    pub handoff_id: String,
    pub target_lane: String,
    pub requested_action_kind: String,
    pub status: String,
    pub readiness: Vec<HandoffReadinessRecord>,
    pub capability_snapshot_id: String,
}

impl DocumentRoundTripInvariantReport {
    pub const fn all_preserved(&self) -> bool {
        self.document_id_preserved
            && self.formula_identity_preserved
            && self.structure_context_preserved
            && self.library_context_snapshot_ref_preserved
            && self.artifact_index_preserved
            && self.effective_display_status_preserved
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivenRunComparison {
    pub left_run_id: String,
    pub right_run_id: String,
    pub same_scenario: bool,
    pub formula_version_changed: bool,
    pub formula_text_changed: bool,
    pub worksheet_value_match: bool,
    pub payload_match: bool,
    pub returned_surface_match: bool,
    pub effective_display_match: bool,
    pub commit_decision_match: bool,
    pub comparison_envelope: String,
    pub reliability_badge: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionProposalSummary {
    pub proposal_kind: String,
    pub display_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionHelpSummary {
    pub display_name: String,
    pub display_signature: String,
    pub active_argument_index: usize,
    pub availability_summary: String,
    pub provisional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAdapter {
    host_profile: OneCalcHostProfile,
}

impl RuntimeAdapter {
    pub const fn new(host_profile: OneCalcHostProfile) -> Self {
        Self { host_profile }
    }

    pub const fn host_profile(&self) -> OneCalcHostProfile {
        self.host_profile
    }

    pub fn packet_kinds(&self) -> &'static [HostPacketKind] {
        match self.host_profile {
            OneCalcHostProfile::OcH0 => OC_H0_PACKET_KINDS,
            OneCalcHostProfile::OcH1 => OC_H1_PACKET_KINDS,
            OneCalcHostProfile::OcH2 => OC_H2_PACKET_KINDS,
        }
    }

    pub const fn platform_gate(&self) -> PlatformGate {
        PlatformGate::DesktopNativeOnly
    }

    pub fn parse_formula_source(&self, source: FormulaSourceRecord) -> ParseSnapshot {
        let formula_token = source.formula_token().0;
        let parse = parse_formula(ParseRequest { source });

        ParseSnapshot {
            formula_token,
            token_count: parse.green_tree.full_fidelity_tokens.len(),
            diagnostic_count: parse.green_tree.diagnostics.len(),
        }
    }

    pub fn dependency_probe(&self) -> Result<DependencyProbeReport, DependencyProbeError> {
        run_dependency_probe()
    }

    pub fn load_function_surface_catalog(&self) -> FunctionSurfaceCatalog {
        FunctionSurfaceCatalog::load_current()
    }

    pub fn function_surface_summary(&self) -> SurfaceLabelSummary {
        self.load_function_surface_catalog().label_summary()
    }

    pub fn evaluate_formula(
        &self,
        formula_text: impl Into<String>,
    ) -> Result<FormulaEvaluationSummary, String> {
        let catalog = self.load_function_surface_catalog();
        let snapshot = catalog.admitted_execution_snapshot();
        let provider = InMemoryLibraryContextProvider::new(snapshot);
        let query_bundle =
            TypedContextQueryBundle::new(None, None, None, Some(46_000.0), Some(0.25));
        let mut host = SingleFormulaHost::new("onecalc.eval", formula_text);
        let output = host.recalc_with_interfaces(
            EvaluationBackend::OxFuncBacked,
            query_bundle,
            Some(&provider),
        )?;

        Ok(summarize_host_output(output))
    }

    pub fn new_driven_single_formula_host(
        &self,
        formula_stable_id: impl Into<String>,
        formula_text: impl Into<String>,
    ) -> Result<DrivenSingleFormulaHost, String> {
        if !self.host_profile.supports_driven_host() {
            return Err(format!(
                "{} does not admit the driven single-formula host model",
                self.host_profile.id()
            ));
        }

        let mut host = SingleFormulaHost::new(formula_stable_id, formula_text);
        host.structure_context_version = "onecalc:single_formula:h1".to_string();
        Ok(DrivenSingleFormulaHost { host })
    }

    pub fn edit_accept_recalc(
        &self,
        driven_host: &mut DrivenSingleFormulaHost,
        formula_text: impl Into<String>,
        recalc_context: RecalcContext,
    ) -> Result<DrivenRecalcSummary, String> {
        if recalc_context.trigger_kind != RecalcTriggerKind::EditAccept {
            return Err("edit_accept_recalc requires RecalcTriggerKind::EditAccept".to_string());
        }

        driven_host.host.set_formula_text(formula_text);
        self.run_driven_recalc(driven_host, recalc_context)
    }

    pub fn manual_recalc(
        &self,
        driven_host: &mut DrivenSingleFormulaHost,
        recalc_context: RecalcContext,
    ) -> Result<DrivenRecalcSummary, String> {
        if recalc_context.trigger_kind != RecalcTriggerKind::Manual {
            return Err("manual_recalc requires RecalcTriggerKind::Manual".to_string());
        }

        self.run_driven_recalc(driven_host, recalc_context)
    }

    pub fn forced_recalc(
        &self,
        driven_host: &mut DrivenSingleFormulaHost,
        recalc_context: RecalcContext,
    ) -> Result<DrivenRecalcSummary, String> {
        if recalc_context.trigger_kind != RecalcTriggerKind::Forced {
            return Err("forced_recalc requires RecalcTriggerKind::Forced".to_string());
        }

        self.run_driven_recalc(driven_host, recalc_context)
    }

    pub fn persist_driven_scenario_run(
        &self,
        store: &RetainedScenarioStore,
        driven_host: &DrivenSingleFormulaHost,
        recalc_context: &RecalcContext,
        recalc_summary: &DrivenRecalcSummary,
        scenario_slug: impl Into<String>,
    ) -> Result<PersistedScenarioRun, String> {
        if !self.host_profile.supports_driven_host() {
            return Err(format!(
                "{} does not admit retained H1 runs",
                self.host_profile.id()
            ));
        }

        let scenario_slug = sanitize_slug(&scenario_slug.into());
        let stable_slug = sanitize_slug(driven_host.formula_stable_id());
        let scenario_id = format!("scenario-{stable_slug}");
        let executed_at_unix_ms = unix_time_millis()?;
        let scenario_run_id = format!(
            "scenario-run-{}-{}-{}",
            stable_slug,
            sanitize_slug(&recalc_summary.packet_kind),
            executed_at_unix_ms
        );
        let snapshot = self
            .load_function_surface_catalog()
            .admitted_execution_snapshot();
        let snapshot_ref = format!("{}@{}", snapshot.snapshot_id, snapshot.snapshot_version);
        let function_surface_policy_id = "onecalc:admitted_execution:supported+preview";
        let capability_snapshot = self
            .emit_capability_snapshot(recalc_summary.packet_kind.as_str(), Some(&snapshot_ref))?;
        let capability_snapshot_ref = capability_snapshot.envelope.stable_ref();
        let scenario_content_hash = stable_hash(&(
            driven_host.formula_stable_id(),
            driven_host.formula_text(),
            recalc_summary.formula_text_version,
            self.host_profile.id(),
            recalc_summary.packet_kind.as_str(),
        ));
        let scenario_envelope = ArtifactEnvelope {
            schema_id: "dnaonecalc.artifact.scenario".to_string(),
            schema_version: "v1".to_string(),
            artifact_kind: ArtifactKind::Scenario.id().to_string(),
            logical_id: scenario_id.clone(),
            content_hash: scenario_content_hash,
            created_at_unix_ms: executed_at_unix_ms,
            created_by_build: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
            host_profile_id: self.host_profile.id().to_string(),
            packet_kind: recalc_summary.packet_kind.clone(),
            seam_pin_set_id: "onecalc:ws-04:h1".to_string(),
            capability_floor: self.host_profile.id().to_string(),
            provisionality_state: "stable".to_string(),
            lineage_refs: Vec::new(),
            attachment_refs: Vec::<ArtifactAttachmentRef>::new(),
            capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
        };
        let scenario_ref = scenario_envelope.stable_ref();

        let scenario = ScenarioRecord {
            envelope: scenario_envelope,
            scenario_id: scenario_id.clone(),
            scenario_slug,
            formula_text: driven_host.formula_text().to_string(),
            formula_channel_kind: format!("{:?}", driven_host.formula_channel_kind()),
            host_profile_id: self.host_profile.id().to_string(),
            host_driving_packet_kind: recalc_summary.packet_kind.clone(),
            host_driving_block: "driven_single_formula_host".to_string(),
            recalc_context: RetainedRecalcContextRecord {
                trigger_kind: recalc_context.trigger_kind.id().to_string(),
                packet_kind: recalc_context.packet_kind().id().to_string(),
                now_serial: recalc_context.now_serial.map(|value| value.to_string()),
                random_value: recalc_context.random_value.map(|value| value.to_string()),
            },
            display_context: "returned_value_surface".to_string(),
            library_context_snapshot_ref: Some(snapshot_ref.clone()),
            function_surface_policy_id: function_surface_policy_id.to_string(),
            retained_notes: Vec::new(),
            provenance: RetainedProvenanceRecord {
                formula_stable_id: driven_host.formula_stable_id().to_string(),
                formula_text_version: recalc_summary.formula_text_version,
                structure_context_version: recalc_summary.structure_context_version.clone(),
            },
        };
        let result_surface_ref = StableArtifactRef {
            artifact_kind: ArtifactKind::ResultSurface.id().to_string(),
            logical_id: format!("result-surface-{}", recalc_summary.evaluation.formula_token),
            content_hash: Some(stable_hash(&(
                recalc_summary.evaluation.formula_token.as_str(),
                recalc_summary.evaluation.worksheet_value_summary.as_str(),
                recalc_summary.evaluation.payload_summary.as_str(),
            ))),
        };
        let candidate_ref = StableArtifactRef {
            artifact_kind: ArtifactKind::CandidateResult.id().to_string(),
            logical_id: format!("candidate-{}", recalc_summary.evaluation.formula_token),
            content_hash: Some(stable_hash(&(
                recalc_summary.evaluation.formula_token.as_str(),
                recalc_summary.evaluation.commit_decision_kind.as_str(),
                "candidate",
            ))),
        };
        let trace_ref = StableArtifactRef {
            artifact_kind: ArtifactKind::ExecutionTrace.id().to_string(),
            logical_id: format!("trace-{}", recalc_summary.evaluation.formula_token),
            content_hash: Some(stable_hash(&(
                recalc_summary.evaluation.formula_token.as_str(),
                recalc_summary.evaluation.trace_event_count,
            ))),
        };
        let commit_ref = if recalc_summary.evaluation.commit_decision_kind == "accepted" {
            Some(StableArtifactRef {
                artifact_kind: ArtifactKind::CommitDecision.id().to_string(),
                logical_id: format!("commit-{}", recalc_summary.evaluation.formula_token),
                content_hash: Some(stable_hash(&(
                    recalc_summary.evaluation.formula_token.as_str(),
                    "accepted",
                ))),
            })
        } else {
            None
        };
        let reject_ref = if recalc_summary.evaluation.commit_decision_kind == "rejected" {
            Some(StableArtifactRef {
                artifact_kind: ArtifactKind::RejectDecision.id().to_string(),
                logical_id: format!("reject-{}", recalc_summary.evaluation.formula_token),
                content_hash: Some(stable_hash(&(
                    recalc_summary.evaluation.formula_token.as_str(),
                    "rejected",
                ))),
            })
        } else {
            None
        };
        let run_content_hash = stable_hash(&(
            scenario_run_id.as_str(),
            recalc_summary.formula_text_version,
            recalc_summary.evaluation.formula_token.as_str(),
            recalc_summary.evaluation.worksheet_value_summary.as_str(),
            recalc_summary.evaluation.commit_decision_kind.as_str(),
        ));
        let run_envelope = ArtifactEnvelope {
            schema_id: "dnaonecalc.artifact.scenario_run".to_string(),
            schema_version: "v1".to_string(),
            artifact_kind: ArtifactKind::ScenarioRun.id().to_string(),
            logical_id: scenario_run_id.clone(),
            content_hash: run_content_hash,
            created_at_unix_ms: executed_at_unix_ms,
            created_by_build: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
            host_profile_id: self.host_profile.id().to_string(),
            packet_kind: recalc_summary.packet_kind.clone(),
            seam_pin_set_id: "onecalc:ws-04:h1".to_string(),
            capability_floor: self.host_profile.id().to_string(),
            provisionality_state: if recalc_summary.packet_kind == HostPacketKind::ForcedRecalc.id()
            {
                "forced".to_string()
            } else {
                "stable".to_string()
            },
            lineage_refs: vec![ArtifactLineageRef {
                relation: "scenario".to_string(),
                artifact_ref: scenario_ref.clone(),
            }],
            attachment_refs: Vec::<ArtifactAttachmentRef>::new(),
            capability_snapshot_ref: Some(capability_snapshot_ref),
        };
        let run = ScenarioRunRecord {
            envelope: run_envelope,
            scenario_run_id,
            scenario_id,
            scenario_ref,
            formula_text_version: recalc_summary.formula_text_version,
            formula_token: recalc_summary.evaluation.formula_token.clone(),
            authored_formula_text: driven_host.formula_text().to_string(),
            build_id: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
            runtime_platform: std::env::consts::OS.to_string(),
            seam_pin_set_id: "onecalc:ws-04:h1".to_string(),
            effective_capability_floor: self.host_profile.id().to_string(),
            result_surface_ref,
            candidate_ref: Some(candidate_ref),
            commit_ref,
            reject_ref,
            trace_ref: Some(trace_ref),
            replay_capture_ref: None,
            function_surface_effective_id: format!(
                "{}:{}",
                function_surface_policy_id, snapshot_ref
            ),
            projection_status: "direct".to_string(),
            provisionality_status: if recalc_summary.packet_kind
                == HostPacketKind::ForcedRecalc.id()
            {
                "forced".to_string()
            } else {
                "stable".to_string()
            },
            worksheet_value_summary: recalc_summary.evaluation.worksheet_value_summary.clone(),
            payload_summary: recalc_summary.evaluation.payload_summary.clone(),
            returned_value_surface_kind: recalc_summary
                .evaluation
                .returned_value_surface_kind
                .clone(),
            effective_display_status: recalc_summary.evaluation.effective_display_status.clone(),
            commit_decision_kind: recalc_summary.evaluation.commit_decision_kind.clone(),
            executed_at_unix_ms,
        };

        store.persist_scenario_and_run(&capability_snapshot, &scenario, &run)
    }

    pub fn reopen_driven_scenario_run(
        &self,
        store: &RetainedScenarioStore,
        scenario_run_id: &str,
    ) -> Result<ReopenedDrivenSingleFormulaRun, String> {
        let retained = store.reopen_run(scenario_run_id)?;
        let mut driven_host = self.new_driven_single_formula_host(
            retained.scenario.provenance.formula_stable_id.clone(),
            retained.run.authored_formula_text.clone(),
        )?;
        driven_host.host.formula_text_version = retained.run.formula_text_version;
        driven_host.host.structure_context_version = retained
            .scenario
            .provenance
            .structure_context_version
            .clone();
        driven_host.host.formula_channel_kind =
            parse_formula_channel_kind(&retained.scenario.formula_channel_kind)?;

        Ok(ReopenedDrivenSingleFormulaRun {
            retained,
            driven_host,
        })
    }

    pub fn persist_isolated_document(
        &self,
        path: impl AsRef<Path>,
        driven_host: &DrivenSingleFormulaHost,
        recalc_context: &RecalcContext,
        recalc_summary: &DrivenRecalcSummary,
        scenario_slug: impl Into<String>,
        retained_run: Option<&PersistedScenarioRun>,
    ) -> Result<PersistedOneCalcDocument, String> {
        if !self.host_profile.supports_driven_host() {
            return Err(format!(
                "{} does not admit persisted isolated documents",
                self.host_profile.id()
            ));
        }

        let scenario_slug = sanitize_slug(&scenario_slug.into());
        let stable_slug = sanitize_slug(driven_host.formula_stable_id());
        let saved_at_unix_ms = unix_time_millis()?;
        let snapshot = self
            .load_function_surface_catalog()
            .admitted_execution_snapshot();
        let snapshot_ref = format!("{}@{}", snapshot.snapshot_id, snapshot.snapshot_version);
        let document_id = format!("document-{stable_slug}-{saved_at_unix_ms}");
        let artifact_index = retained_run
            .map(document_artifact_index_from_retained_run)
            .unwrap_or_default();

        let document = OneCalcDocumentRecord {
            document_id,
            document_title: format!("OneCalc {}", scenario_slug.replace('-', " ")),
            document_scope: "isolated_single_formula_instance".to_string(),
            persistence_format_id: "spreadsheetml2003.onecalc.single_instance.v1".to_string(),
            worksheet_name: "OneCalc".to_string(),
            saved_at_unix_ms,
            host_profile_id: self.host_profile.id().to_string(),
            scenario_slug,
            formula_stable_id: driven_host.formula_stable_id().to_string(),
            formula_text: driven_host.formula_text().to_string(),
            formula_channel_kind: format!("{:?}", driven_host.formula_channel_kind()),
            formula_text_version: driven_host.formula_text_version(),
            structure_context_version: driven_host.structure_context_version().to_string(),
            host_driving_packet_kind: recalc_context.packet_kind().id().to_string(),
            host_driving_block: "driven_single_formula_host".to_string(),
            recalc_trigger_kind: recalc_context.trigger_kind.id().to_string(),
            display_context: "formula_workbench".to_string(),
            effective_display_status: recalc_summary.evaluation.effective_display_status.clone(),
            function_surface_policy_id: "onecalc:admitted_execution:supported+preview".to_string(),
            library_context_snapshot_ref: Some(snapshot_ref),
            view_state: DocumentViewStateRecord {
                active_surface: "formula_workbench".to_string(),
                cursor_offset: driven_host.formula_text().len(),
                selection_anchor: driven_host.formula_text().len(),
                selection_focus: driven_host.formula_text().len(),
            },
            artifact_index,
        };

        write_spreadsheetml_document(path, &document)
    }

    pub fn reopen_isolated_document(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ReopenedOneCalcDocument, String> {
        let document = read_spreadsheetml_document(path)?;
        if document.document_scope != "isolated_single_formula_instance" {
            return Err(format!(
                "document scope {} is not admitted for isolated OneCalc reopen",
                document.document_scope
            ));
        }
        if document.host_profile_id != self.host_profile.id() {
            return Err(format!(
                "document host profile {} does not match runtime {}",
                document.host_profile_id,
                self.host_profile.id()
            ));
        }

        let mut driven_host = self.new_driven_single_formula_host(
            document.formula_stable_id.clone(),
            document.formula_text.clone(),
        )?;
        driven_host.host.formula_text_version = document.formula_text_version;
        driven_host.host.formula_channel_kind =
            parse_formula_channel_kind(&document.formula_channel_kind)?;
        driven_host.host.structure_context_version = document.structure_context_version.clone();

        Ok(ReopenedOneCalcDocument {
            document,
            driven_host,
        })
    }

    pub fn verify_isolated_document_roundtrip_invariants(
        &self,
        persisted_document: &PersistedOneCalcDocument,
    ) -> Result<DocumentRoundTripInvariantReport, String> {
        let reopened = self.reopen_isolated_document(&persisted_document.document_path)?;
        let report = DocumentRoundTripInvariantReport {
            document_id_preserved: reopened.document.document_id
                == persisted_document.document.document_id,
            formula_identity_preserved: reopened.document.formula_stable_id
                == persisted_document.document.formula_stable_id
                && reopened.document.formula_text == persisted_document.document.formula_text
                && reopened.document.formula_text_version
                    == persisted_document.document.formula_text_version
                && reopened.document.formula_channel_kind
                    == persisted_document.document.formula_channel_kind
                && reopened.driven_host.formula_text() == persisted_document.document.formula_text
                && reopened.driven_host.formula_text_version()
                    == persisted_document.document.formula_text_version,
            structure_context_preserved: reopened.document.structure_context_version
                == persisted_document.document.structure_context_version
                && reopened.driven_host.structure_context_version()
                    == persisted_document.document.structure_context_version,
            library_context_snapshot_ref_preserved: reopened.document.library_context_snapshot_ref
                == persisted_document.document.library_context_snapshot_ref,
            artifact_index_preserved: reopened.document.artifact_index
                == persisted_document.document.artifact_index,
            effective_display_status_preserved: reopened.document.effective_display_status
                == persisted_document.document.effective_display_status,
        };

        if report.all_preserved() {
            Ok(report)
        } else {
            let mut failed = Vec::new();
            if !report.document_id_preserved {
                failed.push("document_id");
            }
            if !report.formula_identity_preserved {
                failed.push("formula_identity");
            }
            if !report.structure_context_preserved {
                failed.push("structure_context");
            }
            if !report.library_context_snapshot_ref_preserved {
                failed.push("library_context_snapshot_ref");
            }
            if !report.artifact_index_preserved {
                failed.push("artifact_index");
            }
            if !report.effective_display_status_preserved {
                failed.push("effective_display_status");
            }
            Err(format!(
                "document round-trip invariants failed: {}",
                failed.join(", ")
            ))
        }
    }

    pub fn export_scenario_capsule(
        &self,
        store: &RetainedScenarioStore,
        capsule_root: impl AsRef<Path>,
        selected_run_ids: &[&str],
    ) -> Result<PersistedScenarioCapsule, String> {
        crate::capsule::export_scenario_capsule(store, capsule_root, selected_run_ids)
    }

    pub fn import_scenario_capsule(
        &self,
        store: &RetainedScenarioStore,
        capsule_root: impl AsRef<Path>,
    ) -> Result<ImportedScenarioCapsule, String> {
        crate::capsule::import_scenario_capsule(store, capsule_root)
    }

    pub fn emit_replay_capture_for_run(
        &self,
        store: &RetainedScenarioStore,
        scenario_run_id: &str,
    ) -> Result<PersistedReplayCapture, String> {
        let reopened = store.reopen_run(scenario_run_id)?;
        let capability_snapshot_ref = reopened
            .run
            .envelope
            .capability_snapshot_ref
            .clone()
            .ok_or_else(|| format!("run {scenario_run_id} is missing a capability snapshot ref"))?;
        let replay_scenario = build_replay_scenario(&reopened.scenario, &reopened.run);
        let replay_capture_id = format!("replay-capture-{scenario_run_id}");
        let emitted_at_unix_ms = unix_time_millis()?;
        let replay_floor = format!(
            "{} ({})",
            CapabilityLevel::C1ReplayValid.registry_id(),
            "normalized_replay_open"
        );
        let replay_artifact_path = store
            .root()
            .join("replay-captures")
            .join(format!("{replay_capture_id}.replay.json"));
        let replay_body =
            serde_json::to_string(&replay_scenario).map_err(|error| error.to_string())?;
        let content_hash = stable_hash(&(
            &replay_capture_id,
            &reopened.scenario.scenario_id,
            &reopened.run.scenario_run_id,
            &replay_floor,
            &replay_body,
        ));
        let capture = ReplayCaptureRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.replay_capture".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::ReplayCapture.id().to_string(),
                logical_id: replay_capture_id.clone(),
                content_hash,
                created_at_unix_ms: emitted_at_unix_ms,
                created_by_build: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
                host_profile_id: self.host_profile.id().to_string(),
                packet_kind: HostPacketKind::ReplayCapture.id().to_string(),
                seam_pin_set_id: "onecalc:ws-06:replay".to_string(),
                capability_floor: self.host_profile.id().to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: vec![ArtifactLineageRef {
                    relation: "scenario_run".to_string(),
                    artifact_ref: reopened.run.envelope.stable_ref(),
                }],
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            replay_capture_id: replay_capture_id.clone(),
            scenario_id: reopened.scenario.scenario_id.clone(),
            scenario_run_id: reopened.run.scenario_run_id.clone(),
            scenario_run_ref: reopened.run.envelope.stable_ref(),
            capability_snapshot_ref,
            replay_floor,
            replay_artifact: oxreplay_abstractions::ReplayArtifactRef {
                path: replay_artifact_path.display().to_string(),
            },
            emitted_at_unix_ms,
        };
        let persisted = store.persist_replay_capture(&capture, &replay_scenario)?;

        let mut updated_run = reopened.run;
        updated_run.replay_capture_ref = Some(capture.envelope.stable_ref());
        store.overwrite_run(&updated_run)?;

        Ok(persisted)
    }

    pub fn open_replay_capture(
        &self,
        store: &RetainedScenarioStore,
        replay_capture_id: &str,
    ) -> Result<OpenedReplayCaptureSummary, String> {
        let capture = store.read_replay_capture(replay_capture_id)?;
        let replay_scenario = load_replay_scenario_from_path(&capture.replay_artifact.path)
            .map_err(|error| {
                format!(
                    "failed to open replay capture {}: {}",
                    replay_capture_id, error
                )
            })?;
        let replay_ready = is_replay_ready(&replay_scenario);
        let event_count = replay_scenario.events.len();
        let registry_ref_count = replay_scenario.registry_refs.len();
        let view = ReplayView {
            view_family: "normalized_replay".to_string(),
            artifact_path: capture.replay_artifact.path.clone(),
        };

        Ok(OpenedReplayCaptureSummary {
            replay_capture_id: capture.replay_capture_id,
            scenario_id: replay_scenario.scenario_id,
            replay_floor: capture.replay_floor,
            replay_ready,
            event_count,
            registry_ref_count,
            view_family: view.view_family,
            artifact_path: view.artifact_path,
        })
    }

    pub fn open_retained_run_xray(
        &self,
        store: &RetainedScenarioStore,
        scenario_run_id: &str,
    ) -> Result<RetainedRunXRaySummary, String> {
        let reopened = store.reopen_run(scenario_run_id)?;
        let capability_snapshot_id = reopened
            .run
            .envelope
            .capability_snapshot_ref
            .as_ref()
            .ok_or_else(|| format!("run {scenario_run_id} is missing a capability snapshot ref"))?
            .logical_id
            .clone();
        let (replay_capture_id, replay_floor) = reopened
            .run
            .replay_capture_ref
            .as_ref()
            .map(|replay_ref| {
                let opened = self.open_replay_capture(store, &replay_ref.logical_id)?;
                Ok::<_, String>((
                    Some(replay_ref.logical_id.clone()),
                    Some(opened.replay_floor),
                ))
            })
            .transpose()?
            .unwrap_or((None, None));

        Ok(RetainedRunXRaySummary {
            scenario_id: reopened.scenario.scenario_id,
            scenario_run_id: reopened.run.scenario_run_id,
            formula_text: reopened.scenario.formula_text,
            formula_text_version: reopened.run.formula_text_version,
            host_profile_id: reopened.scenario.host_profile_id,
            packet_kind: reopened.scenario.host_driving_packet_kind,
            worksheet_value_summary: reopened.run.worksheet_value_summary,
            payload_summary: reopened.run.payload_summary,
            effective_display_status: reopened.run.effective_display_status,
            capability_snapshot_id,
            replay_capture_id,
            replay_floor,
        })
    }

    pub fn diff_retained_run_xray(
        &self,
        store: &RetainedScenarioStore,
        left_run_id: &str,
        right_run_id: &str,
    ) -> Result<RetainedRunDiffSummary, String> {
        let comparison = self.compare_retained_driven_runs(store, left_run_id, right_run_id)?;
        let left = store.reopen_run(left_run_id)?;
        let right = store.reopen_run(right_run_id)?;
        let capability_snapshot_changed =
            left.run.envelope.capability_snapshot_ref != right.run.envelope.capability_snapshot_ref;
        let replay_pair_openable = match (
            left.run.replay_capture_ref.as_ref(),
            right.run.replay_capture_ref.as_ref(),
        ) {
            (Some(left_replay), Some(right_replay)) => {
                self.open_replay_capture(store, &left_replay.logical_id)?;
                self.open_replay_capture(store, &right_replay.logical_id)?;
                true
            }
            _ => false,
        };

        Ok(RetainedRunDiffSummary {
            left_run_id: comparison.left_run_id,
            right_run_id: comparison.right_run_id,
            same_scenario: comparison.same_scenario,
            formula_text_changed: comparison.formula_text_changed,
            worksheet_value_match: comparison.worksheet_value_match,
            payload_match: comparison.payload_match,
            capability_snapshot_changed,
            replay_pair_openable,
            diff_floor: "retained_artifact_direct_diff".to_string(),
        })
    }

    pub fn generate_retained_witness(
        &self,
        store: &RetainedScenarioStore,
        left_run_id: &str,
        right_run_id: &str,
    ) -> Result<PersistedWitness, String> {
        let diff = self.diff_retained_run_xray(store, left_run_id, right_run_id)?;
        let left = store.reopen_run(left_run_id)?;
        let right = store.reopen_run(right_run_id)?;
        let emitted_at_unix_ms = unix_time_millis()?;
        let witness_id = format!("witness-{}-{}", left_run_id, right_run_id);
        let explain_floor = "retained_diff_explain_summary".to_string();
        let explanation_lines = vec![
            format!("same_scenario={}", diff.same_scenario),
            format!("formula_text_changed={}", diff.formula_text_changed),
            format!("worksheet_value_match={}", diff.worksheet_value_match),
            format!("payload_match={}", diff.payload_match),
            format!(
                "capability_snapshot_changed={}",
                diff.capability_snapshot_changed
            ),
            format!("replay_pair_openable={}", diff.replay_pair_openable),
        ];
        let blocked_dimensions = vec![
            "distill_not_integrated".to_string(),
            "no_oxreplay_explain_adapter_invocation_yet".to_string(),
        ];
        let content_hash = stable_hash(&(
            &witness_id,
            &left.scenario.scenario_id,
            &left.run.scenario_run_id,
            &right.run.scenario_run_id,
            &explanation_lines,
            &blocked_dimensions,
        ));
        let witness = WitnessRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.witness".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::Witness.id().to_string(),
                logical_id: witness_id.clone(),
                content_hash,
                created_at_unix_ms: emitted_at_unix_ms,
                created_by_build: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
                host_profile_id: self.host_profile.id().to_string(),
                packet_kind: "witness_generation".to_string(),
                seam_pin_set_id: "onecalc:ws-06:witness".to_string(),
                capability_floor: self.host_profile.id().to_string(),
                provisionality_state: "lane_limited".to_string(),
                lineage_refs: vec![
                    ArtifactLineageRef {
                        relation: "left_run".to_string(),
                        artifact_ref: left.run.envelope.stable_ref(),
                    },
                    ArtifactLineageRef {
                        relation: "right_run".to_string(),
                        artifact_ref: right.run.envelope.stable_ref(),
                    },
                ],
                attachment_refs: Vec::new(),
                capability_snapshot_ref: left.run.envelope.capability_snapshot_ref.clone(),
            },
            witness_id,
            scenario_id: left.scenario.scenario_id,
            left_run_ref: left.run.envelope.stable_ref(),
            right_run_ref: right.run.envelope.stable_ref(),
            explain_floor,
            explanation_lines,
            blocked_dimensions,
            emitted_at_unix_ms,
        };

        store.persist_witness(&witness)
    }

    pub fn open_witness(
        &self,
        store: &RetainedScenarioStore,
        witness_id: &str,
    ) -> Result<OpenedWitnessSummary, String> {
        let witness = store.read_witness(witness_id)?;
        Ok(OpenedWitnessSummary {
            witness_id: witness.witness_id,
            scenario_id: witness.scenario_id,
            explain_floor: witness.explain_floor,
            explanation_lines: witness.explanation_lines,
            blocked_dimensions: witness.blocked_dimensions,
        })
    }

    pub fn generate_handoff_packet(
        &self,
        store: &RetainedScenarioStore,
        witness_id: &str,
    ) -> Result<PersistedHandoffPacket, String> {
        let witness = store.read_witness(witness_id)?;
        let source_run = store.reopen_run(&witness.left_run_ref.logical_id)?;
        let capability_snapshot_ref = source_run
            .run
            .envelope
            .capability_snapshot_ref
            .clone()
            .ok_or_else(|| {
                format!(
                    "run {} is missing a capability snapshot ref",
                    source_run.run.scenario_run_id
                )
            })?;
        let emitted_at_unix_ms = unix_time_millis()?;
        let handoff_id = format!("handoff-{}", witness_id);
        let readiness = vec![
            HandoffReadinessRecord {
                item_id: "target_lane_selected".to_string(),
                satisfied: true,
            },
            HandoffReadinessRecord {
                item_id: "requested_action_selected".to_string(),
                satisfied: true,
            },
            HandoffReadinessRecord {
                item_id: "expected_vs_observed_present".to_string(),
                satisfied: true,
            },
            HandoffReadinessRecord {
                item_id: "retained_source_artifact_attached".to_string(),
                satisfied: true,
            },
            HandoffReadinessRecord {
                item_id: "build_seam_platform_context_present".to_string(),
                satisfied: true,
            },
            HandoffReadinessRecord {
                item_id: "reliability_state_present".to_string(),
                satisfied: true,
            },
            HandoffReadinessRecord {
                item_id: "witness_lineage_present".to_string(),
                satisfied: true,
            },
            HandoffReadinessRecord {
                item_id: "lossiness_explicit".to_string(),
                satisfied: true,
            },
        ];
        let status = if readiness.iter().all(|item| item.satisfied) {
            "ready"
        } else {
            "draft"
        }
        .to_string();
        let content_hash = stable_hash(&(
            &handoff_id,
            &witness.scenario_id,
            &witness.explain_floor,
            &status,
            &readiness,
        ));
        let handoff = HandoffPacketRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.handoff_packet".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::HandoffPacket.id().to_string(),
                logical_id: handoff_id.clone(),
                content_hash,
                created_at_unix_ms: emitted_at_unix_ms,
                created_by_build: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
                host_profile_id: self.host_profile.id().to_string(),
                packet_kind: "handoff_generation".to_string(),
                seam_pin_set_id: "onecalc:ws-06:handoff".to_string(),
                capability_floor: self.host_profile.id().to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: vec![
                    ArtifactLineageRef {
                        relation: "source_run".to_string(),
                        artifact_ref: source_run.run.envelope.stable_ref(),
                    },
                    ArtifactLineageRef {
                        relation: "witness".to_string(),
                        artifact_ref: witness.envelope.stable_ref(),
                    },
                ],
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            handoff_id,
            scenario_id: witness.scenario_id.clone(),
            source_run_ref: source_run.run.envelope.stable_ref(),
            witness_ref: witness.envelope.stable_ref(),
            capability_snapshot_ref,
            requested_action_kind: "clarify_contract".to_string(),
            target_lane: "OxReplay/DnaOneCalc".to_string(),
            expected_behavior: "replay, explain, and handoff surfaces should remain capability-gated and lineage-complete".to_string(),
            observed_behavior: format!(
                "witness floor={} with blocked dimensions={}",
                witness.explain_floor,
                witness.blocked_dimensions.join(",")
            ),
            supporting_artifact_refs: vec![source_run.run.envelope.stable_ref(), witness.envelope.stable_ref()],
            reliability_state: "retained_evidence_direct".to_string(),
            status,
            readiness,
            emitted_at_unix_ms,
        };

        store.persist_handoff_packet(&handoff)
    }

    pub fn open_handoff_packet(
        &self,
        store: &RetainedScenarioStore,
        handoff_id: &str,
    ) -> Result<OpenedHandoffPacketSummary, String> {
        let handoff = store.read_handoff_packet(handoff_id)?;
        Ok(OpenedHandoffPacketSummary {
            handoff_id: handoff.handoff_id,
            target_lane: handoff.target_lane,
            requested_action_kind: handoff.requested_action_kind,
            status: handoff.status,
            readiness: handoff.readiness,
            capability_snapshot_id: handoff.capability_snapshot_ref.logical_id,
        })
    }

    pub fn emit_capability_snapshot(
        &self,
        packet_kind: &str,
        diff_base_snapshot_ref: Option<&str>,
    ) -> Result<CapabilityLedgerSnapshotRecord, String> {
        let emitted_at_unix_ms = unix_time_millis()?;
        let function_catalog = self.load_function_surface_catalog();
        let function_summary = function_catalog.label_summary();
        let admitted_snapshot = function_catalog.admitted_execution_snapshot();
        let function_surface_snapshot_ref = format!(
            "{}@{}",
            admitted_snapshot.snapshot_id, admitted_snapshot.snapshot_version
        );
        let packet_kind_register = self
            .packet_kinds()
            .iter()
            .map(|packet| packet.id().to_string())
            .collect::<Vec<_>>();
        let capability_snapshot_id = format!(
            "capability-snapshot-{}-{}-{}",
            sanitize_slug(self.host_profile.id()),
            sanitize_slug(packet_kind),
            emitted_at_unix_ms
        );
        let content_hash = stable_hash(&(
            capability_snapshot_id.as_str(),
            self.host_profile.id(),
            self.platform_gate().id(),
            function_surface_snapshot_ref.as_str(),
            packet_kind,
            packet_kind_register.as_slice(),
            function_summary.supported,
            function_summary.preview,
            function_summary.experimental,
            function_summary.deferred,
            function_summary.catalog_only,
        ));
        let diff_base_refs = diff_base_snapshot_ref
            .map(|snapshot_ref| {
                vec![StableArtifactRef {
                    artifact_kind: ArtifactKind::CapabilityLedgerSnapshot.id().to_string(),
                    logical_id: snapshot_ref.to_string(),
                    content_hash: None,
                }]
            })
            .unwrap_or_default();

        Ok(CapabilityLedgerSnapshotRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.capability_ledger_snapshot".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::CapabilityLedgerSnapshot.id().to_string(),
                logical_id: capability_snapshot_id.clone(),
                content_hash,
                created_at_unix_ms: emitted_at_unix_ms,
                created_by_build: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
                host_profile_id: self.host_profile.id().to_string(),
                packet_kind: packet_kind.to_string(),
                seam_pin_set_id: "onecalc:ws-05:capability".to_string(),
                capability_floor: self.host_profile.id().to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: None,
            },
            capability_snapshot_id,
            emitted_at_unix_ms,
            emitter_build_id: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
            host_kind: "dnaonecalc-host".to_string(),
            runtime_platform: std::env::consts::OS.to_string(),
            runtime_class: self.platform_gate().id().to_string(),
            dependency_set: vec![
                "dnaonecalc-host".to_string(),
                "oxfml_core".to_string(),
                "oxfunc_core".to_string(),
                "oxreplay_abstractions".to_string(),
                "oxreplay_core".to_string(),
            ],
            function_surface_snapshot_ref,
            seam_pin_set_id: "onecalc:ws-05:capability".to_string(),
            capability_floor: self.host_profile.id().to_string(),
            packet_kind_register,
            function_surface_policy_id: format!(
                "onecalc:admitted_execution:supported={}::preview={}::experimental={}::deferred={}::catalog_only={}",
                function_summary.supported,
                function_summary.preview,
                function_summary.experimental,
                function_summary.deferred,
                function_summary.catalog_only
            ),
            mode_availability: vec![
                capability_mode("DNA-only", "available", None),
                capability_mode("Excel-observed", "blocked", Some("Windows observation path not yet integrated")),
                capability_mode("Twin compare", "blocked", Some("retained run comparison exists, observation compare path not yet integrated")),
                capability_mode(
                    "Replay",
                    "available",
                    Some("retained run replay capture and open path available at cap.C1.replay_valid"),
                ),
                capability_mode(
                    "Diff",
                    "available",
                    Some("retained run diff surface is available; observation compare remains blocked"),
                ),
                capability_mode(
                    "Explain",
                    "available",
                    Some("retained diff explain summary available; distill remains blocked"),
                ),
                capability_mode("Distill", "blocked", Some("distill path not yet integrated")),
                capability_mode(
                    "Handoff",
                    "available",
                    Some("retained-evidence handoff packet generation is gated by capability snapshot truth"),
                ),
            ],
            provisional_seams: vec![
                "browser_and_secondary_hosts_not_admitted".to_string(),
                "observation_path_not_integrated".to_string(),
            ],
            capability_ceilings: vec![
                "single_formula_scope_only".to_string(),
                "no_worksheet_environment".to_string(),
                "no_multi_node_recalc".to_string(),
            ],
            lossiness: vec!["capability_snapshot_uses_current_local_dependency_identity_only".to_string()],
            diff_base_refs,
        })
    }

    pub fn compare_retained_driven_runs(
        &self,
        store: &RetainedScenarioStore,
        left_run_id: &str,
        right_run_id: &str,
    ) -> Result<DrivenRunComparison, String> {
        let left = store.reopen_run(left_run_id)?;
        let right = store.reopen_run(right_run_id)?;

        Ok(DrivenRunComparison {
            left_run_id: left.run.scenario_run_id.clone(),
            right_run_id: right.run.scenario_run_id.clone(),
            same_scenario: left.run.scenario_id == right.run.scenario_id,
            formula_version_changed: left.run.formula_text_version != right.run.formula_text_version,
            formula_text_changed: left.run.authored_formula_text != right.run.authored_formula_text,
            worksheet_value_match: left.run.worksheet_value_summary == right.run.worksheet_value_summary,
            payload_match: left.run.payload_summary == right.run.payload_summary,
            returned_surface_match: left.run.returned_value_surface_kind
                == right.run.returned_value_surface_kind,
            effective_display_match: left.run.effective_display_status
                == right.run.effective_display_status,
            commit_decision_match: left.run.commit_decision_kind == right.run.commit_decision_kind,
            comparison_envelope: "formula_text,worksheet_value,payload,returned_surface,effective_display,commit_decision".to_string(),
            reliability_badge: "direct".to_string(),
        })
    }

    pub fn collect_completion_proposals(
        &self,
        session: &FormulaEditorSession,
        cursor_offset: usize,
    ) -> Vec<CompletionProposalSummary> {
        let Some(result) = session.latest_result() else {
            return Vec::new();
        };

        let snapshot = self
            .load_function_surface_catalog()
            .admitted_execution_snapshot();
        let bind_context = build_bind_context(&result.source);
        let completion = collect_completion_proposals(CompletionRequest {
            source: &result.source,
            green_tree: &result.green_tree,
            red_projection: &result.red_projection,
            bind_context: &bind_context,
            library_context_snapshot: Some(&snapshot),
            cursor_offset,
        });

        completion
            .proposals
            .into_iter()
            .map(|proposal| CompletionProposalSummary {
                proposal_kind: format!("{:?}", proposal.proposal_kind),
                display_text: proposal.display_text,
            })
            .collect()
    }

    pub fn current_function_help(
        &self,
        session: &FormulaEditorSession,
        cursor_offset: usize,
    ) -> Option<FunctionHelpSummary> {
        let result = session.latest_result()?;
        let catalog = self.load_function_surface_catalog();
        let snapshot = catalog.admitted_execution_snapshot();
        let request = build_function_help_lookup_request(
            &result.source,
            &result.green_tree,
            cursor_offset,
            Some(&snapshot),
        )?;
        let function_meta = lookup_function_meta_by_surface_name(&request.lookup_key)?;
        let entry = catalog.get(&request.lookup_key)?;
        let display_signature = format!(
            "{}({})",
            request.lookup_key,
            summarize_arity(function_meta.arity.min, function_meta.arity.max)
        );
        let availability_summary =
            format!("{} ({})", entry.admission_category.id(), entry.category);

        Some(FunctionHelpSummary {
            display_name: request.lookup_key,
            display_signature,
            active_argument_index: signature_help_argument_index(
                &result.source,
                &result.green_tree,
                cursor_offset,
            ),
            availability_summary,
            provisional: matches!(entry.admission_category, crate::AdmissionCategory::Preview),
        })
    }

    pub fn apply_formula_edit_packet(
        &self,
        session: &mut FormulaEditorSession,
        formula_text: impl Into<String>,
    ) -> FormulaEditPacketSummary {
        let source = FormulaSourceRecord::new(
            session.formula_stable_id.clone(),
            session.formula_text_version + 1,
            formula_text,
        )
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);

        let bind_context = build_bind_context(&source);

        let previous_result = session.latest_result.as_ref();
        let result = apply_formula_edit(FormulaEditRequest {
            source: source.clone(),
            bind_context,
            previous_green_tree: previous_result.map(|result| &result.green_tree),
            previous_red_projection: previous_result.map(|result| &result.red_projection),
            previous_bound_formula: previous_result
                .and_then(|result| result.bound_formula.as_ref()),
            follow_on_stage: EditFollowOnStage::ParseAndBind,
            semantic_plan_options: None,
        });

        session.formula_text_version += 1;

        let summary = FormulaEditPacketSummary {
            formula_token: result.source.formula_token().0,
            diagnostic_count: result.live_diagnostics.diagnostics.len(),
            text_change_range: result.text_change_range,
            reused_green_tree: result.reuse_summary.reused_green_tree,
            reused_red_projection: result.reuse_summary.reused_red_projection,
            reused_bound_formula: result.reuse_summary.reused_bound_formula,
        };

        session.latest_result = Some(result);
        summary
    }

    fn run_driven_recalc(
        &self,
        driven_host: &mut DrivenSingleFormulaHost,
        recalc_context: RecalcContext,
    ) -> Result<DrivenRecalcSummary, String> {
        if !self.host_profile.supports_driven_host() {
            return Err(format!(
                "{} does not admit the driven single-formula host model",
                self.host_profile.id()
            ));
        }

        let catalog = self.load_function_surface_catalog();
        let snapshot = catalog.admitted_execution_snapshot();
        let provider = InMemoryLibraryContextProvider::new(snapshot);
        driven_host.host.now_serial = recalc_context.now_serial;
        driven_host.host.random_value = recalc_context.random_value;
        let query_bundle = TypedContextQueryBundle::new(
            None,
            None,
            None,
            recalc_context.now_serial,
            recalc_context.random_value,
        );
        let output = driven_host.host.recalc_with_interfaces(
            EvaluationBackend::OxFuncBacked,
            query_bundle,
            Some(&provider),
        )?;

        Ok(DrivenRecalcSummary {
            host_profile_id: self.host_profile.id().to_string(),
            trigger_kind: recalc_context.trigger_kind.id().to_string(),
            packet_kind: recalc_context.packet_kind().id().to_string(),
            formula_text_version: driven_host.host.formula_text_version,
            structure_context_version: driven_host.host.structure_context_version.clone(),
            evaluation: summarize_host_output(output),
        })
    }
}

fn build_bind_context(source: &FormulaSourceRecord) -> BindContext {
    let mut bind_context = BindContext::default();
    bind_context.formula_token = source.formula_token();
    bind_context.structure_context_version =
        StructureContextVersion("onecalc:single_formula:v1".to_string());
    bind_context
}

fn summarize_host_output(output: oxfml_core::HostRecalcOutput) -> FormulaEvaluationSummary {
    let returned_presentation_hint_status =
        summarize_presentation_hint(output.returned_value_surface.presentation_hint);
    let host_style_state_status = summarize_host_style_state();

    FormulaEvaluationSummary {
        formula_token: output.source.formula_token().0,
        worksheet_value_summary: summarize_eval_value(&output.published_worksheet_value),
        payload_summary: output.returned_value_surface.payload_summary.clone(),
        returned_value_surface_kind: format!("{:?}", output.returned_value_surface.kind),
        returned_presentation_hint_status: returned_presentation_hint_status.clone(),
        host_style_state_status: host_style_state_status.clone(),
        effective_display_status: derive_effective_display_status(
            &returned_presentation_hint_status,
            &host_style_state_status,
        ),
        commit_decision_kind: match output.commit_decision {
            oxfml_core::AcceptDecision::Accepted(_) => "accepted".to_string(),
            oxfml_core::AcceptDecision::Rejected(_) => "rejected".to_string(),
        },
        trace_event_count: output.trace_events.len(),
    }
}

fn parse_formula_channel_kind(value: &str) -> Result<FormulaChannelKind, String> {
    match value {
        "WorksheetA1" => Ok(FormulaChannelKind::WorksheetA1),
        "WorksheetR1C1" => Ok(FormulaChannelKind::WorksheetR1C1),
        "ConditionalFormatting" => Ok(FormulaChannelKind::ConditionalFormatting),
        "DataValidation" => Ok(FormulaChannelKind::DataValidation),
        _ => Err(format!(
            "unsupported retained formula channel kind: {value}"
        )),
    }
}

fn capability_mode(
    mode_id: &str,
    state: &str,
    reason: Option<&str>,
) -> CapabilityModeAvailabilityRecord {
    CapabilityModeAvailabilityRecord {
        mode_id: mode_id.to_string(),
        state: state.to_string(),
        reason: reason.map(|value| value.to_string()),
    }
}

fn document_artifact_index_from_retained_run(
    retained_run: &PersistedScenarioRun,
) -> Vec<DocumentArtifactIndexEntry> {
    vec![
        DocumentArtifactIndexEntry {
            artifact_kind: ArtifactKind::Scenario.id().to_string(),
            logical_id: retained_run.scenario.scenario_id.clone(),
            path_hint: format!("scenarios/{}.json", retained_run.scenario.scenario_id),
            content_hash: Some(retained_run.scenario.envelope.content_hash.clone()),
            embedded: false,
        },
        DocumentArtifactIndexEntry {
            artifact_kind: ArtifactKind::ScenarioRun.id().to_string(),
            logical_id: retained_run.run.scenario_run_id.clone(),
            path_hint: format!("scenario-runs/{}.json", retained_run.run.scenario_run_id),
            content_hash: Some(retained_run.run.envelope.content_hash.clone()),
            embedded: false,
        },
        DocumentArtifactIndexEntry {
            artifact_kind: ArtifactKind::CapabilityLedgerSnapshot.id().to_string(),
            logical_id: retained_run
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
                .clone(),
            path_hint: format!(
                "capability-snapshots/{}.json",
                retained_run
                    .capability_snapshot
                    .snapshot
                    .capability_snapshot_id
            ),
            content_hash: Some(
                retained_run
                    .capability_snapshot
                    .snapshot
                    .envelope
                    .content_hash
                    .clone(),
            ),
            embedded: false,
        },
    ]
}

fn build_replay_scenario(scenario: &ScenarioRecord, run: &ScenarioRunRecord) -> ReplayScenario {
    let mut events = vec![
        ReplayEvent {
            event_id: format!("{}-packet", run.scenario_run_id),
            source_label: scenario.host_driving_packet_kind.clone(),
            normalized_family: "packet.received".to_string(),
        },
        ReplayEvent {
            event_id: format!("{}-payload", run.scenario_run_id),
            source_label: run.payload_summary.clone(),
            normalized_family: "publication.payload".to_string(),
        },
    ];

    let terminal_family = if run.commit_decision_kind == "accepted" {
        "publication.committed"
    } else {
        "reject.issued"
    };
    events.push(ReplayEvent {
        event_id: format!("{}-terminal", run.scenario_run_id),
        source_label: run.commit_decision_kind.clone(),
        normalized_family: terminal_family.to_string(),
    });

    ReplayScenario {
        scenario_id: scenario.scenario_id.clone(),
        lane_id: LaneId("dnaonecalc".to_string()),
        events,
        registry_refs: vec![
            RegistryRef {
                family: "dnaonecalc-host".to_string(),
                version: scenario.host_profile_id.clone(),
            },
            RegistryRef {
                family: "replay_floor".to_string(),
                version: CapabilityLevel::C1ReplayValid.registry_id().to_string(),
            },
        ],
    }
}

fn sanitize_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in value.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            slug.push(normalized);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "scenario".to_string()
    } else {
        slug
    }
}

fn unix_time_millis() -> Result<u64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?;
    Ok(duration.as_millis() as u64)
}

fn summarize_arity(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}..{max}")
    }
}

fn signature_help_argument_index(
    source: &FormulaSourceRecord,
    green_tree: &oxfml_core::GreenTreeRoot,
    cursor_offset: usize,
) -> usize {
    oxfml_core::signature_help_context_at_cursor(source, green_tree, cursor_offset)
        .map(|context| context.active_argument_index)
        .unwrap_or(0)
}

fn summarize_eval_value(value: &EvalValue) -> String {
    match value {
        EvalValue::Number(n) => format!("Number({n})"),
        EvalValue::Text(text) => format!("Text({})", text.to_string_lossy()),
        EvalValue::Logical(value) => format!("Logical({value})"),
        EvalValue::Error(code) => format!("Error({code:?})"),
        EvalValue::Array(array) => {
            let shape = array.shape();
            format!("Array({}x{})", shape.rows, shape.cols)
        }
        EvalValue::Reference(reference) => format!("Reference({})", reference.target),
        EvalValue::Lambda(lambda) => format!("Lambda({})", lambda.callable_token),
    }
}

fn summarize_presentation_hint(hint: Option<oxfunc_core::value::PresentationHint>) -> String {
    match hint {
        Some(hint) => {
            let number_format = hint
                .number_format
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|| "none".to_string());
            let style = hint
                .style
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|| "none".to_string());
            format!("number_format:{number_format};style:{style}")
        }
        None => "none".to_string(),
    }
}

fn summarize_host_style_state() -> String {
    "none".to_string()
}

fn derive_effective_display_status(
    returned_presentation_hint_status: &str,
    host_style_state_status: &str,
) -> String {
    match (
        returned_presentation_hint_status == "none",
        host_style_state_status == "none",
    ) {
        (true, true) => "none".to_string(),
        (false, true) => returned_presentation_hint_status.to_string(),
        (true, false) => format!("host_style:{host_style_state_status}"),
        (false, false) => format!(
            "presentation_hint:{returned_presentation_hint_status};host_style:{host_style_state_status}"
        ),
    }
}
