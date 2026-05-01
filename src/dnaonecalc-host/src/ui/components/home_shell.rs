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
use web_sys::{
    HtmlTextAreaElement, InputEvent as WebInputEvent, KeyboardEvent as WebKeyboardEvent,
    MouseEvent as WebMouseEvent,
};

use crate::adapters::oxfml::{FormulaTextSpan, OxfmlEditorBridge};
use crate::app::reducer::{
    accept_completion_by_proposal_id_on_active_formula_space,
    accept_selected_completion_with_suppression_on_active_formula_space,
    apply_editor_box_metrics_to_active_formula_space, apply_editor_input_to_active_formula_space,
    dismiss_completion_popup_on_active_formula_space,
    move_completion_popup_selection_on_active_formula_space,
    toggle_formula_drill_on_active_formula_space, toggle_view_mode_on_workspace,
};
use crate::services::completion_popup::CompletionAcceptance;
use crate::services::home_shell_view_model::{
    build_home_shell_view_model, BridgeHealth, CompletionPopupItemView, CompletionPopupView,
    ContextChipField, DiagnosticSquiggle, EditorMetricsChip, EntryModePill, FormulaDrillNode,
    FormulaDrillPhaseChip, FormulaDrillPhaseState, FormulaDrillView, FunctionHelpCardView,
    ResultClassPill, ResultContextChip, ResultKind, ResultView, SignatureHelpView, StatusView,
};
use crate::state::ViewMode;
use crate::services::live_edit::apply_live_editor_input;
use crate::state::OneCalcHostState;
use crate::ui::design_tokens::theme::ThemeStyleTag;
use crate::ui::editor::caret_box_measurement::measure_textarea_box;
use crate::ui::editor::geometry::caret_box_for_offset;
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

    // Function-help hover state. Component-local because hover is
    // a UI concern that doesn't need to persist into the reducer
    // state. Set by the editor-frame `on:mouseover` delegation
    // handler when the pointer enters a `.syn-fn` span whose name
    // matches the bridge's function-help packet; cleared by the
    // frame's `on:mouseleave` and by an Effect that watches the
    // raw textarea text (any keystroke dismisses the hover).
    //
    // First-version note: the WS-14 plan §2.3 calls for a 400 ms
    // delay before showing the tooltip. v1 ships without the
    // delay (hover shows immediately) — the wiring is what this
    // bead pins; a follow-up bead can layer the delay on without
    // touching the projector or component data flow.
    let hover_target: RwSignal<Option<FunctionHelpHoverTarget>> = RwSignal::new(None);

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
        // Workspace view-mode toggle: Ctrl+Alt+D OR Ctrl+Shift+D.
        // Both are accepted because environments differ:
        //   * Ctrl+Alt+D collides with Windows Magnifier's
        //     "dock mode" shortcut on machines where Magnifier
        //     is active or its shortcuts are registered, and the
        //     OS swallows it before the browser sees it.
        //   * Ctrl+Shift+D is bound by Chrome / Edge to "Bookmark
        //     all tabs" but the page's keydown listener fires
        //     first, so preventDefault here prevents the dialog.
        // Either chord works; the status-foot button (rendered
        // always) is the discoverable fallback for users who
        // don't reach for chords.
        if ev.ctrl_key()
            && (ev.alt_key() || ev.shift_key())
            && ev.key().eq_ignore_ascii_case("d")
        {
            ev.prevent_default();
            state.update(|state| {
                let _ = toggle_view_mode_on_workspace(state);
            });
            return;
        }

        // Ctrl+D (no Shift, no Alt) toggles the formula
        // drill-down. Handled BEFORE the popup-open early-return
        // because the chord is global — works whether the popup
        // is open or closed. preventDefault shadows the browser's
        // native bookmark-this-page behaviour. The shift_key /
        // alt_key gates ensure Ctrl+Shift+D and Ctrl+Alt+D fall
        // through to the view-mode toggle above rather than
        // accidentally toggling the drill.
        if ev.ctrl_key()
            && !ev.shift_key()
            && !ev.alt_key()
            && ev.key().eq_ignore_ascii_case("d")
        {
            ev.prevent_default();
            state.update(|state| {
                let _ = toggle_formula_drill_on_active_formula_space(state);
            });
            return;
        }

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
    let signature_help = move || view_model.get().and_then(|vm| vm.signature_help);
    let function_help_card =
        move || view_model.get().and_then(|vm| vm.function_help_card);
    let hover_target_for_render = hover_target;
    let function_help_hover = move || hover_target_for_render.get();
    let formula_drill = move || view_model.get().map(|vm| vm.formula_drill);
    let result_view = move || view_model.get().map(|vm| vm.result_view);
    let status_view = move || view_model.get().map(|vm| vm.status);
    let view_mode = move || {
        view_model
            .get()
            .map(|vm| vm.view_mode)
            .unwrap_or(ViewMode::User)
    };

    // Trigger row callback shared between the editor-foot toggle
    // and the keyboard chord — both routes through the same
    // reducer entry so the test corpus can pin behaviour
    // identically regardless of input.
    let on_formula_drill_toggle = Callback::new(move |()| {
        state.update(|state| {
            let _ = toggle_formula_drill_on_active_formula_space(state);
        });
    });

    // View-mode toggle callback — used by both the status-foot
    // button (mouse) and the Ctrl+Alt+D / Ctrl+Shift+D chords
    // (keyboard).
    let on_view_mode_toggle = Callback::new(move |()| {
        state.update(|state| {
            let _ = toggle_view_mode_on_workspace(state);
        });
    });

    // Editor-frame mouseover delegation: when the pointer is over a
    // `.syn-fn` span whose `data-token-text` matches the bridge's
    // current `function_help.lookup_key`, surface a hover target.
    // Non-function spans are ignored. We compute the anchor via
    // `caret_box_for_offset(token_start, metrics)` rather than
    // reading the span's bounding-client-rect, so the tooltip
    // stays at the same pixel position the syntax overlay
    // measured for that token (deterministic across reflows).
    let on_overlay_mouseover = move |ev: WebMouseEvent| {
        let target = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        let Some(target) = target else {
            return;
        };
        if target.get_attribute("data-token-role").as_deref() != Some("function") {
            return;
        }
        let Some(token_text) = target.get_attribute("data-token-text") else {
            return;
        };
        let Some(token_start) = target
            .get_attribute("data-token-start")
            .and_then(|s| s.parse::<usize>().ok())
        else {
            return;
        };
        let card_lookup_key = view_model.with_untracked(|vm| {
            vm.as_ref()
                .and_then(|vm| vm.function_help_card.as_ref().map(|c| c.lookup_key.clone()))
        });
        let Some(lookup_key) = card_lookup_key else {
            return;
        };
        if !lookup_key.eq_ignore_ascii_case(&token_text) {
            return;
        }
        let anchor = state.with_untracked(|s| {
            let formula_space = s
                .workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))?;
            let metrics = formula_space.editor_box_metrics?;
            let anchor =
                caret_box_for_offset(&formula_space.raw_entered_cell_text, token_start, metrics);
            Some((anchor.left_px, anchor.top_px, metrics.line_height_px.max(1)))
        });
        let Some((anchor_left_px, anchor_top_px, line_height_px)) = anchor else {
            return;
        };
        hover_target.set(Some(FunctionHelpHoverTarget {
            token_text,
            anchor_left_px,
            anchor_top_px,
            line_height_px,
        }));
    };

    let hover_target_for_clear = hover_target;
    let on_overlay_mouseleave = move |_ev: WebMouseEvent| {
        hover_target_for_clear.set(None);
    };

    // Any input change dismisses the hover — once the user types,
    // the formula structure under the pointer might be stale.
    let hover_target_for_effect = hover_target;
    Effect::new(move |prev: Option<String>| {
        let current = view_model
            .get()
            .map(|vm| vm.raw_entered_cell_text)
            .unwrap_or_default();
        if let Some(prev_value) = prev.as_ref() {
            if prev_value != &current {
                hover_target_for_effect.set(None);
            }
        }
        current
    });
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
        <div
            class="onecalc-home-shell"
            data-view-mode=move || view_mode().slug()
        >
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
                            on:mouseover=on_overlay_mouseover
                            on:mouseleave=on_overlay_mouseleave
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
                            {move || render_signature_help(signature_help())}
                            {move || render_function_help_card(
                                function_help_hover(),
                                function_help_card(),
                            )}
                        </div>
                        <div class="onecalc-home-shell__foot-row">
                            {move || render_editor_metrics_chip(editor_metrics(), view_mode())}
                            {move || render_formula_drill_toggle(
                                formula_drill(),
                                on_formula_drill_toggle,
                            )}
                        </div>
                    </section>

                    <section class="onecalc-home-shell__formula-drill-section">
                        {move || render_formula_drill_panel(formula_drill(), view_mode())}
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
                            {move || render_result_context_chip(result_context(), view_mode())}
                        </div>
                    </section>
                </Show>
            </main>

            <footer class="onecalc-home-shell__statusfoot">
                {move || render_status_foot(status_view())}
                {move || render_view_mode_button(view_mode(), on_view_mode_toggle)}
            </footer>
        </div>
    }
}

