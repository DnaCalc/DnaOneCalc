use leptos::prelude::*;

use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};
use crate::ui::panels::explore::ExploreEditorClusterViewModel;

#[component]
pub fn FormulaEditorSurface(editor: ExploreEditorClusterViewModel) -> impl IntoView {
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

    view! {
        <section class="onecalc-formula-editor-surface" data-component="formula-editor-surface">
            <header class="onecalc-formula-editor-surface__toolbar">
                <span>"Chars: " {editor.raw_entered_cell_text.chars().count()}</span>
                <span>"Tokens: " {editor.syntax_runs.len()}</span>
                <span>"Diagnostics: " {editor.diagnostics.len()}</span>
            </header>

            <div class="onecalc-formula-editor-surface__body">
                <textarea
                    class="onecalc-formula-editor-surface__textarea"
                    data-role="editor-input"
                    prop:value=editor.raw_entered_cell_text.clone()
                />
                <div class="onecalc-formula-editor-surface__syntax-layer" data-role="syntax-layer">
                    {editor
                        .syntax_runs
                        .iter()
                        .map(render_syntax_run)
                        .collect_view()}
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
                }
            />
        }
        .to_html();

        assert!(html.contains("data-component=\"formula-editor-surface\""));
        assert!(html.contains("data-role=\"editor-input\""));
        assert!(html.contains("data-role=\"syntax-layer\""));
        assert!(html.contains("data-token-role=\"function\""));
    }
}
