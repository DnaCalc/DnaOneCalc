#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammaticFormulaCase {
    pub case_id: String,
    pub entered_cell_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammaticHostProfile {
    pub profile_id: String,
    pub requires_excel_observation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammaticCapabilityProfile {
    pub host_summary: String,
    pub excel_observation_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgrammaticComparisonLane {
    OxfmlOnly,
    OxfmlAndExcel,
    ExcelObservationBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammaticBatchPlan {
    pub formula_count: usize,
    pub comparison_lane: ProgrammaticComparisonLane,
    pub discrepancy_index_required: bool,
    pub retained_artifact_kinds: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgrammaticComparisonStatus {
    Matched,
    Mismatched,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgrammaticOpenModeHint {
    Inspect,
    Workbench,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammaticArtifactCatalogEntry {
    pub artifact_id: String,
    pub case_id: String,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub open_mode_hint: ProgrammaticOpenModeHint,
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
        "scenario_input",
        "capability_context",
        "run_result",
        "replay_bundle",
    ];
    match comparison_lane {
        ProgrammaticComparisonLane::OxfmlOnly => {}
        ProgrammaticComparisonLane::OxfmlAndExcel => {
            retained_artifact_kinds.push("comparison_outcome");
            retained_artifact_kinds.push("discrepancy_index");
        }
        ProgrammaticComparisonLane::ExcelObservationBlocked => {
            retained_artifact_kinds.push("comparison_blocked");
            retained_artifact_kinds.push("discrepancy_index");
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
        assert_eq!(plan.comparison_lane, ProgrammaticComparisonLane::OxfmlAndExcel);
        assert!(plan.discrepancy_index_required);
        assert!(plan.retained_artifact_kinds.contains(&"comparison_outcome"));
    }

    #[test]
    fn batch_plan_marks_excel_observation_as_blocked_when_host_cannot_observe() {
        let plan = build_programmatic_batch_plan(
            &[ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "'123.4".to_string(),
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
        assert!(plan.retained_artifact_kinds.contains(&"comparison_blocked"));
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
