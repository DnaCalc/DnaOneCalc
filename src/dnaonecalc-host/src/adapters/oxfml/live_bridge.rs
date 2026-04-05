use std::collections::BTreeMap;
use std::sync::Mutex;

use oxfunc_core::value::{ArrayCellValue, EvalValue, WorksheetErrorCode};
use oxfml_core::consumer::editor::{
    CompletionResult as UpstreamCompletionResult, EditorAnalysisStage as UpstreamEditorAnalysisStage,
    EditorDocument as UpstreamEditorDocument, EditorEditService, EditorEnvironment,
    EditorInteractionResult as UpstreamEditorInteractionResult, FormulaEditReuseSummary as UpstreamReuseSummary,
    FunctionHelpPacket as UpstreamFunctionHelpPacket, FunctionHelpSignatureForm as UpstreamFunctionHelpSignatureForm,
    LiveDiagnostic as UpstreamLiveDiagnostic, SignatureHelpContext as UpstreamSignatureHelpContext,
};
use oxfml_core::consumer::runtime::{RuntimeEnvironment, RuntimeFormulaRequest, RuntimeFormulaResult};
use oxfml_core::interface::{HostProviderOutcomeKind, TypedContextQueryBundle};
use oxfml_core::source::FormulaSourceRecord;
use oxfml_core::{BindContext, FormulaChannelKind};

use super::bridge::{
    EditorAnalysisStage, FormulaEditRequest, FormulaEditResult, OxfmlEditorBridge,
    OxfmlEditorBridgeError,
};
use super::types::{
    BindSummary, CompletionProposal, CompletionProposalKind, EditorDocument, EditorSyntaxSnapshot,
    EditorToken, EvalSummary, FormulaArrayPreview, FormulaEditReuseSummary, FormulaTextChangeRange,
    FormulaTextSpan, FormulaValuePresentation, FormulaWalkNode, FormulaWalkNodeState,
    FunctionHelpPacket, FunctionHelpSignatureForm, LiveDiagnostic, LiveDiagnosticSnapshot,
    ParseSummary, ProvenanceSummary, SignatureHelpContext,
};

#[derive(Debug, Default)]
pub struct LiveOxfmlBridge {
    cached_documents: Mutex<BTreeMap<String, UpstreamEditorDocument>>,
}

impl OxfmlEditorBridge for LiveOxfmlBridge {
    fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
        let source = FormulaSourceRecord::new(
            request.formula_stable_id.clone(),
            1,
            request.entered_text.clone(),
        )
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);

        let previous_document = self.previous_document(&request)?;
        let service = EditorEditService::new(EditorEnvironment::new(BindContext::default()));
        let interaction = service.apply_edit(
            source.clone(),
            previous_document.as_ref(),
            map_analysis_stage(request.analysis_stage),
            None,
        );
        let runtime_result = RuntimeEnvironment::new()
            .execute(RuntimeFormulaRequest::new(source, TypedContextQueryBundle::default()))
            .ok();

        let document = map_editor_document(&request.formula_stable_id, &interaction, runtime_result.as_ref());
        self.cache_document(request.formula_stable_id, interaction.document)?;

        Ok(FormulaEditResult { document })
    }
}

impl LiveOxfmlBridge {
    fn previous_document(
        &self,
        request: &FormulaEditRequest,
    ) -> Result<Option<UpstreamEditorDocument>, OxfmlEditorBridgeError> {
        let cached_documents = self
            .cached_documents
            .lock()
            .map_err(|_| OxfmlEditorBridgeError::UpstreamFailure("Live bridge cache poisoned".to_string()))?;
        let previous = cached_documents.get(&request.formula_stable_id).cloned();
        Ok(previous.filter(|document| {
            request.previous_green_tree_key.as_deref()
                == Some(document.editor_syntax_snapshot.green_tree_key.as_str())
        }))
    }

    fn cache_document(
        &self,
        formula_stable_id: String,
        document: UpstreamEditorDocument,
    ) -> Result<(), OxfmlEditorBridgeError> {
        let mut cached_documents = self
            .cached_documents
            .lock()
            .map_err(|_| OxfmlEditorBridgeError::UpstreamFailure("Live bridge cache poisoned".to_string()))?;
        cached_documents.insert(formula_stable_id, document);
        Ok(())
    }
}

