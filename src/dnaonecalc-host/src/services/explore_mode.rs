use crate::adapters::oxfml::{
    CompletionProposal, CompletionProposalKind, FormulaTextSpan, FunctionHelpPacket,
    SignatureHelpContext,
};
use crate::state::{FormulaArrayPreviewState, FormulaSpaceState};
use crate::ui::editor::geometry::EditorOverlayGeometrySnapshot;
use crate::ui::editor::render_projection::{
    syntax_runs_from_snapshot, syntax_runs_from_text, SyntaxRun,
};
use crate::ui::editor::state::EditorSurfaceState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreViewModel {
    pub scenario_label: String,
    pub truth_source_label: String,
    pub host_profile_summary: String,
    pub packet_kind_summary: String,
    pub capability_floor_summary: String,
    pub mode_availability_summary: String,
    pub trace_summary: Option<String>,
    pub blocked_reason: Option<String>,
    pub raw_entered_cell_text: String,
    pub editor_surface_state: EditorSurfaceState,
    pub overlay_geometry: Option<EditorOverlayGeometrySnapshot>,
    pub syntax_runs: Vec<SyntaxRun>,
    pub diagnostics: Vec<ExploreDiagnosticView>,
    pub completion_count: usize,
    pub completion_items: Vec<ExploreCompletionItemView>,
    pub has_signature_help: bool,
    pub signature_help: Option<ExploreSignatureHelpView>,
    pub function_help: Option<ExploreFunctionHelpView>,
    pub function_help_lookup_key: Option<String>,
    pub result_value_summary: Option<String>,
    pub effective_display_summary: Option<String>,
    pub latest_evaluation_summary: Option<String>,
    pub array_preview: Option<ExploreArrayPreviewView>,
    pub green_tree_key: Option<String>,
    pub reused_green_tree: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreArrayPreviewView {
    pub label: String,
    pub rows: Vec<Vec<String>>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreDiagnosticView {
    pub diagnostic_id: String,
    pub message: String,
    pub span_start: usize,
    pub span_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreCompletionItemView {
    pub proposal_id: String,
    pub proposal_kind: ExploreCompletionKindView,
    pub display_text: String,
    pub insert_text: String,
    pub replacement_span: Option<FormulaTextSpan>,
    pub documentation_ref: Option<String>,
    pub requires_revalidation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExploreCompletionKindView {
    Function,
    DefinedName,
    TableName,
    TableColumn,
    StructuredSelector,
    SyntaxAssist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreSignatureHelpView {
    pub callee_text: String,
    pub call_span: FormulaTextSpan,
    pub active_argument_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreFunctionHelpView {
    pub lookup_key: String,
    pub display_name: String,
    pub signature_forms: Vec<ExploreFunctionHelpSignatureView>,
    pub argument_help: Vec<String>,
    pub short_description: Option<String>,
    pub availability_summary: Option<String>,
    pub deferred_or_profile_limited: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreFunctionHelpSignatureView {
    pub display_signature: String,
    pub min_arity: usize,
    pub max_arity: Option<usize>,
}

pub fn build_explore_view_model(formula_space: &FormulaSpaceState) -> ExploreViewModel {
    let (
        syntax_runs,
        diagnostics,
        green_tree_key,
        reused_green_tree,
        completion_items,
        signature_help,
        function_help,
        document_function_help_lookup_key,
    ) = match &formula_space.editor_document {
        Some(document) if document.source_text == formula_space.raw_entered_cell_text => (
            syntax_runs_from_snapshot(&document.editor_syntax_snapshot),
            document
                .live_diagnostics
                .diagnostics
                .iter()
                .map(|diagnostic| ExploreDiagnosticView {
                    diagnostic_id: diagnostic.diagnostic_id.clone(),
                    message: diagnostic.message.clone(),
                    span_start: diagnostic.span_start,
                    span_len: diagnostic.span_len,
                })
                .collect(),
            Some(document.green_tree_key().to_string()),
            document.reuse_summary.reused_green_tree,
            document
                .completion_proposals
                .iter()
                .map(completion_item_view)
                .collect(),
            document.signature_help.as_ref().map(signature_help_view),
            document.function_help.as_ref().map(function_help_view),
            document
                .function_help
                .as_ref()
                .map(|packet| packet.lookup_key.clone()),
        ),
        None => fallback_projection(&formula_space.raw_entered_cell_text),
        Some(_) => fallback_projection(&formula_space.raw_entered_cell_text),
    };

    let mut editor_surface_state = formula_space.editor_surface_state.clone();
    if !completion_items.is_empty() && editor_surface_state.completion_selected_index.is_none() {
        editor_surface_state.completion_selected_index = Some(0);
    }

    ExploreViewModel {
        scenario_label: formula_space.context.scenario_label.clone(),
        truth_source_label: formula_space.context.truth_source.label().to_string(),
        host_profile_summary: formula_space.context.host_profile.clone(),
        packet_kind_summary: formula_space.context.packet_kind.clone(),
        capability_floor_summary: formula_space.context.capability_floor.clone(),
        mode_availability_summary: formula_space.context.mode_availability.clone(),
        trace_summary: formula_space.context.trace_summary.clone(),
        blocked_reason: formula_space
            .context
            .blocked_reason
            .clone()
            .or_else(|| {
                formula_space
                    .editor_document
                    .as_ref()
                    .and_then(|document| document.provenance_summary.as_ref())
                    .and_then(|summary| summary.blocked_reason.clone())
            }),
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        editor_surface_state,
        overlay_geometry: formula_space.editor_overlay_geometry.clone(),
        syntax_runs,
        diagnostics,
        completion_count: formula_space.completion_help.completion_count,
        completion_items,
        has_signature_help: formula_space.completion_help.has_signature_help,
        signature_help,
        function_help,
        function_help_lookup_key: formula_space
            .completion_help
            .function_help_lookup_key
            .clone()
            .or(document_function_help_lookup_key),
        result_value_summary: formula_space.latest_evaluation_summary.clone(),
        effective_display_summary: formula_space.effective_display_summary.clone(),
        latest_evaluation_summary: formula_space.latest_evaluation_summary.clone(),
        array_preview: formula_space
            .array_preview
            .as_ref()
            .map(array_preview_view),
        green_tree_key,
        reused_green_tree,
    }
}

fn fallback_projection(
    text: &str,
) -> (
    Vec<SyntaxRun>,
    Vec<ExploreDiagnosticView>,
    Option<String>,
    bool,
    Vec<ExploreCompletionItemView>,
    Option<ExploreSignatureHelpView>,
    Option<ExploreFunctionHelpView>,
    Option<String>,
) {
    (
        syntax_runs_from_text(text),
        Vec::new(),
        None,
        false,
        Vec::new(),
        None,
        None,
        None,
    )
}

fn completion_item_view(proposal: &CompletionProposal) -> ExploreCompletionItemView {
    ExploreCompletionItemView {
        proposal_id: proposal.proposal_id.clone(),
        proposal_kind: completion_kind_view(&proposal.proposal_kind),
        display_text: proposal.display_text.clone(),
        insert_text: proposal.insert_text.clone(),
        replacement_span: proposal.replacement_span,
        documentation_ref: proposal.documentation_ref.clone(),
        requires_revalidation: proposal.requires_revalidation,
    }
}

fn signature_help_view(context: &SignatureHelpContext) -> ExploreSignatureHelpView {
    ExploreSignatureHelpView {
        callee_text: context.callee_text.clone(),
        call_span: context.call_span,
        active_argument_index: context.active_argument_index,
    }
}

fn function_help_view(packet: &FunctionHelpPacket) -> ExploreFunctionHelpView {
    ExploreFunctionHelpView {
        lookup_key: packet.lookup_key.clone(),
        display_name: packet.display_name.clone(),
        signature_forms: packet
            .signature_forms
            .iter()
            .map(|form| ExploreFunctionHelpSignatureView {
                display_signature: form.display_signature.clone(),
                min_arity: form.min_arity,
                max_arity: form.max_arity,
            })
            .collect(),
        argument_help: packet.argument_help.clone(),
        short_description: packet.short_description.clone(),
        availability_summary: packet.availability_summary.clone(),
        deferred_or_profile_limited: packet.deferred_or_profile_limited,
    }
}

fn completion_kind_view(kind: &CompletionProposalKind) -> ExploreCompletionKindView {
    match kind {
        CompletionProposalKind::Function => ExploreCompletionKindView::Function,
        CompletionProposalKind::DefinedName => ExploreCompletionKindView::DefinedName,
        CompletionProposalKind::TableName => ExploreCompletionKindView::TableName,
        CompletionProposalKind::TableColumn => ExploreCompletionKindView::TableColumn,
        CompletionProposalKind::StructuredSelector => ExploreCompletionKindView::StructuredSelector,
        CompletionProposalKind::SyntaxAssist => ExploreCompletionKindView::SyntaxAssist,
    }
}

fn array_preview_view(state: &FormulaArrayPreviewState) -> ExploreArrayPreviewView {
    ExploreArrayPreviewView {
        label: state.label.clone(),
        rows: state.rows.clone(),
        truncated: state.truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{
        CompletionProposal, CompletionProposalKind, EditorDocument, EditorSyntaxSnapshot,
        EditorToken, FormulaEditReuseSummary, FormulaTextSpan, FunctionHelpPacket,
        FunctionHelpSignatureForm, LiveDiagnostic, LiveDiagnosticSnapshot, SignatureHelpContext,
    };
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::{CompletionHelpState, FormulaSpaceState};

    #[test]
    fn explore_view_model_projects_editor_document_and_help_state() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
        formula_space.completion_help = CompletionHelpState {
            completion_count: 2,
            has_signature_help: true,
            function_help_lookup_key: Some("SUM".to_string()),
        };
        formula_space.effective_display_summary = Some("3".to_string());
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        formula_space.context.scenario_label = "happy-path sum".to_string();
        formula_space.context.host_profile = "Windows desktop preview".to_string();
        formula_space.context.packet_kind = "preview edit packet".to_string();
        formula_space.context.capability_floor = "Explore + Inspect".to_string();
        formula_space.context.mode_availability = "Explore / Inspect / Workbench".to_string();
        formula_space.context.trace_summary = Some("Preview trace reused bound=false".to_string());
        formula_space.editor_document = Some(EditorDocument {
            source_text: "=SUM(1,2)".to_string(),
            text_change_range: None,
            editor_syntax_snapshot: EditorSyntaxSnapshot {
                formula_stable_id: "formula-1".to_string(),
                green_tree_key: "green-1".to_string(),
                tokens: vec![
                    EditorToken {
                        text: "=".to_string(),
                        span_start: 0,
                        span_len: 1,
                    },
                    EditorToken {
                        text: "SUM".to_string(),
                        span_start: 1,
                        span_len: 3,
                    },
                ],
            },
            live_diagnostics: LiveDiagnosticSnapshot {
                diagnostics: vec![LiveDiagnostic {
                    diagnostic_id: "diag-1".to_string(),
                    message: "sample".to_string(),
                    span_start: 1,
                    span_len: 3,
                }],
            },
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: true,
                reused_red_projection: false,
                reused_bound_formula: false,
            },
            signature_help: Some(SignatureHelpContext {
                callee_text: "SUM".to_string(),
                call_span: FormulaTextSpan { start: 0, len: 9 },
                active_argument_index: 1,
            }),
            function_help: Some(FunctionHelpPacket {
                lookup_key: "SUM".to_string(),
                display_name: "SUM".to_string(),
                signature_forms: vec![FunctionHelpSignatureForm {
                    display_signature: "SUM(number1, number2, ...)".to_string(),
                    min_arity: 1,
                    max_arity: None,
                }],
                argument_help: vec![
                    "number1".to_string(),
                    "number2".to_string(),
                    "additional_numbers".to_string(),
                ],
                short_description: Some("Adds numbers together.".to_string()),
                availability_summary: Some("supported".to_string()),
                deferred_or_profile_limited: false,
            }),
            completion_proposals: vec![CompletionProposal {
                proposal_id: "proposal-1".to_string(),
                proposal_kind: CompletionProposalKind::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: Some(FormulaTextSpan { start: 1, len: 3 }),
                documentation_ref: Some("preview:function:SUM".to_string()),
                requires_revalidation: true,
            }],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
            value_presentation: None,
        });

        let view_model = build_explore_view_model(&formula_space);
        assert_eq!(view_model.scenario_label, "happy-path sum");
        assert_eq!(view_model.truth_source_label, "local-fallback");
        assert_eq!(view_model.host_profile_summary, "Windows desktop preview");
        assert_eq!(view_model.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(view_model.editor_surface_state.caret.offset, 9);
        assert_eq!(view_model.syntax_runs.len(), 2);
        assert_eq!(view_model.diagnostics.len(), 1);
        assert_eq!(view_model.completion_items.len(), 1);
        assert_eq!(
            view_model
                .completion_items
                .first()
                .and_then(|item| item.replacement_span),
            Some(FormulaTextSpan { start: 1, len: 3 })
        );
        assert_eq!(
            view_model.signature_help.as_ref().map(|help| help.active_argument_index),
            Some(1)
        );
        assert_eq!(
            view_model.signature_help.as_ref().map(|help| help.call_span),
            Some(FormulaTextSpan { start: 0, len: 9 })
        );
        assert_eq!(
            view_model.function_help.as_ref().map(|help| help.display_name.as_str()),
            Some("SUM")
        );
        assert_eq!(view_model.function_help_lookup_key.as_deref(), Some("SUM"));
        assert_eq!(view_model.result_value_summary.as_deref(), Some("Number"));
        assert_eq!(view_model.effective_display_summary.as_deref(), Some("3"));
        assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
        assert!(view_model.reused_green_tree);
    }

    #[test]
    fn explore_view_model_falls_back_to_local_tokenization_when_document_is_stale() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=LET(x,1,x)");
        formula_space.editor_document = Some(EditorDocument {
            source_text: "=SUM(1,2)".to_string(),
            text_change_range: None,
            editor_syntax_snapshot: EditorSyntaxSnapshot {
                formula_stable_id: "formula-1".to_string(),
                green_tree_key: "green-1".to_string(),
                tokens: vec![],
            },
            live_diagnostics: LiveDiagnosticSnapshot::default(),
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: true,
                reused_red_projection: false,
                reused_bound_formula: false,
            },
            signature_help: None,
            function_help: None,
            completion_proposals: vec![],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
            value_presentation: None,
        });

        let view_model = build_explore_view_model(&formula_space);
        assert_eq!(view_model.syntax_runs.len(), 9);
        assert!(view_model.diagnostics.is_empty());
        assert!(view_model.completion_items.is_empty());
        assert!(view_model.signature_help.is_none());
        assert!(view_model.function_help.is_none());
        assert!(view_model.green_tree_key.is_none());
    }
}