/// Render the view-mode toggle button in the status-foot.
/// Always rendered so users can opt into Developer view without
/// needing to discover the keyboard chord. The button:
///
/// * In User mode shows a muted "dev" pill with `aria-pressed=
///   false` — clicking flips the workspace into Developer mode.
/// * In Developer mode shows the same pill in the accent palette
///   with `aria-pressed=true` — clicking flips back to User.
///
/// Uses `on:mousedown` (not `on:click`) so a click does not pull
/// focus away from the textarea: the textarea retains its caret
/// throughout the toggle.
fn render_view_mode_button(mode: ViewMode, on_toggle: Callback<()>) -> AnyView {
    let pressed_attr = match mode {
        ViewMode::User => "false",
        ViewMode::Developer => "true",
    };
    let mode_slug = mode.slug();
    view! {
        <button
            type="button"
            class="onecalc-home-shell__statusfoot-mode-button"
            data-view-mode=mode_slug
            aria-label="toggle developer view mode"
            aria-pressed=pressed_attr
            title="Toggle developer view (Ctrl+Alt+D or Ctrl+Shift+D)"
            on:mousedown=move |ev| {
                ev.prevent_default();
                on_toggle.run(());
            }
        >
            "dev"
        </button>
    }
    .into_any()
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
            // data-token-start + data-token-text + data-token-role
            // power the function-hover delegation handler attached
            // on the editor frame (the corpus also reads
            // data-token-text to assert which token is hovered).
            let span_start = run.span_start.to_string();
            let token_text = run.text.clone();
            let role_slug = role_slug(run.role);
            view! {
                <span
                    class=class
                    data-token-start=span_start
                    data-token-text=token_text.clone()
                    data-token-role=role_slug
                >
                    {run.text}
                </span>
            }
            .into_any()
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
/// Render the editor-foot trigger row for the formula drill-down.
/// Always visible alongside the live-metrics chip; aria-expanded
/// follows the panel's expansion state.
fn render_formula_drill_toggle(
    drill: Option<FormulaDrillView>,
    on_toggle: Callback<()>,
) -> AnyView {
    let Some(drill) = drill else {
        return view! { <span></span> }.into_any();
    };
    let aria_expanded = if drill.expanded { "true" } else { "false" };
    let label = if drill.expanded {
        "▾ hide formula drill-down"
    } else {
        "▸ show formula drill-down"
    };
    let row_count = drill.tree.len();
    view! {
        <button
            type="button"
            class="onecalc-home-shell__formula-drill-toggle"
            data-expanded=aria_expanded
            data-row-count=row_count.to_string()
            aria-expanded=aria_expanded
            aria-controls="onecalc-formula-drill-panel"
            on:click=move |_| on_toggle.run(())
        >
            {label}
        </button>
    }
    .into_any()
}

/// Render the formula drill-down panel itself. Always emits the
/// outer panel div (so the corpus can read `data-expanded`); the
/// body content is gated by the `expanded` flag and the rows /
/// phase-strip rendering branches on `view_mode`.
fn render_formula_drill_panel(
    drill: Option<FormulaDrillView>,
    view_mode: ViewMode,
) -> AnyView {
    let Some(drill) = drill else {
        return view! { <span></span> }.into_any();
    };
    let aria_hidden = if drill.expanded { "false" } else { "true" };
    let expanded_attr = if drill.expanded { "true" } else { "false" };
    let fresh_attr = if drill.document_is_fresh { "true" } else { "false" };
    let mode_attr = view_mode.slug();
    let row_count = drill.tree.len();
    let body = if !drill.expanded {
        view! { <span></span> }.into_any()
    } else if !drill.document_is_fresh {
        view! {
            <div class="onecalc-home-shell__formula-drill-loading" role="status">
                "(loading…)"
            </div>
        }
        .into_any()
    } else {
        let nodes_view: Vec<AnyView> = drill
            .tree
            .iter()
            .map(|node| render_formula_drill_row(node.clone(), view_mode))
            .collect();
        let phase_strip = match view_mode {
            ViewMode::Developer => render_formula_drill_phase_strip_developer(&drill.phase_summaries),
            ViewMode::User => render_formula_drill_phase_strip_user(&drill.phase_summaries),
        };
        view! {
            <div
                class="onecalc-home-shell__formula-drill-tree"
                role="tree"
                aria-label="formula walk tree"
                data-mode=mode_attr
            >
                {nodes_view}
            </div>
            {phase_strip}
        }
        .into_any()
    };
    view! {
        <div
            id="onecalc-formula-drill-panel"
            class="onecalc-home-shell__formula-drill-panel"
            data-expanded=expanded_attr
            data-document-fresh=fresh_attr
            data-row-count=row_count.to_string()
            data-mode=mode_attr
            aria-hidden=aria_hidden
            tabindex="-1"
        >
            {body}
        </div>
    }
    .into_any()
}

fn render_formula_drill_row(node: FormulaDrillNode, view_mode: ViewMode) -> AnyView {
    let depth = node.depth;
    let has_children_attr = if node.has_children { "true" } else { "false" };
    let state_slug = formula_drill_state_slug(node.state);
    let value_preview_full = node.value_preview.clone();
    let value_preview = value_preview_full.clone().unwrap_or_default();
    let indent_style = format!("padding-left: {}rem;", (depth as f32) * 1.0);
    let aria_level = (depth + 1).to_string();
    let mode_attr = view_mode.slug();
    let row_inner = match view_mode {
        ViewMode::Developer => view! {
            <>
                <span
                    class="onecalc-home-shell__formula-drill-state"
                    aria-label=state_slug
                    data-state=state_slug
                >
                    {formula_drill_state_label(node.state)}
                </span>
                <span class="onecalc-home-shell__formula-drill-label">{node.label}</span>
                <span
                    class="onecalc-home-shell__formula-drill-value"
                    title=value_preview.clone()
                >
                    {truncate_for_drill(value_preview.clone())}
                </span>
            </>
        }
        .into_any(),
        ViewMode::User => render_formula_drill_row_user_mode(
            node.label,
            node.state,
            value_preview_full,
        ),
    };
    view! {
        <div
            class="onecalc-home-shell__formula-drill-row"
            role="treeitem"
            data-depth=depth.to_string()
            data-has-children=has_children_attr
            data-state=state_slug
            data-node-id=node.node_id
            data-aria-level=aria_level
            data-mode=mode_attr
            style=indent_style
        >
            {row_inner}
        </div>
    }
    .into_any()
}

/// User-mode row layout: `label = value` (or `label · blocked
/// <reason>` for blocked rows). The state chip is dropped; the
/// only non-text element is a tiny inline tag for blocked rows
/// because that is the one row state an Excel user genuinely
/// needs to notice.
fn render_formula_drill_row_user_mode(
    label: String,
    state: crate::adapters::oxfml::FormulaWalkNodeState,
    value_preview: Option<String>,
) -> AnyView {
    use crate::adapters::oxfml::FormulaWalkNodeState as State;
    let label_view = view! {
        <span class="onecalc-home-shell__formula-drill-label">{label}</span>
    };
    match state {
        State::Blocked => {
            let value_text = value_preview.clone().unwrap_or_default();
            let truncated = truncate_for_drill(value_text.clone());
            view! {
                <>
                    {label_view}
                    <span class="onecalc-home-shell__formula-drill-blocked-tag">"blocked"</span>
                    <span
                        class="onecalc-home-shell__formula-drill-value"
                        title=value_text
                    >
                        {truncated}
                    </span>
                </>
            }
            .into_any()
        }
        _ => {
            let (value_text, value_title) = match value_preview {
                Some(v) => (truncate_for_drill(v.clone()), v),
                None => ("…".to_string(), String::new()),
            };
            view! {
                <>
                    {label_view}
                    <span class="onecalc-home-shell__formula-drill-equals" aria-hidden="true">"="</span>
                    <span
                        class="onecalc-home-shell__formula-drill-value"
                        title=value_title
                    >
                        {value_text}
                    </span>
                </>
            }
            .into_any()
        }
    }
}

/// Developer-mode phase strip: parse / bind / eval chips, one per phase.
fn render_formula_drill_phase_strip_developer(
    chips: &[FormulaDrillPhaseChip],
) -> AnyView {
    let phase_view: Vec<AnyView> = chips
        .iter()
        .map(|chip| render_formula_drill_phase_chip(chip.clone()))
        .collect();
    view! {
        <div
            class="onecalc-home-shell__formula-drill-phase-strip"
            data-mode="developer"
        >
            {phase_view}
        </div>
    }
    .into_any()
}

/// User-mode phase strip: a single status line. Reads as
/// "evaluated in <duration>" when clean, or "blocked: <reason>"
/// when any phase is blocked. The eval-phase chip's detail
/// carries the duration_text in the form "<n> step(s) · <ms>"
/// so we extract the duration suffix; if the format changes we
/// fall back to the raw detail.
fn render_formula_drill_phase_strip_user(
    chips: &[FormulaDrillPhaseChip],
) -> AnyView {
    if chips.is_empty() {
        return view! { <span></span> }.into_any();
    }
    let any_blocked = chips
        .iter()
        .any(|c| c.state == FormulaDrillPhaseState::Blocked);
    let status_class = if any_blocked {
        "onecalc-home-shell__formula-drill-status onecalc-home-shell__formula-drill-status--blocked"
    } else {
        "onecalc-home-shell__formula-drill-status onecalc-home-shell__formula-drill-status--ok"
    };
    let summary = if any_blocked {
        chips
            .iter()
            .find(|c| c.state == FormulaDrillPhaseState::Blocked)
            .map(|c| format!("blocked at {}: {}", c.label, c.detail))
            .unwrap_or_else(|| "blocked".to_string())
    } else {
        chips
            .iter()
            .find(|c| c.label == "eval")
            .map(|c| {
                // eval detail is "<n> step(s) · <duration_text>"
                // — pull the segment after the last " · ". If
                // unavailable, fall back to the whole detail.
                let last_segment = c
                    .detail
                    .rsplit(" · ")
                    .next()
                    .unwrap_or_else(|| c.detail.as_str());
                format!("evaluated in {last_segment}")
            })
            .unwrap_or_else(|| "evaluated".to_string())
    };
    let status_state = if any_blocked { "blocked" } else { "ok" };
    view! {
        <div
            class="onecalc-home-shell__formula-drill-phase-strip"
            data-mode="user"
        >
            <span class=status_class data-status=status_state>{summary}</span>
        </div>
    }
    .into_any()
}

fn render_formula_drill_phase_chip(chip: FormulaDrillPhaseChip) -> AnyView {
    let state_slug = chip.state.slug();
    let label = chip.label;
    view! {
        <span
            class="onecalc-home-shell__formula-drill-phase"
            data-phase=label
            data-state=state_slug
        >
            <strong>{label}</strong>
            ": "
            {chip.detail}
        </span>
    }
    .into_any()
}

fn formula_drill_state_slug(state: crate::adapters::oxfml::FormulaWalkNodeState) -> &'static str {
    use crate::adapters::oxfml::FormulaWalkNodeState as State;
    match state {
        State::Evaluated => "evaluated",
        State::Bound => "bound",
        State::Opaque => "opaque",
        State::Blocked => "blocked",
    }
}

