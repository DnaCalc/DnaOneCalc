use leptos::prelude::*;

use crate::ui::panels::explore::{
    ExploreEditorClusterViewModel, ExploreResultClusterViewModel,
};

#[component]
pub fn ExploreShell(
    editor: ExploreEditorClusterViewModel,
    result: ExploreResultClusterViewModel,
) -> impl IntoView {
    let function_help = editor
        .function_help_lookup_key
        .clone()
        .unwrap_or_else(|| "None".to_string());
    let effective_display = result
        .effective_display_summary
        .clone()
        .unwrap_or_else(|| "Unavailable".to_string());
    let evaluation_summary = result
        .latest_evaluation_summary
        .clone()
        .unwrap_or_else(|| "Unavailable".to_string());
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
        <section class="onecalc-explore-shell">
            <header class="onecalc-explore-shell__header">
                <h1>"Formula Explorer"</h1>
                <div class="onecalc-explore-shell__meta">
                    <span>"Green tree: " {editor.green_tree_key.unwrap_or_else(|| "none".to_string())}</span>
                    <span>"Reused: " {if editor.reused_green_tree { "yes" } else { "no" }}</span>
                </div>
            </header>

            <div class="onecalc-explore-shell__body">
                <section class="onecalc-explore-shell__editor-cluster">
                    <h2>"Editor"</h2>
                    <pre class="onecalc-explore-shell__editor-text">{editor.raw_entered_cell_text}</pre>
                    <div class="onecalc-explore-shell__editor-meta">
                        <span>"Completions: " {editor.completion_count}</span>
                        <span>"Signature help: " {if editor.has_signature_help { "on" } else { "off" }}</span>
                        <span>"Function help: " {function_help}</span>
                    </div>
                    <div class="onecalc-explore-shell__diagnostics">{diagnostics_text}</div>
                </section>

                <section class="onecalc-explore-shell__result-cluster">
                    <h2>"Result"</h2>
                    <div>"Effective display: " {effective_display}</div>
                    <div>"Evaluation summary: " {evaluation_summary}</div>
                </section>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::explore_mode::{ExploreDiagnosticView, ExploreViewModel};
    use crate::ui::editor::render_projection::SyntaxRun;
    use crate::ui::panels::explore::{
        build_explore_editor_cluster, build_explore_result_cluster,
    };

    #[test]
    fn explore_shell_renders_editor_and_result_content() {
        let view_model = ExploreViewModel {
            raw_entered_cell_text: "=SUM(1,2)".to_string(),
            syntax_runs: vec![SyntaxRun {
                text: "SUM".to_string(),
                span_start: 1,
                span_len: 3,
            }],
            diagnostics: vec![ExploreDiagnosticView {
                diagnostic_id: "diag-1".to_string(),
                message: "sample".to_string(),
                span_start: 1,
                span_len: 3,
            }],
            completion_count: 2,
            has_signature_help: true,
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
        assert!(html.contains("=SUM(1,2)"));
        assert!(html.contains("Effective display: "));
        assert!(html.contains(">3<"));
        assert!(html.contains("Function help: "));
        assert!(html.contains("SUM"));
    }
}