fn map_analysis_stage(stage: EditorAnalysisStage) -> UpstreamEditorAnalysisStage {
    match stage {
        EditorAnalysisStage::SyntaxOnly => UpstreamEditorAnalysisStage::SyntaxOnly,
        EditorAnalysisStage::SyntaxAndBind => UpstreamEditorAnalysisStage::SyntaxAndBind,
        EditorAnalysisStage::FullSemanticPlan => UpstreamEditorAnalysisStage::FullSemanticPlan,
    }
}

fn map_editor_document(
    formula_stable_id: &str,
    interaction: &UpstreamEditorInteractionResult,
    runtime_result: Option<&RuntimeFormulaResult>,
) -> EditorDocument {
    let document = &interaction.document;
    let syntax_snapshot = &document.editor_syntax_snapshot;
    let completion_proposals =
        map_completion_proposals(interaction.completion_result.as_ref(), syntax_snapshot.formula_stable_id.as_str());
    let parse_status = if document.live_diagnostics.diagnostics.is_empty() {
        "Valid".to_string()
    } else {
        "Diagnostics".to_string()
    };
    let blocked_reason = runtime_result.and_then(blocked_reason_from_runtime);

    EditorDocument {
        source_text: document.source.entered_formula_text.clone(),
        text_change_range: document.text_change_range.map(|range| FormulaTextChangeRange {
            start: range.start,
            old_len: range.old_len,
            new_len: range.new_len,
        }),
        editor_syntax_snapshot: EditorSyntaxSnapshot {
            formula_stable_id: formula_stable_id.to_string(),
            green_tree_key: syntax_snapshot.green_tree_key.clone(),
            tokens: syntax_snapshot
                .tokens
                .iter()
                .map(|token| EditorToken {
                    text: token.text.clone(),
                    span_start: token.span.start,
                    span_len: token.span.len,
                })
                .collect(),
        },
        live_diagnostics: LiveDiagnosticSnapshot {
            diagnostics: document
                .live_diagnostics
                .diagnostics
                .iter()
                .map(map_live_diagnostic)
                .collect(),
        },
        reuse_summary: map_reuse_summary(&document.reuse_summary),
        signature_help: interaction
            .signature_help_context
            .as_ref()
            .map(map_signature_help_context),
        function_help: interaction
            .function_help_packet
            .as_ref()
            .map(map_function_help_packet),
        completion_proposals,
        formula_walk: runtime_result
            .map(map_formula_walk)
            .unwrap_or_else(|| vec![FormulaWalkNode {
                node_id: "node:source".to_string(),
                label: "CellEntry".to_string(),
                value_preview: Some(document.source.entered_formula_text.clone()),
                state: if blocked_reason.is_some() {
                    FormulaWalkNodeState::Blocked
                } else if document.bound_formula.is_some() {
                    FormulaWalkNodeState::Evaluated
                } else {
                    FormulaWalkNodeState::Opaque
                },
                children: Vec::new(),
            }]),
        parse_summary: Some(ParseSummary {
            status: parse_status,
            token_count: syntax_snapshot.tokens.len(),
        }),
        bind_summary: Some(BindSummary {
            variable_count: usize::from(document.bound_formula.is_some()),
            reference_count: runtime_result
                .map(|result| {
                    result
                        .evaluation
                        .trace
                        .prepared_calls
                        .iter()
                        .flat_map(|call| call.prepared_arguments.iter())
                        .filter(|argument| argument.reference_target.is_some())
                        .count()
                })
                .unwrap_or(0),
        }),
        eval_summary: Some(EvalSummary {
            step_count: runtime_result
                .map(|result| result.evaluation.trace.prepared_calls.len())
                .unwrap_or_else(|| usize::from(document.semantic_plan.is_some())),
            duration_text: runtime_result
                .map(|result| format!("{} prepared call(s)", result.evaluation.trace.prepared_calls.len()))
                .unwrap_or_else(|| "edit-only".to_string()),
        }),
        provenance_summary: Some(ProvenanceSummary {
            profile_summary: runtime_result
                .map(|result| format!("OxFml runtime · {:?}", result.returned_value_surface.kind))
                .unwrap_or_else(|| "OxFml editor".to_string()),
            blocked_reason,
        }),
        value_presentation: runtime_result.map(map_value_presentation),
    }
}

