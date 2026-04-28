//! WS-14 Pre-MVP home shell.
//!
//! Single-component shell that replaces (eventually retires) the legacy
//! `OneCalcShellApp` + mode shells. The pre-MVP slice mounts only a
//! formula caption + native `<textarea>` + result block + status foot,
//! driven through the existing `LiveOxfmlBridge`.
//!
//! Subsequent WS-14 phases grow this file into the progressive-disclosure
//! home (drill-downs, scenario breadcrumb, compare entry, command palette,
//! …). The signature, props, and bridge plumbing established here remain
//! stable across those phases.
//!
//! References:
//! * `docs/WS14_PRE_MVP_PATH.md` §4 — eight-step slice
//! * `docs/APP_UX_REALIZATION.md` §4.1 — eventual editor-hero contract
//! * `docs/WS14_DESIGN_FORMULA_EDITOR.md` §4 AD-1..AD-5 — native textarea
//!   discipline this slice already follows

use std::sync::Arc;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlTextAreaElement, InputEvent as WebInputEvent};

use crate::adapters::oxfml::OxfmlEditorBridge;
use crate::app::reducer::apply_editor_input_to_active_formula_space;
use crate::services::home_shell_view_model::{
    build_home_shell_view_model, BridgeHealth, ResultKind, ResultView, StatusView,
};
use crate::services::live_edit::apply_live_editor_input;
use crate::state::OneCalcHostState;
use crate::ui::design_tokens::theme::ThemeStyleTag;
use crate::ui::editor::commands::{classify_dom_input, EditorInputEvent, EditorInputKind};

#[component]
pub fn HomeShell(
    initial_state: OneCalcHostState,
    #[prop(default = None)] editor_bridge: Option<Arc<dyn OxfmlEditorBridge + Send + Sync>>,
) -> impl IntoView {
    let state: RwSignal<OneCalcHostState> = RwSignal::new(initial_state);

    // Reactive view-model: rebuilds whenever the state signal changes.
    let view_model = Memo::new(move |_| state.with(build_home_shell_view_model));

    // Bridge dispatcher closure shared with the textarea's on:input.
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

    // Reactive readers. Each closure runs whenever the underlying signal
    // it touches changes; Leptos handles the diff.
    let textarea_value = move || {
        view_model
            .get()
            .map(|vm| vm.raw_entered_cell_text)
            .unwrap_or_default()
    };
    let has_active_formula_space = move || view_model.get().is_some();
    let result_view = move || view_model.get().map(|vm| vm.result_view);
    let status_view = move || view_model.get().map(|vm| vm.status);

    view! {
        <ThemeStyleTag />
        <div class="onecalc-home-shell">
            <header class="onecalc-home-shell__titlebar">
                <span class="onecalc-home-shell__brand">"DnaOneCalc"</span>
            </header>

            <main class="onecalc-home-shell__body">
                <Show
                    when=has_active_formula_space
                    fallback=|| view! {
                        <p class="onecalc-home-shell__no-formula-space">
                            "No active formula space."
                        </p>
                    }
                >
                    <section class="onecalc-home-shell__editor">
                        <div class="onecalc-home-shell__caption">"formula"</div>
                        <textarea
                            class="onecalc-home-shell__textarea"
                            spellcheck="false"
                            autocomplete="off"
                            aria-label="formula editor"
                            prop:value=textarea_value
                            on:input=move |ev| {
                                let textarea = event_target::<HtmlTextAreaElement>(&ev);
                                let web_input_event = ev.dyn_ref::<WebInputEvent>();
                                let event = EditorInputEvent {
                                    text: event_target_value(&ev),
                                    selection_start: textarea
                                        .selection_start()
                                        .ok()
                                        .flatten()
                                        .map(|offset| offset as usize),
                                    selection_end: textarea
                                        .selection_end()
                                        .ok()
                                        .flatten()
                                        .map(|offset| offset as usize),
                                    input_kind: web_input_event
                                        .map(|input_event| {
                                            classify_dom_input(&input_event.input_type())
                                        })
                                        .unwrap_or(EditorInputKind::Other),
                                    inserted_text: web_input_event
                                        .and_then(|input_event| input_event.data()),
                                };
                                on_editor_input.run(event);
                            }
                        ></textarea>
                    </section>

                    <section class="onecalc-home-shell__result-section">
                        <div class="onecalc-home-shell__caption">"result"</div>
                        <div
                            class="onecalc-home-shell__result-block"
                            data-kind=move || result_view().map(result_kind_attr).unwrap_or("none")
                        >
                            {move || render_result_view(result_view())}
                        </div>
                    </section>
                </Show>
            </main>

            <footer class="onecalc-home-shell__statusfoot">
                {move || render_status_foot(status_view())}
            </footer>
        </div>
    }
}

