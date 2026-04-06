use crate::adapters::oxfml::{BindSummary, EvalSummary, ParseSummary, ProvenanceSummary};
use crate::services::inspect_mode::{
    InspectFormulaWalkNodeView, InspectRetainedArtifactContextView, InspectViewModel,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectWalkClusterViewModel {
    pub scenario_label: String,
    pub truth_source_label: String,
    pub host_profile_summary: String,
    pub raw_entered_cell_text: String,
    pub green_tree_key: Option<String>,
    pub formula_walk_nodes: Vec<InspectFormulaWalkNodeView>,
    pub inspect_result_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSummaryClusterViewModel {
    pub packet_kind_summary: String,
    pub capability_floor_summary: String,
    pub mode_availability_summary: String,
    pub trace_summary: Option<String>,
    pub blocked_reason: Option<String>,
    pub parse_summary: Option<ParseSummary>,
    pub bind_summary: Option<BindSummary>,
    pub eval_summary: Option<EvalSummary>,
    pub provenance_summary: Option<ProvenanceSummary>,
    pub retained_artifact_context: Option<InspectRetainedArtifactContextView>,
}

pub fn build_inspect_walk_cluster(view_model: &InspectViewModel) -> InspectWalkClusterViewModel {
    InspectWalkClusterViewModel {
        scenario_label: view_model.scenario_label.clone(),
        truth_source_label: view_model.truth_source_label.clone(),
        host_profile_summary: view_model.host_profile_summary.clone(),
        raw_entered_cell_text: view_model.raw_entered_cell_text.clone(),
        green_tree_key: view_model.green_tree_key.clone(),
        formula_walk_nodes: view_model.formula_walk_nodes.clone(),
        inspect_result_summary: view_model.inspect_result_summary.clone(),
    }
}

pub fn build_inspect_summary_cluster(
    view_model: &InspectViewModel,
) -> InspectSummaryClusterViewModel {
    InspectSummaryClusterViewModel {
        packet_kind_summary: view_model.packet_kind_summary.clone(),
        capability_floor_summary: view_model.capability_floor_summary.clone(),
        mode_availability_summary: view_model.mode_availability_summary.clone(),
        trace_summary: view_model.trace_summary.clone(),
        blocked_reason: view_model.blocked_reason.clone(),
        parse_summary: view_model.parse_summary.clone(),
        bind_summary: view_model.bind_summary.clone(),
        eval_summary: view_model.eval_summary.clone(),
        provenance_summary: view_model.provenance_summary.clone(),
        retained_artifact_context: view_model.retained_artifact_context.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{FormulaWalkNodeState, ParseSummary};
    use crate::services::inspect_mode::{InspectFormulaWalkNodeView, InspectViewModel};

    #[test]
    fn inspect_walk_cluster_keeps_formula_walk_surface_fields() {
        let view_model = InspectViewModel {
            raw_entered_cell_text: "=LET(x,1,x)".to_string(),
            scenario_label: "let binding".to_string(),
            truth_source_label: "preview-backed".to_string(),
            host_profile_summary: "Windows desktop preview".to_string(),
            inspect_result_summary: Some("Number".to_string()),
            green_tree_key: Some("green-1".to_string()),
            formula_walk_nodes: vec![InspectFormulaWalkNodeView {
                node_id: "node-1".to_string(),
                label: "LET".to_string(),
                value_preview: Some("1".to_string()),
                state: FormulaWalkNodeState::Evaluated,
                children: vec![],
            }],
            parse_summary: Some(ParseSummary {
                status: "Valid".to_string(),
                token_count: 7,
            }),
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
            packet_kind_summary: "preview edit packet".to_string(),
            capability_floor_summary: "Explore + Inspect".to_string(),
            mode_availability_summary: "Explore / Inspect / Workbench".to_string(),
            trace_summary: Some("Preview trace".to_string()),
            blocked_reason: None,
            retained_artifact_context: None,
        };

        let cluster = build_inspect_walk_cluster(&view_model);
        assert_eq!(cluster.truth_source_label, "preview-backed");
        assert_eq!(cluster.raw_entered_cell_text, "=LET(x,1,x)");
        assert_eq!(cluster.green_tree_key.as_deref(), Some("green-1"));
        assert_eq!(cluster.formula_walk_nodes.len(), 1);
    }

    #[test]
    fn inspect_summary_cluster_keeps_summary_surface_fields() {
        let view_model = InspectViewModel {
            raw_entered_cell_text: "=LET(x,1,x)".to_string(),
            scenario_label: "let binding".to_string(),
            truth_source_label: "preview-backed".to_string(),
            host_profile_summary: "Windows desktop preview".to_string(),
            inspect_result_summary: Some("Number".to_string()),
            green_tree_key: None,
            formula_walk_nodes: vec![],
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
            packet_kind_summary: "preview edit packet".to_string(),
            capability_floor_summary: "Explore + Inspect".to_string(),
            mode_availability_summary: "Explore / Inspect / Workbench".to_string(),
            trace_summary: Some("Preview trace".to_string()),
            blocked_reason: None,
            retained_artifact_context: Some(InspectRetainedArtifactContextView {
                artifact_id: "artifact-1".to_string(),
                case_id: "case-1".to_string(),
                comparison_status: "blocked".to_string(),
                discrepancy_summary: Some("excel lane unavailable".to_string()),
                bundle_report_path: Some("target/onecalc-verification/example".to_string()),
                xml_source_summary: Some("Input @ Input!A1 | format $#,##0.00".to_string()),
                display_comparison_summary: Some("OxFml 6 vs Excel $6.00".to_string()),
                upstream_gap_summary: vec!["OxXlPlay missing: effective_display_text".to_string()],
            }),
        };

        let cluster = build_inspect_summary_cluster(&view_model);
        assert_eq!(cluster.packet_kind_summary, "preview edit packet");
        assert_eq!(
            cluster.parse_summary.as_ref().map(|x| x.token_count),
            Some(7)
        );
        assert_eq!(
            cluster.bind_summary.as_ref().map(|x| x.variable_count),
            Some(1)
        );
        assert_eq!(cluster.eval_summary.as_ref().map(|x| x.step_count), Some(2));
        assert_eq!(
            cluster
                .provenance_summary
                .as_ref()
                .map(|x| x.profile_summary.as_str()),
            Some("OC-H0")
        );
        assert_eq!(
            cluster
                .retained_artifact_context
                .as_ref()
                .map(|x| x.artifact_id.as_str()),
            Some("artifact-1")
        );
    }
}
