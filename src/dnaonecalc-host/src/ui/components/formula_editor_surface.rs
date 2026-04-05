use leptos::prelude::*;

use crate::ui::editor::commands::{keydown_to_command, EditorCommand, EditorInputEvent};
use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};
use crate::ui::panels::explore::ExploreEditorClusterViewModel;

#[component]
pub fn FormulaEditorSurface(
    editor: ExploreEditorClusterViewModel,
    #[prop(default = None)] on_input_event: Option<Callback<EditorInputEvent>>,
    #[prop(default = None)] on_command: Option<Callback<EditorCommand>>,
) -> impl IntoView {
    let diagnostics_text = if editor.diagnostics.is_empty() {
        "No diagnostics".to_string()
    } else {
        editor
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.diagnostic_id, diagnostic.message))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let editor_state = editor.editor_surface_state.clone();
    let selection_start = editor_state.selection.start();
    let selection_end = editor_state.selection.end();
    let selection_label = if editor_state.selection.is_collapsed() {
        "collapsed"
    } else {
        "range"
    };

    view! {
        <section class="onecalc-formula-editor-surface" data-component="formula-editor-surface">
            <header class="onecalc-formula-editor-surface__toolbar">
                <span>"Chars: " {editor.raw_entered_cell_text.chars().count()}</span>
                <span>"Tokens: " {editor.syntax_runs.len()}</span>
                <span>"Diagnostics: " {editor.diagnostics.len()}</span>
            </header>

            <div class="onecalc-formula-editor-surface__body">
                <div class="onecalc-formula-editor-surface__native-input-layer" data-role="native-input-layer">
                    <textarea
                        class="onecalc-formula-editor-surface__textarea"
                        data-role="editor-input"
                        prop:value=editor.raw_entered_cell_text.clone()
                        on:input=move |ev| {
                            if let Some(callback) = on_input_event.as_ref() {
                                callback.run(EditorInputEvent {
                                    text: event_target_value(&ev),
                                });
                            }
                        }
                        on:keydown=move |ev| {
                            if let Some(command) = keydown_to_command(&ev.key(), ev.shift_key()) {
                                if let Some(command_callback) = on_command.as_ref() {
                                    command_callback.run(command);
                                }
                            }
                        }
                    />
                </div>
                <div class="onecalc-formula-editor-surface__overlay-layer" data-role="overlay-layer">
                    <div class="onecalc-formula-editor-surface__syntax-layer" data-role="syntax-layer">
                        {editor
                            .syntax_runs
                            .iter()
                            .map(render_syntax_run)
                            .collect_view()}
                    </div>
                    <div class="onecalc-formula-editor-surface__diagnostic-markers" data-role="diagnostic-markers">
                        {editor
                            .diagnostics
                            .iter()
                            .map(|diagnostic| {
                                view! {
                                    <span
                                        class="onecalc-formula-editor-surface__diagnostic-marker"
                                        data-diagnostic-id=diagnostic.diagnostic_id.clone()
                                        data-span-start=diagnostic.span_start
                                        data-span-len=diagnostic.span_len
                                    >
                                        {diagnostic.message.clone()}
                                    </span>
                                }
                            })
                            .collect_view()}
                    </div>
                    <div
                        class="onecalc-formula-editor-surface__selection-indicator"
                        data-role="selection-indicator"
                        data-selection-start=selection_start
                        data-selection-end=selection_end
                        data-selection-kind=selection_label
                    >
                        "Selection: "
                        {selection_start}
                        ".."
                        {selection_end}
                    </div>
                    <div
                        class="onecalc-formula-editor-surface__caret-indicator"
                        data-role="caret-indicator"
                        data-caret-offset=editor_state.caret.offset
                    >
                        "Caret: "
                        {editor_state.caret.offset}
                    </div>
                    <div
                        class="onecalc-formula-editor-surface__scroll-indicator"
                        data-role="scroll-indicator"
                        data-first-visible-line=editor_state.scroll_window.first_visible_line
                        data-visible-lines=editor_state.scroll_window.visible_line_count
                    >
                        "Scroll: "
                        {editor_state.scroll_window.first_visible_line}
                        "/"
                        {editor_state.scroll_window.visible_line_count}
                    </div>
                </div>
            </div>

            <footer class="onecalc-formula-editor-surface__footer">
                <div class="onecalc-formula-editor-surface__editor-state">
                    <span>"Green tree: " {editor.green_tree_key.unwrap_or_else(|| "none".to_string())}</span>
                    <span>"Reused: " {if editor.reused_green_tree { "yes" } else { "no" }}</span>
                </div>
                <div class="onecalc-formula-editor-surface__diagnostics" data-role="diagnostics">
                    {diagnostics_text}
                </div>
            </footer>
        </section>
    }
}

fn render_syntax_run(run: &SyntaxRun) -> AnyView {
    let token_role = match run.role {
        SyntaxTokenRole::Operator => "operator",
        SyntaxTokenRole::Function => "function",
        SyntaxTokenRole::Number => "number",
        SyntaxTokenRole::Delimiter => "delimiter",
        SyntaxTokenRole::Identifier => "identifier",
        SyntaxTokenRole::Text => "text",
    };

    view! {
        <span class="onecalc-token" data-token-role=token_role>
            {run.text.clone()}
        </span>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::explore_mode::ExploreDiagnosticView;
    use crate::ui::editor::state::{
        EditorCaret, EditorScrollWindow, EditorSelection, EditorSurfaceState,
    };

    #[test]
    fn formula_editor_surface_renders_textarea_and_token_layer() {
        let html = view! {
            <FormulaEditorSurface
                editor=ExploreEditorClusterViewModel {
                    raw_entered_cell_text: "=SUM(1,2)".to_string(),
                    syntax_runs: vec![
                        SyntaxRun {
                            text: "=".to_string(),
                            span_start: 0,
                            span_len: 1,
                            role: SyntaxTokenRole::Operator,
                        },
                        SyntaxRun {
                            text: "SUM".to_string(),
                            span_start: 1,
                            span_len: 3,
                            role: SyntaxTokenRole::Function,
                        },
                    ],
                    diagnostics: vec![ExploreDiagnosticView {
                        diagnostic_id: "diag-1".to_string(),
                        message: "sample".to_string(),
                        span_start: 1,
                        span_len: 3,
                    }],
                    completion_count: 2,
                    has_signature_help: true,
                    function_help_lookup_key: Some("SUM".to_string()),
                    green_tree_key: Some("green-1".to_string()),
                    reused_green_tree: false,
                    editor_surface_state: EditorSurfaceState {
                        caret: EditorCaret { offset: 4 },
                        selection: EditorSelection { anchor: 1, focus: 4 },
                        scroll_window: EditorScrollWindow {
                            first_visible_line: 0,
                            visible_line_count: 6,
                        },
                    },
                }
            />
        }
        .to_html();

        assert!(html.contains("data-component=\"formula-editor-surface\""));
        assert!(html.contains("data-role=\"editor-input\""));
        assert!(html.contains("data-role=\"native-input-layer\""));
        assert!(html.contains("data-role=\"overlay-layer\""));
        assert!(html.contains("data-role=\"syntax-layer\""));
        assert!(html.contains("data-role=\"diagnostic-markers\""));
        assert!(html.contains("data-diagnostic-id=\"diag-1\""));
        assert!(html.contains("data-token-role=\"function\""));
        assert!(html.contains("data-role=\"caret-indicator\""));
        assert!(html.contains("data-role=\"selection-indicator\""));
        assert!(html.contains("data-selection-kind=\"range\""));
    }
}
