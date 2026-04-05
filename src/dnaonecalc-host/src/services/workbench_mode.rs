use crate::state::FormulaSpaceState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchViewModel {
    pub raw_entered_cell_text: String,
    pub outcome_summary: Option<String>,
    pub evidence_summary: Option<String>,
    pub recommended_action: String,
}

pub fn build_workbench_view_model(formula_space: &FormulaSpaceState) -> WorkbenchViewModel {
    let evidence_summary = formula_space.editor_document.as_ref().map(|document| {
        format!(
            "green={}, diagnostics={}",
            document.green_tree_key(),
            document.live_diagnostics.diagnostics.len()
        )
    });

    WorkbenchViewModel {
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        outcome_summary: formula_space.latest_evaluation_summary.clone(),
        evidence_summary,
        recommended_action: if formula_space.latest_evaluation_summary.is_some() {
            "Retain and compare".to_string()
        } else {
            "Evaluate before retaining evidence".to_string()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::FormulaSpaceState;
    use crate::test_support::sample_editor_document;

    #[test]
    fn workbench_view_model_projects_outcome_and_evidence_summary() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());

        let view_model = build_workbench_view_model(&formula_space);
        assert_eq!(view_model.outcome_summary.as_deref(), Some("Number"));
        assert!(view_model
            .evidence_summary
            .as_deref()
            .is_some_and(|value| value.contains("green=green-1")));
        assert_eq!(view_model.recommended_action, "Retain and compare");
    }
}