fn formula_drill_state_label(state: crate::adapters::oxfml::FormulaWalkNodeState) -> &'static str {
    use crate::adapters::oxfml::FormulaWalkNodeState as State;
    match state {
        State::Evaluated => "evaluated",
        State::Bound => "bound",
        State::Opaque => "opaque",
        State::Blocked => "blocked",
    }
}

fn truncate_for_drill(value: String) -> String {
    let limit = 32;
    if value.chars().count() <= limit {
        value
    } else {
        let mut out: String = value.chars().take(limit).collect();
        out.push('…');
        out
    }
}

/// Hover-state for the function-help tooltip. Component-local
/// (not in the reducer state) because hover is purely a UI
/// concern. Set by the editor-frame `on:mouseover` handler when
/// the pointer enters a `.syn-fn` span whose `data-token-text`
/// matches the bridge's `function_help.lookup_key`. Cleared by
/// the frame's `on:mouseleave` and by an Effect that watches
/// `raw_entered_cell_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionHelpHoverTarget {
    token_text: String,
    anchor_left_px: usize,
    anchor_top_px: usize,
    line_height_px: usize,
}

/// Render the function-help tooltip. Returns an empty span when
/// either the hover state or the function-help card is missing,
/// so visibility is reactive on both signals at once. The tooltip
/// is positioned BELOW the hovered token (anchor_top + line_height
/// + small gap) — different from the signature help which sits
/// above the caret. Layout-wise it lives in the same editor-frame
/// container as the popup and signature help, so it stays inside
/// the editor's coordinate system.
///
/// Wrapper is `pointer-events: none` so the user can move the
/// mouse off the function token without the tooltip itself
/// stealing the hover.
fn render_function_help_card(
    hover: Option<FunctionHelpHoverTarget>,
    card: Option<FunctionHelpCardView>,
) -> AnyView {
    let (Some(hover), Some(card)) = (hover, card) else {
        return view! { <span></span> }.into_any();
    };
    if !card.lookup_key.eq_ignore_ascii_case(&hover.token_text) {
        return view! { <span></span> }.into_any();
    }
    let style = format!(
        "left: {}px; top: {}px;",
        hover.anchor_left_px,
        hover.anchor_top_px.saturating_add(hover.line_height_px),
    );
    let availability = card.availability_summary.clone().unwrap_or_default();
    let signature_view = card
        .signature
        .clone()
        .map(|sig| view! { <div class="onecalc-function-help__signature">{sig}</div> }.into_any())
        .unwrap_or_else(|| view! { <span></span> }.into_any());
    let description_view = card
        .short_description
        .clone()
        .map(|desc| {
            view! { <div class="onecalc-function-help__description">{desc}</div> }.into_any()
        })
        .unwrap_or_else(|| view! { <span></span> }.into_any());
    let availability_view = if !availability.is_empty() {
        view! {
            <div class="onecalc-function-help__availability">{availability}</div>
        }
        .into_any()
    } else {
        view! { <span></span> }.into_any()
    };
    let deferred_attr = if card.deferred_or_profile_limited {
        "true"
    } else {
        "false"
    };
    view! {
        <div
            class="onecalc-function-help"
            role="tooltip"
            data-lookup-key=card.lookup_key.clone()
            data-deferred=deferred_attr
            style=style
        >
            <div class="onecalc-function-help__heading">{card.display_name.clone()}</div>
            {signature_view}
            {description_view}
            {availability_view}
        </div>
    }
    .into_any()
}

