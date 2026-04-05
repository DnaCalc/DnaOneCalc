use leptos::prelude::*;

use crate::ui::components::formula_editor_surface::FormulaEditorSurface;
use crate::ui::editor::commands::{EditorCommand, EditorInputEvent};
use crate::ui::panels::explore::{ExploreEditorClusterViewModel, ExploreResultClusterViewModel};

#[component]
fn ExploreEditorPanel(
    editor: ExploreEditorClusterViewModel,
    on_input_event: Option<Callback<EditorInputEvent>>,
    on_command: Option<Callback<EditorCommand>>,
) -> impl IntoView {
    let function_help = editor
        .function_help_lookup_key
        .clone()
        .unwrap_or_else(|| "None".to_string());

    view! {
        <section class="onecalc-explore-shell__editor-panel" data-panel="explore-editor">
            <h2>"Editor"</h2>
            <FormulaEditorSurface
                editor=editor.clone()
                on_input_event=on_input_event
                on_command=on_command
            />
            <div class="onecalc-explore-shell__help-hint">
                "Function help target: "
                {function_help}
            </div>
        </section>
    }
}

#[component]
fn ExploreResultPanel(result: ExploreResultClusterViewModel) -> impl IntoView {
    let effective_display = result
        .effective_display_summary
        .clone()
        .unwrap_or_else(|| "Unavailable".to_string());
    let evaluation_summary = result
        .latest_evaluation_summary
        .clone()
        .unwrap_or_else(|| "Unavailable".to_string());

    view! {
        <section class="onecalc-explore-shell__result-panel" data-panel="explore-result">
            <h2>"Result"</h2>
            <div>"Effective display: " {effective_display}</div>
            <div>"Evaluation summary: " {evaluation_summary}</div>
        </section>
    }
}

#[component]
fn ExploreHelpPanel(editor: ExploreEditorClusterViewModel) -> impl IntoView {
    let function_help = editor
        .function_help_lookup_key
        .clone()
        .unwrap_or_else(|| "None".to_string());
    let help_summary = if editor.has_signature_help {
        "Signature help available"
    } else {
        "Signature help unavailable"
    };

    view! {
        <section class="onecalc-explore-shell__help-panel" data-panel="explore-help">
            <h2>"Help"</h2>
            <div>"Function target: " {function_help}</div>
            <div>{help_summary}</div>
            <div>"Completion entries: " {editor.completion_count}</div>
        </section>
    }
}

#[component]
pub fn ExploreShell(
    editor: ExploreEditorClusterViewModel,
    result: ExploreResultClusterViewModel,
    #[prop(default = None)] on_input_event: Option<Callback<EditorInputEvent>>,
    #[prop(default = None)] on_command: Option<Callback<EditorCommand>>,
) -> impl IntoView {
    view! {
        <section class="onecalc-explore-shell" data-screen="explore">
            <header class="onecalc-explore-shell__header">
                <h1>"Formula Explorer"</h1>
            </header>

            <div class="onecalc-explore-shell__body">
                <ExploreEditorPanel
                    editor=editor.clone()
                    on_input_event=on_input_event
                    on_command=on_command
                />
                <ExploreResultPanel result=result />
                <ExploreHelpPanel editor=editor />
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::explore_mode::{
        ExploreCompletionItemView, ExploreDiagnosticView, ExploreSignatureHelpView,
        ExploreViewModel,
    };
    use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};
    use crate::ui::panels::explore::{
        build_explore_editor_cluster, build_explore_result_cluster,
    };

    #[test]
    fn explore_shell_renders_editor_and_result_content() {
        let view_model = ExploreViewModel {
            raw_entered_cell_text: "=SUM(1,2)".to_string(),
            editor_surface_state: crate::ui::editor::state::EditorSurfaceState::for_text(
                "=SUM(1,2)",
            ),
            syntax_runs: vec![SyntaxRun {
                text: "SUM".to_string(),
                span_start: 1,
                span_len: 3,
                role: SyntaxTokenRole::Function,
            }],
            diagnostics: vec![ExploreDiagnosticView {
                diagnostic_id: "diag-1".to_string(),
                message: "sample".to_string(),
                span_start: 1,
                span_len: 3,
            }],
            completion_count: 2,
            completion_items: vec![ExploreCompletionItemView {
                proposal_id: "proposal-1".to_string(),
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
            }],
            has_signature_help: true,
            signature_help: Some(ExploreSignatureHelpView {
                callee_text: "SUM".to_string(),
                active_argument_index: 1,
            }),
            function_help_lookup_key: Some("SUM".to_string()),
            effective_display_summary: Some("3".to_string()),
            latest_evaluation_summary: Some("Number".to_string()),
            green_tree_key: Some("green-1".to_string()),
            reused_green_tree: true,
        };

        let html = view! {
            <ExploreShell
                editor=build_explore_editor_cluster(&view_model)
                result=build_explore_result_cluster(&view_model)
            />
        }
        .to_html();

        assert!(html.contains("Formula Explorer"));
        assert!(html.contains("data-panel=\"explore-editor\""));
        assert!(html.contains("data-component=\"formula-editor-surface\""));
        assert!(html.contains("data-role=\"editor-input\""));
        assert!(html.contains("data-token-role=\"function\""));
        assert!(html.contains("data-panel=\"explore-help\""));
        assert!(html.contains(">3<"));
        assert!(html.contains("Function target: "));
        assert!(html.contains("SUM"));
        assert!(html.contains("Completion entries: "));
    }
}
