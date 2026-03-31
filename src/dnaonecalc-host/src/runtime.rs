use oxfml_core::{
    apply_formula_edit, parse_formula, BindContext, EditFollowOnStage, FormulaChannelKind,
    FormulaEditRequest, FormulaEditResult, FormulaSourceRecord, FormulaTextChangeRange,
    ParseRequest, StructureContextVersion,
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

        let mut bind_context = BindContext::default();
        bind_context.formula_token = source.formula_token();
        bind_context.structure_context_version =
            StructureContextVersion("onecalc:single_formula:v1".to_string());

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
