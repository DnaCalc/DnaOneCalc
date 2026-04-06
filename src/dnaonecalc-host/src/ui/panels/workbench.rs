use crate::services::workbench_mode::{
    WorkbenchComparisonRecordView, WorkbenchExplainRecordView, WorkbenchRetainedCatalogItemView,
    WorkbenchViewModel,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchOutcomeClusterViewModel {
    pub scenario_label: String,
    pub truth_source_label: String,
    pub host_profile_summary: String,
    pub capability_floor_summary: String,
    pub outcome_summary: Option<String>,
    pub recommended_action: String,
    pub retained_artifact_id: Option<String>,
    pub retained_case_id: Option<String>,
    pub comparison_status_summary: Option<String>,
    pub visible_output_match: Option<bool>,
    pub replay_equivalent: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchEvidenceClusterViewModel {
    pub raw_entered_cell_text: String,
    pub evidence_summary: Option<String>,
    pub retained_discrepancy_summary: Option<String>,
    pub trace_summary: Option<String>,
    pub imported_bundle_summary: Option<String>,
    pub xml_source_summary: Option<String>,
    pub display_comparison_summary: Option<String>,
    pub upstream_gap_summary: Vec<String>,
    pub comparison_records: Vec<WorkbenchComparisonRecordView>,
    pub explain_records: Vec<WorkbenchExplainRecordView>,
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
        scenario_label: view_model.scenario_label.clone(),
        truth_source_label: view_model.truth_source_label.clone(),
        host_profile_summary: view_model.host_profile_summary.clone(),
        capability_floor_summary: view_model.capability_floor_summary.clone(),
        outcome_summary: view_model.outcome_summary.clone(),
        recommended_action: view_model.recommended_action.clone(),
        retained_artifact_id: view_model.retained_artifact_id.clone(),
        retained_case_id: view_model.retained_case_id.clone(),
        comparison_status_summary: view_model.comparison_status_summary.clone(),
        visible_output_match: view_model.visible_output_match,
        replay_equivalent: view_model.replay_equivalent,
    }
}

pub fn build_workbench_evidence_cluster(
    view_model: &WorkbenchViewModel,
) -> WorkbenchEvidenceClusterViewModel {
    WorkbenchEvidenceClusterViewModel {
        raw_entered_cell_text: view_model.raw_entered_cell_text.clone(),
        evidence_summary: view_model.evidence_summary.clone(),
        retained_discrepancy_summary: view_model.retained_discrepancy_summary.clone(),
        trace_summary: view_model.trace_summary.clone(),
        imported_bundle_summary: view_model.imported_bundle_summary.clone(),
        xml_source_summary: view_model.xml_source_summary.clone(),
        display_comparison_summary: view_model.display_comparison_summary.clone(),
        upstream_gap_summary: view_model.upstream_gap_summary.clone(),
        comparison_records: view_model.comparison_records.clone(),
        explain_records: view_model.explain_records.clone(),
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
            scenario_label: "mismatch case".to_string(),
            truth_source_label: "preview-backed".to_string(),
            host_profile_summary: "Windows desktop preview".to_string(),
            capability_floor_summary: "Workbench with retained artifacts".to_string(),
            outcome_summary: Some("Number".to_string()),
            evidence_summary: Some("green=green-1, diagnostics=1".to_string()),
            lineage_items: vec!["Scenario opened".to_string()],
            action_items: vec!["Retain snapshot".to_string()],
            recommended_action: "Retain and compare".to_string(),
            retained_artifact_id: Some("artifact-1".to_string()),
            retained_case_id: Some("case-1".to_string()),
            comparison_status_summary: Some("mismatched".to_string()),
            visible_output_match: Some(false),
            replay_equivalent: Some(false),
            retained_discrepancy_summary: Some("dna=1 excel=2".to_string()),
            trace_summary: Some("Preview trace".to_string()),
            imported_bundle_summary: Some("Imported bundle: target/example".to_string()),
            xml_source_summary: Some("Input @ Input!A1 | format $#,##0.00".to_string()),
            display_comparison_summary: Some(
                "Display divergence (effective_display_text): OxFml 6 vs Excel $6.00"
                    .to_string(),
            ),
            upstream_gap_summary: vec![
                "Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side"
                    .to_string(),
            ],
            comparison_records: vec![WorkbenchComparisonRecordView {
                mismatch_kind: "effective_display_text".to_string(),
                severity: "informational".to_string(),
                view_family: Some("effective_display_text".to_string()),
                family_label: "Effective display".to_string(),
                status_label: "Display divergence".to_string(),
                summary: "comparison view values diverged".to_string(),
                left_value_repr: Some("6".to_string()),
                right_value_repr: Some("$6.00".to_string()),
                detail: Some("comparison view values diverged".to_string()),
                is_projection_gap: false,
            }],
            explain_records: vec![WorkbenchExplainRecordView {
                query_id: Some("explain-01".to_string()),
                mismatch_kind: "effective_display_text".to_string(),
                severity: "informational".to_string(),
                view_family: Some("effective_display_text".to_string()),
                family_label: "Effective display".to_string(),
                summary: "comparison diverged on `effective_display_text`".to_string(),
                left_value_repr: Some("6".to_string()),
                right_value_repr: Some("$6.00".to_string()),
                detail: Some("comparison view values diverged".to_string()),
            }],
            retained_catalog_items: vec![WorkbenchRetainedCatalogItemView {
                artifact_id: "artifact-1".to_string(),
                case_id: "case-1".to_string(),
                comparison_status: "mismatched".to_string(),
                discrepancy_summary: Some("dna=1 excel=2".to_string()),
                xml_source_summary: Some("Input @ Input!A1".to_string()),
                is_open: true,
            }],
        };

        let outcome = build_workbench_outcome_cluster(&view_model);
        let evidence = build_workbench_evidence_cluster(&view_model);
        let lineage = build_workbench_lineage_cluster(&view_model);
        let actions = build_workbench_actions_cluster(&view_model);
        let catalog = build_workbench_catalog_cluster(&view_model);

        assert_eq!(outcome.truth_source_label, "preview-backed");
        assert_eq!(outcome.outcome_summary.as_deref(), Some("Number"));
        assert_eq!(outcome.recommended_action, "Retain and compare");
        assert_eq!(outcome.retained_artifact_id.as_deref(), Some("artifact-1"));
        assert_eq!(outcome.retained_case_id.as_deref(), Some("case-1"));
        assert_eq!(outcome.visible_output_match, Some(false));
        assert_eq!(evidence.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(
            evidence.retained_discrepancy_summary.as_deref(),
            Some("dna=1 excel=2")
        );
        assert_eq!(evidence.trace_summary.as_deref(), Some("Preview trace"));
        assert_eq!(
            evidence.display_comparison_summary.as_deref(),
            Some("Display divergence (effective_display_text): OxFml 6 vs Excel $6.00")
        );
        assert_eq!(evidence.upstream_gap_summary.len(), 1);
        assert_eq!(evidence.comparison_records.len(), 1);
        assert_eq!(evidence.explain_records.len(), 1);
        assert_eq!(lineage.lineage_items.len(), 1);
        assert_eq!(actions.action_items.len(), 1);
        assert_eq!(catalog.retained_catalog_items.len(), 1);
        assert!(catalog.retained_catalog_items[0].is_open);
        assert_eq!(catalog.retained_catalog_items[0].case_id, "case-1");
        assert!(evidence
            .evidence_summary
            .as_deref()
            .is_some_and(|value| value.contains("green-1")));
    }
}
