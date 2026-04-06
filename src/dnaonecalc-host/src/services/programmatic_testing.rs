use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticSpreadsheetXmlSource {
    pub workbook_path: String,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticFormulaCase {
    pub case_id: String,
    pub entered_cell_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spreadsheet_xml_source: Option<ProgrammaticSpreadsheetXmlSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticHostProfile {
    pub profile_id: String,
    pub requires_excel_observation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticCapabilityProfile {
    pub host_summary: String,
    pub excel_observation_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammaticComparisonLane {
    OxfmlOnly,
    OxfmlAndExcel,
    ExcelObservationBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticBatchPlan {
    pub formula_count: usize,
    pub comparison_lane: ProgrammaticComparisonLane,
    pub discrepancy_index_required: bool,
    pub retained_artifact_kinds: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammaticComparisonStatus {
    Matched,
    Mismatched,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammaticOpenModeHint {
    Inspect,
    Workbench,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticArtifactCatalogEntry {
    pub artifact_id: String,
    pub case_id: String,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub open_mode_hint: ProgrammaticOpenModeHint,
}

pub fn default_windows_excel_host_profile() -> ProgrammaticHostProfile {
    ProgrammaticHostProfile {
        profile_id: "windows_excel_default".to_string(),
        requires_excel_observation: true,
    }
}

pub fn default_windows_excel_capability_profile() -> ProgrammaticCapabilityProfile {
    ProgrammaticCapabilityProfile {
        host_summary: "windows_native_excel_default".to_string(),
        excel_observation_available: true,
    }
}

pub fn build_programmatic_batch_plan(
    cases: &[ProgrammaticFormulaCase],
    host_profile: &ProgrammaticHostProfile,
    capabilities: &ProgrammaticCapabilityProfile,
) -> ProgrammaticBatchPlan {
    let comparison_lane = match (
        host_profile.requires_excel_observation,
        capabilities.excel_observation_available,
    ) {
        (false, _) => ProgrammaticComparisonLane::OxfmlOnly,
        (true, true) => ProgrammaticComparisonLane::OxfmlAndExcel,
        (true, false) => ProgrammaticComparisonLane::ExcelObservationBlocked,
    };

    let mut retained_artifact_kinds = vec![
        "scenario_input".to_string(),
        "capability_context".to_string(),
        "run_result".to_string(),
        "replay_bundle".to_string(),
    ];
    match comparison_lane {
        ProgrammaticComparisonLane::OxfmlOnly => {}
        ProgrammaticComparisonLane::OxfmlAndExcel => {
            retained_artifact_kinds.push("comparison_outcome".to_string());
            retained_artifact_kinds.push("discrepancy_index".to_string());
        }
        ProgrammaticComparisonLane::ExcelObservationBlocked => {
            retained_artifact_kinds.push("comparison_blocked".to_string());
            retained_artifact_kinds.push("discrepancy_index".to_string());
        }
    }

    ProgrammaticBatchPlan {
        formula_count: cases.len(),
        discrepancy_index_required: matches!(
            comparison_lane,
            ProgrammaticComparisonLane::OxfmlAndExcel
                | ProgrammaticComparisonLane::ExcelObservationBlocked
        ),
        comparison_lane,
        retained_artifact_kinds,
    }
}

pub fn build_programmatic_artifact_catalog_entry(
    artifact_id: impl Into<String>,
    case_id: impl Into<String>,
    comparison_status: ProgrammaticComparisonStatus,
) -> ProgrammaticArtifactCatalogEntry {
    let open_mode_hint = match comparison_status {
        ProgrammaticComparisonStatus::Matched => ProgrammaticOpenModeHint::Inspect,
        ProgrammaticComparisonStatus::Mismatched | ProgrammaticComparisonStatus::Blocked => {
            ProgrammaticOpenModeHint::Workbench
        }
    };

    ProgrammaticArtifactCatalogEntry {
        artifact_id: artifact_id.into(),
        case_id: case_id.into(),
        comparison_status,
        open_mode_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_plan_uses_excel_lane_when_profile_requires_it_and_host_can_observe() {
        let plan = build_programmatic_batch_plan(
            &[ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "=SUM(1,2)".to_string(),
                spreadsheet_xml_source: None,
            }],
            &ProgrammaticHostProfile {
                profile_id: "windows-excel".to_string(),
                requires_excel_observation: true,
            },
            &ProgrammaticCapabilityProfile {
                host_summary: "windows-native".to_string(),
                excel_observation_available: true,
            },
        );

        assert_eq!(plan.formula_count, 1);
        assert_eq!(
            plan.comparison_lane,
            ProgrammaticComparisonLane::OxfmlAndExcel
        );
        assert!(plan.discrepancy_index_required);
        assert!(plan
            .retained_artifact_kinds
            .contains(&"comparison_outcome".to_string()));
    }

    #[test]
    fn batch_plan_marks_excel_observation_as_blocked_when_host_cannot_observe() {
        let plan = build_programmatic_batch_plan(
            &[ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "'123.4".to_string(),
                spreadsheet_xml_source: None,
            }],
            &ProgrammaticHostProfile {
                profile_id: "browser".to_string(),
                requires_excel_observation: true,
            },
            &ProgrammaticCapabilityProfile {
                host_summary: "browser-web".to_string(),
                excel_observation_available: false,
            },
        );

        assert_eq!(
            plan.comparison_lane,
            ProgrammaticComparisonLane::ExcelObservationBlocked
        );
        assert!(plan
            .retained_artifact_kinds
            .contains(&"comparison_blocked".to_string()));
    }

    #[test]
    fn mismatches_and_blocked_results_open_in_workbench() {
        let mismatch = build_programmatic_artifact_catalog_entry(
            "artifact-1",
            "case-1",
            ProgrammaticComparisonStatus::Mismatched,
        );
        let blocked = build_programmatic_artifact_catalog_entry(
            "artifact-2",
            "case-2",
            ProgrammaticComparisonStatus::Blocked,
        );
        let matched = build_programmatic_artifact_catalog_entry(
            "artifact-3",
            "case-3",
            ProgrammaticComparisonStatus::Matched,
        );

        assert_eq!(mismatch.open_mode_hint, ProgrammaticOpenModeHint::Workbench);
        assert_eq!(blocked.open_mode_hint, ProgrammaticOpenModeHint::Workbench);
        assert_eq!(matched.open_mode_hint, ProgrammaticOpenModeHint::Inspect);
    }
}
