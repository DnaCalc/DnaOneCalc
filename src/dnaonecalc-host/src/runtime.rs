use oxfml_core::{
    apply_formula_edit, build_function_help_lookup_request, collect_completion_proposals,
    parse_formula, BindContext, CompletionRequest, EditFollowOnStage, EvaluationBackend,
    FormulaChannelKind, FormulaEditRequest, FormulaEditResult, FormulaSourceRecord,
    FormulaTextChangeRange, InMemoryLibraryContextProvider, ParseRequest, SingleFormulaHost,
    StructureContextVersion, TypedContextQueryBundle,
};
use oxfunc_core::value::EvalValue;
use oxfunc_core::xll_export_specs::lookup_function_meta_by_surface_name;

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

        Ok(FormulaEvaluationSummary {
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
}

fn build_bind_context(source: &FormulaSourceRecord) -> BindContext {
    let mut bind_context = BindContext::default();
    bind_context.formula_token = source.formula_token();
    bind_context.structure_context_version =
        StructureContextVersion("onecalc:single_formula:v1".to_string());
    bind_context
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
