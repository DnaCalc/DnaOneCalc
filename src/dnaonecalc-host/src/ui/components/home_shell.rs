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
use web_sys::{HtmlTextAreaElement, InputEvent as WebInputEvent, KeyboardEvent as WebKeyboardEvent};

use crate::adapters::oxfml::{FormulaTextSpan, OxfmlEditorBridge};
use crate::app::reducer::{
    accept_completion_by_proposal_id_on_active_formula_space,
    accept_selected_completion_with_suppression_on_active_formula_space,
    apply_editor_box_metrics_to_active_formula_space, apply_editor_input_to_active_formula_space,
    dismiss_completion_popup_on_active_formula_space,
    move_completion_popup_selection_on_active_formula_space,
};
use crate::services::completion_popup::CompletionAcceptance;
use crate::services::home_shell_view_model::{
    build_home_shell_view_model, BridgeHealth, CompletionPopupItemView, CompletionPopupView,
    ContextChipField, DiagnosticSquiggle, EditorMetricsChip, EntryModePill, ResultClassPill,
    ResultContextChip, ResultKind, ResultView, StatusView,
};
use crate::services::live_edit::apply_live_editor_input;
use crate::state::OneCalcHostState;
use crate::ui::design_tokens::theme::ThemeStyleTag;
use crate::ui::editor::caret_box_measurement::measure_textarea_box;
use crate::ui::editor::commands::{classify_dom_input, EditorInputEvent, EditorInputKind};
use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};

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

    // Helper: apply a CompletionAcceptance — splice the textarea
    // value, build a synthetic input event, and run it through the
    // bridge so proposals / diagnostics / metrics refresh. Used by
    // both the click-to-accept and keyboard-accept paths. Wrapped
    // in a `Callback` so multiple long-lived event listeners can
    // share it without each needing a unique clone of every captured
    // state slot.
    let editor_bridge_for_accept = editor_bridge.clone();
    let apply_acceptance: Callback<CompletionAcceptance> =
        Callback::new(move |acceptance: CompletionAcceptance| {
            let bridge = editor_bridge_for_accept.clone();
            state.update(|state| {
                if let Some(formula_space) = state
                    .workspace_shell
                    .active_formula_space_id
                    .clone()
                    .and_then(|id| state.formula_spaces.get(&id))
                {
                    let new_text = splice_textarea_value(
                        &formula_space.raw_entered_cell_text,
                        acceptance.replacement_span,
                        &acceptance.insert_text,
                    );
                    let event = EditorInputEvent {
                        text: new_text,
                        selection_start: Some(acceptance.new_caret_offset),
                        selection_end: Some(acceptance.new_caret_offset),
                        input_kind: EditorInputKind::InsertText,
                        inserted_text: Some(acceptance.insert_text),
                    };
                    if let Some(bridge) = bridge.as_ref() {
                        let _ = apply_live_editor_input(bridge.as_ref(), state, event);
                    } else {
                        let _ = apply_editor_input_to_active_formula_space(state, event);
                    }
                }
            });
        });

    // Click-to-accept closure for popup rows. Splices the proposal's
    // `insert_text` into the textarea's value at `replacement_span`,
    // moves the caret to the end of the inserted text, dispatches a
    // synthetic input event so the bridge re-runs, then transitions
    // the popup to Hidden via the reducer entry point. The reducer
    // entry point also sets the suppression flag so the bridge
    // refresh that the synthetic input triggers does NOT auto-reopen
    // the popup over the just-accepted proposal.
    let on_completion_click = Callback::new(move |proposal_id: String| {
        let mut acceptance_holder: Option<CompletionAcceptance> = None;
        state.update(|state| {
            acceptance_holder =
                accept_completion_by_proposal_id_on_active_formula_space(state, &proposal_id);
        });
        if let Some(acceptance) = acceptance_holder {
            apply_acceptance.run(acceptance);
        }
    });

    // Keyboard policy. The handler is INSTALLED unconditionally on
    // the textarea, but is a no-op (no preventDefault, no reducer
    // call) when the popup is Hidden — so native textarea behaviour
    // (Arrow / Home / End / Backspace / Delete / IME / clipboard /
    // selection) is preserved verbatim. This is the discipline WS-13
    // got wrong: handlers leaked onto the textarea even when no
    // popup was visible.
    //
    // When the popup IS Open, the handler intercepts ONLY the five
    // popup keys (ArrowUp, ArrowDown, Tab, Enter, Escape) and
    // preventDefault's them. Every other key is allowed through to
    // the textarea unchanged.
    let on_textarea_keydown = move |ev: WebKeyboardEvent| {
        // Read popup-open state directly from the source signal, NOT
        // via the `view_model` memo. The memo recomputes lazily and
        // synthetic-event keystrokes fire inside `dispatchEvent`
        // synchronously — there is no microtask boundary between the
        // last reducer-driven state mutation and the keydown handler,
        // so the memo's cached value can be one tick behind. Reading
        // the popup state straight off `FormulaSpaceState` sidesteps
        // any memo staleness.
        let popup_open = state.with_untracked(|s| {
            s.workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))
                .map(|fs| {
                    matches!(
                        fs.completion_popup,
                        crate::services::completion_popup::CompletionPopupState::Open { .. }
                    )
                })
                .unwrap_or(false)
        });
        if !popup_open {
            return;
        }
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                state.update(|state| {
                    let _ = move_completion_popup_selection_on_active_formula_space(state, 1);
                });
            }
            "ArrowUp" => {
                ev.prevent_default();
                state.update(|state| {
                    let _ = move_completion_popup_selection_on_active_formula_space(state, -1);
                });
            }
            "Tab" | "Enter" => {
                ev.prevent_default();
                let mut acceptance_holder: Option<CompletionAcceptance> = None;
                state.update(|state| {
                    acceptance_holder =
                        accept_selected_completion_with_suppression_on_active_formula_space(
                            state,
                        );
                });
                if let Some(acceptance) = acceptance_holder {
                    apply_acceptance.run(acceptance);
                }
            }
            "Escape" => {
                ev.prevent_default();
                state.update(|state| {
                    let _ = dismiss_completion_popup_on_active_formula_space(state);
                });
            }
            _ => {
                // All other keys (Arrow Left/Right, plain typing, IME
                // composition, clipboard shortcuts) fall through to
                // the textarea's native handling. NO preventDefault.
            }
        }
    };

    // Focus-out: when the textarea loses focus (user clicks
    // elsewhere, Tab navigates away, ...) dismiss the popup so it
    // doesn't sit stale on an unfocused editor.
    let on_textarea_focusout = move |_| {
        state.update(|state| {
            let _ = dismiss_completion_popup_on_active_formula_space(state);
        });
    };

    // Reactive readers. Each closure runs whenever the underlying signal
    // it touches changes; Leptos handles the diff.
    let textarea_value = move || {
        view_model
            .get()
            .map(|vm| vm.raw_entered_cell_text)
            .unwrap_or_default()
    };
    let has_active_formula_space = move || view_model.get().is_some();
    let entry_mode_pill = move || view_model.get().map(|vm| vm.entry_mode_pill);
    let result_class_pill = move || view_model.get().and_then(|vm| vm.result_class_pill);
    let syntax_runs = move || view_model.get().map(|vm| vm.syntax_runs).unwrap_or_default();
    let diagnostic_squiggles = move || {
        view_model
            .get()
            .map(|vm| vm.diagnostic_squiggles)
            .unwrap_or_default()
    };
    let editor_metrics = move || view_model.get().map(|vm| vm.editor_metrics);
    let result_context = move || view_model.get().map(|vm| vm.result_context);
    let completion_popup = move || view_model.get().and_then(|vm| vm.completion_popup);
    let result_view = move || view_model.get().map(|vm| vm.result_view);
    let status_view = move || view_model.get().map(|vm| vm.status);
    // Browser-measured caret-box metrics surfaced as data-attributes on
    // the editor frame. The corpus uses these to assert that
    // measurement actually happened on the first keystroke.
    let editor_box_char_width = move || {
        state.with(|s| {
            s.workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))
                .and_then(|fs| fs.editor_box_metrics.map(|m| m.char_width_px))
        })
    };
    let editor_box_line_height = move || {
        state.with(|s| {
            s.workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))
                .and_then(|fs| fs.editor_box_metrics.map(|m| m.line_height_px))
        })
    };
    let editor_box_measure_tick = move || {
        state.with(|s| {
            s.workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))
                .map(|fs| fs.editor_box_metrics_tick)
                .unwrap_or(0)
        })
    };

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
                        <div class="onecalc-home-shell__caption-row">
                            <span class="onecalc-home-shell__caption">"formula ▸"</span>
                            {move || render_entry_mode_pill(entry_mode_pill())}
                        </div>
                        <div
                            class="onecalc-home-shell__editor-frame"
                            data-char-width=move || {
                                editor_box_char_width()
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "0".to_string())
                            }
                            data-line-height=move || {
                                editor_box_line_height()
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "0".to_string())
                            }
                            data-measure-tick=move || editor_box_measure_tick().to_string()
                        >
                            <div
                                class="onecalc-home-shell__editor-overlay"
                                aria-hidden="true"
                            >
                                {move || render_syntax_overlay(syntax_runs(), textarea_value())}
                            </div>
                            <div
                                class="onecalc-home-shell__editor-squiggles"
                                aria-hidden="true"
                            >
                                {move || render_diagnostic_squiggle_overlay(
                                    diagnostic_squiggles(),
                                    textarea_value(),
                                )}
                            </div>
                            <textarea
                                class="onecalc-home-shell__textarea"
                                spellcheck="false"
                                autocomplete="off"
                                aria-label="formula editor"
                                prop:value=textarea_value
                                on:keydown=on_textarea_keydown
                                on:focusout=on_textarea_focusout
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
                                    // Measure first so the geometry layer
                                    // has fresh metrics by the time the
                                    // popup view-model needs them this
                                    // tick. Self-correcting on resize and
                                    // first input: even if the very-first
                                    // mount happens before any layout,
                                    // the user's first keystroke will
                                    // measure before any popup is shown.
                                    if let Some(document) = web_sys::window()
                                        .and_then(|w| w.document())
                                    {
                                        if let Some(metrics) =
                                            measure_textarea_box(&textarea, &document)
                                        {
                                            state.update(|state| {
                                                let _ =
                                                    apply_editor_box_metrics_to_active_formula_space(
                                                        state, metrics,
                                                    );
                                            });
                                        }
                                    }
                                    on_editor_input.run(event);
                                }
                            ></textarea>
                            {move || render_completion_popup(completion_popup(), on_completion_click)}
                        </div>
                        <div class="onecalc-home-shell__foot-row">
                            {move || render_editor_metrics_chip(editor_metrics())}
                        </div>
                    </section>

                    <section class="onecalc-home-shell__result-section">
                        <div class="onecalc-home-shell__caption-row">
                            <span class="onecalc-home-shell__caption">"result ▸"</span>
                            {move || render_result_class_pill(result_class_pill())}
                        </div>
                        <div
                            class="onecalc-home-shell__result-block"
                            data-kind=move || result_view().map(result_kind_attr).unwrap_or("none")
                        >
                            {move || render_result_view(result_view())}
                        </div>
                        <div class="onecalc-home-shell__foot-row">
                            {move || render_result_context_chip(result_context())}
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

