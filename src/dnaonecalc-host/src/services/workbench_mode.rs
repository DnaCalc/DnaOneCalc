use crate::state::{FormulaSpaceState, RetainedArtifactRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchViewModel {
    pub scenario_label: String,
    pub truth_source_label: String,
    pub host_profile_summary: String,
    pub capability_floor_summary: String,
    pub trace_summary: Option<String>,
    pub raw_entered_cell_text: String,
    pub outcome_summary: Option<String>,
    pub evidence_summary: Option<String>,
    pub lineage_items: Vec<String>,
    pub action_items: Vec<String>,
    pub recommended_action: String,
    pub retained_artifact_id: Option<String>,
    pub retained_discrepancy_summary: Option<String>,
    pub imported_bundle_summary: Option<String>,
    pub xml_source_summary: Option<String>,
    pub display_comparison_summary: Option<String>,
    pub upstream_gap_summary: Vec<String>,
    pub retained_catalog_items: Vec<WorkbenchRetainedCatalogItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchRetainedCatalogItemView {
    pub artifact_id: String,
    pub case_id: String,
    pub comparison_status: String,
    pub discrepancy_summary: Option<String>,
    pub xml_source_summary: Option<String>,
    pub is_open: bool,
}

pub fn build_workbench_view_model(
    formula_space: &FormulaSpaceState,
    retained_artifact: Option<&RetainedArtifactRecord>,
    retained_catalog: &[&RetainedArtifactRecord],
) -> WorkbenchViewModel {
    let evidence_summary = formula_space.editor_document.as_ref().map(|document| {
        format!(
            "green={}, diagnostics={}",
            document.green_tree_key(),
            document.live_diagnostics.diagnostics.len()
        )
    });
    let retained_discrepancy_summary =
        retained_artifact.and_then(|artifact| artifact.discrepancy_summary.clone());
    let imported_bundle_summary = retained_artifact.and_then(|artifact| {
        artifact
            .bundle_report_path
            .as_ref()
            .map(|bundle| format!("Imported bundle: {bundle}"))
    });
    let xml_source_summary = retained_artifact.and_then(|artifact| {
        artifact.xml_extraction.as_ref().map(|xml| {
            format!(
                "{} @ {} | format {}",
                xml.workbook_path,
                xml.locator,
                xml.number_format_code
                    .clone()
                    .unwrap_or_else(|| "<none>".to_string())
            )
        })
    });
    let display_comparison_summary = retained_artifact.and_then(|artifact| {
        match (
            artifact.oxfml_effective_display_summary.as_deref(),
            artifact.excel_observed_value_repr.as_deref(),
        ) {
            (Some(oxfml), Some(excel)) => Some(format!("OxFml {oxfml} vs Excel {excel}")),
            _ => None,
        }
    });
    let upstream_gap_summary = retained_artifact
        .and_then(|artifact| artifact.upstream_gap_report.as_ref())
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
        .unwrap_or_default();

    WorkbenchViewModel {
        scenario_label: formula_space.context.scenario_label.clone(),
        truth_source_label: formula_space.context.truth_source.label().to_string(),
        host_profile_summary: formula_space.context.host_profile.clone(),
        capability_floor_summary: formula_space.context.capability_floor.clone(),
        trace_summary: formula_space.context.trace_summary.clone(),
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        outcome_summary: retained_artifact
            .map(|artifact| match artifact.comparison_status {
                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Matched => {
                    "Matched".to_string()
                }
                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Mismatched => {
                    "Mismatched".to_string()
                }
                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Blocked => {
                    "Blocked".to_string()
                }
            })
            .or_else(|| formula_space.latest_evaluation_summary.clone()),
        evidence_summary: retained_discrepancy_summary
            .clone()
            .or(evidence_summary),
        lineage_items: {
            let mut items = vec![
            "Scenario opened".to_string(),
            "Editor document projected".to_string(),
            if formula_space.latest_evaluation_summary.is_some() {
                "Evaluation captured".to_string()
            } else {
                "Evaluation pending".to_string()
            },
        ];
            if let Some(artifact) = retained_artifact {
                items.push(format!("Retained artifact opened: {}", artifact.artifact_id));
            }
            items
        },
        action_items: {
            let mut items = vec![
            "Retain snapshot".to_string(),
            "Open compare".to_string(),
            "Prepare handoff".to_string(),
        ];
            if retained_artifact.is_some() {
                items.push("Review retained discrepancy".to_string());
            }
            items
        },
        recommended_action: match retained_artifact.map(|artifact| artifact.comparison_status) {
            Some(crate::services::programmatic_testing::ProgrammaticComparisonStatus::Mismatched) => {
                "Review discrepancy in workbench".to_string()
            }
            Some(crate::services::programmatic_testing::ProgrammaticComparisonStatus::Blocked) => {
                "Review blocked comparison and host policy".to_string()
            }
            _ if formula_space.latest_evaluation_summary.is_some() => "Retain and compare".to_string(),
            _ => "Evaluate before retaining evidence".to_string(),
        },
        retained_artifact_id: retained_artifact.map(|artifact| artifact.artifact_id.clone()),
        retained_discrepancy_summary,
        imported_bundle_summary,
        xml_source_summary,
        display_comparison_summary,
        upstream_gap_summary,
        retained_catalog_items: retained_catalog
            .iter()
            .map(|artifact| WorkbenchRetainedCatalogItemView {
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
                xml_source_summary: artifact.xml_extraction.as_ref().map(|xml| {
                    format!("{} @ {}", xml.worksheet_name, xml.locator)
                }),
                is_open: retained_artifact
                    .is_some_and(|open_artifact| open_artifact.artifact_id == artifact.artifact_id),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::services::programmatic_testing::{
        ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
    };
    use crate::state::FormulaSpaceState;
    use crate::test_support::sample_editor_document;

    #[test]
    fn workbench_view_model_projects_outcome_and_evidence_summary() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());

        let view_model = build_workbench_view_model(&formula_space, None, &[]);
        assert_eq!(view_model.truth_source_label, "local-fallback");
        assert_eq!(view_model.outcome_summary.as_deref(), Some("Number"));
        assert!(view_model
            .evidence_summary
            .as_deref()
            .is_some_and(|value| value.contains("green=green-1")));
        assert_eq!(view_model.lineage_items.len(), 3);
        assert_eq!(view_model.action_items.len(), 3);
        assert_eq!(view_model.recommended_action, "Retain and compare");
    }

    #[test]
    fn workbench_view_model_prefers_open_retained_discrepancy_artifact() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());

        let retained_artifact = RetainedArtifactRecord {
            artifact_id: "artifact-1".to_string(),
            case_id: "case-1".to_string(),
            formula_space_id: FormulaSpaceId::new("space-1"),
            comparison_status: ProgrammaticComparisonStatus::Mismatched,
            open_mode_hint: ProgrammaticOpenModeHint::Workbench,
            discrepancy_summary: Some("dna=1 excel=2".to_string()),
            bundle_report_path: None,
            case_output_dir: None,
            xml_extraction: None,
            upstream_gap_report: None,
            visible_output_match: None,
            replay_equivalent: None,
            oxfml_effective_display_summary: None,
            excel_observed_value_repr: None,
        };

        let view_model = build_workbench_view_model(
            &formula_space,
            Some(&retained_artifact),
            &[&retained_artifact],
        );
        assert_eq!(view_model.outcome_summary.as_deref(), Some("Mismatched"));
        assert_eq!(
            view_model.retained_artifact_id.as_deref(),
            Some("artifact-1")
        );
        assert_eq!(
            view_model.retained_discrepancy_summary.as_deref(),
            Some("dna=1 excel=2")
        );
        assert_eq!(
            view_model.recommended_action,
            "Review discrepancy in workbench"
        );
        assert_eq!(view_model.retained_catalog_items.len(), 1);
        assert!(view_model.retained_catalog_items[0].is_open);
        assert_eq!(view_model.retained_catalog_items[0].case_id, "case-1");
    }
}