fn map_live_diagnostic(diagnostic: &UpstreamLiveDiagnostic) -> LiveDiagnostic {
    LiveDiagnostic {
        diagnostic_id: diagnostic.diagnostic_id.clone(),
        message: diagnostic.message.clone(),
        span_start: diagnostic.primary_span.start,
        span_len: diagnostic.primary_span.len,
    }
}

fn map_reuse_summary(summary: &UpstreamReuseSummary) -> FormulaEditReuseSummary {
    FormulaEditReuseSummary {
        reused_green_tree: summary.reused_green_tree,
        reused_red_projection: summary.reused_red_projection,
        reused_bound_formula: summary.reused_bound_formula,
    }
}

fn map_completion_proposals(
    completion_result: Option<&UpstreamCompletionResult>,
    formula_stable_id: &str,
) -> Vec<CompletionProposal> {
    completion_result
        .map(|result| {
            result
                .proposals
                .iter()
                .map(|proposal| CompletionProposal {
                    proposal_id: proposal.proposal_id.clone(),
                    proposal_kind: map_completion_proposal_kind(proposal.proposal_kind),
                    display_text: proposal.display_text.clone(),
                    insert_text: proposal.insert_text.clone(),
                    replacement_span: proposal
                        .replacement_span
                        .or(result.replacement_span)
                        .map(|span| FormulaTextSpan {
                            start: span.start,
                            len: span.len,
                        }),
                    documentation_ref: proposal
                        .documentation_ref
                        .clone()
                        .or_else(|| Some(format!("oxfml:function:{formula_stable_id}:{}", proposal.display_text))),
                    requires_revalidation: proposal.requires_revalidation,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn map_completion_proposal_kind(
    kind: oxfml_core::consumer::editor::CompletionProposalKind,
) -> CompletionProposalKind {
    match kind {
        oxfml_core::consumer::editor::CompletionProposalKind::Function => CompletionProposalKind::Function,
        oxfml_core::consumer::editor::CompletionProposalKind::DefinedName => CompletionProposalKind::DefinedName,
        oxfml_core::consumer::editor::CompletionProposalKind::TableName => CompletionProposalKind::TableName,
        oxfml_core::consumer::editor::CompletionProposalKind::TableColumn => CompletionProposalKind::TableColumn,
        oxfml_core::consumer::editor::CompletionProposalKind::StructuredSelector => {
            CompletionProposalKind::StructuredSelector
        }
        oxfml_core::consumer::editor::CompletionProposalKind::SyntaxAssist => {
            CompletionProposalKind::SyntaxAssist
        }
    }
}

fn map_signature_help_context(context: &UpstreamSignatureHelpContext) -> SignatureHelpContext {
    SignatureHelpContext {
        callee_text: context.callee_text.clone(),
        call_span: FormulaTextSpan {
            start: context.call_span.start,
            len: context.call_span.len,
        },
        active_argument_index: context.active_argument_index,
    }
}

fn map_function_help_packet(packet: &UpstreamFunctionHelpPacket) -> FunctionHelpPacket {
    FunctionHelpPacket {
        lookup_key: packet.lookup_key.clone(),
        display_name: packet.display_name.clone(),
        signature_forms: packet
            .signature_forms
            .iter()
            .map(map_function_help_signature_form)
            .collect(),
        argument_help: packet.argument_help.clone(),
        short_description: packet.short_description.clone(),
        availability_summary: packet.availability_summary.clone(),
        deferred_or_profile_limited: packet.deferred_or_profile_limited,
    }
}

fn map_function_help_signature_form(
    form: &UpstreamFunctionHelpSignatureForm,
) -> FunctionHelpSignatureForm {
    FunctionHelpSignatureForm {
        display_signature: form.display_signature.clone(),
        min_arity: form.min_arity,
        max_arity: form.max_arity,
    }
}

fn map_formula_walk(result: &RuntimeFormulaResult) -> Vec<FormulaWalkNode> {
    if result.evaluation.trace.prepared_calls.is_empty() {
        return vec![FormulaWalkNode {
            node_id: "node:formula".to_string(),
            label: "Formula".to_string(),
            value_preview: Some(result.evaluation.result.payload_summary.clone()),
            state: FormulaWalkNodeState::Evaluated,
            children: Vec::new(),
        }];
    }

    result
        .evaluation
        .trace
        .prepared_calls
        .iter()
        .enumerate()
        .map(|(call_index, call)| FormulaWalkNode {
            node_id: format!("node:call:{call_index}"),
            label: call.function_name.clone(),
            value_preview: Some(result.evaluation.result.payload_summary.clone()),
            state: if result
                .returned_value_surface
                .host_provider_outcome
                .as_ref()
                .is_some_and(|outcome| outcome.outcome_kind == HostProviderOutcomeKind::CapabilityDenied)
            {
                FormulaWalkNodeState::Blocked
            } else {
                FormulaWalkNodeState::Evaluated
            },
            children: call
                .prepared_arguments
                .iter()
                .enumerate()
                .map(|(arg_index, argument)| FormulaWalkNode {
                    node_id: format!("node:call:{call_index}:arg:{arg_index}"),
                    label: format!("Arg {} · {:?}", arg_index + 1, argument.source_class),
                    value_preview: argument
                        .reference_target
                        .clone()
                        .or_else(|| argument.opaque_reason.clone()),
                    state: if argument.opaque_reason.is_some() {
                        FormulaWalkNodeState::Opaque
                    } else if argument.reference_target.is_some() {
                        FormulaWalkNodeState::Bound
                    } else {
                        FormulaWalkNodeState::Evaluated
                    },
                    children: Vec::new(),
                })
                .collect(),
        })
        .collect()
}

fn map_value_presentation(result: &RuntimeFormulaResult) -> FormulaValuePresentation {
    let blocked_reason = blocked_reason_from_runtime(result);
    let array_preview = match &result.published_worksheet_value {
        EvalValue::Array(array) => {
            let shape = array.shape();
            let max_rows = shape.rows.min(4);
            let max_cols = shape.cols.min(4);
            let mut rows = Vec::with_capacity(max_rows);
            for row in 0..max_rows {
                let cells = array
                    .row_slice(row)
                    .unwrap_or(&[])
                    .iter()
                    .take(max_cols)
                    .map(format_array_cell_value)
                    .collect::<Vec<_>>();
                rows.push(cells);
            }

            Some(FormulaArrayPreview {
                label: format!("{}x{} spill preview", shape.rows, shape.cols),
                rows,
                truncated: shape.rows > max_rows || shape.cols > max_cols,
            })
        }
        _ => None,
    };

    FormulaValuePresentation {
        evaluation_summary: format_eval_summary(&result.evaluation.result.payload_summary),
        effective_display_summary: Some(format_eval_value_for_display(
            &result.published_worksheet_value,
            array_preview.as_ref(),
        )),
        array_preview,
        blocked_reason,
    }
}

fn blocked_reason_from_runtime(result: &RuntimeFormulaResult) -> Option<String> {
    result
        .returned_value_surface
        .host_provider_outcome
        .as_ref()
        .and_then(|outcome| match outcome.outcome_kind {
            HostProviderOutcomeKind::CapabilityDenied => Some(
                outcome
                    .detail
                    .clone()
                    .unwrap_or_else(|| "Host capability denied".to_string()),
            ),
            _ => None,
        })
}

fn format_eval_summary(payload_summary: &str) -> String {
    if let Some(inner) = payload_summary
        .strip_prefix("Number(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Number · {inner}");
    }
    if let Some(inner) = payload_summary
        .strip_prefix("Text(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Text · {inner}");
    }
    if let Some(inner) = payload_summary
        .strip_prefix("Logical(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Logical · {inner}");
    }
    if let Some(inner) = payload_summary
        .strip_prefix("Array(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Array · {inner} dynamic result");
    }
    if let Some(inner) = payload_summary
        .strip_prefix("Error(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Error · {}", worksheet_error_literal_from_name(inner));
    }
    payload_summary.to_string()
}