/// Render the syntax-coloured overlay that sits behind the textarea.
///
/// When `runs` is empty (no editor document, or document is a stale
/// snapshot from a prior keystroke), fall back to rendering the raw
/// textarea text uncoloured so the overlay stays character-aligned with
/// the textarea contents and the user never sees coloured tokens at the
/// wrong offset. The trailing newline preserves the textarea's last line
/// height in the overlay box (`white-space: pre-wrap` swallows it
/// otherwise).
fn render_syntax_overlay(runs: Vec<SyntaxRun>, fallback_text: String) -> AnyView {
    if runs.is_empty() {
        return view! {
            <span class="syn-text">{fallback_text}{"\n"}</span>
        }
        .into_any();
    }
    let spans: Vec<AnyView> = runs
        .into_iter()
        .map(|run| {
            let class = format!("syn {}", role_class(run.role));
            view! { <span class=class>{run.text}</span> }.into_any()
        })
        .collect();
    view! {
        <>
            {spans}
            {"\n"}
        </>
    }
    .into_any()
}

/// Render the diagnostic-squiggle overlay. Splits `text` into alternating
/// non-squiggled / squiggled segments by character offset, so that the
/// wavy underline lines up with the textarea characters at each
/// `LiveDiagnostic.primary_span`. Squiggled segments are also given a
/// `title` attribute so a browser-native hover-tooltip carries the
/// `diagnostic_id: message` summary without any JS popover work.
///
/// Both layers (this one and the syntax overlay) render the same text;
/// CSS makes only the wavy underlines visible by setting the text colour
/// transparent here.
fn render_diagnostic_squiggle_overlay(
    squiggles: Vec<DiagnosticSquiggle>,
    text: String,
) -> AnyView {
    if squiggles.is_empty() {
        // No diagnostics: render the raw text invisibly so the squiggle
        // box keeps the same height as the textarea (whitespace-pre-wrap
        // collapses zero-content boxes otherwise).
        return view! { <span>{text}{"\n"}</span> }.into_any();
    }

    // Walk the text in character offsets, building segments. The
    // squiggle list is already sorted-and-deduped by the projector.
    let chars: Vec<char> = text.chars().collect();
    let mut segments: Vec<AnyView> = Vec::new();
    let mut cursor: usize = 0;
    for squiggle in squiggles {
        let span_start = squiggle.span_start.min(chars.len());
        let span_end = span_start
            .saturating_add(squiggle.span_len)
            .min(chars.len());
        if span_start > cursor {
            let segment: String = chars[cursor..span_start].iter().collect();
            segments.push(view! { <span>{segment}</span> }.into_any());
        }
        if span_end > span_start {
            let segment: String = chars[span_start..span_end].iter().collect();
            let class = format!("squiggle squiggle--{}", squiggle.severity.slug());
            let title = format!("{}: {}", squiggle.diagnostic_id, squiggle.message);
            segments.push(
                view! {
                    <span
                        class=class
                        data-diagnostic-id=squiggle.diagnostic_id
                        data-severity=squiggle.severity.slug()
                        title=title
                    >
                        {segment}
                    </span>
                }
                .into_any(),
            );
            cursor = span_end;
        } else if cursor < span_start {
            cursor = span_start;
        }
    }
    if cursor < chars.len() {
        let trailing: String = chars[cursor..].iter().collect();
        segments.push(view! { <span>{trailing}</span> }.into_any());
    }
    view! {
        <>
            {segments}
            {"\n"}
        </>
    }
    .into_any()
}

