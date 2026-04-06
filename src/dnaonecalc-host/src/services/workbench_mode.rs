use crate::services::verification_bundle::{
    replay_display_comparison_summary, replay_projection_coverage_gap_summaries,
    OxReplayExplainRecord, OxReplayMismatchRecord,
};
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
    pub retained_case_id: Option<String>,
    pub comparison_status_summary: Option<String>,
    pub visible_output_match: Option<bool>,
    pub replay_equivalent: Option<bool>,
    pub retained_discrepancy_summary: Option<String>,
    pub imported_bundle_summary: Option<String>,
    pub xml_source_summary: Option<String>,
    pub display_comparison_summary: Option<String>,
    pub upstream_gap_summary: Vec<String>,
    pub comparison_records: Vec<WorkbenchComparisonRecordView>,
    pub explain_records: Vec<WorkbenchExplainRecordView>,
    pub retained_catalog_items: Vec<WorkbenchRetainedCatalogItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchComparisonRecordView {
    pub mismatch_kind: String,
    pub severity: String,
    pub view_family: Option<String>,
    pub family_label: String,
    pub status_label: String,
    pub summary: String,
    pub left_value_repr: Option<String>,
    pub right_value_repr: Option<String>,
    pub detail: Option<String>,
    pub is_projection_gap: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchExplainRecordView {
    pub query_id: Option<String>,
    pub mismatch_kind: String,
    pub severity: String,
    pub view_family: Option<String>,
    pub family_label: String,
    pub summary: String,
    pub left_value_repr: Option<String>,
    pub right_value_repr: Option<String>,
    pub detail: Option<String>,
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
        replay_display_comparison_summary(
            &artifact.replay_mismatch_records,
            artifact.oxfml_effective_display_summary.as_deref(),
            artifact.excel_observed_value_repr.as_deref(),
        )
    });
    let upstream_gap_summary = retained_artifact
        .map(|artifact| {
            let per_family =
                replay_projection_coverage_gap_summaries(&artifact.replay_mismatch_records);
            if !per_family.is_empty() {
                return per_family;
            }

            artifact
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
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let comparison_records = retained_artifact
        .map(|artifact| {
            artifact
                .replay_mismatch_records
                .iter()
                .map(project_replay_mismatch_record)
                .collect()
        })
        .unwrap_or_default();
    let explain_records = retained_artifact
        .map(|artifact| {
            artifact
                .replay_explain_records
                .iter()
                .map(project_replay_explain_record)
                .collect()
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
        evidence_summary: retained_discrepancy_summary.clone().or(evidence_summary),
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
                items.push(format!(
                    "Retained artifact opened: {}",
                    artifact.artifact_id
                ));
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
            Some(
                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Mismatched,
            ) => {
                if !upstream_gap_summary.is_empty() {
                    "Review projection coverage gaps before claiming semantic mismatch".to_string()
                } else {
                    "Review discrepancy in workbench".to_string()
                }
            }
            Some(crate::services::programmatic_testing::ProgrammaticComparisonStatus::Blocked) => {
                "Review blocked comparison and host policy".to_string()
            }
            _ if formula_space.latest_evaluation_summary.is_some() => {
                "Retain and compare".to_string()
            }
            _ => "Evaluate before retaining evidence".to_string(),
        },
        retained_artifact_id: retained_artifact.map(|artifact| artifact.artifact_id.clone()),
        retained_case_id: retained_artifact.map(|artifact| artifact.case_id.clone()),
        comparison_status_summary: retained_artifact.map(comparison_status_label),
        visible_output_match: retained_artifact.and_then(|artifact| artifact.visible_output_match),
        replay_equivalent: retained_artifact.and_then(|artifact| artifact.replay_equivalent),
        retained_discrepancy_summary,
        imported_bundle_summary,
        xml_source_summary,
        display_comparison_summary,
        upstream_gap_summary,
        comparison_records,
        explain_records,
        retained_catalog_items: retained_catalog
            .iter()
            .map(|artifact| WorkbenchRetainedCatalogItemView {
                artifact_id: artifact.artifact_id.clone(),
                case_id: artifact.case_id.clone(),
                comparison_status: comparison_status_label(artifact),
                discrepancy_summary: artifact.discrepancy_summary.clone(),
                xml_source_summary: artifact
                    .xml_extraction
                    .as_ref()
                    .map(|xml| format!("{} @ {}", xml.worksheet_name, xml.locator)),
                is_open: retained_artifact
                    .is_some_and(|open_artifact| open_artifact.artifact_id == artifact.artifact_id),
            })
            .collect(),
    }
}

fn comparison_status_label(artifact: &RetainedArtifactRecord) -> String {
    match artifact.comparison_status {
        crate::services::programmatic_testing::ProgrammaticComparisonStatus::Matched => {
            "matched".to_string()
        }
        crate::services::programmatic_testing::ProgrammaticComparisonStatus::Mismatched => {
            "mismatched".to_string()
        }
        crate::services::programmatic_testing::ProgrammaticComparisonStatus::Blocked => {
            "blocked".to_string()
        }
    }
}

fn project_replay_mismatch_record(
    record: &OxReplayMismatchRecord,
) -> WorkbenchComparisonRecordView {
    WorkbenchComparisonRecordView {
        mismatch_kind: record.mismatch_kind.clone(),
        severity: record
            .severity
            .clone()
            .unwrap_or_else(|| default_record_severity(&record.mismatch_kind)),
        view_family: record.view_family.clone(),
        family_label: replay_family_label(record.view_family.as_deref(), &record.mismatch_kind),
        status_label: replay_status_label(&record.mismatch_kind),
        summary: replay_record_summary(
            record.view_family.as_deref(),
            &record.mismatch_kind,
            record.detail.as_deref(),
        ),
        left_value_repr: record.left_value_repr.clone(),
        right_value_repr: record.right_value_repr.clone(),
        detail: record.detail.clone(),
        is_projection_gap: record.mismatch_kind == "projection_coverage_gap",
    }
}

fn project_replay_explain_record(record: &OxReplayExplainRecord) -> WorkbenchExplainRecordView {
    WorkbenchExplainRecordView {
        query_id: record.query_id.clone(),
        mismatch_kind: record.mismatch_kind.clone(),
        severity: record
            .severity
            .clone()
            .unwrap_or_else(|| default_record_severity(&record.mismatch_kind)),
        view_family: record.view_family.clone(),
        family_label: replay_family_label(record.view_family.as_deref(), &record.mismatch_kind),
        summary: record.summary.clone().unwrap_or_else(|| {
            replay_record_summary(
                record.view_family.as_deref(),
                &record.mismatch_kind,
                record.detail.as_deref(),
            )
        }),
        left_value_repr: record.left_value_repr.clone(),
        right_value_repr: record.right_value_repr.clone(),
        detail: record.detail.clone(),
    }
}

fn replay_family_label(view_family: Option<&str>, mismatch_kind: &str) -> String {
    match view_family.unwrap_or(mismatch_kind) {
        "effective_display_text" => "Effective display".to_string(),
        "visible_value" | "view_value" => "Visible value".to_string(),
        "formatting_view" => "Formatting".to_string(),
        "conditional_formatting_view" => "Conditional formatting".to_string(),
        other => other.replace('_', " "),
    }
}

fn replay_status_label(mismatch_kind: &str) -> String {
    match mismatch_kind {
        "projection_coverage_gap" => "Coverage gap".to_string(),
        "effective_display_text" => "Display divergence".to_string(),
        "visible_value" | "view_value" => "Visible value divergence".to_string(),
        other => other.replace('_', " "),
    }
}

fn replay_record_summary(
    view_family: Option<&str>,
    mismatch_kind: &str,
    detail: Option<&str>,
) -> String {
    if let Some(detail) = detail {
        return detail.to_string();
    }

    match (view_family, mismatch_kind) {
        (Some("effective_display_text"), _) => "Effective display diverged".to_string(),
        (Some("formatting_view"), "projection_coverage_gap") => {
            "Formatting comparison family is missing on one side".to_string()
        }
        (Some("conditional_formatting_view"), "projection_coverage_gap") => {
            "Conditional-formatting family is missing on one side".to_string()
        }
        (Some(family), _) => format!("Comparison diverged for `{family}`"),
        (None, "view_value") => "Visible values diverged".to_string(),
        _ => "Comparison diverged".to_string(),
    }
}

fn default_record_severity(mismatch_kind: &str) -> String {
    match mismatch_kind {
        "projection_coverage_gap" => "coverage".to_string(),
        "effective_display_text" => "informational".to_string(),
        _ => "semantic".to_string(),
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
            replay_mismatch_records: Vec::new(),
            replay_explain_records: Vec::new(),
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
        assert_eq!(view_model.retained_case_id.as_deref(), Some("case-1"));
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

    #[test]
    fn workbench_view_model_prefers_per_family_gap_and_display_summaries() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));

        let retained_artifact = RetainedArtifactRecord {
            artifact_id: "artifact-xml".to_string(),
            case_id: "xml-case-1".to_string(),
            formula_space_id: FormulaSpaceId::new("space-1"),
            comparison_status: ProgrammaticComparisonStatus::Mismatched,
            open_mode_hint: ProgrammaticOpenModeHint::Workbench,
            discrepancy_summary: Some("legacy mismatch summary".to_string()),
            bundle_report_path: None,
            case_output_dir: None,
            xml_extraction: None,
            upstream_gap_report: Some(
                crate::services::verification_bundle::VerificationObservationGapReport {
                    oxfml_scope_required: vec![],
                    oxxlplay_supported_surfaces: vec!["cell_value".to_string()],
                    oxxlplay_missing_surfaces: vec!["effective_display_text".to_string()],
                    oxreplay_required_views: vec!["formatting_view".to_string()],
                    oxreplay_current_bundle_views: vec!["visible_value".to_string()],
                    oxreplay_missing_views: vec!["formatting_view".to_string()],
                },
            ),
            visible_output_match: Some(false),
            replay_equivalent: Some(false),
            replay_mismatch_records: vec![
                crate::services::verification_bundle::OxReplayMismatchRecord {
                    mismatch_kind: "effective_display_text".to_string(),
                    severity: Some("informational".to_string()),
                    view_family: Some("effective_display_text".to_string()),
                    left_value_repr: Some("6".to_string()),
                    right_value_repr: Some("$6.00".to_string()),
                    detail: Some("comparison view values diverged".to_string()),
                },
                crate::services::verification_bundle::OxReplayMismatchRecord {
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("formatting_view".to_string()),
                    left_value_repr: None,
                    right_value_repr: Some("{\"number_format_code\":\"$#,##0.00\"}".to_string()),
                    detail: Some(
                        "comparison view family `formatting_view` is missing on one side"
                            .to_string(),
                    ),
                },
            ],
            replay_explain_records: vec![
                crate::services::verification_bundle::OxReplayExplainRecord {
                    query_id: Some("explain-xml-1".to_string()),
                    summary: Some("comparison diverged on `effective_display_text`".to_string()),
                    mismatch_kind: "effective_display_text".to_string(),
                    severity: Some("informational".to_string()),
                    view_family: Some("effective_display_text".to_string()),
                    left_value_repr: Some("6".to_string()),
                    right_value_repr: Some("$6.00".to_string()),
                    detail: Some("comparison view values diverged".to_string()),
                },
            ],
            oxfml_effective_display_summary: Some("6".to_string()),
            excel_observed_value_repr: Some("$6.00".to_string()),
        };

        let view_model = build_workbench_view_model(
            &formula_space,
            Some(&retained_artifact),
            &[&retained_artifact],
        );

        assert_eq!(
            view_model.display_comparison_summary.as_deref(),
            Some("Display divergence (effective_display_text): OxFml 6 vs Excel $6.00")
        );
        assert_eq!(
            view_model.upstream_gap_summary,
            vec![
                "Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side".to_string()
            ]
        );
        assert_eq!(view_model.comparison_records.len(), 2);
        assert_eq!(
            view_model.comparison_records[0].family_label,
            "Effective display"
        );
        assert_eq!(view_model.explain_records.len(), 1);
        assert_eq!(
            view_model.recommended_action,
            "Review projection coverage gaps before claiming semantic mismatch"
        );
    }
}
