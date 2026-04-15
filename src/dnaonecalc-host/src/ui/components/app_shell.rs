use std::sync::Arc;

use crate::adapters::oxfml::OxfmlEditorBridge;
use leptos::prelude::*;

use crate::app::reducer::{
    apply_editor_command_to_active_formula_space, apply_editor_input_to_active_formula_space,
    apply_editor_overlay_measurement_to_active_formula_space,
    import_manual_retained_artifact_into_active_formula_space,
    import_verification_bundle_report_into_workspace, open_retained_artifact_from_catalog,
    open_retained_artifact_from_catalog_in_inspect,
};
use crate::services::live_edit::{apply_live_editor_command, apply_live_editor_input};
use crate::services::shell_composition::{
    build_active_mode_projection, build_shell_frame_view_model, select_active_formula_space,
    switch_active_mode, ActiveModeProjection,
};
use crate::state::OneCalcHostState;
use crate::ui::components::explore_shell::ExploreShell;
use crate::ui::components::inspect_shell::InspectShell;
use crate::ui::components::shell_frame::ShellFrame;
use crate::ui::components::workbench_shell::WorkbenchShell;
use crate::ui::design_tokens::theme::ThemeStyleTag;
use crate::ui::editor::commands::{EditorCommand, EditorInputEvent};
use crate::ui::editor::geometry::EditorOverlayMeasurementEvent;
use crate::ui::panels::explore::{build_explore_editor_cluster, build_explore_result_cluster};
use crate::ui::panels::inspect::{build_inspect_summary_cluster, build_inspect_walk_cluster};
use crate::ui::panels::workbench::{
    build_workbench_actions_cluster, build_workbench_catalog_cluster,
    build_workbench_evidence_cluster, build_workbench_lineage_cluster,
    build_workbench_outcome_cluster,
};