/// Splice `insert_text` into `raw_text` at `replacement_span`.
/// Splits / joins on Rust `char` boundaries so non-ASCII inputs do not
/// corrupt. When `replacement_span` is `None`, the insertion is
/// appended at the end (matches the popup-state model's "no anchor"
/// behaviour for proposals without a replacement context).
fn splice_textarea_value(
    raw_text: &str,
    replacement_span: Option<FormulaTextSpan>,
    insert_text: &str,
) -> String {
    let chars: Vec<char> = raw_text.chars().collect();
    let (start, end) = match replacement_span {
        Some(span) => {
            let start = span.start.min(chars.len());
            let end = start
                .saturating_add(span.len)
                .min(chars.len());
            (start, end)
        }
        None => {
            let end = chars.len();
            (end, end)
        }
    };
    let mut out: String = chars[..start].iter().collect();
    out.push_str(insert_text);
    let trailing: String = chars[end..].iter().collect();
    out.push_str(&trailing);
    out
}

/// Render the completion popup. Returns an empty fragment when the
/// view-model has `None` (popup hidden or not yet measurable).
/// Positioned absolutely within the editor frame at the caret anchor;
/// the popup wrapper is `pointer-events: none` so background clicks
/// fall through to the textarea, while each item row reactivates
/// `pointer-events: auto` for click handling.
fn render_completion_popup(
    popup: Option<CompletionPopupView>,
    on_click: Callback<String>,
) -> AnyView {
    let Some(popup) = popup else {
        return view! { <span></span> }.into_any();
    };
    let style = format!(
        "left: {}px; top: {}px;",
        popup.anchor_left_px,
        popup.anchor_top_px.saturating_add(popup.line_height_px),
    );
    let item_count = popup.items.len();
    let items = popup
        .items
        .into_iter()
        .map(|item| render_completion_popup_item(item, on_click))
        .collect::<Vec<_>>();
    view! {
        <div
            class="onecalc-completion-popup"
            data-selected-index=popup.selected_index.to_string()
            data-item-count=item_count.to_string()
            role="listbox"
            aria-label="completion proposals"
            style=style
        >
            {items}
        </div>
    }
    .into_any()
}