/// Render the signature-help line ABOVE the caret.
///
/// The view-model emits `None` whenever the help should be hidden
/// (no call in progress, document stale, metrics unmeasured, popup
/// open at the same caret). This function only positions the help
/// and renders the parameter list with the active parameter
/// bolded; all suppression is the projector's responsibility.
///
/// Anchor strategy: the projector hands us the caret-box top-left in
/// pixels; we offset upward by `signature_help_height + gap`. The
/// help line is `max-height: 28px` so it's narrow enough not to
/// fight the syntax overlay or the squiggle layer for stacking
/// space. Below-the-caret fallback (when the line would clip the
/// frame top) is handled with CSS `transform: translateY(...)` —
/// see the theme's `.onecalc-signature-help--flipped` rule.
///
/// Wrapper is `pointer-events: none` so background clicks fall
/// through to the textarea (the help is non-interactive).
fn render_signature_help(help: Option<SignatureHelpView>) -> AnyView {
    let Some(help) = help else {
        return view! { <span></span> }.into_any();
    };
    // Position UPWARD from the caret-box top by the help line's
    // approximate height plus a small gap. We use a CSS transform
    // (translateY(-100%) - small gap) so the actual rendered height
    // is what's used, not a guessed pixel count — this keeps the
    // anchor exact across font-metrics changes.
    let style = format!(
        "left: {}px; top: {}px;",
        help.anchor_left_px,
        help.anchor_top_px,
    );
    let parameter_count = help.parameters.len();
    let active_index_attr = help
        .active_parameter
        .map(|i| i.to_string())
        .unwrap_or_else(|| "-1".to_string());
    let parameters = help
        .parameters
        .into_iter()
        .enumerate()
        .map(|(index, param)| {
            let is_last = index + 1 == parameter_count;
            let separator = if is_last { "" } else { ", " };
            let class = if param.is_active {
                "onecalc-signature-help__parameter onecalc-signature-help__parameter--active"
            } else {
                "onecalc-signature-help__parameter"
            };
            let active_attr = if param.is_active { "true" } else { "false" };
            view! {
                <span class=class data-active=active_attr>{param.name}</span>
                <span class="onecalc-signature-help__separator" aria-hidden="true">
                    {separator}
                </span>
            }
            .into_any()
        })
        .collect::<Vec<_>>();
    view! {
        <div
            class="onecalc-signature-help"
            role="status"
            aria-live="polite"
            data-active-parameter=active_index_attr
            data-parameter-count=parameter_count.to_string()
            style=style
        >
            <span class="onecalc-signature-help__callee">{help.callee_text}</span>
            <span class="onecalc-signature-help__paren" aria-hidden="true">"("</span>
            {parameters}
            <span class="onecalc-signature-help__paren" aria-hidden="true">")"</span>
        </div>
    }
    .into_any()
}

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

