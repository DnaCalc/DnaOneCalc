use crate::services::explore_mode::{build_explore_view_model, ExploreViewModel};
use crate::services::inspect_mode::{build_inspect_view_model, InspectViewModel};
use crate::services::retained_artifacts::{
    active_retained_artifact, retained_artifacts_for_formula_space,
};
use crate::services::workbench_mode::{build_workbench_view_model, WorkbenchViewModel};
use crate::state::{AppMode, FormulaSpaceState, OneCalcHostState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveModeProjection {
    Explore(ExploreViewModel),
    Inspect(InspectViewModel),
    Workbench(WorkbenchViewModel),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellModeTabViewModel {
    pub mode: AppMode,
    pub label: &'static str,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFormulaSpaceListItemViewModel {
    pub formula_space_id: String,
    pub label: String,
    pub truth_source_label: String,
    pub packet_kind_summary: String,
    pub is_active: bool,
    pub is_pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFrameViewModel {
    pub active_formula_space_label: String,
    pub active_mode_label: &'static str,
    pub active_truth_source_label: String,
    pub active_host_profile_summary: String,
    pub active_packet_kind_summary: String,
    pub active_capability_floor_summary: String,
    pub context_facts: Vec<ShellChromeFactViewModel>,
    pub footer_facts: Vec<ShellChromeFactViewModel>,
    pub workspace_summary: String,
    pub mode_tabs: Vec<ShellModeTabViewModel>,
    pub formula_spaces: Vec<ShellFormulaSpaceListItemViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellChromeFactViewModel {
    pub label: &'static str,
    pub value: String,
    pub tone: &'static str,
}

pub fn active_formula_space(state: &OneCalcHostState) -> Option<&FormulaSpaceState> {
    let formula_space_id = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .or(state
            .active_formula_space_view
            .selected_formula_space_id
            .as_ref())?;
    state.formula_spaces.get(formula_space_id)
}

pub fn build_active_mode_projection(state: &OneCalcHostState) -> Option<ActiveModeProjection> {
    let formula_space = active_formula_space(state)?;
    match state.active_formula_space_view.active_mode {
        AppMode::Explore => Some(ActiveModeProjection::Explore(build_explore_view_model(
            formula_space,
        ))),
        AppMode::Inspect => Some(ActiveModeProjection::Inspect(build_inspect_view_model(
            formula_space,
            active_retained_artifact(state)
                .filter(|artifact| artifact.formula_space_id == formula_space.formula_space_id),
        ))),
        AppMode::Workbench => Some(ActiveModeProjection::Workbench(build_workbench_view_model(
            formula_space,
            active_retained_artifact(state)
                .filter(|artifact| artifact.formula_space_id == formula_space.formula_space_id),
            &retained_artifacts_for_formula_space(state, &formula_space.formula_space_id),
        ))),
    }
}

pub fn build_shell_frame_view_model(state: &OneCalcHostState) -> Option<ShellFrameViewModel> {
    let active_formula_space = active_formula_space(state)?;
    let active_formula_space_id = &active_formula_space.formula_space_id;
    let active_mode = state.active_formula_space_view.active_mode;
    let active_mode_label = match active_mode {
        AppMode::Explore => "Explore",
        AppMode::Inspect => "Inspect",
        AppMode::Workbench => "Workbench",
    };

    let mode_tabs = [AppMode::Explore, AppMode::Inspect, AppMode::Workbench]
        .into_iter()
        .map(|mode| ShellModeTabViewModel {
            mode,
            label: match mode {
                AppMode::Explore => "Explore",
                AppMode::Inspect => "Inspect",
                AppMode::Workbench => "Workbench",
            },
            is_active: state.active_formula_space_view.active_mode == mode,
        })
        .collect();

    let formula_spaces = state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .filter_map(|formula_space_id| {
            state
                .formula_spaces
                .get(formula_space_id)
                .map(|formula_space| ShellFormulaSpaceListItemViewModel {
                    formula_space_id: formula_space.formula_space_id.as_str().to_string(),
                    label: formula_space.context.scenario_label.clone(),
                    truth_source_label: formula_space.context.truth_source.label().to_string(),
                    packet_kind_summary: formula_space.context.packet_kind.clone(),
                    is_active: &formula_space.formula_space_id == active_formula_space_id,
                    is_pinned: state
                        .workspace_shell
                        .pinned_formula_space_ids
                        .contains(&formula_space.formula_space_id),
                })
        })
        .collect();

    let workspace_summary = format!(
        "{} open · {} pinned",
        state.workspace_shell.open_formula_space_order.len(),
        state.workspace_shell.pinned_formula_space_ids.len()
    );

    let context_facts = vec![
        ShellChromeFactViewModel {
            label: "Truth",
            value: active_formula_space
                .context
                .truth_source
                .label()
                .to_string(),
            tone: "accent",
        },
        ShellChromeFactViewModel {
            label: "Host",
            value: active_formula_space.context.host_profile.clone(),
            tone: "default",
        },
        ShellChromeFactViewModel {
            label: "Packet",
            value: active_formula_space.context.packet_kind.clone(),
            tone: "default",
        },
    ];

    let mut footer_facts = vec![
        ShellChromeFactViewModel {
            label: "Capability",
            value: active_formula_space.context.capability_floor.clone(),
            tone: "default",
        },
        ShellChromeFactViewModel {
            label: "Modes",
            value: active_formula_space.context.mode_availability.clone(),
            tone: "default",
        },
        ShellChromeFactViewModel {
            label: "Workspace",
            value: workspace_summary.clone(),
            tone: "muted",
        },
    ];
    if let Some(blocked_reason) = active_formula_space.context.blocked_reason.as_ref() {
        footer_facts.push(ShellChromeFactViewModel {
            label: "Blocked",
            value: blocked_reason.clone(),
            tone: "warning",
        });
    } else if let Some(trace_summary) = active_formula_space.context.trace_summary.as_ref() {
        footer_facts.push(ShellChromeFactViewModel {
            label: "Trace",
            value: trace_summary.clone(),
            tone: "muted",
        });
    }

    Some(ShellFrameViewModel {
        active_formula_space_label: active_formula_space.context.scenario_label.clone(),
        active_mode_label,
        active_truth_source_label: active_formula_space
            .context
            .truth_source
            .label()
            .to_string(),
        active_host_profile_summary: active_formula_space.context.host_profile.clone(),
        active_packet_kind_summary: active_formula_space.context.packet_kind.clone(),
        active_capability_floor_summary: active_formula_space.context.capability_floor.clone(),
        context_facts,
        footer_facts,
        workspace_summary,
        mode_tabs,
        formula_spaces,
    })
}

pub fn switch_active_mode(state: &mut OneCalcHostState, next_mode: AppMode) {
    state.active_formula_space_view.active_mode = next_mode;
}

pub fn select_active_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) {
    let Some(formula_space_id) = state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .find(|id| id.as_str() == formula_space_id)
        .cloned()
    else {
        return;
    };

    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.active_formula_space_view.selected_formula_space_id = Some(formula_space_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::services::programmatic_testing::{
        ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
    };
    use crate::services::retained_artifacts::import_programmatic_artifact;
    use crate::services::retained_artifacts::RetainedArtifactImportRequest;
    use crate::state::{FormulaSpaceCollectionState, FormulaSpaceState, OneCalcHostState};
    use crate::test_support::sample_editor_document;

    #[test]
    fn active_formula_space_prefers_workspace_shell_selection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let active = active_formula_space(&state).expect("active space should exist");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2)");
    }

    #[test]
    fn build_active_mode_projection_routes_to_explore_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Explore(view_model) => {
                assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
            }
            other => panic!("expected explore projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_routes_to_inspect_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Inspect(view_model) => {
                assert_eq!(view_model.formula_walk_nodes.len(), 1);
                assert!(view_model.retained_artifact_context.is_none());
            }
            other => panic!("expected inspect projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_returns_none_without_active_space() {
        let state = OneCalcHostState {
            formula_spaces: FormulaSpaceCollectionState::default(),
            ..Default::default()
        };

        assert!(build_active_mode_projection(&state).is_none());
    }

    #[test]
    fn shell_frame_view_model_tracks_active_space_and_mode_tabs() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let frame = build_shell_frame_view_model(&state).expect("frame should exist");
        assert_eq!(frame.active_formula_space_label, "space-1");
        assert_eq!(frame.active_mode_label, "Inspect");
        assert_eq!(frame.active_truth_source_label, "local-fallback");
        assert_eq!(frame.workspace_summary, "1 open · 0 pinned");
        assert_eq!(frame.formula_spaces.len(), 1);
        assert!(frame.formula_spaces[0].is_active);
        assert!(!frame.formula_spaces[0].is_pinned);
        assert_eq!(frame.formula_spaces[0].truth_source_label, "local-fallback");
        assert_eq!(frame.context_facts.len(), 3);
        assert!(frame
            .footer_facts
            .iter()
            .any(|fact| fact.label == "Capability" && fact.value == "pending"));
        assert!(frame
            .mode_tabs
            .iter()
            .any(|tab| tab.mode == AppMode::Inspect && tab.is_active));
    }

    #[test]
    fn switch_active_mode_updates_state() {
        let mut state = OneCalcHostState::default();
        assert_eq!(
            state.active_formula_space_view.active_mode,
            AppMode::Explore
        );

        switch_active_mode(&mut state, AppMode::Inspect);

        assert_eq!(
            state.active_formula_space_view.active_mode,
            AppMode::Inspect
        );
    }

    #[test]
    fn select_active_formula_space_updates_shell_selection() {
        let first_id = FormulaSpaceId::new("space-1");
        let second_id = FormulaSpaceId::new("space-2");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(first_id.clone());
        state.workspace_shell.open_formula_space_order = vec![first_id.clone(), second_id.clone()];
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(first_id, "=SUM(1,2)"));
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(second_id.clone(), "=SEQUENCE(2,2)"));

        select_active_formula_space(&mut state, "space-2");

        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&second_id)
        );
        assert_eq!(
            state
                .active_formula_space_view
                .selected_formula_space_id
                .as_ref(),
            Some(&second_id)
        );
    }

    #[test]
    fn build_active_mode_projection_routes_to_workbench_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(view_model.outcome_summary.as_deref(), Some("Number"));
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_routes_open_retained_artifact_into_workbench() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id,
                catalog_entry:
                    crate::services::programmatic_testing::ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Blocked,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );
        state.retained_artifacts.open_artifact_id = Some("artifact-1".to_string());

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(view_model.outcome_summary.as_deref(), Some("Blocked"));
                assert_eq!(
                    view_model.retained_discrepancy_summary.as_deref(),
                    Some("excel lane unavailable")
                );
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_updates_open_catalog_item_when_active_artifact_changes() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        state.formula_spaces.insert(formula_space);

        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: formula_space_id.clone(),
                catalog_entry:
                    crate::services::programmatic_testing::ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Mismatched,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                discrepancy_summary: Some("dna=1 excel=2".to_string()),
            },
        );
        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id,
                catalog_entry:
                    crate::services::programmatic_testing::ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-2".to_string(),
                        case_id: "case-2".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Blocked,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        state.retained_artifacts.open_artifact_id = Some("artifact-1".to_string());
        let first_projection =
            build_active_mode_projection(&state).expect("first projection should be available");

        match first_projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(
                    view_model.retained_artifact_id.as_deref(),
                    Some("artifact-1")
                );
                let first_item = view_model
                    .retained_catalog_items
                    .iter()
                    .find(|item| item.artifact_id == "artifact-1")
                    .expect("artifact-1 catalog item");
                let second_item = view_model
                    .retained_catalog_items
                    .iter()
                    .find(|item| item.artifact_id == "artifact-2")
                    .expect("artifact-2 catalog item");
                assert!(first_item.is_open);
                assert!(!second_item.is_open);
                assert_eq!(
                    view_model.retained_discrepancy_summary.as_deref(),
                    Some("dna=1 excel=2")
                );
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }

        state.retained_artifacts.open_artifact_id = Some("artifact-2".to_string());
        let second_projection =
            build_active_mode_projection(&state).expect("second projection should be available");

        match second_projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(
                    view_model.retained_artifact_id.as_deref(),
                    Some("artifact-2")
                );
                let first_item = view_model
                    .retained_catalog_items
                    .iter()
                    .find(|item| item.artifact_id == "artifact-1")
                    .expect("artifact-1 catalog item");
                let second_item = view_model
                    .retained_catalog_items
                    .iter()
                    .find(|item| item.artifact_id == "artifact-2")
                    .expect("artifact-2 catalog item");
                assert!(!first_item.is_open);
                assert!(second_item.is_open);
                assert_eq!(view_model.outcome_summary.as_deref(), Some("Blocked"));
                assert_eq!(
                    view_model.retained_discrepancy_summary.as_deref(),
                    Some("excel lane unavailable")
                );
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_routes_open_retained_artifact_into_inspect_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id,
                catalog_entry:
                    crate::services::programmatic_testing::ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Blocked,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );
        state.retained_artifacts.open_artifact_id = Some("artifact-1".to_string());

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Inspect(view_model) => {
                let context = view_model
                    .retained_artifact_context
                    .expect("retained context");
                assert_eq!(context.artifact_id, "artifact-1");
                assert_eq!(context.comparison_status, "blocked");
                assert_eq!(
                    context.discrepancy_summary.as_deref(),
                    Some("excel lane unavailable")
                );
            }
            other => panic!("expected inspect projection, got {other:?}"),
        }
    }
}
