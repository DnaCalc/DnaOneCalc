use dnaonecalc_host::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, build_programmatic_batch_plan,
    ProgrammaticCapabilityProfile, ProgrammaticComparisonLane, ProgrammaticComparisonStatus,
    ProgrammaticFormulaCase, ProgrammaticHostProfile, ProgrammaticOpenModeHint,
};

#[test]
fn builds_headless_batch_plan_for_excel_comparison_runs() {
    let plan = build_programmatic_batch_plan(
        &[
            ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "=SUM(1,2)".to_string(),
            },
            ProgrammaticFormulaCase {
                case_id: "case-2".to_string(),
                entered_cell_text: "'123.4".to_string(),
            },
        ],
        &ProgrammaticHostProfile {
            profile_id: "windows-excel".to_string(),
            requires_excel_observation: true,
        },
        &ProgrammaticCapabilityProfile {
            host_summary: "windows-native".to_string(),
            excel_observation_available: true,
        },
    );

    assert_eq!(plan.formula_count, 2);
    assert_eq!(plan.comparison_lane, ProgrammaticComparisonLane::OxfmlAndExcel);
    assert!(plan.retained_artifact_kinds.contains(&"replay_bundle"));
    assert!(plan.retained_artifact_kinds.contains(&"comparison_outcome"));
}

#[test]
fn blocked_excel_lane_still_produces_openable_workbench_artifacts() {
    let blocked_entry = build_programmatic_artifact_catalog_entry(
        "artifact-1",
        "case-blocked",
        ProgrammaticComparisonStatus::Blocked,
    );

    assert_eq!(blocked_entry.open_mode_hint, ProgrammaticOpenModeHint::Workbench);
}