/// Render the status-foot strip: a colored dot reflecting bridge health
/// (sage when Live, amber when Stale), the literal "live-bridge" label,
/// a separator, and the current green-tree key (or "—" placeholder).
fn render_status_foot(status: Option<StatusView>) -> AnyView {
    let status = match status {
        Some(s) => s,
        None => {
            return view! {
                <span class="onecalc-home-shell__statusfoot-dot" data-health="stale"></span>
                <span>"no formula space"</span>
            }
            .into_any();
        }
    };
    let dot_health = match status.bridge_health {
        BridgeHealth::Live => "live",
        BridgeHealth::Stale => "stale",
    };
    let green_key = status
        .green_tree_key
        .as_deref()
        .map(short_green_tree_key)
        .unwrap_or_else(|| "—".to_string());
    view! {
        <span class="onecalc-home-shell__statusfoot-dot" data-health=dot_health></span>
        <span>"live-bridge"</span>
        <span class="onecalc-home-shell__statusfoot-sep">"·"</span>
        <span>{format!("green-tree {green_key}")}</span>
    }
    .into_any()
}

/// Trim a long `green:abcdef0123...` key down to a status-foot-friendly
/// `abcdef…` form, matching the WS-14 mockup convention.
fn short_green_tree_key(key: &str) -> String {
    let body = key.strip_prefix("green:").unwrap_or(key);
    if body.chars().count() <= 7 {
        return body.to_string();
    }
    let mut short = body.chars().take(6).collect::<String>();
    short.push('…');
    short
}

#[cfg(test)]
mod tests {
    use super::short_green_tree_key;

    #[test]
    fn short_green_tree_key_strips_prefix_and_trims_long_keys() {
        let key = "green:a3f91eabc1234";
        assert_eq!(short_green_tree_key(key), "a3f91e…");
    }

    #[test]
    fn short_green_tree_key_passes_short_keys_through() {
        assert_eq!(short_green_tree_key("green:abc123"), "abc123");
        assert_eq!(short_green_tree_key("abc123"), "abc123");
        assert_eq!(short_green_tree_key(""), "");
    }

    #[test]
    fn short_green_tree_key_handles_non_prefixed_keys() {
        let key = "abcdefghijklmnop";
        assert_eq!(short_green_tree_key(key), "abcdef…");
    }
}

/// Render the appropriate result-block content per `ResultView` variant.
/// All variants reach into the result-block container and supply class +
/// content; the container's CSS supplies the layout (centered, large).
fn render_result_view(view: Option<ResultView>) -> AnyView {
    match view {
        None => view! { <em class="muted">"awaiting input"</em> }.into_any(),
        Some(ResultView::Empty) => view! { <em class="muted">"awaiting input"</em> }.into_any(),
        Some(ResultView::Pending) => view! { <em class="muted">"…"</em> }.into_any(),
        Some(ResultView::Display { text, kind }) => view! {
            <span class="value" data-kind=display_kind_attr(kind)>{text}</span>
        }
        .into_any(),
        Some(ResultView::Error { code, surface_repr }) => {
            let code_for_attr = code.clone();
            view! {
                <span class="value error" data-code=code_for_attr>
                    <span class="value__code">{code}</span>
                    {surface_repr.map(|repr| view! {
                        <span class="value__surface">{repr}</span>
                    })}
                </span>
            }
            .into_any()
        }
        Some(ResultView::Array { rows, cols, label: _ }) => view! {
            <span class="value array">
                {format!("Array[{rows} × {cols}]")}
            </span>
        }
        .into_any(),
    }
}

fn result_kind_attr(view: ResultView) -> &'static str {
    match view {
        ResultView::Empty => "empty",
        ResultView::Pending => "pending",
        ResultView::Display { .. } => "display",
        ResultView::Error { .. } => "error",
        ResultView::Array { .. } => "array",
    }
}

fn display_kind_attr(kind: ResultKind) -> &'static str {
    match kind {
        ResultKind::Number => "number",
        ResultKind::Text => "text",
        ResultKind::Logical => "logical",
        ResultKind::RichValue => "rich-value",
        ResultKind::Other => "other",
    }
}