/// Render the editor-foot live-metrics chip. Output shape branches
/// on the view-mode:
///
/// * Developer mode: full counts — `tokens N · functions M ·
///   diagnostics K`. Same as before this bead.
/// * User mode (default): a single status chip carrying the
///   actionable signal an Excel user wants. `<N> issue<s>: <first
///   message>` in the warning palette when diagnostics exist;
///   muted "ready" when the formula is well-formed; nothing when
///   the textarea is empty (no document, all counts zero).
///
/// The data-tokens / data-functions / data-diagnostics attributes
/// stay on the rendered span in BOTH modes so the seam-status
/// board (later bead) and the corpus can read them without
/// switching modes.
fn render_editor_metrics_chip(
    metrics: Option<EditorMetricsChip>,
    view_mode: ViewMode,
) -> AnyView {
    let Some(metrics) = metrics else {
        return view! { <span></span> }.into_any();
    };
    let data_tokens = metrics.token_count.to_string();
    let data_functions = metrics.function_count.to_string();
    let data_diagnostics = metrics.diagnostic_count.to_string();
    match view_mode {
        ViewMode::Developer => {
            let summary = format!(
                "tokens {} · functions {} · diagnostics {}",
                metrics.token_count, metrics.function_count, metrics.diagnostic_count
            );
            view! {
                <span
                    class="onecalc-home-shell__chip onecalc-home-shell__chip--metrics"
                    data-mode="developer"
                    data-tokens=data_tokens
                    data-functions=data_functions
                    data-diagnostics=data_diagnostics
                >
                    {summary}
                </span>
            }
            .into_any()
        }
        ViewMode::User => {
            // Empty mount (no document yet, no input): omit the
            // chip entirely — nothing useful to say.
            if metrics.token_count == 0 && metrics.diagnostic_count == 0 {
                return view! { <span></span> }.into_any();
            }
            if metrics.diagnostic_count == 0 {
                return view! {
                    <span
                        class="onecalc-home-shell__chip \
                               onecalc-home-shell__chip--metrics \
                               onecalc-home-shell__chip--ready"
                        data-mode="user"
                        data-status="ready"
                        data-tokens=data_tokens
                        data-functions=data_functions
                        data-diagnostics=data_diagnostics
                    >
                        "ready"
                    </span>
                }
                .into_any();
            }
            let plural = if metrics.diagnostic_count == 1 {
                "issue"
            } else {
                "issues"
            };
            let message = metrics
                .first_diagnostic_message
                .clone()
                .unwrap_or_default();
            let summary = if message.is_empty() {
                format!("{} {plural}", metrics.diagnostic_count)
            } else {
                format!("{} {plural}: {}", metrics.diagnostic_count, message)
            };
            view! {
                <span
                    class="onecalc-home-shell__chip \
                           onecalc-home-shell__chip--metrics \
                           onecalc-home-shell__chip--warning"
                    data-mode="user"
                    data-status="diagnostic"
                    data-tokens=data_tokens
                    data-functions=data_functions
                    data-diagnostics=data_diagnostics
                >
                    {summary}
                </span>
            }
            .into_any()
        }
    }
}

