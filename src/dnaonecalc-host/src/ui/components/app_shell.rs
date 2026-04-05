use std::sync::Arc;

use crate::adapters::oxfml::OxfmlEditorBridge;
use leptos::prelude::*;

use crate::app::reducer::{
    apply_editor_command_to_active_formula_space, apply_editor_input_to_active_formula_space,
    apply_editor_overlay_measurement_to_active_formula_space,
    open_retained_artifact_from_catalog,
};
use crate::services::live_edit::{apply_live_editor_command, apply_live_editor_input};
use crate::services::shell_composition::{
    build_active_mode_projection, build_shell_frame_view_model, switch_active_mode,
    ActiveModeProjection,
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
    let on_editor_overlay_measurement = Callback::new(move |measurement_event: EditorOverlayMeasurementEvent| {
        state.update(|state| {
            let _ = apply_editor_overlay_measurement_to_active_formula_space(state, measurement_event);
        });
    });
    let on_open_retained_artifact = Callback::new(move |artifact_id: String| {
        state.update(|state| {
            let _ = open_retained_artifact_from_catalog(state, &artifact_id);
        });
    });

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
                            <ShellFrame frame=frame on_mode_select=Some(on_mode_select)>
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
                            <ShellFrame frame=frame on_mode_select=Some(on_mode_select)>
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
                            <ShellFrame frame=frame on_mode_select=Some(on_mode_select)>
                                <WorkbenchShell
                                    outcome=build_workbench_outcome_cluster(&view_model)
                                    evidence=build_workbench_evidence_cluster(&view_model)
                                    lineage=build_workbench_lineage_cluster(&view_model)
                                    actions=build_workbench_actions_cluster(&view_model)
                                    catalog=build_workbench_catalog_cluster(&view_model)
                                    on_open_retained_artifact=Some(on_open_retained_artifact)
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
        assert!(html.contains("Formula Explorer"));
        assert!(html.contains("data-mode=\"Explore\""));
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
