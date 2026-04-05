use crate::services::workbench_mode::{WorkbenchRetainedCatalogItemView, WorkbenchViewModel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchOutcomeClusterViewModel {
    pub outcome_summary: Option<String>,
    pub recommended_action: String,
    pub retained_artifact_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchEvidenceClusterViewModel {
    pub raw_entered_cell_text: String,
    pub evidence_summary: Option<String>,
    pub retained_discrepancy_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchLineageClusterViewModel {
    pub lineage_items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchActionsClusterViewModel {
    pub action_items: Vec<String>,
    pub recommended_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchCatalogClusterViewModel {
    pub retained_catalog_items: Vec<WorkbenchRetainedCatalogItemView>,
}

pub fn build_workbench_outcome_cluster(
    view_model: &WorkbenchViewModel,
) -> WorkbenchOutcomeClusterViewModel {
    WorkbenchOutcomeClusterViewModel {
        outcome_summary: view_model.outcome_summary.clone(),
        recommended_action: view_model.recommended_action.clone(),
        retained_artifact_id: view_model.retained_artifact_id.clone(),
    }
}

pub fn build_workbench_evidence_cluster(
    view_model: &WorkbenchViewModel,
) -> WorkbenchEvidenceClusterViewModel {
    WorkbenchEvidenceClusterViewModel {
        raw_entered_cell_text: view_model.raw_entered_cell_text.clone(),
        evidence_summary: view_model.evidence_summary.clone(),
        retained_discrepancy_summary: view_model.retained_discrepancy_summary.clone(),
    }
}

pub fn build_workbench_lineage_cluster(
    view_model: &WorkbenchViewModel,
) -> WorkbenchLineageClusterViewModel {
    WorkbenchLineageClusterViewModel {
        lineage_items: view_model.lineage_items.clone(),
    }
}

pub fn build_workbench_actions_cluster(
    view_model: &WorkbenchViewModel,
) -> WorkbenchActionsClusterViewModel {
    WorkbenchActionsClusterViewModel {
        action_items: view_model.action_items.clone(),
        recommended_action: view_model.recommended_action.clone(),
    }
}

pub fn build_workbench_catalog_cluster(
    view_model: &WorkbenchViewModel,
) -> WorkbenchCatalogClusterViewModel {
    WorkbenchCatalogClusterViewModel {
        retained_catalog_items: view_model.retained_catalog_items.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::workbench_mode::WorkbenchViewModel;

    #[test]
    fn workbench_clusters_split_outcome_and_evidence_fields() {
        let view_model = WorkbenchViewModel {
            raw_entered_cell_text: "=SUM(1,2)".to_string(),
            outcome_summary: Some("Number".to_string()),
            evidence_summary: Some("green=green-1, diagnostics=1".to_string()),
            lineage_items: vec!["Scenario opened".to_string()],
            action_items: vec!["Retain snapshot".to_string()],
            recommended_action: "Retain and compare".to_string(),
            retained_artifact_id: Some("artifact-1".to_string()),
            retained_discrepancy_summary: Some("dna=1 excel=2".to_string()),
            retained_catalog_items: vec![WorkbenchRetainedCatalogItemView {
                artifact_id: "artifact-1".to_string(),
                comparison_status: "mismatched".to_string(),
                discrepancy_summary: Some("dna=1 excel=2".to_string()),
                is_open: true,
            }],
        };

        let outcome = build_workbench_outcome_cluster(&view_model);
        let evidence = build_workbench_evidence_cluster(&view_model);
        let lineage = build_workbench_lineage_cluster(&view_model);
        let actions = build_workbench_actions_cluster(&view_model);
        let catalog = build_workbench_catalog_cluster(&view_model);

        assert_eq!(outcome.outcome_summary.as_deref(), Some("Number"));
        assert_eq!(outcome.recommended_action, "Retain and compare");
        assert_eq!(outcome.retained_artifact_id.as_deref(), Some("artifact-1"));
        assert_eq!(evidence.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(evidence.retained_discrepancy_summary.as_deref(), Some("dna=1 excel=2"));
        assert_eq!(lineage.lineage_items.len(), 1);
        assert_eq!(actions.action_items.len(), 1);
        assert_eq!(catalog.retained_catalog_items.len(), 1);
        assert!(catalog.retained_catalog_items[0].is_open);
        assert!(evidence
            .evidence_summary
            .as_deref()
            .is_some_and(|value| value.contains("green-1")));
    }
}
