use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use oxfml_core::consumer::editor::{
    EditorAnalysisStage, EditorDocument, EditorEditService, EditorEnvironment,
    FormulaTextChangeRange,
};
use oxfml_core::consumer::replay::{
    ReplayProjectionRequest, ReplayProjectionResult, ReplayProjectionService,
};
use oxfml_core::consumer::runtime::{
    RuntimeEnvironment, RuntimeFormulaRequest, RuntimeFormulaResult, RuntimeSessionFacade,
};
use oxfml_core::{
    BindContext, FormulaChannelKind, FormulaSourceRecord, LibraryContextSnapshotRef,
    StructureContextVersion, TypedContextQueryBundle,
};
use oxfunc_core::value::EvalValue;
use oxreplay_abstractions::CapabilityLevel;
use oxreplay_core::{
    is_replay_ready, load_oxfml_v1_replay_projection, ReplayView,
};

use crate::artifact::{
    stable_hash, ArtifactAttachmentRef, ArtifactEnvelope, ArtifactKind, ArtifactLineageRef,
    StableArtifactRef,
};
use crate::capsule::{ImportedScenarioCapsule, PersistedScenarioCapsule};
use crate::conditional_formatting::IsolatedConditionalFormattingCarrier;
use crate::document::{
    read_spreadsheetml_document, write_spreadsheetml_document, DocumentArtifactIndexEntry,
    DocumentViewStateRecord, OneCalcDocumentRecord, PersistedOneCalcDocument,
};
use crate::extension::{
    activate_windows_rtd_topic, advance_rtd_topic, admitted_extension_abi,
    extension_root_runtime_truth, invoke_extension_provider, linux_rtd_registry_truth,
    load_extension_root, validate_extension_manifest, ActivatedRtdTopicSession,
    ExtensionAbiContract, ExtensionInvocationArgument, ExtensionInvocationSummary,
    ExtensionProviderManifest, ExtensionRootLoadSummary, ExtensionRootRuntimeTruthSummary,
    ExtensionValidationResult, LinuxRtdRegistrySummary, RtdTopicUpdateSummary,
};
use crate::observation::{invoke_live_windows_capture, load_observation_source_bundle};
use crate::retained::{
    CapabilityLedgerSnapshotRecord, CapabilityModeAvailabilityRecord, ComparisonMismatchRecord,
    ComparisonRecord, HandoffPacketRecord, HandoffReadinessRecord, ObservationRecord,
    OxfmlReplayProjectionRecord,
    PersistedCapabilitySnapshot, PersistedComparison, PersistedHandoffPacket, PersistedObservation,
    PersistedReplayCapture, PersistedScenarioRun, PersistedWitness, ReplayCaptureRecord,
    RetainedProvenanceRecord, RetainedRecalcContextRecord, RetainedScenarioStore, ScenarioRecord,
    ScenarioRunRecord, WitnessRecord,
};
use crate::workspace::{
    read_workspace_manifest, write_workspace_manifest, OneCalcWorkspaceManifest,
    PersistedOneCalcWorkspace,
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
    ObservationCapture,
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
            Self::ObservationCapture => "observation_capture",
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
    HostPacketKind::ObservationCapture,
];

const OC_H2_PACKET_KINDS: &[HostPacketKind] = &[
    HostPacketKind::FormulaEdit,
    HostPacketKind::EditAcceptRecalc,
    HostPacketKind::ManualRecalc,
    HostPacketKind::ForcedRecalc,
    HostPacketKind::ReplayCapture,
    HostPacketKind::ObservationCapture,
    HostPacketKind::ExtensionRegistration,
    HostPacketKind::RtdUpdate,
];

#[derive(Debug, Clone)]
pub struct FormulaEditorSession {
    formula_stable_id: String,
    formula_text_version: u64,
    latest_result: Option<EditorDocument>,
}

impl FormulaEditorSession {
    pub fn new(formula_stable_id: impl Into<String>) -> Self {
        Self {
            formula_stable_id: formula_stable_id.into(),
            formula_text_version: 0,
            latest_result: None,
        }
    }

    pub fn latest_result(&self) -> Option<&EditorDocument> {
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

pub struct DrivenSingleFormulaHost {
    formula_stable_id: String,
    formula_text: String,
    formula_text_version: u64,
    formula_channel_kind: FormulaChannelKind,
    structure_context_version: String,
    session: RuntimeSessionFacade<'static>,
}

impl fmt::Debug for DrivenSingleFormulaHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrivenSingleFormulaHost")
            .field("formula_stable_id", &self.formula_stable_id)
            .field("formula_text", &self.formula_text)
            .field("formula_text_version", &self.formula_text_version)
            .field("formula_channel_kind", &self.formula_channel_kind)
            .field("structure_context_version", &self.structure_context_version)
            .finish_non_exhaustive()
    }
}

impl DrivenSingleFormulaHost {
    pub fn formula_stable_id(&self) -> &str {
        &self.formula_stable_id
    }

    pub fn formula_text(&self) -> &str {
        &self.formula_text
    }

    pub const fn formula_text_version(&self) -> u64 {
        self.formula_text_version
    }

    pub const fn formula_channel_kind(&self) -> FormulaChannelKind {
        self.formula_channel_kind
    }

    pub fn structure_context_version(&self) -> &str {
        &self.structure_context_version
    }

    fn formula_source(&self) -> FormulaSourceRecord {
        FormulaSourceRecord::new(
            self.formula_stable_id.clone(),
            self.formula_text_version,
            self.formula_text.clone(),
        )
        .with_formula_channel_kind(self.formula_channel_kind)
    }

    fn set_formula_text(&mut self, formula_text: impl Into<String>) {
        self.formula_text = formula_text.into();
        self.formula_text_version += 1;
    }