/// Render the result-foot active-context chip: `locale · format ·
/// policy`. Output shape branches on the view-mode:
///
/// * Developer mode: SEAM-pending fields carry a trailing
///   `<NOT IMPL:SEAM-id>` sentinel and the `data-seam-id` /
///   `aria-describedby` attributes (same as before this bead).
/// * User mode (default): plain `value · value · value` — no SEAM
///   sentinels, no warning palette. The data-seam-id attribute
///   stays on the field span so the seam-status board can read
///   it without switching modes; only the user-visible badge text
///   is hidden.
fn render_result_context_chip(
    chip: Option<ResultContextChip>,
    view_mode: ViewMode,
) -> AnyView {
    let Some(chip) = chip else {
        return view! { <span></span> }.into_any();
    };
    let mode_attr = view_mode.slug();
    view! {
        <span
            class="onecalc-home-shell__chip onecalc-home-shell__chip--context"
            data-mode=mode_attr
        >
            {render_context_field(&chip.locale, "locale", view_mode)}
            <span class="onecalc-home-shell__chip-sep">" · "</span>
            {render_context_field(&chip.format, "format", view_mode)}
            <span class="onecalc-home-shell__chip-sep">" · "</span>
            {render_context_field(&chip.policy, "policy", view_mode)}
        </span>
    }
    .into_any()
}