fn format_eval_value_for_display(
    value: &EvalValue,
    array_preview: Option<&FormulaArrayPreview>,
) -> String {
    match value {
        EvalValue::Number(number) => format_number(*number),
        EvalValue::Text(text) => text.to_string_lossy(),
        EvalValue::Logical(value) => {
            if *value {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        EvalValue::Error(code) => worksheet_error_literal(*code).to_string(),
        EvalValue::Array(_) => array_preview
            .map(|preview| format!("{{{}}}", preview.rows.iter().map(|row| row.join(",")).collect::<Vec<_>>().join(";")))
            .unwrap_or_else(|| "Array result".to_string()),
        EvalValue::Reference(reference) => reference.target.clone(),
        EvalValue::Lambda(lambda) => format!("Lambda({})", lambda.callable_token),
    }
}

fn format_array_cell_value(cell: &ArrayCellValue) -> String {
    match cell {
        ArrayCellValue::Number(number) => format_number(*number),
        ArrayCellValue::Text(text) => text.to_string_lossy(),
        ArrayCellValue::Logical(value) => {
            if *value {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        ArrayCellValue::Error(code) => worksheet_error_literal(*code).to_string(),
        ArrayCellValue::EmptyCell => String::new(),
    }
}

fn worksheet_error_literal(code: WorksheetErrorCode) -> &'static str {
    match code {
        WorksheetErrorCode::Null => "#NULL!",
        WorksheetErrorCode::Div0 => "#DIV/0!",
        WorksheetErrorCode::Value => "#VALUE!",
        WorksheetErrorCode::Ref => "#REF!",
        WorksheetErrorCode::Name => "#NAME?",
        WorksheetErrorCode::Num => "#NUM!",
        WorksheetErrorCode::NA => "#N/A",
        WorksheetErrorCode::Busy => "#BUSY!",
        WorksheetErrorCode::GettingData => "#GETTING_DATA",
        WorksheetErrorCode::Spill => "#SPILL!",
        WorksheetErrorCode::Calc => "#CALC!",
        WorksheetErrorCode::Field => "#FIELD!",
        WorksheetErrorCode::Blocked => "#BLOCKED!",
        WorksheetErrorCode::Connect => "#CONNECT!",
    }
}

fn worksheet_error_literal_from_name(name: &str) -> &'static str {
    match name {
        "Null" => "#NULL!",
        "Div0" => "#DIV/0!",
        "Value" => "#VALUE!",
        "Ref" => "#REF!",
        "Name" => "#NAME?",
        "Num" => "#NUM!",
        "NA" => "#N/A",
        "Busy" => "#BUSY!",
        "GettingData" => "#GETTING_DATA",
        "Spill" => "#SPILL!",
        "Calc" => "#CALC!",
        "Field" => "#FIELD!",
        "Blocked" => "#BLOCKED!",
        "Connect" => "#CONNECT!",
        _ => "#VALUE!",
    }
}

fn format_number(number: f64) -> String {
    if (number.fract()).abs() < f64::EPSILON {
        format!("{}", number as i64)
    } else {
        number.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_bridge_maps_local_analysis_stage_to_upstream_stage() {
        let stage = map_analysis_stage(EditorAnalysisStage::SyntaxAndBind);
        assert_eq!(stage, UpstreamEditorAnalysisStage::SyntaxAndBind);
    }

    #[test]
    fn live_bridge_evaluates_sum_formula_through_oxfml_runtime() {
        let bridge = LiveOxfmlBridge::default();
        let result = bridge
            .apply_formula_edit(FormulaEditRequest {
                formula_stable_id: "formula-live-sum".to_string(),
                entered_text: "=SUM(1,2,3)".to_string(),
                cursor_offset: 11,
                previous_green_tree_key: None,
                analysis_stage: EditorAnalysisStage::FullSemanticPlan,
            })
            .expect("live bridge should evaluate SUM");

        assert_eq!(
            result
                .document
                .value_presentation
                .as_ref()
                .map(|presentation| presentation.evaluation_summary.as_str()),
            Some("Number · 6")
        );
        assert_eq!(
            result
                .document
                .value_presentation
                .as_ref()
                .and_then(|presentation| presentation.effective_display_summary.as_deref()),
            Some("6")
        );
    }

    #[test]
    fn live_bridge_evaluates_direct_numeric_entry() {
        let bridge = LiveOxfmlBridge::default();
        let result = bridge
            .apply_formula_edit(FormulaEditRequest {
                formula_stable_id: "formula-live-number".to_string(),
                entered_text: "123.4".to_string(),
                cursor_offset: 5,
                previous_green_tree_key: None,
                analysis_stage: EditorAnalysisStage::FullSemanticPlan,
            })
            .expect("live bridge should evaluate direct numeric entry");

        assert_eq!(
            result
                .document
                .value_presentation
                .as_ref()
                .map(|presentation| presentation.evaluation_summary.as_str()),
            Some("Number · 123.4")
        );
        assert_eq!(
            result
                .document
                .value_presentation
                .as_ref()
                .and_then(|presentation| presentation.effective_display_summary.as_deref()),
            Some("123.4")
        );
    }
}