#[component]
pub fn OneCalcShellApp(
    initial_state: OneCalcHostState,
    #[prop(default = None)] editor_bridge: Option<Arc<dyn OxfmlEditorBridge + Send + Sync>>,
) -> impl IntoView {
    let state = RwSignal::new(initial_state);

    let on_mode_select = Callback::new(move |next_mode| {
        state.update(|state| switch_active_mode(state, next_mode));
    });
    let on_formula_space_select = Callback::new(move |formula_space_id: String| {
        state.update(|state| select_active_formula_space(state, &formula_space_id));
    });
    let on_new_formula_space = Callback::new(move |_: ()| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::new_formula_space(state);
        });
    });
    let on_close_formula_space = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::close_formula_space(state, &formula_space_id);
        });
    });
    let on_toggle_pin_formula_space = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::toggle_pin_formula_space(state, &formula_space_id);
        });
    });
    let on_configure_toggle = Callback::new(move |_: ()| {
        state.update(|state| {
            let _ = apply_editor_command_to_active_formula_space(
                state,
                EditorCommand::ToggleConfigureDrawer,
            );
        });
    });
    let editor_bridge_for_input = editor_bridge.clone();
    let on_editor_input = Callback::new(move |event: EditorInputEvent| {
        state.update(|state| {
            if let Some(bridge) = editor_bridge_for_input.as_ref() {
                let _ = apply_live_editor_input(bridge.as_ref(), state, event);
            } else {
                let _ = apply_editor_input_to_active_formula_space(state, event);
            }
        });
    });
    let editor_bridge_for_command = editor_bridge.clone();
    let on_editor_command = Callback::new(move |command: EditorCommand| {
        state.update(|state| {
            if let Some(bridge) = editor_bridge_for_command.as_ref() {
                let _ = apply_live_editor_command(bridge.as_ref(), state, command);
            } else {
                let _ = apply_editor_command_to_active_formula_space(state, command);
            }
        });
    });
    let on_editor_overlay_measurement =
        Callback::new(move |measurement_event: EditorOverlayMeasurementEvent| {
            state.update(|state| {
                let _ = apply_editor_overlay_measurement_to_active_formula_space(
                    state,
                    measurement_event,
                );
            });
        });
    let on_open_retained_artifact = Callback::new(move |artifact_id: String| {
        state.update(|state| {
            let _ = open_retained_artifact_from_catalog(state, &artifact_id);
        });
    });
    let on_open_retained_artifact_in_inspect = Callback::new(move |artifact_id: String| {
        state.update(|state| {
            let _ = open_retained_artifact_from_catalog_in_inspect(state, &artifact_id);
        });
    });
    let on_import_retained_artifact = Callback::new(
        move |request: crate::services::retained_artifacts::ManualRetainedArtifactImportRequest| {
            state.update(|state| {
                let _ = import_manual_retained_artifact_into_active_formula_space(state, request);
            });
        },
    );
    let on_import_verification_bundle = Callback::new(
        move |request: crate::services::retained_artifacts::VerificationBundleImportRequest| {
            state.update(|state| {
                let _ = import_verification_bundle_report_into_workspace(state, request);
            });
        },
    );

    view! {
        <div class="onecalc-app" data-host-app="onecalc">
            <ThemeStyleTag />
            {move || {
                let current_state = state.get();
                match (
                    build_shell_frame_view_model(&current_state),
                    build_active_mode_projection(&current_state),
                ) {
                    (Some(frame), Some(ActiveModeProjection::Explore(view_model))) => {
                        view! {
                            <ShellFrame
                                frame=frame
                                on_mode_select=Some(on_mode_select)
                                on_formula_space_select=Some(on_formula_space_select)
                                on_new_formula_space=Some(on_new_formula_space)
                                on_close_formula_space=Some(on_close_formula_space)
                                on_toggle_pin_formula_space=Some(on_toggle_pin_formula_space)
                                on_configure_toggle=Some(on_configure_toggle)
                                configure_drawer_open=current_state.global_ui_chrome.configure_drawer_open
                            >
                                <ExploreShell
                                    editor=build_explore_editor_cluster(&view_model)
                                    result=build_explore_result_cluster(&view_model)
                                    on_input_event=Some(on_editor_input)
                                    on_command=Some(on_editor_command)
                                    on_overlay_measurement=Some(on_editor_overlay_measurement)
                                />
                            </ShellFrame>
                        }
                        .into_any()
                    }
                    (Some(frame), Some(ActiveModeProjection::Inspect(view_model))) => {
                        view! {
                            <ShellFrame
                                frame=frame
                                on_mode_select=Some(on_mode_select)
                                on_formula_space_select=Some(on_formula_space_select)
                                on_new_formula_space=Some(on_new_formula_space)
                                on_close_formula_space=Some(on_close_formula_space)
                                on_toggle_pin_formula_space=Some(on_toggle_pin_formula_space)
                                on_configure_toggle=Some(on_configure_toggle)
                                configure_drawer_open=current_state.global_ui_chrome.configure_drawer_open
                            >
                                <InspectShell
                                    walk=build_inspect_walk_cluster(&view_model)
                                    summary=build_inspect_summary_cluster(&view_model)
                                />
                            </ShellFrame>
                        }
                        .into_any()
                    }
                    (Some(frame), Some(ActiveModeProjection::Workbench(view_model))) => {
                        view! {
                            <ShellFrame
                                frame=frame
                                on_mode_select=Some(on_mode_select)
                                on_formula_space_select=Some(on_formula_space_select)
                                on_new_formula_space=Some(on_new_formula_space)
                                on_close_formula_space=Some(on_close_formula_space)
                                on_toggle_pin_formula_space=Some(on_toggle_pin_formula_space)
                                on_configure_toggle=Some(on_configure_toggle)
                                configure_drawer_open=current_state.global_ui_chrome.configure_drawer_open
                            >
                                <WorkbenchShell
                                    outcome=build_workbench_outcome_cluster(&view_model)
                                    evidence=build_workbench_evidence_cluster(&view_model)
                                    lineage=build_workbench_lineage_cluster(&view_model)
                                    actions=build_workbench_actions_cluster(&view_model)
                                    catalog=build_workbench_catalog_cluster(&view_model)
                                    on_open_retained_artifact=Some(on_open_retained_artifact)
                                    on_open_retained_artifact_in_inspect=Some(on_open_retained_artifact_in_inspect)
                                    on_import_retained_artifact=Some(on_import_retained_artifact)
                                    on_import_verification_bundle=Some(on_import_verification_bundle)
                                />
                            </ShellFrame>
                        }
                        .into_any()
                    }
                    _ => view! {
                        <section class="onecalc-shell-frame__empty">
                            <h1>"DNA OneCalc"</h1>
                            <div>"No active formula space"</div>
                        </section>
                    }
                    .into_any(),
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::{AppMode, FormulaSpaceState};
    use crate::test_support::sample_editor_document;

    #[test]
    fn app_shell_renders_explore_projection_in_shared_frame() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        state.formula_spaces.insert(formula_space);

        let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

        assert!(html.contains("data-theme=\"onecalc-theme\""));
        assert!(html.contains("DNA OneCalc"));
        assert!(html.contains("data-mode=\"Explore\""));
        assert!(html.contains("data-role=\"shell-frame-configure-toggle\""));
        assert!(html.contains("data-component=\"formula-editor-surface\""));
        // Explore layout discipline: no Formula Explorer hero, no overview deck.
        assert!(!html.contains("Formula Explorer"));
        assert!(!html.contains("data-role=\"explore-overview-deck\""));
    }

    #[test]
    fn app_shell_renders_inspect_projection_in_shared_frame() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        state.formula_spaces.insert(formula_space);

        let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

        assert!(html.contains("data-theme=\"onecalc-theme\""));
        assert!(html.contains("DNA OneCalc"));
        assert!(html.contains("Semantic Inspect"));
        assert!(html.contains("data-mode=\"Inspect\""));
    }

    #[test]
    fn app_shell_renders_workbench_projection_in_shared_frame() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        state.formula_spaces.insert(formula_space);

        let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

        assert!(html.contains("data-theme=\"onecalc-theme\""));
        assert!(html.contains("DNA OneCalc"));
        assert!(html.contains("Twin Oracle Workbench"));
        assert!(html.contains("data-mode=\"Workbench\""));
    }
}
