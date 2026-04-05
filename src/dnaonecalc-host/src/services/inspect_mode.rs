use crate::adapters::oxfml::{
    BindSummary, EvalSummary, FormulaWalkNode, FormulaWalkNodeState, ParseSummary,
    ProvenanceSummary,
};
use crate::state::FormulaSpaceState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectViewModel {
    pub raw_entered_cell_text: String,
    pub inspect_result_summary: Option<String>,
    pub green_tree_key: Option<String>,
    pub formula_walk_nodes: Vec<InspectFormulaWalkNodeView>,
    pub parse_summary: Option<ParseSummary>,
    pub bind_summary: Option<BindSummary>,
    pub eval_summary: Option<EvalSummary>,
    pub provenance_summary: Option<ProvenanceSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectFormulaWalkNodeView {
    pub node_id: String,
    pub label: String,
    pub value_preview: Option<String>,
    pub state: FormulaWalkNodeState,
    pub children: Vec<InspectFormulaWalkNodeView>,
}

pub fn build_inspect_view_model(formula_space: &FormulaSpaceState) -> InspectViewModel {
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

    InspectViewModel {
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        inspect_result_summary: formula_space.latest_evaluation_summary.clone(),
        green_tree_key,
        formula_walk_nodes,
        parse_summary,
        bind_summary,
        eval_summary,
        provenance_summary,
    }
}

fn project_formula_walk_node(node: &FormulaWalkNode) -> InspectFormulaWalkNodeView {
    InspectFormulaWalkNodeView {
        node_id: node.node_id.clone(),
        label: node.label.clone(),
        value_preview: node.value_preview.clone(),
        state: node.state,
        children: node.children.iter().map(project_formula_walk_node).collect(),
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
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=LET(x,1,x)");
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
        });

        let view_model = build_inspect_view_model(&formula_space);
        assert_eq!(view_model.raw_entered_cell_text, "=LET(x,1,x)");
        assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
        assert_eq!(view_model.formula_walk_nodes.len(), 1);
        assert_eq!(view_model.formula_walk_nodes[0].children.len(), 1);
        assert_eq!(view_model.parse_summary.as_ref().map(|x| x.token_count), Some(7));
        assert_eq!(view_model.bind_summary.as_ref().map(|x| x.variable_count), Some(1));
        assert_eq!(view_model.eval_summary.as_ref().map(|x| x.step_count), Some(2));
        assert_eq!(
            view_model
                .provenance_summary
                .as_ref()
                .map(|x| x.profile_summary.as_str()),
            Some("OC-H0")
        );
    }
}
