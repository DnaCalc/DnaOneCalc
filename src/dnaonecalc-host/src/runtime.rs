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

use crate::retained::{
    PersistedScenarioRun, RetainedProvenanceRecord, RetainedRecalcContextRecord,
    RetainedScenarioStore, ScenarioRecord, ScenarioRunRecord,
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

        let scenario = ScenarioRecord {
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
        let run = ScenarioRunRecord {
            scenario_run_id,
            scenario_id,
            formula_text_version: recalc_summary.formula_text_version,
            formula_token: recalc_summary.evaluation.formula_token.clone(),
            authored_formula_text: driven_host.formula_text().to_string(),
            build_id: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
            runtime_platform: std::env::consts::OS.to_string(),
            seam_pin_set_id: "onecalc:ws-04:h1".to_string(),
            effective_capability_floor: self.host_profile.id().to_string(),
            result_surface_ref: format!("result-surface:{}", recalc_summary.evaluation.formula_token),
            candidate_ref: Some(format!(
                "candidate:{}",
                recalc_summary.evaluation.formula_token
            )),
            commit_ref: if recalc_summary.evaluation.commit_decision_kind == "accepted" {
                Some(format!("commit:{}", recalc_summary.evaluation.formula_token))
            } else {
                None
            },
            reject_ref: if recalc_summary.evaluation.commit_decision_kind == "rejected" {
                Some(format!("reject:{}", recalc_summary.evaluation.formula_token))
            } else {
                None
            },
            trace_ref: Some(format!("trace:{}", recalc_summary.evaluation.formula_token)),
            replay_capture_ref: None,
            function_surface_effective_id: format!(
                "{}:{}",
                function_surface_policy_id, snapshot_ref
            ),
            projection_status: "direct".to_string(),
            provisionality_status: if recalc_summary.packet_kind == HostPacketKind::ForcedRecalc.id()
            {
                "forced".to_string()
            } else {
                "stable".to_string()
            },
            worksheet_value_summary: recalc_summary.evaluation.worksheet_value_summary.clone(),
            payload_summary: recalc_summary.evaluation.payload_summary.clone(),
            returned_value_surface_kind: recalc_summary.evaluation.returned_value_surface_kind.clone(),
            effective_display_status: recalc_summary.evaluation.effective_display_status.clone(),
            commit_decision_kind: recalc_summary.evaluation.commit_decision_kind.clone(),
            executed_at_unix_ms,
        };

        store.persist_scenario_and_run(&scenario, &run)
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
        driven_host.host.structure_context_version =
            retained.scenario.provenance.structure_context_version.clone();
        driven_host.host.formula_channel_kind =
            parse_formula_channel_kind(&retained.scenario.formula_channel_kind)?;

        Ok(ReopenedDrivenSingleFormulaRun {
            retained,
            driven_host,
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
    FormulaEvaluationSummary {
        formula_token: output.source.formula_token().0,
        worksheet_value_summary: summarize_eval_value(&output.published_worksheet_value),
        payload_summary: output.returned_value_surface.payload_summary.clone(),
        returned_value_surface_kind: format!("{:?}", output.returned_value_surface.kind),
        effective_display_status: summarize_presentation_hint(
            output.returned_value_surface.presentation_hint,
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
        _ => Err(format!("unsupported retained formula channel kind: {value}")),
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
