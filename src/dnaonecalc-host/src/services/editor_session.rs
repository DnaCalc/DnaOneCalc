use crate::adapters::oxfml::{
    EditorDocument, FormulaEditRequest, FormulaValuePresentation, OxfmlEditorBridge,
    OxfmlEditorBridgeError,
};
use crate::app::intents::ApplyFormulaEditIntent;
use crate::domain::ids::FormulaSpaceId;
use crate::state::{
    CompletionHelpState, FormulaArrayPreviewState, FormulaSpaceCollectionState, FormulaSpaceState,
    ProjectionTruthSource,
};
use crate::ui::editor::state::EditorSurfaceState;

#[derive(Debug, Default)]
pub struct EditorSessionService;

impl EditorSessionService {
    pub fn handle_formula_edit_intent(
        bridge: &dyn OxfmlEditorBridge,
        formula_spaces: &mut FormulaSpaceCollectionState,
        intent: ApplyFormulaEditIntent,
    ) -> Result<(), EditorSessionError> {
        let formula_space = formula_spaces
            .get(&intent.formula_space_id)
            .ok_or_else(|| {
                EditorSessionError::UnknownFormulaSpace(intent.formula_space_id.clone())
            })?;
        let request = FormulaEditRequest {
            formula_stable_id: intent.formula_stable_id,
            entered_text: intent.entered_text,
            cursor_offset: intent.cursor_offset,
            previous_green_tree_key: formula_space
                .editor_document
                .as_ref()
                .map(|document| document.green_tree_key().to_string()),
            analysis_stage: intent.analysis_stage,
        };
        let result = bridge
            .apply_formula_edit(request)
            .map_err(EditorSessionError::Bridge)?;
        Self::apply_editor_document(formula_spaces, &intent.formula_space_id, result.document)
    }

    pub fn apply_editor_document(
        formula_spaces: &mut FormulaSpaceCollectionState,
        formula_space_id: &FormulaSpaceId,
        document: EditorDocument,
    ) -> Result<(), EditorSessionError> {
        let formula_space = formula_spaces
            .get_mut(formula_space_id)
            .ok_or_else(|| EditorSessionError::UnknownFormulaSpace(formula_space_id.clone()))?;
        update_formula_space_from_editor_document(formula_space, document);
        Ok(())
    }
}

fn update_formula_space_from_editor_document(
    formula_space: &mut FormulaSpaceState,
    document: EditorDocument,
) {
    let truth_source = infer_truth_source(&document);
    let mut editor_surface_state = EditorSurfaceState::for_text_with_selection(
        &document.source_text,
        formula_space.editor_surface_state.selection.anchor,
        formula_space.editor_surface_state.selection.focus,
    );
    editor_surface_state.scroll_window = formula_space.editor_surface_state.scroll_window.clone();
    editor_surface_state.completion_anchor_offset = None;
    editor_surface_state.completion_selected_index =
        (!document.completion_proposals.is_empty()).then_some(0);
    editor_surface_state.signature_help_anchor_offset = None;

    formula_space.raw_entered_cell_text = document.source_text.clone();
    formula_space.editor_surface_state = editor_surface_state;
    formula_space.completion_help = CompletionHelpState {
        completion_count: document.completion_proposals.len(),
        has_signature_help: document.signature_help.is_some(),
        function_help_lookup_key: document
            .function_help
            .as_ref()
            .map(|packet| packet.lookup_key.clone()),
    };
    let derived_presentation = derive_formula_presentation(&document.source_text, &document);
    formula_space.editor_document = Some(document);
    formula_space.latest_evaluation_summary = derived_presentation.evaluation_summary;
    formula_space.effective_display_summary = derived_presentation.effective_display_summary;
    formula_space.array_preview = derived_presentation.array_preview;
    formula_space.context.truth_source = truth_source;
    if let Some(blocked_reason) = derived_presentation.blocked_reason {
        formula_space.context.blocked_reason = Some(blocked_reason);
    }
}