fn render_completion_popup_item(
    item: CompletionPopupItemView,
    on_click: Callback<String>,
) -> AnyView {
    let proposal_id_for_click = item.proposal_id.clone();
    let proposal_id_for_attr = item.proposal_id.clone();
    let kind_label = item.kind_label;
    view! {
        <div
            class="onecalc-completion-popup__item"
            data-proposal-id=proposal_id_for_attr
            data-selected=if item.is_selected { "true" } else { "false" }
            data-kind=item.kind_label.to_ascii_lowercase()
            role="option"
            aria-selected=if item.is_selected { "true" } else { "false" }
            on:mousedown=move |ev| {
                // mousedown (not click) so the textarea doesn't lose
                // focus before the splice runs; preventDefault keeps
                // the focus on the textarea throughout.
                ev.prevent_default();
                on_click.run(proposal_id_for_click.clone());
            }
        >
            <span class="onecalc-completion-popup__glyph" aria-hidden="true">
                {item.kind_glyph.to_string()}
            </span>
            <span class="onecalc-completion-popup__text">{item.display_text}</span>
            <span class="onecalc-completion-popup__kind" aria-hidden="true">
                {kind_label}
            </span>
        </div>
    }
    .into_any()
}

/// Render the editor-foot live-metrics chip:
/// `tokens N · functions M · diagnostics K`. Counts come straight from
/// the view-model; rendering does no arithmetic.
fn render_editor_metrics_chip(metrics: Option<EditorMetricsChip>) -> AnyView {
    let Some(metrics) = metrics else {
        return view! { <span></span> }.into_any();
    };
    let summary = format!(
        "tokens {} · functions {} · diagnostics {}",
        metrics.token_count, metrics.function_count, metrics.diagnostic_count
    );
    view! {
        <span
            class="onecalc-home-shell__chip onecalc-home-shell__chip--metrics"
            data-tokens=metrics.token_count.to_string()
            data-functions=metrics.function_count.to_string()
            data-diagnostics=metrics.diagnostic_count.to_string()
        >
            {summary}
        </span>
    }
    .into_any()
}