fn render_context_field(
    field: &ContextChipField,
    role: &'static str,
    view_mode: ViewMode,
) -> AnyView {
    let value = field.value().to_string();
    let render_seam_label = matches!(view_mode, ViewMode::Developer);
    match field.seam_id() {
        None => view! {
            <span class="onecalc-home-shell__chip-field" data-role=role>
                {value}
            </span>
        }
        .into_any(),
        Some(seam_id) => {
            let seam_owned = seam_id.to_string();
            let aria_owned = seam_id.to_string();
            // Always carry data-seam-id so the seam-status board
            // can find these regardless of mode. Only the badge
            // TEXT is mode-conditional.
            let badge = if render_seam_label {
                let seam_label = format!("<NOT IMPL:{seam_id}>");
                view! {
                    <span class="onecalc-home-shell__chip-seam">{seam_label}</span>
                }
                .into_any()
            } else {
                view! { <span></span> }.into_any()
            };
            let class = if render_seam_label {
                "onecalc-home-shell__chip-field onecalc-home-shell__chip-field--seam"
            } else {
                "onecalc-home-shell__chip-field"
            };
            view! {
                <span
                    class=class
                    data-role=role
                    data-seam-id=seam_owned
                    aria-describedby=aria_owned
                >
                    {value}
                    {badge}
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

/// Slug for `data-token-role` attribute on syntax-overlay spans.
/// Mirrors `role_class` but stripped of the `syn-` prefix so the
/// attribute reads like an enum tag.
fn role_slug(role: SyntaxTokenRole) -> &'static str {
    match role {
        SyntaxTokenRole::Operator => "operator",
        SyntaxTokenRole::Function => "function",
        SyntaxTokenRole::Number => "number",
        SyntaxTokenRole::Delimiter => "delimiter",
        SyntaxTokenRole::Identifier => "identifier",
        SyntaxTokenRole::Text => "text",
        SyntaxTokenRole::Trivia => "trivia",
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