fn infer_truth_source(document: &EditorDocument) -> ProjectionTruthSource {
    if let Some(provenance_summary) = document.provenance_summary.as_ref() {
        if provenance_summary.profile_summary.contains("PreviewBridge") {
            return ProjectionTruthSource::PreviewBacked;
        }
        if provenance_summary.profile_summary.contains("OxFml") {
            return ProjectionTruthSource::LiveBacked;
        }
    }

    if document.value_presentation.is_some() {
        return ProjectionTruthSource::LiveBacked;
    }

    ProjectionTruthSource::LocalFallback
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DerivedFormulaPresentation {
    evaluation_summary: Option<String>,
    effective_display_summary: Option<String>,
    array_preview: Option<FormulaArrayPreviewState>,
    blocked_reason: Option<String>,
}

fn derive_formula_presentation(
    source_text: &str,
    document: &EditorDocument,
) -> DerivedFormulaPresentation {
    if let Some(value_presentation) = document.value_presentation.as_ref() {
        return derived_presentation_from_value_presentation(value_presentation);
    }

    if let Some(blocked_reason) = document
        .provenance_summary
        .as_ref()
        .and_then(|summary| summary.blocked_reason.clone())
    {
        return DerivedFormulaPresentation {
            evaluation_summary: Some(format!("Blocked · {blocked_reason}")),
            effective_display_summary: Some("Blocked on host lane".to_string()),
            array_preview: None,
            blocked_reason: Some(blocked_reason),
        };
    }

    if let Some(diagnostic) = document.live_diagnostics.diagnostics.first() {
        return DerivedFormulaPresentation {
            evaluation_summary: Some(format!("Diagnostic · {}", diagnostic.message)),
            effective_display_summary: Some("Input incomplete".to_string()),
            array_preview: None,
            blocked_reason: None,
        };
    }

    if let Some(forced_text) = source_text.strip_prefix('\'') {
        return DerivedFormulaPresentation {
            evaluation_summary: Some(format!("Text · {forced_text}")),
            effective_display_summary: Some(forced_text.to_string()),
            array_preview: None,
            blocked_reason: None,
        };
    }

    if !source_text.starts_with('=') {
        if let Ok(number) = source_text.parse::<f64>() {
            return DerivedFormulaPresentation {
                evaluation_summary: Some(format!("Number · {}", format_number(number))),
                effective_display_summary: Some(source_text.to_string()),
                array_preview: None,
                blocked_reason: None,
            };
        }

        if !source_text.is_empty() {
            return DerivedFormulaPresentation {
                evaluation_summary: Some(format!("Text · {source_text}")),
                effective_display_summary: Some(source_text.to_string()),
                array_preview: None,
                blocked_reason: None,
            };
        }
    }

    if let Some(sum_value) = parse_sum_formula(source_text) {
        let display = format_number(sum_value);
        return DerivedFormulaPresentation {
            evaluation_summary: Some(format!("Number · {display}")),
            effective_display_summary: Some(display),
            array_preview: None,
            blocked_reason: None,
        };
    }

    if let Some((rows, cols, preview_rows)) = parse_sequence_formula(source_text) {
        let effective_display_summary = format_array_display(&preview_rows);
        return DerivedFormulaPresentation {
            evaluation_summary: Some(format!("Array · {rows}x{cols} dynamic result")),
            effective_display_summary: Some(effective_display_summary),
            array_preview: Some(FormulaArrayPreviewState {
                label: format!("{rows}x{cols} spill preview"),
                rows: preview_rows,
                truncated: false,
            }),
            blocked_reason: None,
        };
    }

    if source_text.eq_ignore_ascii_case("=LET(x,1,x)") {
        return DerivedFormulaPresentation {
            evaluation_summary: Some("Number · 1".to_string()),
            effective_display_summary: Some("1".to_string()),
            array_preview: None,
            blocked_reason: None,
        };
    }

    DerivedFormulaPresentation {
        evaluation_summary: None,
        effective_display_summary: None,
        array_preview: None,
        blocked_reason: None,
    }
}

fn derived_presentation_from_value_presentation(
    value_presentation: &FormulaValuePresentation,
) -> DerivedFormulaPresentation {
    DerivedFormulaPresentation {
        evaluation_summary: Some(value_presentation.evaluation_summary.clone()),
        effective_display_summary: value_presentation.effective_display_summary.clone(),
        array_preview: value_presentation.array_preview.as_ref().map(|preview| {
            FormulaArrayPreviewState {
                label: preview.label.clone(),
                rows: preview.rows.clone(),
                truncated: preview.truncated,
            }
        }),
        blocked_reason: value_presentation.blocked_reason.clone(),
    }
}

fn parse_sum_formula(source_text: &str) -> Option<f64> {
    let inner = source_text
        .strip_prefix("=SUM(")
        .and_then(|text| text.strip_suffix(')'))?;
    let mut total = 0.0f64;
    let mut count = 0usize;
    for part in inner.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return None;
        }
        total += trimmed.parse::<f64>().ok()?;
        count += 1;
    }
    (count > 0).then_some(total)
}

