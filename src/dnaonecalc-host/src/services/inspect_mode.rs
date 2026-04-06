use crate::adapters::oxfml::{
    BindSummary, EvalSummary, FormulaWalkNode, FormulaWalkNodeState, ParseSummary,
    ProvenanceSummary,
};
use crate::state::{FormulaSpaceState, RetainedArtifactRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectViewModel {
    pub scenario_label: String,
    pub truth_source_label: String,
    pub host_profile_summary: String,
    pub packet_kind_summary: String,
    pub capability_floor_summary: String,
    pub mode_availability_summary: String,
    pub trace_summary: Option<String>,
    pub blocked_reason: Option<String>,
    pub raw_entered_cell_text: String,
    pub inspect_result_summary: Option<String>,
    pub green_tree_key: Option<String>,
    pub formula_walk_nodes: Vec<InspectFormulaWalkNodeView>,
    pub parse_summary: Option<ParseSummary>,
    pub bind_summary: Option<BindSummary>,
    pub eval_summary: Option<EvalSummary>,
    pub provenance_summary: Option<ProvenanceSummary>,
    pub retained_artifact_context: Option<InspectRetainedArtifactContextView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectFormulaWalkNodeView {
    pub node_id: String,
    pub label: String,
    pub value_preview: Option<String>,
    pub state: FormulaWalkNodeState,
    pub children: Vec<InspectFormulaWalkNodeView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectRetainedArtifactContextView {
    pub artifact_id: String,
    pub case_id: String,
    pub comparison_status: String,
    pub discrepancy_summary: Option<String>,
    pub bundle_report_path: Option<String>,
    pub xml_source_summary: Option<String>,
    pub display_comparison_summary: Option<String>,
    pub upstream_gap_summary: Vec<String>,
}

pub fn build_inspect_view_model(
    formula_space: &FormulaSpaceState,
    retained_artifact: Option<&RetainedArtifactRecord>,
) -> InspectViewModel {
    let (
        green_tree_key,
        formula_walk_nodes,
        parse_summary,
        bind_summary,
        eval_summary,
        provenance_summary,
    ) = match &formula_space.editor_document {
        Some(document) => (
            Some(document.green_tree_key().to_string()),
            document
                .formula_walk
                .iter()
                .map(project_formula_walk_node)
                .collect(),
            document.parse_summary.clone(),
            document.bind_summary.clone(),
            document.eval_summary.clone(),
            document.provenance_summary.clone(),
        ),
        None => (None, Vec::new(), None, None, None, None),
    };

    let retained_artifact_context =
        retained_artifact.map(|artifact| InspectRetainedArtifactContextView {
            artifact_id: artifact.artifact_id.clone(),
            case_id: artifact.case_id.clone(),
            comparison_status: match artifact.comparison_status {
                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Matched => {
                    "matched".to_string()
                }
                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Mismatched => {
                    "mismatched".to_string()
                }
                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Blocked => {
                    "blocked".to_string()
                }
            },
            discrepancy_summary: artifact.discrepancy_summary.clone(),
            bundle_report_path: artifact.bundle_report_path.clone(),
            xml_source_summary: artifact.xml_extraction.as_ref().map(|xml| {
                format!(
                    "{} @ {} | format {}",
                    xml.workbook_path,
                    xml.locator,
                    xml.number_format_code
                        .clone()
                        .unwrap_or_else(|| "<none>".to_string())
                )
            }),
            display_comparison_summary: match (
                artifact.oxfml_effective_display_summary.as_deref(),
                artifact.excel_observed_value_repr.as_deref(),
            ) {
                (Some(oxfml), Some(excel)) => Some(format!("OxFml {oxfml} vs Excel {excel}")),
                _ => None,
            },
            upstream_gap_summary: artifact
                .upstream_gap_report
                .as_ref()
                .map(|gap| {
                    let mut items = Vec::new();
                    if !gap.oxxlplay_missing_surfaces.is_empty() {
                        items.push(format!(
                            "OxXlPlay missing: {}",
                            gap.oxxlplay_missing_surfaces.join(", ")
                        ));
                    }
                    if !gap.oxreplay_missing_views.is_empty() {
                        items.push(format!(
                            "OxReplay missing: {}",
                            gap.oxreplay_missing_views.join(", ")
                        ));
                    }
                    items
                })
                .unwrap_or_default(),
        });

    InspectViewModel {
        scenario_label: formula_space.context.scenario_label.clone(),
        truth_source_label: formula_space.context.truth_source.label().to_string(),
        host_profile_summary: formula_space.context.host_profile.clone(),
        packet_kind_summary: formula_space.context.packet_kind.clone(),
        capability_floor_summary: formula_space.context.capability_floor.clone(),
        mode_availability_summary: formula_space.context.mode_availability.clone(),
        trace_summary: formula_space.context.trace_summary.clone(),
        blocked_reason: formula_space.context.blocked_reason.clone().or_else(|| {
            provenance_summary
                .as_ref()
                .and_then(|summary| summary.blocked_reason.clone())
        }),
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        inspect_result_summary: formula_space.latest_evaluation_summary.clone(),
        green_tree_key,
        formula_walk_nodes,
        parse_summary,
        bind_summary,
        eval_summary,
        provenance_summary,
        retained_artifact_context,
    }
}

fn project_formula_walk_node(node: &FormulaWalkNode) -> InspectFormulaWalkNodeView {
    InspectFormulaWalkNodeView {
        node_id: node.node_id.clone(),
        label: node.label.clone(),
        value_preview: node.value_preview.clone(),
        state: node.state,
        children: node
            .children
            .iter()
            .map(project_formula_walk_node)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{
        EditorDocument, EditorSyntaxSnapshot, FormulaEditReuseSummary, FormulaWalkNode,
        FormulaWalkNodeState,
    };
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::FormulaSpaceState;

    #[test]
    fn inspect_view_model_projects_walk_and_summary_state() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=LET(x,1,x)");
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        formula_space.editor_document = Some(EditorDocument {
            source_text: "=LET(x,1,x)".to_string(),
            text_change_range: None,
            editor_syntax_snapshot: EditorSyntaxSnapshot {
                formula_stable_id: "formula-1".to_string(),
                green_tree_key: "green-1".to_string(),
                tokens: vec![],
            },
            live_diagnostics: Default::default(),
            reuse_summary: FormulaEditReuseSummary::default(),
            signature_help: None,
            function_help: None,
            completion_proposals: vec![],
            formula_walk: vec![FormulaWalkNode {
                node_id: "walk-1".to_string(),
                label: "LET".to_string(),
                value_preview: Some("1".to_string()),
                state: FormulaWalkNodeState::Evaluated,
                children: vec![FormulaWalkNode {
                    node_id: "walk-1-1".to_string(),
                    label: "x".to_string(),
                    value_preview: Some("1".to_string()),
                    state: FormulaWalkNodeState::Bound,
                    children: vec![],
                }],
            }],
            parse_summary: Some(ParseSummary {
                status: "Valid".to_string(),
                token_count: 7,
            }),
            bind_summary: Some(BindSummary {
                variable_count: 1,
                reference_count: 0,
            }),
            eval_summary: Some(EvalSummary {
                step_count: 2,
                duration_text: "1.2ms".to_string(),
            }),
            provenance_summary: Some(ProvenanceSummary {
                profile_summary: "OC-H0".to_string(),
                blocked_reason: None,
            }),
            value_presentation: None,
        });

        let view_model = build_inspect_view_model(&formula_space, None);
        assert_eq!(view_model.truth_source_label, "local-fallback");
        assert_eq!(view_model.raw_entered_cell_text, "=LET(x,1,x)");
        assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
        assert_eq!(view_model.formula_walk_nodes.len(), 1);
        assert_eq!(view_model.formula_walk_nodes[0].children.len(), 1);
        assert_eq!(
            view_model.parse_summary.as_ref().map(|x| x.token_count),
            Some(7)
        );
        assert_eq!(
            view_model.bind_summary.as_ref().map(|x| x.variable_count),
            Some(1)
        );
        assert_eq!(
            view_model.eval_summary.as_ref().map(|x| x.step_count),
            Some(2)
        );
        assert_eq!(
            view_model
                .provenance_summary
                .as_ref()
                .map(|x| x.profile_summary.as_str()),
            Some("OC-H0")
        );
        assert!(view_model.retained_artifact_context.is_none());
    }

    #[test]
    fn inspect_view_model_projects_open_retained_artifact_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        let retained_artifact = crate::state::RetainedArtifactRecord {
            artifact_id: "artifact-1".to_string(),
            case_id: "case-1".to_string(),
            formula_space_id,
            comparison_status:
                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Blocked,
            open_mode_hint:
                crate::services::programmatic_testing::ProgrammaticOpenModeHint::Workbench,
            discrepancy_summary: Some("excel lane unavailable".to_string()),
            bundle_report_path: Some("target/onecalc-verification/example".to_string()),
            case_output_dir: Some("target/onecalc-verification/example/cases/case-1".to_string()),
            xml_extraction: Some(crate::services::spreadsheet_xml::SpreadsheetXmlCellExtraction {
                workbook_path: "C:/tmp/workbook.xml".to_string(),
                locator: "Input!A1".to_string(),
                worksheet_name: "Input".to_string(),
                workbook_format_profile_hint: "excel-spreadsheetml-2003-default".to_string(),
                formula_text: Some("=SUM(1,2)".to_string()),
                entered_cell_text: "=SUM(1,2)".to_string(),
                data_type: Some("Number".to_string()),
                style_id: Some("calc".to_string()),
                number_format_code: Some("$#,##0.00".to_string()),
                font_color: Some("#112233".to_string()),
                fill_color: Some("#445566".to_string()),
                conditional_formats: vec![],
                date1904: Some(false),
                observation_scope: crate::services::spreadsheet_xml::VerificationObservationScope {
                    oxfml_required_scope: vec!["format_profile".to_string()],
                    oxxlplay_required_surfaces: vec!["effective_display_text".to_string()],
                    oxreplay_required_views: vec!["formatting_view".to_string()],
                },
            }),
            upstream_gap_report: Some(
                crate::services::verification_bundle::VerificationObservationGapReport {
                    oxfml_scope_required: vec!["format_profile".to_string()],
                    oxxlplay_supported_surfaces: vec!["cell_value".to_string()],
                    oxxlplay_missing_surfaces: vec!["effective_display_text".to_string()],
                    oxreplay_required_views: vec!["formatting_view".to_string()],
                    oxreplay_current_bundle_views: vec!["visible_value".to_string()],
                    oxreplay_missing_views: vec!["formatting_view".to_string()],
                },
            ),
            visible_output_match: Some(false),
            replay_equivalent: Some(false),
            oxfml_effective_display_summary: Some("6".to_string()),
            excel_observed_value_repr: Some("$6.00".to_string()),
        };

        let view_model = build_inspect_view_model(&formula_space, Some(&retained_artifact));

        let context = view_model
            .retained_artifact_context
            .expect("retained artifact context");
        assert_eq!(context.artifact_id, "artifact-1");
        assert_eq!(context.case_id, "case-1");
        assert_eq!(context.comparison_status, "blocked");
        assert_eq!(
            context.discrepancy_summary.as_deref(),
            Some("excel lane unavailable")
        );
        assert_eq!(
            context.bundle_report_path.as_deref(),
            Some("target/onecalc-verification/example")
        );
        assert!(context
            .xml_source_summary
            .as_deref()
            .is_some_and(|value| value.contains("Input!A1")));
        assert_eq!(
            context.display_comparison_summary.as_deref(),
            Some("OxFml 6 vs Excel $6.00")
        );
        assert_eq!(context.upstream_gap_summary.len(), 2);
    }
}
