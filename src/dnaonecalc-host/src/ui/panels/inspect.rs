use crate::adapters::oxfml::{BindSummary, EvalSummary, ParseSummary, ProvenanceSummary};
use crate::services::inspect_mode::{InspectFormulaWalkNodeView, InspectViewModel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectWalkClusterViewModel {
    pub raw_entered_cell_text: String,
    pub green_tree_key: Option<String>,
    pub formula_walk_nodes: Vec<InspectFormulaWalkNodeView>,
    pub inspect_result_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSummaryClusterViewModel {
    pub parse_summary: Option<ParseSummary>,
    pub bind_summary: Option<BindSummary>,
    pub eval_summary: Option<EvalSummary>,
    pub provenance_summary: Option<ProvenanceSummary>,
}

pub fn build_inspect_walk_cluster(
    view_model: &InspectViewModel,
) -> InspectWalkClusterViewModel {
    InspectWalkClusterViewModel {
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
        parse_summary: view_model.parse_summary.clone(),
        bind_summary: view_model.bind_summary.clone(),
        eval_summary: view_model.eval_summary.clone(),
        provenance_summary: view_model.provenance_summary.clone(),
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
        };

        let cluster = build_inspect_walk_cluster(&view_model);
        assert_eq!(cluster.raw_entered_cell_text, "=LET(x,1,x)");
        assert_eq!(cluster.green_tree_key.as_deref(), Some("green-1"));
        assert_eq!(cluster.formula_walk_nodes.len(), 1);
    }

    #[test]
    fn inspect_summary_cluster_keeps_summary_surface_fields() {
        let view_model = InspectViewModel {
            raw_entered_cell_text: "=LET(x,1,x)".to_string(),
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
        };

        let cluster = build_inspect_summary_cluster(&view_model);
        assert_eq!(cluster.parse_summary.as_ref().map(|x| x.token_count), Some(7));
        assert_eq!(cluster.bind_summary.as_ref().map(|x| x.variable_count), Some(1));
        assert_eq!(cluster.eval_summary.as_ref().map(|x| x.step_count), Some(2));
        assert_eq!(
            cluster
                .provenance_summary
                .as_ref()
                .map(|x| x.profile_summary.as_str()),
            Some("OC-H0")
        );
    }
}