fn parse_sequence_formula(source_text: &str) -> Option<(usize, usize, Vec<Vec<String>>)> {
    let inner = source_text
        .strip_prefix("=SEQUENCE(")
        .and_then(|text| text.strip_suffix(')'))?;
    let mut parts = inner.split(',').map(|part| part.trim());
    let rows = parts.next()?.parse::<usize>().ok()?;
    let cols = parts
        .next()
        .map(|value| value.parse::<usize>().ok())
        .flatten()
        .unwrap_or(1);
    if rows == 0 || cols == 0 {
        return None;
    }

    let mut next_value = 1usize;
    let mut preview_rows = Vec::with_capacity(rows);
    for _ in 0..rows {
        let mut row = Vec::with_capacity(cols);
        for _ in 0..cols {
            row.push(next_value.to_string());
            next_value += 1;
        }
        preview_rows.push(row);
    }

    Some((rows, cols, preview_rows))
}

fn format_array_display(rows: &[Vec<String>]) -> String {
    let row_strings = rows.iter().map(|row| row.join(",")).collect::<Vec<_>>();
    format!("{{{}}}", row_strings.join(";"))
}

fn format_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSessionError {
    UnknownFormulaSpace(FormulaSpaceId),
    Bridge(OxfmlEditorBridgeError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{
        CompletionProposal, CompletionProposalKind, EditorAnalysisStage, EditorSyntaxSnapshot,
        FormulaEditResult, FormulaEditReuseSummary, FormulaTextSpan, LiveDiagnosticSnapshot,
        ProvenanceSummary, SignatureHelpContext,
    };

    fn sample_document(source_text: &str) -> EditorDocument {
        EditorDocument {
            source_text: source_text.to_string(),
            text_change_range: None,
            editor_syntax_snapshot: EditorSyntaxSnapshot {
                formula_stable_id: "formula-1".to_string(),
                green_tree_key: "green-1".to_string(),
                tokens: vec![],
            },
            live_diagnostics: LiveDiagnosticSnapshot::default(),
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: true,
                reused_red_projection: true,
                reused_bound_formula: false,
            },
            signature_help: Some(SignatureHelpContext {
                callee_text: "SUM".to_string(),
                call_span: FormulaTextSpan {
                    start: 0,
                    len: source_text.chars().count(),
                },
                active_argument_index: 1,
            }),
            function_help: None,
            completion_proposals: vec![CompletionProposal {
                proposal_id: "proposal-1".to_string(),
                proposal_kind: CompletionProposalKind::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: None,
                documentation_ref: None,
                requires_revalidation: true,
            }],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
            value_presentation: None,
        }
    }

    #[test]
    fn apply_editor_document_updates_formula_space_text_and_help() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1+1"));

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            sample_document("'123.4"),
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(updated.raw_entered_cell_text, "'123.4");
        assert_eq!(updated.completion_help.completion_count, 1);
        assert!(updated.completion_help.has_signature_help);
        assert_eq!(updated.editor_surface_state.completion_anchor_offset, None);
        assert_eq!(
            updated.editor_surface_state.completion_selected_index,
            Some(0)
        );
        assert_eq!(
            updated.editor_surface_state.signature_help_anchor_offset,
            None
        );
        assert_eq!(
            updated.latest_evaluation_summary.as_deref(),
            Some("Text · 123.4")
        );
        assert_eq!(updated.effective_display_summary.as_deref(), Some("123.4"));
        assert_eq!(
            updated.context.truth_source,
            ProjectionTruthSource::LocalFallback
        );
        assert_eq!(
            updated
                .editor_document
                .as_ref()
                .expect("editor document retained")
                .green_tree_key(),
            "green-1"
        );
    }

    #[test]
    fn apply_editor_document_derives_preview_sum_result() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2,3)",
        ));

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            sample_document("=SUM(1,2,3)"),
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(
            updated.latest_evaluation_summary.as_deref(),
            Some("Number · 6")
        );
        assert_eq!(updated.effective_display_summary.as_deref(), Some("6"));
        assert!(updated.array_preview.is_none());
    }

    #[test]
    fn apply_editor_document_derives_sequence_array_preview() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SEQUENCE(2,2)",
        ));

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            sample_document("=SEQUENCE(2,2)"),
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(
            updated.latest_evaluation_summary.as_deref(),
            Some("Array · 2x2 dynamic result")
        );
        assert_eq!(
            updated.effective_display_summary.as_deref(),
            Some("{1,2;3,4}")
        );
        assert_eq!(
            updated
                .array_preview
                .as_ref()
                .map(|preview| preview.rows.len()),
            Some(2)
        );
    }

    struct FakeBridge {
        document: EditorDocument,
    }

    impl OxfmlEditorBridge for FakeBridge {
        fn apply_formula_edit(
            &self,
            request: FormulaEditRequest,
        ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
            assert_eq!(request.formula_stable_id, "formula-1");
            assert_eq!(request.entered_text, "=SUM(1,2,3)");
            assert_eq!(request.cursor_offset, 4);
            assert_eq!(request.analysis_stage, EditorAnalysisStage::SyntaxAndBind);
            assert!(request.previous_green_tree_key.is_none());
            Ok(FormulaEditResult {
                document: self.document.clone(),
            })
        }
    }

    #[test]
    fn handle_formula_edit_intent_routes_through_bridge_and_updates_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1+1"));
        let bridge = FakeBridge {
            document: sample_document("=SUM(1,2,3)"),
        };

        EditorSessionService::handle_formula_edit_intent(
            &bridge,
            &mut formula_spaces,
            ApplyFormulaEditIntent {
                formula_space_id: formula_space_id.clone(),
                formula_stable_id: "formula-1".to_string(),
                entered_text: "=SUM(1,2,3)".to_string(),
                cursor_offset: 4,
                analysis_stage: EditorAnalysisStage::SyntaxAndBind,
            },
        )
        .expect("edit intent should update via bridge");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(updated.raw_entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(
            updated.context.truth_source,
            ProjectionTruthSource::LocalFallback
        );
    }

    #[test]
    fn apply_editor_document_marks_live_oxfml_documents_as_live_backed() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1+1"));
        let mut document = sample_document("=SUM(1,2,3)");
        document.provenance_summary = Some(ProvenanceSummary {
            profile_summary: "OxFml runtime · Number".to_string(),
            blocked_reason: None,
        });

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            document,
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(
            updated.context.truth_source,
            ProjectionTruthSource::LiveBacked
        );
    }

    #[test]
    fn apply_editor_document_marks_preview_bridge_documents_as_preview_backed() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1+1"));
        let mut document = sample_document("=SUM(1,2,3)");
        document.provenance_summary = Some(ProvenanceSummary {
            profile_summary: "PreviewBridge".to_string(),
            blocked_reason: None,
        });

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            document,
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(
            updated.context.truth_source,
            ProjectionTruthSource::PreviewBacked
        );
    }
}