/// Render the result-foot active-context chip: `locale · format · policy`.
/// Each field is rendered as its own span; SEAM-pending fields carry a
/// trailing `<NOT IMPL:SEAM-id>` sentinel with `data-seam-id` and an
/// `aria-describedby`-style attribute so the seam-status board (later
/// bead) can surface them.
fn render_result_context_chip(chip: Option<ResultContextChip>) -> AnyView {
    let Some(chip) = chip else {
        return view! { <span></span> }.into_any();
    };
    view! {
        <span class="onecalc-home-shell__chip onecalc-home-shell__chip--context">
            {render_context_field(&chip.locale, "locale")}
            <span class="onecalc-home-shell__chip-sep">" · "</span>
            {render_context_field(&chip.format, "format")}
            <span class="onecalc-home-shell__chip-sep">" · "</span>
            {render_context_field(&chip.policy, "policy")}
        </span>
    }
    .into_any()
}

fn render_context_field(field: &ContextChipField, role: &'static str) -> AnyView {
    let value = field.value().to_string();
    match field.seam_id() {
        None => view! {
            <span class="onecalc-home-shell__chip-field" data-role=role>
                {value}
            </span>
        }
        .into_any(),
        Some(seam_id) => {
            let seam_owned = seam_id.to_string();
            let seam_label = format!("<NOT IMPL:{seam_id}>");
            let aria_owned = seam_id.to_string();
            view! {
                <span
                    class="onecalc-home-shell__chip-field onecalc-home-shell__chip-field--seam"
                    data-role=role
                    data-seam-id=seam_owned
                    aria-describedby=aria_owned
                >
                    {value}
                    <span class="onecalc-home-shell__chip-seam">{seam_label}</span>
                </span>
            }
            .into_any()
        }
    }
}

fn role_class(role: SyntaxTokenRole) -> &'static str {
    match role {
        SyntaxTokenRole::Operator => "syn-op",
        SyntaxTokenRole::Function => "syn-fn",
        SyntaxTokenRole::Number => "syn-num",
        SyntaxTokenRole::Delimiter => "syn-delim",
        SyntaxTokenRole::Identifier => "syn-id",
        SyntaxTokenRole::Text => "syn-text",
        SyntaxTokenRole::Trivia => "syn-trivia",
    }
}

/// Render the editor-caption entry-mode pill. The pill is always present
/// (even for `Empty`) so the caption row keeps a stable height.
fn render_entry_mode_pill(pill: Option<EntryModePill>) -> AnyView {
    let Some(pill) = pill else {
        return view! { <span></span> }.into_any();
    };
    view! {
        <span
            class="onecalc-home-shell__caption-pill onecalc-home-shell__caption-pill--entry"
            data-mode=pill.slug()
        >
            {pill.label()}
        </span>
    }
    .into_any()
}

/// Render the result-caption result-class pill. Suppressed entirely for
/// `Empty` and `Pending` so the caption reads simply "result ▸".
fn render_result_class_pill(pill: Option<ResultClassPill>) -> AnyView {
    let Some(pill) = pill else {
        return view! { <span></span> }.into_any();
    };
    view! {
        <span
            class="onecalc-home-shell__caption-pill onecalc-home-shell__caption-pill--result"
            data-class=pill.slug()
        >
            {pill.label()}
        </span>
    }
    .into_any()
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