    fn restore_retained_state(
        &mut self,
        formula_text_version: u64,
        formula_channel_kind: FormulaChannelKind,
        structure_context_version: impl Into<String>,
    ) {
        self.formula_text_version = formula_text_version;
        self.formula_channel_kind = formula_channel_kind;
        self.structure_context_version = structure_context_version.into();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivenRecalcSummary {
    pub host_profile_id: String,
    pub trigger_kind: String,
    pub packet_kind: String,
    pub formula_text_version: u64,
    pub structure_context_version: String,
    pub library_context_snapshot_ref: Option<String>,
    pub replay_projection: OxfmlReplayProjectionRecord,
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

#[derive(Debug)]
pub struct OpenedOneCalcWorkspace {
    pub manifest: OneCalcWorkspaceManifest,
    pub reopened_documents: Vec<ReopenedOneCalcDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedCapabilitySnapshotSummary {
    pub capability_snapshot_id: String,
    pub host_kind: String,
    pub runtime_platform: String,
    pub runtime_class: String,
    pub dependency_set: Vec<String>,
    pub seam_pin_set_id: String,
    pub capability_floor: String,
    pub packet_kind_register: Vec<String>,
    pub function_surface_policy_id: String,
    pub mode_availability: Vec<CapabilityModeAvailabilityRecord>,
    pub provisional_seams: Vec<String>,
    pub capability_ceilings: Vec<String>,
    pub lossiness: Vec<String>,
    pub diff_base_snapshot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySnapshotDiffSummary {
    pub left_snapshot_id: String,
    pub right_snapshot_id: String,
    pub dependencies_added: Vec<String>,
    pub dependencies_removed: Vec<String>,
    pub packet_kinds_added: Vec<String>,
    pub packet_kinds_removed: Vec<String>,
    pub mode_changes: Vec<String>,
    pub provisional_seams_added: Vec<String>,
    pub provisional_seams_removed: Vec<String>,
    pub capability_ceilings_added: Vec<String>,
    pub capability_ceilings_removed: Vec<String>,
    pub function_surface_policy_changed: bool,
    pub runtime_class_changed: bool,
    pub diff_floor: String,
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
    pub projection_source_artifact_family: String,
    pub projection_phase: Option<String>,
    pub projection_alias: Option<String>,
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
    pub formatting_truth_plane: String,
    pub conditional_formatting_scope: String,
    pub blocked_dimensions: Vec<String>,
    pub capability_snapshot_id: String,
    pub replay_capture_id: Option<String>,
    pub replay_floor: Option<String>,
    pub replay_projection_source_artifact_family: Option<String>,
    pub replay_projection_phase: Option<String>,
    pub replay_projection_alias: Option<String>,
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
    pub formatting_truth_plane: String,
    pub conditional_formatting_scope: String,
    pub blocked_dimensions: Vec<String>,
    pub diff_floor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedWitnessSummary {
    pub witness_id: String,
    pub scenario_id: String,
    pub explain_floor: String,
    pub explanation_lines: Vec<String>,
    pub blocked_dimensions: Vec<String>,
    pub replay_projection_aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedHandoffPacketSummary {
    pub handoff_id: String,
    pub target_lane: String,
    pub requested_action_kind: String,
    pub status: String,
    pub readiness: Vec<HandoffReadinessRecord>,
    pub capability_snapshot_id: String,
    pub replay_projection_alias: Option<String>,
    pub replay_projection_phase: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedTwinCompareSummary {
    pub comparison_id: String,
    pub left_run_id: String,
    pub observation_id: String,
    pub comparison_envelope: Vec<String>,
    pub reliability_badge: String,
    pub mismatch_lines: Vec<String>,
    pub projection_limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotedScenarioIndexRow {
    pub row_id: String,
    pub scenario_id: String,
    pub scenario_slug: String,
    pub latest_run_id: String,
    pub host_profile_id: String,
    pub runtime_platform: String,
    pub formula_text: String,
    pub worksheet_value_summary: String,
    pub replay_capture_ids: Vec<String>,
    pub comparison_ids: Vec<String>,
    pub witness_ids: Vec<String>,
    pub handoff_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotedScenarioIndex {
    pub rows: Vec<PromotedScenarioIndexRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScenarioLibraryFilter {
    #[serde(default)]
    pub host_profile_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison_required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness_required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff_required: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioLibrarySavedView {
    pub view_id: String,
    pub display_name: String,
    pub filter: ScenarioLibraryFilter,
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

    pub fn extension_abi_contract(&self) -> ExtensionAbiContract {
        admitted_extension_abi(self.host_profile.id(), self.platform_gate().id())
    }

    pub fn validate_extension_manifest(
        &self,
        manifest: &ExtensionProviderManifest,
    ) -> ExtensionValidationResult {
        validate_extension_manifest(manifest, self.host_profile.id(), self.platform_gate().id())
    }

    pub fn load_extension_root(
        &self,
        extension_root: impl AsRef<Path>,
    ) -> Result<ExtensionRootLoadSummary, String> {
        load_extension_root(extension_root, self.host_profile.id(), self.platform_gate().id())
    }

    pub fn invoke_extension_provider(
        &self,
        extension_root: impl AsRef<Path>,
        provider_id: &str,
        function_name: &str,
        arguments: &[ExtensionInvocationArgument],
    ) -> Result<ExtensionInvocationSummary, String> {
        invoke_extension_provider(
            extension_root,
            self.host_profile.id(),
            self.platform_gate().id(),
            provider_id,
            function_name,
            arguments,
        )
    }

    pub fn extension_root_runtime_truth(
        &self,
        extension_root: impl AsRef<Path>,
    ) -> Result<ExtensionRootRuntimeTruthSummary, String> {
        extension_root_runtime_truth(
            extension_root,
            self.host_profile.id(),
            self.platform_gate().id(),
            std::env::consts::OS,
        )
    }

    pub fn activate_windows_rtd_topic(
        &self,
        extension_root: impl AsRef<Path>,
        provider_id: &str,
        topic_id: &str,
    ) -> Result<ActivatedRtdTopicSession, String> {
        activate_windows_rtd_topic(
            extension_root,
            self.host_profile.id(),
            self.platform_gate().id(),
            std::env::consts::OS,
            provider_id,
            topic_id,
        )
    }

    pub fn advance_rtd_topic(
        &self,
        session: &mut ActivatedRtdTopicSession,
    ) -> RtdTopicUpdateSummary {
        let _ = self;
        advance_rtd_topic(session)
    }

    pub fn linux_rtd_registry_truth(
        &self,
        extension_root: impl AsRef<Path>,
    ) -> Result<LinuxRtdRegistrySummary, String> {
        linux_rtd_registry_truth(
            extension_root,
            self.host_profile.id(),
            self.platform_gate().id(),
            std::env::consts::OS,
        )
    }

    pub fn build_promoted_scenario_index(
        &self,
        store: &RetainedScenarioStore,
    ) -> Result<PromotedScenarioIndex, String> {
        let _ = self;
        let scenarios = store
            .list_scenarios()?
            .into_iter()
            .map(|scenario| (scenario.scenario_id.clone(), scenario))
            .collect::<BTreeMap<_, _>>();
        let runs = store.list_runs()?;
        let replay_captures = store.list_replay_captures()?;
        let comparisons = store.list_comparisons()?;
        let witnesses = store.list_witnesses()?;
        let handoffs = store.list_handoff_packets()?;

        let mut latest_runs: BTreeMap<String, ScenarioRunRecord> = BTreeMap::new();
        for run in runs {
            match latest_runs.get(&run.scenario_id) {
                Some(current) if current.executed_at_unix_ms >= run.executed_at_unix_ms => {}
                _ => {
                    latest_runs.insert(run.scenario_id.clone(), run);
                }
            }
        }

        let mut rows = Vec::new();
        for (scenario_id, run) in latest_runs {
            let scenario = scenarios
                .get(&scenario_id)
                .ok_or_else(|| format!("scenario {} is missing for retained run {}", scenario_id, run.scenario_run_id))?;
            let replay_capture_ids = replay_captures
                .iter()
                .filter(|capture| capture.scenario_run_id == run.scenario_run_id)
                .map(|capture| capture.replay_capture_id.clone())
                .collect::<Vec<_>>();
            let comparison_ids = comparisons
                .iter()
                .filter(|comparison| {
                    comparison.left_artifact_ref.logical_id == run.scenario_run_id
                        || comparison.right_artifact_ref.logical_id == run.scenario_run_id
                })
                .map(|comparison| comparison.comparison_id.clone())
                .collect::<Vec<_>>();
            let witness_ids = witnesses
                .iter()
                .filter(|witness| witness.scenario_id == scenario_id)
                .map(|witness| witness.witness_id.clone())
                .collect::<Vec<_>>();
            let handoff_ids = handoffs
                .iter()
                .filter(|handoff| handoff.scenario_id == scenario_id)
                .map(|handoff| handoff.handoff_id.clone())
                .collect::<Vec<_>>();

            rows.push(PromotedScenarioIndexRow {
                row_id: format!("promoted-scenario:{scenario_id}"),
                scenario_id: scenario_id.clone(),
                scenario_slug: scenario.scenario_slug.clone(),
                latest_run_id: run.scenario_run_id.clone(),
                host_profile_id: run.envelope.host_profile_id.clone(),
                runtime_platform: run.runtime_platform.clone(),
                formula_text: run.authored_formula_text.clone(),
                worksheet_value_summary: run.worksheet_value_summary.clone(),
                replay_capture_ids,
                comparison_ids,
                witness_ids,
                handoff_ids,
            });
        }

        Ok(PromotedScenarioIndex { rows })
    }

    pub fn apply_scenario_library_filter(
        &self,
        index: &PromotedScenarioIndex,
        filter: &ScenarioLibraryFilter,
    ) -> Vec<PromotedScenarioIndexRow> {
        let _ = self;
        index
            .rows
            .iter()
            .filter(|row| {
                (filter.host_profile_ids.is_empty()
                    || filter.host_profile_ids.iter().any(|id| id == &row.host_profile_id))
                    && filter
                        .runtime_platform
                        .as_ref()
                        .map(|platform| platform == &row.runtime_platform)
                        .unwrap_or(true)
                    && filter
                        .replay_required
                        .map(|required| required == !row.replay_capture_ids.is_empty())
                        .unwrap_or(true)
                    && filter
                        .comparison_required
                        .map(|required| required == !row.comparison_ids.is_empty())
                        .unwrap_or(true)
                    && filter
                        .witness_required
                        .map(|required| required == !row.witness_ids.is_empty())
                        .unwrap_or(true)
                    && filter
                        .handoff_required
                        .map(|required| required == !row.handoff_ids.is_empty())
                        .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>()
    }

    pub fn save_scenario_library_view(
        &self,
        path: impl AsRef<Path>,
        view: &ScenarioLibrarySavedView,
    ) -> Result<(), String> {
        let _ = self;
        let body = serde_json::to_string_pretty(view).map_err(|error| error.to_string())?;
        fs::write(path, body).map_err(|error| error.to_string())
    }

    pub fn read_scenario_library_view(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ScenarioLibrarySavedView, String> {
        let _ = self;
        let body = fs::read_to_string(path).map_err(|error| error.to_string())?;
        serde_json::from_str(&body).map_err(|error| error.to_string())
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
        let snapshot_ref = LibraryContextSnapshotRef::from(&snapshot);
        let environment = RuntimeEnvironment::new()
            .with_structure_context_version(StructureContextVersion(
                "onecalc:single_formula:v1".to_string(),
            ))
            .with_resolved_library_context(None, Some(snapshot_ref), Some(snapshot));
        let source =
            FormulaSourceRecord::new("onecalc.eval", 1, formula_text)
                .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
        let request = RuntimeFormulaRequest::new(
            source,
            TypedContextQueryBundle::new(None, None, None, Some(46_000.0), Some(0.25)),
        );
        let result = environment.execute(request)?;

        Ok(summarize_runtime_result(result))
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

        let formula_stable_id = formula_stable_id.into();
        let formula_text = formula_text.into();
        let structure_context_version = "onecalc:single_formula:h1".to_string();
        let session =
            RuntimeSessionFacade::new(build_driven_runtime_environment(structure_context_version.clone()));
        Ok(DrivenSingleFormulaHost {
            formula_stable_id,
            formula_text,
            formula_text_version: 1,
            formula_channel_kind: FormulaChannelKind::WorksheetA1,
            structure_context_version,
            session,
        })
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

        driven_host.set_formula_text(formula_text);
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
        let snapshot_ref = recalc_summary
            .library_context_snapshot_ref
            .clone()
            .ok_or_else(|| "driven recalc did not surface a library context snapshot ref".to_string())?;
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
            replay_projection: Some(recalc_summary.replay_projection.clone()),
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
        driven_host.restore_retained_state(
            retained.run.formula_text_version,
            parse_formula_channel_kind(&retained.scenario.formula_channel_kind)?,
            retained
                .scenario
                .provenance
                .structure_context_version
                .clone(),
        );

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
        let snapshot_ref = recalc_summary.library_context_snapshot_ref.clone();
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
            library_context_snapshot_ref: snapshot_ref,
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
        driven_host.restore_retained_state(
            document.formula_text_version,
            parse_formula_channel_kind(&document.formula_channel_kind)?,
            document.structure_context_version.clone(),
        );

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

    pub fn persist_workspace_manifest(
        &self,
        manifest_path: impl AsRef<Path>,
        workspace_name: impl Into<String>,
        document_paths: &[impl AsRef<Path>],
    ) -> Result<PersistedOneCalcWorkspace, String> {
        write_workspace_manifest(manifest_path, workspace_name, document_paths)
    }

    pub fn open_workspace(
        &self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<OpenedOneCalcWorkspace, String> {
        let manifest = read_workspace_manifest(manifest_path)?;
        let reopened_documents = manifest
            .document_entries
            .iter()
            .map(|entry| self.reopen_isolated_document(&entry.document_path))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(OpenedOneCalcWorkspace {
            manifest,
            reopened_documents,
        })
    }

    pub fn capture_windows_observation(
        &self,
        store: &RetainedScenarioStore,
        output_root: impl AsRef<Path>,
    ) -> Result<PersistedObservation, String> {
        let source = invoke_live_windows_capture(output_root)?;
        self.persist_observation_from_source(store, source)
    }

    pub fn persist_observation_from_existing_source(
        &self,
        store: &RetainedScenarioStore,
        source_root: impl AsRef<Path>,
    ) -> Result<PersistedObservation, String> {
        let source = load_observation_source_bundle(source_root)?;
        self.persist_observation_from_source(store, source)
    }

    pub fn persist_capability_snapshot(
        &self,
        store: &RetainedScenarioStore,
        packet_kind: &str,
        diff_base_snapshot_ref: Option<&str>,
    ) -> Result<PersistedCapabilitySnapshot, String> {
        let snapshot = self.emit_capability_snapshot(packet_kind, diff_base_snapshot_ref)?;
        store.persist_capability_snapshot(&snapshot)
    }

    pub fn open_capability_snapshot(
        &self,
        store: &RetainedScenarioStore,
        capability_snapshot_id: &str,
    ) -> Result<OpenedCapabilitySnapshotSummary, String> {
        let snapshot = store.read_capability_snapshot(capability_snapshot_id)?;

        Ok(OpenedCapabilitySnapshotSummary {
            capability_snapshot_id: snapshot.capability_snapshot_id,
            host_kind: snapshot.host_kind,
            runtime_platform: snapshot.runtime_platform,
            runtime_class: snapshot.runtime_class,
            dependency_set: snapshot.dependency_set,
            seam_pin_set_id: snapshot.seam_pin_set_id,
            capability_floor: snapshot.capability_floor,
            packet_kind_register: snapshot.packet_kind_register,
            function_surface_policy_id: snapshot.function_surface_policy_id,
            mode_availability: snapshot.mode_availability,
            provisional_seams: snapshot.provisional_seams,
            capability_ceilings: snapshot.capability_ceilings,
            lossiness: snapshot.lossiness,
            diff_base_snapshot_id: snapshot
                .diff_base_refs
                .first()
                .map(|snapshot_ref| snapshot_ref.logical_id.clone()),
        })
    }

    pub fn diff_capability_snapshots(
        &self,
        store: &RetainedScenarioStore,
        left_snapshot_id: &str,
        right_snapshot_id: &str,
    ) -> Result<CapabilitySnapshotDiffSummary, String> {
        let left = store.read_capability_snapshot(left_snapshot_id)?;
        let right = store.read_capability_snapshot(right_snapshot_id)?;

        let left_dependencies = to_sorted_set(left.dependency_set.iter().cloned());
        let right_dependencies = to_sorted_set(right.dependency_set.iter().cloned());
        let left_packet_kinds = to_sorted_set(left.packet_kind_register.iter().cloned());
        let right_packet_kinds = to_sorted_set(right.packet_kind_register.iter().cloned());
        let left_seams = to_sorted_set(left.provisional_seams.iter().cloned());
        let right_seams = to_sorted_set(right.provisional_seams.iter().cloned());
        let left_ceilings = to_sorted_set(left.capability_ceilings.iter().cloned());
        let right_ceilings = to_sorted_set(right.capability_ceilings.iter().cloned());

        Ok(CapabilitySnapshotDiffSummary {
            left_snapshot_id: left.capability_snapshot_id,
            right_snapshot_id: right.capability_snapshot_id,
            dependencies_added: set_added(&left_dependencies, &right_dependencies),
            dependencies_removed: set_removed(&left_dependencies, &right_dependencies),
            packet_kinds_added: set_added(&left_packet_kinds, &right_packet_kinds),
            packet_kinds_removed: set_removed(&left_packet_kinds, &right_packet_kinds),
            mode_changes: mode_changes(&left.mode_availability, &right.mode_availability),
            provisional_seams_added: set_added(&left_seams, &right_seams),
            provisional_seams_removed: set_removed(&left_seams, &right_seams),
            capability_ceilings_added: set_added(&left_ceilings, &right_ceilings),
            capability_ceilings_removed: set_removed(&left_ceilings, &right_ceilings),
            function_surface_policy_changed: left.function_surface_policy_id
                != right.function_surface_policy_id,
            runtime_class_changed: left.runtime_class != right.runtime_class,
            diff_floor: "immutable_capability_snapshot_diff".to_string(),
        })
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
        let replay_projection = reopened
            .run
            .replay_projection
            .clone()
            .unwrap_or_else(|| replay_projection_from_retained_run(&reopened.scenario, &reopened.run));
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
            serde_json::to_string_pretty(&replay_projection).map_err(|error| error.to_string())?;
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
        let persisted = store.persist_replay_capture(&capture, &replay_projection)?;

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
        let projection = read_replay_projection_record(&capture.replay_artifact.path)?;
        let replay_scenario = load_oxfml_v1_replay_projection(&capture.replay_artifact.path)
            .map_err(|error| {
                format!(
                    "failed to open replay capture {}: {}",
                    replay_capture_id, error
                )
            })?;
        let replay_ready = is_replay_ready(&replay_scenario);
        let event_count = replay_scenario.events.len();
        let registry_ref_count = replay_scenario.registry_refs.len();
        let projection_alias = replay_projection_alias(&projection).map(str::to_string);
        let projection_phase = projection.phase.clone();
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
            projection_source_artifact_family: projection.source_artifact_family,
            projection_phase,
            projection_alias,
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
        let (replay_capture_id, replay_floor, replay_projection_source_artifact_family, replay_projection_phase, replay_projection_alias) = reopened
            .run
            .replay_capture_ref
            .as_ref()
            .map(|replay_ref| {
                let opened = self.open_replay_capture(store, &replay_ref.logical_id)?;
                Ok::<_, String>((
                    Some(replay_ref.logical_id.clone()),
                    Some(opened.replay_floor),
                    Some(opened.projection_source_artifact_family),
                    opened.projection_phase,
                    opened.projection_alias,
                ))
            })
            .transpose()?
            .unwrap_or((None, None, None, None, None));

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
            formatting_truth_plane: formatting_truth_plane().to_string(),
            conditional_formatting_scope: conditional_formatting_truth_plane(),
            blocked_dimensions: vec![
                "conditional_formatting_rules_not_attached_to_retained_run".to_string()
            ],
            capability_snapshot_id,
            replay_capture_id,
            replay_floor,
            replay_projection_source_artifact_family,
            replay_projection_phase,
            replay_projection_alias,
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
            formatting_truth_plane: formatting_truth_plane().to_string(),
            conditional_formatting_scope: conditional_formatting_truth_plane(),
            blocked_dimensions: vec![
                "conditional_formatting_rules_not_attached_to_retained_run".to_string()
            ],
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
        let replay_projection_aliases = [left.run.replay_projection.as_ref(), right.run.replay_projection.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(replay_projection_alias)
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut explanation_lines = explanation_lines;
        if !replay_projection_aliases.is_empty() {
            explanation_lines.push(format!(
                "replay_projection_aliases={}",
                replay_projection_aliases.join("|")
            ));
        }
        let replay_projection_phases = [left.run.replay_projection.as_ref(), right.run.replay_projection.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(|projection| projection.phase.clone())
            .collect::<Vec<_>>();
        if !replay_projection_phases.is_empty() {
            explanation_lines.push(format!(
                "replay_projection_phases={}",
                replay_projection_phases.join("|")
            ));
        }
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
        let replay_projection_aliases = witness
            .explanation_lines
            .iter()
            .find_map(|line| line.strip_prefix("replay_projection_aliases="))
            .map(|value| value.split('|').map(str::to_string).collect())
            .unwrap_or_default();
        Ok(OpenedWitnessSummary {
            witness_id: witness.witness_id,
            scenario_id: witness.scenario_id,
            explain_floor: witness.explain_floor,
            explanation_lines: witness.explanation_lines,
            blocked_dimensions: witness.blocked_dimensions,
            replay_projection_aliases,
        })
    }

    pub fn generate_handoff_packet(
        &self,
        store: &RetainedScenarioStore,
        witness_id: &str,
    ) -> Result<PersistedHandoffPacket, String> {
        let witness = store.read_witness(witness_id)?;
        let source_run = store.reopen_run(&witness.left_run_ref.logical_id)?;
        let replay_projection_alias = source_run
            .run
            .replay_projection
            .as_ref()
            .and_then(replay_projection_alias)
            .map(str::to_string);
        let replay_projection_phase = source_run
            .run
            .replay_projection
            .as_ref()
            .and_then(|projection| projection.phase.clone());
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
                "witness floor={} with blocked dimensions={} replay_projection_alias={} replay_projection_phase={}",
                witness.explain_floor,
                witness.blocked_dimensions.join(","),
                replay_projection_alias.as_deref().unwrap_or("none"),
                replay_projection_phase.as_deref().unwrap_or("none")
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
            replay_projection_alias: handoff
                .observed_behavior
                .split(" replay_projection_alias=")
                .nth(1)
                .and_then(|tail| tail.split(" replay_projection_phase=").next())
                .filter(|value| *value != "none")
                .map(str::to_string),
            replay_projection_phase: handoff
                .observed_behavior
                .split(" replay_projection_phase=")
                .nth(1)
                .filter(|value| *value != "none")
                .map(str::to_string),
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
                capability_mode(
                    "Excel-observed",
                    "available",
                    Some("Windows OxXlObs capture-run integration persists retained Observation artifacts"),
                ),
                capability_mode(
                    "Twin compare",
                    "available",
                    Some("retained run versus Observation comparison artifacts and view opening are available for the first comparison envelope"),
                ),
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
                "observation_envelope_narrow".to_string(),
            ],
            capability_ceilings: vec![
                "single_formula_scope_only".to_string(),
                "no_worksheet_environment".to_string(),
                "no_multi_node_recalc".to_string(),
                "extension_abi_v1_host_managed_function_only".to_string(),
                "rtd_provider_not_admitted_yet".to_string(),
                "vba_bridge_not_admitted".to_string(),
            ],
            lossiness: vec!["capability_snapshot_uses_current_local_dependency_identity_only".to_string()],
            diff_base_refs,
        })
    }

    fn persist_observation_from_source(
        &self,
        store: &RetainedScenarioStore,
        source: crate::LoadedObservationSourceBundle,
    ) -> Result<PersistedObservation, String> {
        let capability_snapshot =
            self.emit_capability_snapshot(HostPacketKind::ObservationCapture.id(), None)?;
        let persisted_capability = store.persist_capability_snapshot(&capability_snapshot)?;
        let observation_id = format!("observation-{}", sanitize_slug(&source.provenance.run_id));
        let emitted_at_unix_ms = unix_time_millis()?;
        let source_artifact_ref = StableArtifactRef {
            artifact_kind: "oxxlobs_bundle".to_string(),
            logical_id: source
                .bundle_path
                .as_ref()
                .unwrap_or(&source.capture_path)
                .display()
                .to_string(),
            content_hash: None,
        };
        let replay_manifest_ref =
            source
                .replay_manifest_path
                .as_ref()
                .map(|path| StableArtifactRef {
                    artifact_kind: "oxxlobs_replay_manifest".to_string(),
                    logical_id: path.display().to_string(),
                    content_hash: None,
                });
        let normalized_replay_ref =
            source
                .normalized_replay_path
                .as_ref()
                .map(|path| StableArtifactRef {
                    artifact_kind: "oxxlobs_normalized_replay".to_string(),
                    logical_id: path.display().to_string(),
                    content_hash: None,
                });
        let projection_status = if normalized_replay_ref.is_some() {
            "lossy_normalized_replay_available"
        } else {
            "source_bundle_only"
        };
        let lossiness = observation_lossiness(&source);
        let capture_body =
            serde_json::to_string(&source.capture).map_err(|error| error.to_string())?;
        let provenance_body =
            serde_json::to_string(&source.provenance).map_err(|error| error.to_string())?;
        let content_hash = stable_hash(&(
            &observation_id,
            &source.provenance.scenario_id,
            &source.provenance.run_id,
            &capture_body,
            &provenance_body,
            projection_status,
            lossiness.as_slice(),
        ));
        let observation = ObservationRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.observation".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::Observation.id().to_string(),
                logical_id: observation_id.clone(),
                content_hash,
                created_at_unix_ms: emitted_at_unix_ms,
                created_by_build: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
                host_profile_id: self.host_profile.id().to_string(),
                packet_kind: HostPacketKind::ObservationCapture.id().to_string(),
                seam_pin_set_id: "onecalc:ws-08:observation".to_string(),
                capability_floor: self.host_profile.id().to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(persisted_capability.snapshot.envelope.stable_ref()),
            },
            observation_id,
            scenario_id: source.provenance.scenario_id.clone(),
            source_lane_id: "OxXlObs/Excel-observed".to_string(),
            source_schema_id: "oxxlobs.capture_surface_basic.v1".to_string(),
            source_artifact_ref,
            capture_mode: "capture-run".to_string(),
            projection_status: projection_status.to_string(),
            provenance_ref: StableArtifactRef {
                artifact_kind: "oxxlobs_provenance".to_string(),
                logical_id: source.provenance_path.display().to_string(),
                content_hash: None,
            },
            capture_loss_ref: StableArtifactRef {
                artifact_kind: "oxxlobs_capture".to_string(),
                logical_id: source.capture_path.display().to_string(),
                content_hash: None,
            },
            platform_scope: "windows_live_capture;cross_platform_retained_consumption".to_string(),
            replay_manifest_ref,
            normalized_replay_ref,
            capture: source.capture,
            provenance: source.provenance,
            lossiness,
        };

        store.persist_observation(&observation)
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

    pub fn compare_run_with_observation(
        &self,
        store: &RetainedScenarioStore,
        scenario_run_id: &str,
        observation_id: &str,
    ) -> Result<PersistedComparison, String> {
        let reopened = store.reopen_run(scenario_run_id)?;
        let observation = store.read_observation(observation_id)?;
        let value_surface = observation
            .capture
            .surfaces
            .iter()
            .find(|surface| surface.surface.surface_kind == "cell_value")
            .ok_or_else(|| {
                format!("observation {observation_id} is missing a cell_value surface")
            })?;
        let formula_surface = observation
            .capture
            .surfaces
            .iter()
            .find(|surface| surface.surface.surface_kind == "formula_text")
            .ok_or_else(|| {
                format!("observation {observation_id} is missing a formula_text surface")
            })?;
        let left_value = reopened.run.worksheet_value_summary.clone();
        let right_value = normalize_observation_surface_value(value_surface);
        let value_agreement = left_value == right_value;
        let formula_agreement = reopened.run.authored_formula_text
            == formula_surface.value_repr.clone().unwrap_or_default();
        let reliability_badge =
            comparison_reliability_badge(&observation, value_surface, formula_surface);
        let projection_limitations = vec![
            "display_state_not_in_current_observation_envelope".to_string(),
            "formatting_not_in_current_observation_envelope".to_string(),
            "conditional_formatting_not_in_current_observation_envelope".to_string(),
        ];
        let comparison_id = format!(
            "comparison-{}-{}",
            sanitize_slug(scenario_run_id),
            sanitize_slug(observation_id)
        );
        let content_hash = stable_hash(&(
            &comparison_id,
            scenario_run_id,
            observation_id,
            &left_value,
            &right_value,
            reopened.run.authored_formula_text.as_str(),
            formula_surface.value_repr.as_deref().unwrap_or(""),
            &reliability_badge,
        ));
        let comparison = ComparisonRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.comparison".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::Comparison.id().to_string(),
                logical_id: comparison_id.clone(),
                content_hash,
                created_at_unix_ms: unix_time_millis()?,
                created_by_build: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
                host_profile_id: self.host_profile.id().to_string(),
                packet_kind: HostPacketKind::ObservationCapture.id().to_string(),
                seam_pin_set_id: "onecalc:ws-08:comparison".to_string(),
                capability_floor: self.host_profile.id().to_string(),
                provisionality_state: if reliability_badge == "provisional" {
                    "provisional".to_string()
                } else {
                    "stable".to_string()
                },
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: observation.envelope.capability_snapshot_ref.clone(),
            },
            comparison_id,
            left_artifact_ref: reopened.run.envelope.stable_ref(),
            right_artifact_ref: observation.envelope.stable_ref(),
            comparison_envelope: vec!["worksheet_value".to_string(), "formula_text".to_string()],
            mismatches: vec![
                ComparisonMismatchRecord {
                    dimension_id: "worksheet_value".to_string(),
                    left_summary: left_value,
                    right_summary: right_value,
                    agreement: value_agreement,
                    status: if value_agreement { "match" } else { "mismatch" }.to_string(),
                    note: Some(format!(
                        "observation surface status={} capture_loss={}",
                        value_surface.status, value_surface.capture_loss
                    )),
                },
                ComparisonMismatchRecord {
                    dimension_id: "formula_text".to_string(),
                    left_summary: reopened.run.authored_formula_text,
                    right_summary: formula_surface.value_repr.clone().unwrap_or_default(),
                    agreement: formula_agreement,
                    status: if formula_agreement {
                        "match"
                    } else {
                        "mismatch"
                    }
                    .to_string(),
                    note: Some(format!(
                        "observation surface status={} capture_loss={}",
                        formula_surface.status, formula_surface.capture_loss
                    )),
                },
            ],
            reliability_badge,
            projection_limitations,
            explanation_refs: Vec::new(),
            witness_candidate_refs: Vec::new(),
        };

        store.persist_comparison(&comparison)
    }

    pub fn open_twin_compare(
        &self,
        store: &RetainedScenarioStore,
        comparison_id: &str,
    ) -> Result<OpenedTwinCompareSummary, String> {
        let comparison = store.read_comparison(comparison_id)?;

        Ok(OpenedTwinCompareSummary {
            comparison_id: comparison.comparison_id,
            left_run_id: comparison.left_artifact_ref.logical_id,
            observation_id: comparison.right_artifact_ref.logical_id,
            comparison_envelope: comparison.comparison_envelope,
            reliability_badge: comparison.reliability_badge,
            mismatch_lines: comparison
                .mismatches
                .into_iter()
                .map(|mismatch| {
                    format!(
                        "{}:{}:{}:{}:{}",
                        mismatch.dimension_id,
                        mismatch.status,
                        mismatch.left_summary,
                        mismatch.right_summary,
                        mismatch.note.unwrap_or_default()
                    )
                })
                .collect(),
            projection_limitations: comparison.projection_limitations,
        })
    }

    pub fn generate_observation_widening_handoff(
        &self,
        store: &RetainedScenarioStore,
        comparison_id: &str,
    ) -> Result<PersistedHandoffPacket, String> {
        let comparison = store.read_comparison(comparison_id)?;
        if comparison.projection_limitations.is_empty() {
            return Err(format!(
                "comparison {comparison_id} does not have blocked dimensions to widen"
            ));
        }

        let source_run = store.reopen_run(&comparison.left_artifact_ref.logical_id)?;
        let observation = store.read_observation(&comparison.right_artifact_ref.logical_id)?;
        let capability_snapshot_ref = observation
            .envelope
            .capability_snapshot_ref
            .clone()
            .or_else(|| source_run.run.envelope.capability_snapshot_ref.clone())
            .ok_or_else(|| {
                format!(
                    "compare pair {} / {} is missing a capability snapshot ref",
                    source_run.run.scenario_run_id, observation.observation_id
                )
            })?;
        let emitted_at_unix_ms = unix_time_millis()?;
        let handoff_id = format!("handoff-widen-{}", comparison_id);
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
        let status = "ready".to_string();
        let content_hash = stable_hash(&(
            &handoff_id,
            &comparison.comparison_id,
            comparison.projection_limitations.as_slice(),
            &status,
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
                seam_pin_set_id: "onecalc:ws-08:widening".to_string(),
                capability_floor: self.host_profile.id().to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: vec![
                    ArtifactLineageRef {
                        relation: "source_run".to_string(),
                        artifact_ref: source_run.run.envelope.stable_ref(),
                    },
                    ArtifactLineageRef {
                        relation: "observation".to_string(),
                        artifact_ref: observation.envelope.stable_ref(),
                    },
                    ArtifactLineageRef {
                        relation: "comparison".to_string(),
                        artifact_ref: comparison.envelope.stable_ref(),
                    },
                ],
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            handoff_id,
            scenario_id: source_run.run.scenario_id.clone(),
            source_run_ref: source_run.run.envelope.stable_ref(),
            witness_ref: comparison.envelope.stable_ref(),
            capability_snapshot_ref,
            requested_action_kind: "widen_observation_envelope".to_string(),
            target_lane: "OxXlObs/DnaOneCalc".to_string(),
            expected_behavior: "the observation envelope should cover the validation dimensions needed by the active twin compare".to_string(),
            observed_behavior: format!(
                "current compare envelope={} blocked dimensions={}",
                comparison.comparison_envelope.join(","),
                comparison.projection_limitations.join(",")
            ),
            supporting_artifact_refs: vec![
                source_run.run.envelope.stable_ref(),
                observation.envelope.stable_ref(),
                comparison.envelope.stable_ref(),
            ],
            reliability_state: format!("compare_{}", comparison.reliability_badge),
            status,
            readiness,
            emitted_at_unix_ms,
        };

        store.persist_handoff_packet(&handoff)
    }

    pub fn collect_completion_proposals(
        &self,
        session: &FormulaEditorSession,
        cursor_offset: usize,
    ) -> Vec<CompletionProposalSummary> {
        let Some(document) = session.latest_result() else {
            return Vec::new();
        };
        let completion = build_editor_service(&document.source).completion_at_cursor(document, cursor_offset);

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
        let document = session.latest_result()?;
        let interaction = build_editor_service(&document.source).interact_at_cursor(document, cursor_offset);
        let function_help = interaction.function_help_packet?;
        let display_signature = function_help
            .signature_forms
            .first()
            .map(|signature| signature.display_signature.clone())
            .unwrap_or_else(|| function_help.display_name.clone());
        let availability_summary = function_help
            .availability_summary
            .unwrap_or_else(|| "availability unknown".to_string());

        Some(FunctionHelpSummary {
            display_name: function_help.display_name,
            display_signature,
            active_argument_index: interaction
                .signature_help_context
                .map(|context| context.active_argument_index)
                .unwrap_or(0),
            availability_summary,
            provisional: function_help.deferred_or_profile_limited,
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
        let result = build_editor_service(&source).apply_edit(
            source,
            session.latest_result.as_ref(),
            EditorAnalysisStage::SyntaxAndBind,
            None,
        );
        let document = result.document;

        session.formula_text_version += 1;

        let summary = FormulaEditPacketSummary {
            formula_token: document.source.formula_token().0,
            diagnostic_count: document.live_diagnostics.diagnostics.len(),
            text_change_range: document.text_change_range,
            reused_green_tree: document.reuse_summary.reused_green_tree,
            reused_red_projection: document.reuse_summary.reused_red_projection,
            reused_bound_formula: document.reuse_summary.reused_bound_formula,
        };

        session.latest_result = Some(document);
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

        let query_bundle = TypedContextQueryBundle::new(
            None,
            None,
            None,
            recalc_context.now_serial,
            recalc_context.random_value,
        );
        let result = driven_host.session.execute(RuntimeFormulaRequest::new(
            driven_host.formula_source(),
            query_bundle,
        ))?;
        let replay_projection = oxfml_replay_projection_record(
            ReplayProjectionService::project(ReplayProjectionRequest::runtime_result(&result)),
        );
        let library_context_snapshot_ref = result
            .library_context_snapshot_ref
            .as_ref()
            .map(LibraryContextSnapshotRef::compound_ref);
        let evaluation = summarize_runtime_result(result);

        Ok(DrivenRecalcSummary {
            host_profile_id: self.host_profile.id().to_string(),
            trigger_kind: recalc_context.trigger_kind.id().to_string(),
            packet_kind: recalc_context.packet_kind().id().to_string(),
            formula_text_version: driven_host.formula_text_version,
            structure_context_version: driven_host.structure_context_version.clone(),
            library_context_snapshot_ref,
            replay_projection,
            evaluation,
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

fn build_editor_service(source: &FormulaSourceRecord) -> EditorEditService<'static> {
    let bind_context = build_bind_context(source);
    let snapshot = FunctionSurfaceCatalog::load_current().admitted_execution_snapshot();
    let environment =
        EditorEnvironment::new(bind_context).with_inline_library_context_snapshot(snapshot);
    EditorEditService::new(environment)
}

fn build_driven_runtime_environment(
    structure_context_version: impl Into<String>,
) -> RuntimeEnvironment<'static> {
    let snapshot = FunctionSurfaceCatalog::load_current().admitted_execution_snapshot();
    let snapshot_ref = LibraryContextSnapshotRef::from(&snapshot);
    RuntimeEnvironment::new()
        .with_structure_context_version(StructureContextVersion(structure_context_version.into()))
        .with_resolved_library_context(None, Some(snapshot_ref), Some(snapshot))
}

fn oxfml_replay_projection_record(
    projection: ReplayProjectionResult,
) -> OxfmlReplayProjectionRecord {
    OxfmlReplayProjectionRecord {
        source_artifact_family: projection.source_artifact_family,
        source_case_id: projection.source_case_id,
        source_case_ids: projection.source_case_ids,
        shared_scenario_alias: projection.shared_scenario_alias,
        formula_stable_id: projection.formula_stable_id,
        session_id: projection.session_id,
        library_context_snapshot_ref: projection
            .library_context_snapshot_ref
            .map(|snapshot_ref| snapshot_ref.compound_ref()),
        phase: projection.phase,
        candidate_result_id: projection.candidate_result_id,
        commit_decision_kind: projection.commit_decision_kind,
        trace_event_kinds: projection.trace_event_kinds,
    }
}

fn replay_projection_from_retained_run(
    scenario: &ScenarioRecord,
    run: &ScenarioRunRecord,
) -> OxfmlReplayProjectionRecord {
    OxfmlReplayProjectionRecord {
        source_artifact_family: "runtime_formula_result".to_string(),
        source_case_id: None,
        source_case_ids: Vec::new(),
        shared_scenario_alias: Some(scenario.scenario_slug.clone()),
        formula_stable_id: scenario.provenance.formula_stable_id.clone(),
        session_id: run
            .candidate_ref
            .as_ref()
            .map(|candidate| candidate.logical_id.clone()),
        library_context_snapshot_ref: scenario.library_context_snapshot_ref.clone(),
        phase: Some("CommittedOrRejected".to_string()),
        candidate_result_id: run.candidate_ref.as_ref().map(|candidate| candidate.logical_id.clone()),
        commit_decision_kind: Some(run.commit_decision_kind.clone()),
        trace_event_kinds: match run.commit_decision_kind.as_str() {
            "accepted" => vec![
                "CandidateAccepted".to_string(),
                "PublicationCommitted".to_string(),
            ],
            _ => vec!["RejectIssued".to_string()],
        },
    }
}

fn read_replay_projection_record(path: impl AsRef<Path>) -> Result<OxfmlReplayProjectionRecord, String> {
    let source = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&source).map_err(|error| error.to_string())
}

fn replay_projection_alias(projection: &OxfmlReplayProjectionRecord) -> Option<&str> {
    projection
        .shared_scenario_alias
        .as_deref()
        .or(projection.source_case_id.as_deref())
        .or_else(|| projection.source_case_ids.first().map(String::as_str))
}

fn summarize_runtime_result(result: RuntimeFormulaResult) -> FormulaEvaluationSummary {
    let returned_presentation_hint_status =
        summarize_presentation_hint(result.returned_value_surface.presentation_hint);
    let host_style_state_status = summarize_host_style_state();

    FormulaEvaluationSummary {
        formula_token: result.source.formula_token().0,
        worksheet_value_summary: summarize_eval_value(&result.published_worksheet_value),
        payload_summary: result.returned_value_surface.payload_summary.clone(),
        returned_value_surface_kind: format!("{:?}", result.returned_value_surface.kind),
        returned_presentation_hint_status: returned_presentation_hint_status.clone(),
        host_style_state_status: host_style_state_status.clone(),
        effective_display_status: derive_effective_display_status(
            &returned_presentation_hint_status,
            &host_style_state_status,
        ),
        commit_decision_kind: match result.commit_decision {
            oxfml_core::AcceptDecision::Accepted(_) => "accepted".to_string(),
            oxfml_core::AcceptDecision::Rejected(_) => "rejected".to_string(),
        },
        trace_event_count: result.trace_events.len(),
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

fn to_sorted_set(values: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

fn set_added(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    right.difference(left).cloned().collect()
}

fn set_removed(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn mode_changes(
    left: &[CapabilityModeAvailabilityRecord],
    right: &[CapabilityModeAvailabilityRecord],
) -> Vec<String> {
    let left_map = left
        .iter()
        .map(|mode| {
            (
                mode.mode_id.clone(),
                (mode.state.clone(), mode.reason.clone().unwrap_or_default()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let right_map = right
        .iter()
        .map(|mode| {
            (
                mode.mode_id.clone(),
                (mode.state.clone(), mode.reason.clone().unwrap_or_default()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mode_ids = left_map
        .keys()
        .chain(right_map.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();

    for mode_id in mode_ids {
        match (left_map.get(&mode_id), right_map.get(&mode_id)) {
            (Some((left_state, left_reason)), Some((right_state, right_reason))) => {
                if left_state != right_state || left_reason != right_reason {
                    let left_text = if left_reason.is_empty() {
                        left_state.clone()
                    } else {
                        format!("{left_state} ({left_reason})")
                    };
                    let right_text = if right_reason.is_empty() {
                        right_state.clone()
                    } else {
                        format!("{right_state} ({right_reason})")
                    };
                    changes.push(format!("{mode_id}: {left_text} -> {right_text}"));
                }
            }
            (None, Some((right_state, right_reason))) => {
                let right_text = if right_reason.is_empty() {
                    right_state.clone()
                } else {
                    format!("{right_state} ({right_reason})")
                };
                changes.push(format!("{mode_id}: added -> {right_text}"));
            }
            (Some((left_state, left_reason)), None) => {
                let left_text = if left_reason.is_empty() {
                    left_state.clone()
                } else {
                    format!("{left_state} ({left_reason})")
                };
                changes.push(format!("{mode_id}: {left_text} -> removed"));
            }
            (None, None) => {}
        }
    }

    changes
}

fn observation_lossiness(source: &crate::LoadedObservationSourceBundle) -> Vec<String> {
    let mut lossiness = Vec::new();

    if source.normalized_replay_path.is_some() {
        lossiness.push("normalized_replay_projection_is_lossy".to_string());
    }
    if source.capture.interpretation.bridge_influenced {
        lossiness.push("bridge_influenced_capture".to_string());
    }
    for item in &source.provenance.capture_loss_summary {
        lossiness.push(format!("capture_loss:{item}"));
    }
    for item in &source.provenance.uncertainty_summary {
        lossiness.push(format!("uncertainty:{item}"));
    }
    if lossiness.is_empty() {
        lossiness.push("none".to_string());
    }

    lossiness
}

fn normalize_observation_surface_value(surface: &crate::ObservationSurfaceValue) -> String {
    match surface.surface.surface_kind.as_str() {
        "cell_value" => surface
            .value_repr
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .map(|value| format!("Number({value})"))
            .unwrap_or_else(|| {
                surface
                    .value_repr
                    .clone()
                    .unwrap_or_else(|| "none".to_string())
            }),
        _ => surface
            .value_repr
            .clone()
            .unwrap_or_else(|| "none".to_string()),
    }
}

fn comparison_reliability_badge(
    observation: &ObservationRecord,
    value_surface: &crate::ObservationSurfaceValue,
    formula_surface: &crate::ObservationSurfaceValue,
) -> String {
    if observation.capture.interpretation.bridge_influenced {
        return "provisional".to_string();
    }
    if value_surface.status == "derived" || formula_surface.status == "derived" {
        return "derived".to_string();
    }
    if observation
        .lossiness
        .iter()
        .any(|item| item == "normalized_replay_projection_is_lossy")
        && (value_surface.status != "direct" || formula_surface.status != "direct")
    {
        return "lossy".to_string();
    }
    "direct".to_string()
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

fn formatting_truth_plane() -> &'static str {
    "returned_presentation_hint+host_style_state=>effective_display"
}

fn conditional_formatting_truth_plane() -> String {
    IsolatedConditionalFormattingCarrier::policy_text()
}
