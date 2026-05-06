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
    close_scenario_breadcrumb, dismiss_completion_popup_on_active_formula_space,
    move_completion_popup_selection_on_active_formula_space,
    toggle_formula_drill_on_active_formula_space, toggle_scenario_breadcrumb,
    toggle_view_mode_on_workspace,
};
use crate::services::completion_popup::CompletionAcceptance;
use crate::services::home_shell_view_model::{
    build_home_shell_view_model, ArrayCellFormatView, BridgeHealth, CompletionPopupItemView,
    CompletionPopupView, ConditionalFormattingRuleView, ContextChipField, DataBarDirectionView,
    DiagnosticSquiggle, EditorMetricsChip, EntryModePill, FormattingControlsView,
    FormulaDrillDiagnosticRow, FormulaDrillNode, FormulaDrillPhaseChip, FormulaDrillPhaseState,
    FormulaDrillView, FunctionHelpCardView, NumberFormatPreset, ResultClassPill, ResultContextChip,
    ResultKind, ResultView, ScenarioBreadcrumbAction, ScenarioBreadcrumbActionId,
    ScenarioBreadcrumbEntry, ScenarioBreadcrumbView, ScenarioPolicyView, SignatureHelpView,
    StatusView,
};
use crate::services::live_edit::apply_live_editor_input;
use crate::state::OneCalcHostState;
use crate::state::ViewMode;
use crate::ui::design_tokens::theme::ThemeStyleTag;
use crate::ui::editor::caret_box_measurement::measure_textarea_box;
use crate::ui::editor::commands::{classify_dom_input, EditorInputEvent, EditorInputKind};
use crate::ui::editor::geometry::caret_box_for_offset;
use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};

#[component]
pub fn HomeShell(
    initial_state: OneCalcHostState,
    #[prop(default = None)] editor_bridge: Option<Arc<dyn OxfmlEditorBridge + Send + Sync>>,
) -> impl IntoView {
    // Hydrate from `localStorage["dnaonecalc.workspace.v1"]` before
    // the state signal sees its first subscriber, so the user's
    // pinned ids and last-edited formula land in `initial_state`
    // *before* the reactive view-model first runs. On non-wasm
    // targets this is a no-op (the SSR build doesn't have
    // localStorage); the same call site keeps both branches
    // visually identical.
    let mut initial_state = initial_state;
    crate::persistence::hydrate_state_from_local_storage(&mut initial_state);

    let state: RwSignal<OneCalcHostState> = RwSignal::new(initial_state);

    // Auto-save the workspace envelope to localStorage on every
    // state change. The serialise + write path is cheap (<1 ms for
    // a typical workspace) and fires inside the browser's main
    // thread, so any heavier persistence mechanism would have to
    // schedule itself anyway. Storage failures (quota, disabled
    // site data) log to console without taking the rest of the app
    // down.
    Effect::new(move |_| {
        state.with(crate::persistence::save_workspace_to_local_storage);
    });

    // Reactive view-model: rebuilds whenever the state signal changes.
    let view_model = Memo::new(move |_| state.with(build_home_shell_view_model));

    // NodeRef on the editor textarea so we can imperatively sync
    // `value` + `selectionStart/End` from host state after each
    // reactive flush. Without this, two failure modes appear under
    // slow recalc:
    //
    // 1. `prop:value=textarea_value` writes `textarea.value = X`
    //    even when `X` is what the textarea already has. Some
    //    browsers reset the caret to the end of the field on any
    //    `node.value = …` assignment. The user clicks at offset
    //    10 → bridge runs → state re-renders → `prop:value`
    //    re-applies the same string → caret jumps to end.
    //
    // 2. The host's `editor_surface_state.selection` is the source
    //    of truth for where the caret should be after a bridge
    //    round-trip (bridge result rebuilds it from the prior host
    //    selection). If the DOM disagrees with the host (cursor
    //    reset, completion accept that splices text without
    //    matching caret update, scenario load), we need to
    //    actively restore.
    //
    // The effect below is conservative: it reads the host text +
    // selection on every state change, compares to the DOM, and
    // writes only when divergent. Idempotent; cheap when nothing
    // moved. Skipping the effect when the textarea is unmounted
    // (NodeRef::get returns None) keeps the SSR-render path inert.
    let textarea_ref: NodeRef<leptos::html::Textarea> = NodeRef::new();
    Effect::new(move |_| {
        // Subscribe to the state signal so the effect re-runs on
        // every reducer-driven update.
        let (host_text, host_anchor, host_focus) = state.with(|s| {
            let active_id = s
                .workspace_shell
                .active_formula_space_id
                .clone()
                .or_else(|| {
                    s.active_formula_space_view
                        .selected_formula_space_id
                        .clone()
                });
            let space = active_id.as_ref().and_then(|id| s.formula_spaces.get(id));
            let text = space
                .map(|sp| sp.raw_entered_cell_text.clone())
                .unwrap_or_default();
            let anchor = space
                .map(|sp| sp.editor_surface_state.selection.anchor as u32)
                .unwrap_or(0);
            let focus = space
                .map(|sp| sp.editor_surface_state.selection.focus as u32)
                .unwrap_or(0);
            (text, anchor, focus)
        });
        let Some(textarea_el) = textarea_ref.get() else {
            return;
        };
        // Sync text only when divergent. On a match this is a
        // no-op (browser does NOT reset caret because we never
        // assigned to .value).
        if textarea_el.value() != host_text {
            textarea_el.set_value(&host_text);
        }
        // Restore selection from host state when the DOM diverges.
        // After a pure caret-only round-trip (mouse click, arrow
        // navigation), this is the call that pins the caret back
        // to where the click landed even if some upstream prop
        // binding momentarily reset it.
        let dom_anchor = textarea_el.selection_start().ok().flatten();
        let dom_focus = textarea_el.selection_end().ok().flatten();
        if dom_anchor != Some(host_anchor) || dom_focus != Some(host_focus) {
            let _ = textarea_el.set_selection_range(host_anchor, host_focus);
        }
    });

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
    let on_textarea_keydown_inner = move |ev: WebKeyboardEvent| {
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
        if ev.ctrl_key() && (ev.alt_key() || ev.shift_key()) && ev.key().eq_ignore_ascii_case("d") {
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
        if ev.ctrl_key() && !ev.shift_key() && !ev.alt_key() && ev.key().eq_ignore_ascii_case("d") {
            ev.prevent_default();
            state.update(|state| {
                let _ = toggle_formula_drill_on_active_formula_space(state);
            });
            return;
        }

        // (F9 handling moved to the outer shell `on:keydown` so
        // it works even when focus is outside the textarea — see
        // the shell-level handler below.)

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
                        accept_selected_completion_with_suppression_on_active_formula_space(state);
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
    // Wrap the keydown closure in a Callback so the host shell's
    // section render closure (called multiple times during reactive
    // re-renders) can pass it via `on:keydown` without consuming it
    // — `Callback` is `Copy`, the inner closure is not (it captures
    // the bridge `Arc` and other non-Copy state).
    let on_textarea_keydown: Callback<WebKeyboardEvent> = Callback::new(on_textarea_keydown_inner);

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
    let syntax_runs = move || {
        view_model
            .get()
            .map(|vm| vm.syntax_runs)
            .unwrap_or_default()
    };
    // WS-14 plan §2.3: bracket-pair highlight at the caret. The matcher
    // returns the open / close offsets that pair under the cursor; the
    // syntax overlay surfaces them as `data-bracket-active="true"` on the
    // matching delimiter spans. Returns `None` when the caret is not
    // adjacent to a bracket or the brackets are unbalanced — the
    // highlight simply turns off in that case.
    let bracket_pair_highlight = move || {
        view_model.get().and_then(|vm| {
            crate::ui::editor::bracket_matcher::bracket_pair_for_caret(
                &vm.raw_entered_cell_text,
                vm.editor_surface_state.caret.offset,
            )
        })
    };
    let diagnostic_squiggles = move || {
        view_model
            .get()
            .map(|vm| vm.diagnostic_squiggles)
            .unwrap_or_default()
    };
    let editor_metrics = move || view_model.get().map(|vm| vm.editor_metrics);
    // The view-model returns `Option<ResultContextChip>` directly
    // (the chip collapses on default-state formulas — see
    // `project_result_context`). Flatten through `and_then` so the
    // renderer's `Option<ResultContextChip>` parameter has a single
    // None for "no active formula" or "default formula".
    let result_context = move || view_model.get().and_then(|vm| vm.result_context);
    let completion_popup = move || view_model.get().and_then(|vm| vm.completion_popup);
    let signature_help = move || view_model.get().and_then(|vm| vm.signature_help);
    let function_help_card = move || view_model.get().and_then(|vm| vm.function_help_card);
    let hover_target_for_render = hover_target;
    let function_help_hover = move || hover_target_for_render.get();
    let formula_drill = move || view_model.get().map(|vm| vm.formula_drill);
    let result_view = move || view_model.get().map(|vm| vm.result_view);
    let status_view = move || view_model.get().map(|vm| vm.status);
    let scenario_breadcrumb = move || view_model.get().map(|vm| vm.scenario_breadcrumb);
    let formatting_controls = move || view_model.get().map(|vm| vm.formatting_controls);
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

    // Slice 5 — formatting-control callbacks. Each setter dispatches
    // to the matching reducer AND, on a real change, re-runs the live
    // bridge so the publication-surface
    // `effective_display_text` produced by the new format code flows
    // straight into the result hero. Without the post-mutation bridge
    // refresh the formatting only takes effect on the next keystroke,
    // which feels broken — the user clicks "Currency" and nothing
    // changes until they type something.
    let editor_bridge_for_formatting = editor_bridge.clone();
    let refresh_after_formatting_change = move |state: &mut crate::state::OneCalcHostState| {
        if let Some(bridge) = editor_bridge_for_formatting.as_ref() {
            let _ =
                crate::services::live_edit::refresh_active_formula_space(bridge.as_ref(), state);
        }
    };
    let on_set_number_format_code = {
        let refresh = refresh_after_formatting_change.clone();
        Callback::new(move |value: String| {
            state.update(|s| {
                if crate::app::reducer::set_active_number_format_code(s, value) {
                    refresh(s);
                }
            });
        })
    };
    let on_set_font_color = {
        let refresh = refresh_after_formatting_change.clone();
        Callback::new(move |value: String| {
            state.update(|s| {
                if crate::app::reducer::set_active_font_color(s, value) {
                    refresh(s);
                }
            });
        })
    };
    let on_set_fill_color = {
        let refresh = refresh_after_formatting_change.clone();
        Callback::new(move |value: String| {
            state.update(|s| {
                if crate::app::reducer::set_active_fill_color(s, value) {
                    refresh(s);
                }
            });
        })
    };
    let on_set_date1904 = {
        let refresh = refresh_after_formatting_change.clone();
        Callback::new(move |value: bool| {
            state.update(|s| {
                if crate::app::reducer::set_active_date1904(s, value) {
                    refresh(s);
                }
            });
        })
    };
    // WS-14 plan §5.3, item 8: collapsible formatting panel above the
    // result section. Click the summary chip to expand the full
    // formatting controls row; click again to collapse back. The
    // reducer flips `formula_space.formatting_panel_open`; the view
    // model lifts that flag onto `FormattingControlsView.is_open`.
    let on_formatting_panel_toggle = Callback::new(move |()| {
        state.update(|s| {
            let _ = crate::app::reducer::toggle_formatting_panel_on_active_formula_space(s);
        });
    });
    // Calc-options + CF rule callbacks. Each chains a bridge refresh
    // when the underlying state actually changed, so the result hero
    // updates live (same pattern as `refresh_after_formatting_change`
    // for the per-field setters above).
    let editor_bridge_for_calc_opts = editor_bridge.clone();
    let refresh_after_calc_opts_change = move |state: &mut crate::state::OneCalcHostState| {
        if let Some(bridge) = editor_bridge_for_calc_opts.as_ref() {
            let _ =
                crate::services::live_edit::refresh_active_formula_space(bridge.as_ref(), state);
        }
    };
    let on_set_scenario_policy = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(move |policy: crate::persistence::ScenarioPolicy| {
            state.update(|s| {
                if crate::app::reducer::set_active_scenario_policy(s, policy) {
                    refresh(s);
                }
            });
        })
    };
    // Workspace locale preset. Drives the date / datetime / time
    // format-code defaults applied to General-format result heroes
    // via the presentation hint. Surface-only today: month / weekday
    // names and numeric separators stay en-US until OxFml's locale
    // tables land (`SEAM-OXFML-LOCALE-EXPAND`).
    let on_set_locale_preset = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(move |language_tag: String| {
            state.update(|s| {
                if crate::app::reducer::set_workspace_locale_preset(s, language_tag) {
                    refresh(s);
                }
            });
        })
    };
    let on_add_cf_rule = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(move |()| {
            state.update(|s| {
                let default_rule = crate::state::FormulaConditionalFormattingRule {
                    rule_kind: "cell_value".to_string(),
                    operator: Some("greaterThan".to_string()),
                    thresholds: vec!["0".to_string()],
                    font_color: None,
                    fill_color: Some("#ffe9b3".to_string()),
                    typed_rule: None,
                };
                if crate::app::reducer::add_active_conditional_formatting_rule(s, default_rule)
                    .is_some()
                {
                    refresh(s);
                }
            });
        })
    };
    let on_remove_cf_rule = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(move |index: usize| {
            state.update(|s| {
                if crate::app::reducer::remove_active_conditional_formatting_rule(s, index) {
                    refresh(s);
                }
            });
        })
    };
    let on_update_cf_rule = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(
            move |(index, rule): (usize, crate::state::FormulaConditionalFormattingRule)| {
                state.update(|s| {
                    if crate::app::reducer::update_active_conditional_formatting_rule(
                        s, index, rule,
                    ) {
                        refresh(s);
                    }
                });
            },
        )
    };
    // Manual recalculate trigger. Wired both to the F9 key
    // (handled in `on_textarea_keydown`) and to a small button in
    // the editor-foot row. In Deterministic policy this re-runs
    // the bridge against the same fixed seeds; in LiveRecalc the
    // bridge picks fresh seeds so volatile functions advance. In
    // ManualRecalc this is the *only* path that runs the runtime
    // pass — the keystroke-driven bridge skips it.
    let editor_bridge_for_recalc = editor_bridge.clone();
    let on_recalculate = Callback::new(move |()| {
        let bridge = editor_bridge_for_recalc.clone();
        state.update(|state| {
            if let Some(bridge) = bridge.as_ref() {
                let _ = crate::services::live_edit::force_runtime_recalc_on_active_formula_space(
                    bridge.as_ref(),
                    state,
                );
            }
        });
    });

    // Scenario breadcrumb dropdown lifecycle. The toggle is wired
    // to the breadcrumb button click. The close callback fires
    // from outside-click delegation in the shell and from `Esc`
    // in the global keydown handler.
    let on_scenario_breadcrumb_toggle = Callback::new(move |()| {
        state.update(|state| {
            let _ = toggle_scenario_breadcrumb(state);
        });
    });
    let on_scenario_breadcrumb_close = Callback::new(move |()| {
        state.update(|state| {
            let _ = close_scenario_breadcrumb(state);
        });
    });
    // Click on a Recent / Pinned row → switch to that formula. The
    // dropdown closes after the switch so the user gets visible
    // feedback. `reopen_formula_space` handles both the
    // open-but-not-active case (just flip active id) and the
    // closed-and-recent case (re-mount from `recent_formula_spaces`).
    let on_scenario_entry_select = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::reopen_formula_space(state, &formula_space_id);
            let _ = close_scenario_breadcrumb(state);
        });
    });
    // Pin glyph on a Recent / Pinned row → toggle the pin without
    // switching the active formula. Stops click propagation in the
    // renderer so the row's select handler doesn't also fire.
    let on_scenario_entry_pin_toggle = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::toggle_pin_formula_space(state, &formula_space_id);
        });
    });
    // Scenario action dispatcher (slice 1b). NewScenario / Duplicate
    // run synchronously through their existing reducers. SaveAs
    // projects the active formula to the persisted `Scenario` shape,
    // serialises to XML, and triggers a browser-native download.
    // Open spawns an async task that surfaces the file picker, reads
    // the chosen file, parses it, and inserts it into the workspace.
    // ManageScenarios is still a SEAM stub (no UI for the manage
    // page yet — that's a later slice).
    let on_scenario_action = Callback::new(move |action_id: ScenarioBreadcrumbActionId| {
        // Always close the dropdown so the user gets visible feedback
        // that the click was received, regardless of which action.
        let close_dropdown = || {
            state.update(|state| {
                let _ = close_scenario_breadcrumb(state);
            });
        };

        match action_id {
            ScenarioBreadcrumbActionId::NewScenario => {
                state.update(|state| {
                    let _ = crate::app::case_lifecycle::new_formula_space(state);
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::Duplicate => {
                state.update(|state| {
                    let _ = crate::app::case_lifecycle::clone_active_formula_space(state);
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::PinActive => {
                state.update(|state| {
                    let _ = crate::app::case_lifecycle::pin_active_formula_space(state);
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::UnpinActive => {
                state.update(|state| {
                    if let Some(active_id) = state.workspace_shell.active_formula_space_id.clone() {
                        let _ = crate::app::case_lifecycle::unpin_formula_space(
                            state,
                            active_id.as_str(),
                        );
                    }
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::SaveAs => {
                #[cfg(target_arch = "wasm32")]
                {
                    let payload = state.with_untracked(|s| build_save_payload(s));
                    if let Some((filename, xml)) = payload {
                        match crate::persistence::save_xml_via_download(&filename, &xml) {
                            Ok(()) => {
                                // Save established the canonical
                                // dna: extension on disk, so any
                                // "imported from Excel-only" warning
                                // is no longer accurate. Clear it.
                                state.update(|s| {
                                    if let Some(active_id) =
                                        s.workspace_shell.active_formula_space_id.clone()
                                    {
                                        if let Some(formula_space) =
                                            s.formula_spaces.get_mut(&active_id)
                                        {
                                            formula_space.load_diagnostics.clear();
                                        }
                                    }
                                });
                            }
                            Err(error) => {
                                web_sys::console::error_1(
                                    &format!("[onecalc] save failed: {error}").into(),
                                );
                            }
                        }
                    }
                }
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::Open => {
                #[cfg(target_arch = "wasm32")]
                {
                    let state = state;
                    wasm_bindgen_futures::spawn_local(async move {
                        match crate::persistence::open_xml_via_file_input().await {
                            Ok(Some(opened)) => {
                                match crate::persistence::read_formula_xml(&opened.xml) {
                                    Ok(loaded) => {
                                        state.update(|s| {
                                        let _ =
                                            crate::app::case_lifecycle::open_loaded_scenario_into_workspace(
                                                s, loaded,
                                            );
                                    });
                                    }
                                    Err(error) => {
                                        web_sys::console::error_1(
                                            &format!(
                                                "[onecalc] failed to parse `{}`: {error}",
                                                opened.filename,
                                            )
                                            .into(),
                                        );
                                    }
                                }
                            }
                            Ok(None) => {
                                // user cancelled — no-op
                            }
                            Err(error) => {
                                web_sys::console::error_1(
                                    &format!("[onecalc] open dialog failed: {error}").into(),
                                );
                            }
                        }
                    });
                }
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::ManageScenarios => {
                // SEAM-ONECALC-SCENARIO-PERSIST — full-screen
                // management page lands in a later slice; for now
                // just close the dropdown so the user sees the click
                // was received.
                web_sys::console::log_1(&"[onecalc] manage formulas: pending UI slice".into());
                close_dropdown();
            }
        }
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
            on:keydown=move |ev: WebKeyboardEvent| {
                if ev.key() == "Escape"
                    && state.with_untracked(|s| s.global_ui_chrome.scenario_breadcrumb_open)
                {
                    on_scenario_breadcrumb_close.run(());
                    return;
                }
                // F9 — recalculate the active formula. Bound at the
                // shell level (rather than the textarea) so it works
                // when focus is on the formatting panel, the
                // recalc button, or anywhere else inside the shell.
                // `preventDefault` shadows Firefox's "find again"
                // browser default.
                if !ev.ctrl_key()
                    && !ev.shift_key()
                    && !ev.alt_key()
                    && ev.key() == "F9"
                {
                    ev.prevent_default();
                    on_recalculate.run(());
                    return;
                }
                // Ctrl+K — command palette (placeholder; opens once
                // the palette UI lands). Ctrl+P collides with the
                // browser's print dialog, so the canonical chord is
                // Ctrl+K (modern app convention) plus Ctrl+Shift+P
                // as a discoverable secondary chord. Today both are
                // wired but produce a no-op until
                // `services::command_palette` lands.
                if ev.ctrl_key()
                    && !ev.alt_key()
                    && (ev.key() == "k"
                        || ev.key() == "K"
                        || (ev.shift_key() && (ev.key() == "p" || ev.key() == "P")))
                {
                    ev.prevent_default();
                    // SEAM-ONECALC-COMMAND-PALETTE — palette wiring
                    // pending. Today this is a no-op; the chord is
                    // claimed so the user can rely on muscle memory
                    // when the palette ships.
                    return;
                }
            }
            on:click=move |ev: WebMouseEvent| {
                if !state.with_untracked(|s| s.global_ui_chrome.scenario_breadcrumb_open) {
                    return;
                }
                let inside_breadcrumb = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .and_then(|el| el.closest(".onecalc-home-shell__breadcrumb-wrap").ok().flatten())
                    .is_some();
                if !inside_breadcrumb {
                    on_scenario_breadcrumb_close.run(());
                }
            }
        >
            <header class="onecalc-home-shell__titlebar">
                <span class="onecalc-home-shell__brand">"DnaOneCalc"</span>
                {move || render_scenario_breadcrumb(
                    scenario_breadcrumb(),
                    on_scenario_breadcrumb_toggle,
                    on_scenario_breadcrumb_close,
                    on_scenario_action,
                    on_scenario_entry_select,
                    on_scenario_entry_pin_toggle,
                )}
                <span class="onecalc-home-shell__titlebar-hint" aria-hidden="true">
                    "Ctrl+P · command palette"
                </span>
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
                                {move || render_syntax_overlay(
                                    syntax_runs(),
                                    textarea_value(),
                                    bracket_pair_highlight(),
                                )}
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
                                node_ref=textarea_ref
                                on:keydown=move |ev| on_textarea_keydown.run(ev)
                                on:focusout=on_textarea_focusout
                                on:keyup=move |ev: WebKeyboardEvent| {
                                    // Caret-only navigation keys (arrows, Home,
                                    // End, PageUp/Down) don't fire `on:input` —
                                    // browser moves the caret natively. Fire a
                                    // synthetic `EditorInputEvent` with the
                                    // current text + new selection so the
                                    // bridge re-runs and popups update against
                                    // the new caret position. Filtering by key
                                    // avoids double-firing on text-input keys
                                    // (those go through `on:input`).
                                    if !is_caret_navigation_key(&ev.key()) {
                                        return;
                                    }
                                    if let Some(textarea) = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok())
                                    {
                                        on_editor_input.run(synthesize_caret_sync_event(&textarea));
                                    }
                                }
                                on:click=move |ev: WebMouseEvent| {
                                    // Mouse-click positions the caret. Same
                                    // synthesis path so popups reflect the new
                                    // caret position.
                                    if let Some(textarea) = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok())
                                    {
                                        on_editor_input.run(synthesize_caret_sync_event(&textarea));
                                    }
                                }
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
                            {render_recalculate_button(on_recalculate)}
                        </div>
                    </section>

                    <section class="onecalc-home-shell__formula-drill-section">
                        {move || render_formula_drill_panel(
                            formula_drill(),
                            view_mode(),
                            on_view_mode_toggle,
                        )}
                    </section>

                    <section class="onecalc-home-shell__formatting-section">
                        {move || render_formatting_panel(
                            formatting_controls(),
                            on_formatting_panel_toggle,
                            on_set_number_format_code,
                            on_set_font_color,
                            on_set_fill_color,
                            on_set_date1904,
                            on_set_scenario_policy,
                            on_set_locale_preset,
                            on_add_cf_rule,
                            on_remove_cf_rule,
                            on_update_cf_rule,
                        )}
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
/// Render the titlebar scenario-breadcrumb button + dropdown.
///
/// The button is always rendered when there is an active formula
/// space (the view-model returns `None` when none is active and
/// this helper short-circuits to an empty span). The dropdown
/// menu is keyboard-focusable; Esc inside it closes via the
/// `on_close` callback. Outside-click is handled by the document
/// listener wired in the parent component.
fn render_scenario_breadcrumb(
    breadcrumb: Option<ScenarioBreadcrumbView>,
    on_toggle: Callback<()>,
    on_close: Callback<()>,
    on_action: Callback<ScenarioBreadcrumbActionId>,
    on_entry_select: Callback<String>,
    on_entry_pin_toggle: Callback<String>,
) -> AnyView {
    let Some(breadcrumb) = breadcrumb else {
        return view! { <span class="onecalc-home-shell__breadcrumb-wrap" /> }.into_any();
    };
    let dirty_attr = if breadcrumb.is_dirty { "true" } else { "false" };
    let open_attr = if breadcrumb.is_open { "true" } else { "false" };
    let aria_expanded = if breadcrumb.is_open { "true" } else { "false" };
    let aria_hidden = if breadcrumb.is_open { "false" } else { "true" };
    let label = breadcrumb.active_label.clone();
    let label_for_button = label.clone();
    let recent = breadcrumb.recent.clone();
    let pinned = breadcrumb.pinned.clone();
    let actions = breadcrumb.actions.clone();
    view! {
        <span
            class="onecalc-home-shell__breadcrumb-wrap"
            data-open=open_attr
        >
            <button
                type="button"
                class="onecalc-home-shell__breadcrumb-button"
                data-dirty=dirty_attr
                aria-haspopup="menu"
                aria-expanded=aria_expanded
                aria-label=format!("formula: {}", label_for_button)
                on:click=move |_| {
                    on_toggle.run(());
                }
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        ev.prevent_default();
                        on_close.run(());
                    }
                }
            >
                <span class="onecalc-home-shell__breadcrumb-dot" aria-hidden="true"></span>
                <span class="onecalc-home-shell__breadcrumb-label">{label}</span>
                <span class="onecalc-home-shell__breadcrumb-caret" aria-hidden="true">"▾"</span>
            </button>
            <div
                class="onecalc-home-shell__scenario-menu"
                role="menu"
                aria-hidden=aria_hidden
                data-open=open_attr
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        ev.prevent_default();
                        on_close.run(());
                    }
                }
            >
                <div class="onecalc-home-shell__scenario-menu-section" data-section="recent">
                    <div class="onecalc-home-shell__scenario-menu-heading">"Recent"</div>
                    {render_scenario_menu_entries(recent, "recent", on_entry_select, on_entry_pin_toggle)}
                </div>
                <div class="onecalc-home-shell__scenario-menu-section" data-section="pinned">
                    <div class="onecalc-home-shell__scenario-menu-heading">"Pinned"</div>
                    {render_scenario_menu_entries(pinned, "pinned", on_entry_select, on_entry_pin_toggle)}
                </div>
                <div class="onecalc-home-shell__scenario-menu-section" data-section="actions">
                    <div class="onecalc-home-shell__scenario-menu-heading">"Actions"</div>
                    {render_scenario_menu_actions(actions, on_action)}
                </div>
            </div>
        </span>
    }
    .into_any()
}

fn render_scenario_menu_entries(
    entries: Vec<ScenarioBreadcrumbEntry>,
    section: &'static str,
    on_entry_select: Callback<String>,
    on_entry_pin_toggle: Callback<String>,
) -> AnyView {
    if entries.is_empty() {
        return view! {
            <div
                class="onecalc-home-shell__scenario-menu-empty"
                data-section=section
            >
                {match section {
                    "pinned" => "No pinned formulas",
                    "recent" => "No recent formulas",
                    _ => "(empty)",
                }}
            </div>
        }
        .into_any();
    }
    let rows: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            let is_active_attr = if entry.is_active { "true" } else { "false" };
            let is_pinned_attr = if entry.is_pinned { "true" } else { "false" };
            let formula_space_id = entry.formula_space_id.clone();
            let display_name = entry.display_name.clone();
            let meta = entry.meta.clone();
            // Click row → switch to this formula. Click pin glyph
            // → toggle pin without switching. The two actions are
            // separate buttons to keep the click target unambiguous
            // (the pin glyph stops propagation so the row's click
            // handler doesn't also fire).
            let id_for_select = formula_space_id.clone();
            let id_for_pin = formula_space_id.clone();
            let pin_title = if entry.is_pinned { "Unpin" } else { "Pin" };
            let pin_glyph = if entry.is_pinned { "★" } else { "☆" };
            let id_for_outer = formula_space_id.clone();
            view! {
                <div
                    class="onecalc-home-shell__scenario-menu-row"
                    data-formula-space-id=id_for_outer
                    data-is-active=is_active_attr
                    data-is-pinned=is_pinned_attr
                    data-section=section
                >
                    <button
                        type="button"
                        class="onecalc-home-shell__scenario-menu-item"
                        role="menuitem"
                        data-formula-space-id=formula_space_id
                        data-is-active=is_active_attr
                        data-is-pinned=is_pinned_attr
                        data-section=section
                        on:click=move |_| on_entry_select.run(id_for_select.clone())
                    >
                        <span class="onecalc-home-shell__scenario-menu-item-name">
                            {display_name}
                        </span>
                        <span class="onecalc-home-shell__scenario-menu-item-meta">
                            {meta}
                        </span>
                    </button>
                    <button
                        type="button"
                        class="onecalc-home-shell__scenario-menu-pin"
                        data-action="pin-toggle"
                        data-is-pinned=is_pinned_attr
                        title=pin_title
                        aria-label=pin_title
                        on:click=move |ev| {
                            ev.stop_propagation();
                            on_entry_pin_toggle.run(id_for_pin.clone());
                        }
                    >
                        {pin_glyph}
                    </button>
                </div>
            }
            .into_any()
        })
        .collect();
    view! { <>{rows}</> }.into_any()
}

fn render_scenario_menu_actions(
    actions: Vec<ScenarioBreadcrumbAction>,
    on_action: Callback<ScenarioBreadcrumbActionId>,
) -> AnyView {
    let buttons: Vec<_> = actions
        .into_iter()
        .map(|action| {
            let action_id = action.action_id;
            let chord = action.chord_label;
            let label = action.label;
            let seam = action.seam_id;
            let title = seam.map(|s| format!("Pending: {s}")).unwrap_or_default();
            view! {
                <button
                    type="button"
                    class="onecalc-home-shell__scenario-menu-item"
                    role="menuitem"
                    data-action-id=action_id.slug()
                    data-section="actions"
                    data-seam-id=seam.unwrap_or("")
                    title=title
                    on:click=move |_| {
                        on_action.run(action_id);
                    }
                >
                    <span class="onecalc-home-shell__scenario-menu-item-name">
                        {label}
                    </span>
                    <span class="onecalc-home-shell__scenario-menu-item-meta">
                        {if chord.is_empty() {
                            seam.map(|s| s.to_string()).unwrap_or_default()
                        } else {
                            chord.to_string()
                        }}
                    </span>
                </button>
            }
            .into_any()
        })
        .collect();
    view! { <>{buttons}</> }.into_any()
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
    let scenario_label = status.scenario_label.clone();
    let scenario_label_attr = scenario_label.clone();
    let load_diagnostics = status.load_diagnostics.clone();
    view! {
        <span class="onecalc-home-shell__statusfoot-dot" data-health=dot_health></span>
        <span>"live-bridge"</span>
        <span class="onecalc-home-shell__statusfoot-sep">"·"</span>
        <span>{format!("green-tree {green_key}")}</span>
        <span class="onecalc-home-shell__statusfoot-sep">"·"</span>
        <span class="onecalc-home-shell__statusfoot-scenario">
            "formula · "
            <span
                class="onecalc-home-shell__statusfoot-scenario-name"
                data-scenario-label=scenario_label_attr
            >
                {scenario_label}
            </span>
        </span>
        {render_load_diagnostic_chips(load_diagnostics)}
    }
    .into_any()
}

/// Render the WS-14 §5.3-item-8 collapsible formatting panel that sits
/// between the formula drill-down and the result section. The panel
/// has two surfaces:
///
/// * **Collapsed (default)**: a single `format ▸ <summary>` chip the
///   user can click to expand. The summary string comes from
///   `FormattingControlsView.summary` and reads e.g.
///   `"General"` (defaults), `"Currency"` (matched preset), or
///   `"$#,##0.00 · font #ff0000 · Date1904"` (multi-override).
/// * **Expanded**: the full `render_formatting_controls` row plus a
///   ▾ caption that flips back to ▸ when the user clicks again.
///
/// The chip emits `data-formatting-panel-expanded` (`"true" | "false"`)
/// and `data-formatting-summary` so the browser corpus can pin both.
fn render_formatting_panel(
    controls: Option<FormattingControlsView>,
    on_toggle: Callback<()>,
    on_set_number_format_code: Callback<String>,
    on_set_font_color: Callback<String>,
    on_set_fill_color: Callback<String>,
    on_set_date1904: Callback<bool>,
    on_set_scenario_policy: Callback<crate::persistence::ScenarioPolicy>,
    on_set_locale_preset: Callback<String>,
    on_add_cf_rule: Callback<()>,
    on_remove_cf_rule: Callback<usize>,
    on_update_cf_rule: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let Some(controls) = controls else {
        return view! { <></> }.into_any();
    };
    let is_open = controls.is_open;
    let summary = controls.summary.clone();
    let summary_for_attr = summary.clone();
    let expanded_attr = if is_open { "true" } else { "false" };
    let aria_expanded = expanded_attr;
    let aria_hidden_body = if is_open { "false" } else { "true" };
    let toggle_label = if is_open {
        format!("▾ format ▸ {}", summary)
    } else {
        format!("▸ format ▸ {}", summary)
    };
    view! {
        <div
            class="onecalc-home-shell__formatting-panel"
            data-formatting-panel-expanded=expanded_attr
            data-formatting-summary=summary_for_attr
        >
            <button
                type="button"
                class="onecalc-home-shell__formatting-toggle-button"
                data-expanded=expanded_attr
                aria-expanded=aria_expanded
                aria-controls="onecalc-formatting-panel-body"
                on:click=move |_| on_toggle.run(())
            >
                {toggle_label}
            </button>
            <div
                id="onecalc-formatting-panel-body"
                class="onecalc-home-shell__formatting-panel-body"
                data-expanded=expanded_attr
                aria-hidden=aria_hidden_body
            >
                {if is_open {
                    render_formatting_controls(
                        controls,
                        on_set_number_format_code,
                        on_set_font_color,
                        on_set_fill_color,
                        on_set_date1904,
                        on_set_scenario_policy,
                        on_set_locale_preset,
                        on_add_cf_rule,
                        on_remove_cf_rule,
                        on_update_cf_rule,
                    )
                } else {
                    view! { <></> }.into_any()
                }}
            </div>
        </div>
    }
    .into_any()
}

/// Render the formatting-controls body. Three rows:
///
/// 1. **Format row** — number-format text input + the full family
///    preset chip strip + font / fill colour pickers + Date1904
///    toggle.
/// 2. **Calc-options row** — Deterministic / LiveRecalc segmented
///    control. Drives `now_serial` / `random_value` seeding for the
///    bridge.
/// 3. **Conditional formatting** — list of rules with per-rule
///    remove + a `+ add rule` affordance.
fn render_formatting_controls(
    controls: FormattingControlsView,
    on_set_number_format_code: Callback<String>,
    on_set_font_color: Callback<String>,
    on_set_fill_color: Callback<String>,
    on_set_date1904: Callback<bool>,
    on_set_scenario_policy: Callback<crate::persistence::ScenarioPolicy>,
    on_set_locale_preset: Callback<String>,
    on_add_cf_rule: Callback<()>,
    on_remove_cf_rule: Callback<usize>,
    on_update_cf_rule: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let number_format_code_value = controls.number_format_code.clone();
    let number_format_code_attr = number_format_code_value.clone();
    let font_color_value = controls.font_color.clone();
    let fill_color_value = controls.fill_color.clone();
    let date1904 = controls.date1904;
    let scenario_policy = controls.scenario_policy;
    let cf_rules = controls.conditional_formatting_rules.clone();
    let locale_language_tag = controls.locale_language_tag.clone();
    let locale_presets = controls.locale_presets.clone();
    let locale_seam_id_for_panel = controls.locale_seam_id;
    view! {
        <div class="onecalc-home-shell__formatting-rows" role="group" aria-label="formula formatting">
            <div class="onecalc-home-shell__formatting-row">
                <span class="onecalc-home-shell__formatting-caption">"format ▸"</span>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"number format"</span>
                    <input
                        type="text"
                        class="onecalc-home-shell__formatting-input"
                        data-formatting-field="number-format-code"
                        placeholder="General"
                        prop:value=number_format_code_value
                        value=number_format_code_attr
                        on:input=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_set_number_format_code.run(target.value());
                        }
                    />
                </label>
                <span class="onecalc-home-shell__formatting-presets">
                    {render_number_format_presets(
                        controls.number_format_presets.clone(),
                        on_set_number_format_code,
                    )}
                </span>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"font color"</span>
                    <input
                        type="color"
                        class="onecalc-home-shell__formatting-color"
                        data-formatting-field="font-color"
                        prop:value=move || normalize_color_for_input(&font_color_value)
                        value=normalize_color_for_input(&controls.font_color)
                        on:input=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_set_font_color.run(target.value());
                        }
                    />
                </label>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"fill color"</span>
                    <input
                        type="color"
                        class="onecalc-home-shell__formatting-color"
                        data-formatting-field="fill-color"
                        prop:value=move || normalize_color_for_input(&fill_color_value)
                        value=normalize_color_for_input(&controls.fill_color)
                        on:input=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_set_fill_color.run(target.value());
                        }
                    />
                </label>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"1904 dates"</span>
                    <input
                        type="checkbox"
                        class="onecalc-home-shell__formatting-toggle"
                        data-formatting-field="date1904"
                        prop:checked=date1904
                        on:change=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_set_date1904.run(target.checked());
                        }
                    />
                </label>
            </div>
            <div class="onecalc-home-shell__formatting-row" data-formatting-row="calc-options">
                <span class="onecalc-home-shell__formatting-caption">"calc ▸"</span>
                {render_scenario_policy_toggle(scenario_policy, on_set_scenario_policy)}
                {render_locale_picker(&locale_language_tag, &locale_presets, locale_seam_id_for_panel, on_set_locale_preset)}
            </div>
            <div class="onecalc-home-shell__formatting-row" data-formatting-row="cf-rules">
                <span class="onecalc-home-shell__formatting-caption">"CF ▸"</span>
                {render_conditional_formatting_section(
                    cf_rules,
                    on_add_cf_rule,
                    on_remove_cf_rule,
                    on_update_cf_rule,
                )}
            </div>
        </div>
    }
    .into_any()
}

/// Read-only locale chip rendered inside the formatting panel's
/// calc-options row. Shows the workspace's ambient-locale label
/// (derived from `navigator.language`) plus a SEAM marker when
/// OxFml's per-locale tables aren't yet rendered. Becomes a
/// proper picker once the locale-expansion handoff lands.
/// Workspace-locale picker. Replaces the prior read-only chip with
/// a `<select>` that updates the workspace's `AmbientAppContext`
/// (date / datetime / time format-code triple). The SEAM badge
/// continues to flag the runtime locale-table gap upstream — month
/// and weekday names, numeric separators, and currency symbols stay
/// en-US until OxFml lands `SEAM-OXFML-LOCALE-EXPAND`. The dropdown
/// is functional today: switching to e.g. `de-DE` flips the
/// presentation-hint default from `m/d/yyyy h:mm:ss AM/PM` to
/// `dd.mm.yyyy HH:mm:ss` immediately.
fn render_locale_picker(
    language_tag: &str,
    presets: &[(&'static str, &'static str)],
    seam_id: Option<&'static str>,
    on_set_locale_preset: Callback<String>,
) -> AnyView {
    let seam_attr = seam_id.unwrap_or("").to_string();
    let title = match seam_id {
        Some(seam) => format!("Workspace locale (runtime tables pending: {seam})",),
        None => "Workspace locale".to_string(),
    };
    let seam_badge = seam_id.map(|seam| {
        view! {
            <span class="onecalc-home-shell__formatting-locale-seam"
                data-seam-id=seam.to_string()
                title=format!("<NOT IMPLEMENTED> {seam}")
            >"⚠"</span>
        }
        .into_any()
    });
    let current = language_tag.to_string();
    let options: Vec<_> = presets
        .iter()
        .map(|(tag, label)| {
            let tag_str = (*tag).to_string();
            let label_str = (*label).to_string();
            let selected = current == *tag;
            view! {
                <option value=tag_str.clone() selected=selected>
                    {format!("{label_str} ({tag_str})")}
                </option>
            }
            .into_any()
        })
        .collect();
    view! {
        <label
            class="onecalc-home-shell__formatting-locale"
            data-seam-id=seam_attr
            title=title
        >
            <span class="onecalc-home-shell__formatting-field-label">"locale"</span>
            <select
                class="onecalc-home-shell__formatting-locale-select"
                data-formatting-field="locale-language-tag"
                on:change=move |ev| {
                    let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                    on_set_locale_preset.run(target.value());
                }
            >
                {options}
            </select>
            {seam_badge}
        </label>
    }
    .into_any()
}

/// Three-button segmented control: Deterministic | Live | Manual.
/// Clicking the inactive button switches the active formula's
/// scenario policy; the active button is highlighted via the
/// `[data-active="true"]` selector. Manual recalc is the user's
/// lever for keeping the editor responsive when the formula is
/// expensive (large REDUCE / MAKEARRAY / LAMBDA workloads); typing
/// runs parse / bind / popups every keystroke but skips the
/// runtime-evaluation pass until F9 / Calculate.
fn render_scenario_policy_toggle(
    current: ScenarioPolicyView,
    on_set: Callback<crate::persistence::ScenarioPolicy>,
) -> AnyView {
    let is_deterministic = matches!(current, ScenarioPolicyView::Deterministic);
    let is_live = matches!(current, ScenarioPolicyView::LiveRecalc);
    let is_manual = matches!(current, ScenarioPolicyView::ManualRecalc);
    view! {
        <div
            class="onecalc-home-shell__formatting-policy-toggle"
            role="group"
            aria-label="scenario calc-options policy"
        >
            <button
                type="button"
                class="onecalc-home-shell__formatting-policy-button"
                data-policy="deterministic"
                data-active=if is_deterministic { "true" } else { "false" }
                aria-pressed=if is_deterministic { "true" } else { "false" }
                title="Pin NOW / RAND seeds for reproducible authoring"
                on:click=move |_| on_set.run(crate::persistence::ScenarioPolicy::Deterministic)
            >
                "Deterministic"
            </button>
            <button
                type="button"
                class="onecalc-home-shell__formatting-policy-button"
                data-policy="live"
                data-active=if is_live { "true" } else { "false" }
                aria-pressed=if is_live { "true" } else { "false" }
                title="NOW advances per round-trip; RAND rolls each time"
                on:click=move |_| on_set.run(crate::persistence::ScenarioPolicy::LiveRecalc)
            >
                "Live"
            </button>
            <button
                type="button"
                class="onecalc-home-shell__formatting-policy-button"
                data-policy="manual"
                data-active=if is_manual { "true" } else { "false" }
                aria-pressed=if is_manual { "true" } else { "false" }
                title="Skip runtime evaluation on text edits; recalc on F9 / Calculate only"
                on:click=move |_| on_set.run(crate::persistence::ScenarioPolicy::ManualRecalc)
            >
                "Manual"
            </button>
        </div>
    }
    .into_any()
}

/// Render the conditional-formatting rules section: zero or more
/// rule cards followed by a `+ add rule` button. Each rule card
/// lets the user edit operator / threshold / font / fill inline,
/// and remove the rule. SEAM-marked rule kinds (color scales, data
/// bars, icon sets, …) render with a `<NOT IMPLEMENTED>` chip.
fn render_conditional_formatting_section(
    rules: Vec<ConditionalFormattingRuleView>,
    on_add: Callback<()>,
    on_remove: Callback<usize>,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let rule_cards: Vec<_> = rules
        .into_iter()
        .enumerate()
        .map(|(index, rule)| render_cf_rule_card(index, rule, on_remove, on_update))
        .collect();
    view! {
        <div class="onecalc-home-shell__cf-rules" role="group" aria-label="conditional formatting rules">
            {rule_cards}
            <button
                type="button"
                class="onecalc-home-shell__cf-add-button"
                title="Add a default cell-value > 0 rule; edit thresholds / colours inline"
                on:click=move |_| on_add.run(())
            >
                "+ add rule"
            </button>
        </div>
    }
    .into_any()
}

fn render_cf_rule_card(
    index: usize,
    rule: ConditionalFormattingRuleView,
    on_remove: Callback<usize>,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let rule_kind_value = rule.rule_kind.clone();
    let operator_value = rule.operator.clone().unwrap_or_default();
    let threshold_value = rule.thresholds.first().cloned().unwrap_or_default();
    let font_color_value = rule.font_color.clone().unwrap_or_default();
    let fill_color_value = rule.fill_color.clone().unwrap_or_default();
    let seam_badge = rule.seam_id.map(|seam| {
        view! {
            <span
                class="onecalc-home-shell__cf-rule-seam"
                data-seam-id=seam.to_string()
                title=format!("<NOT IMPLEMENTED> {seam}")
            >"⚠ NOT IMPL"</span>
        }
        .into_any()
    });
    let rule_for_kind = rule.clone();
    let rule_for_op = rule.clone();
    let rule_for_threshold = rule.clone();
    let rule_for_font = rule.clone();
    let rule_for_fill = rule.clone();
    view! {
        <div
            class="onecalc-home-shell__cf-rule"
            data-cf-rule-index=index.to_string()
            data-cf-rule-kind=rule.rule_kind.clone()
        >
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"kind"</span>
                <select
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-rule-field="kind"
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                        let mut next = host_cf_rule_from_view(&rule_for_kind);
                        next.rule_kind = target.value();
                        seed_visualization_rule_defaults(&mut next);
                        on_update.run((index, next));
                    }
                >
                    <optgroup label="Per-cell">
                        <option value="cell_value" selected=rule_kind_value == "cell_value">"cell value"</option>
                        <option value="text" selected=rule_kind_value == "text">"text"</option>
                        <option value="dates" selected=rule_kind_value == "dates">"dates"</option>
                        <option value="blanks" selected=rule_kind_value == "blanks">"blanks"</option>
                        <option value="noBlanks" selected=rule_kind_value == "noBlanks">"no blanks"</option>
                        <option value="errors" selected=rule_kind_value == "errors">"errors"</option>
                        <option value="noErrors" selected=rule_kind_value == "noErrors">"no errors"</option>
                        <option value="expression" selected=rule_kind_value == "expression">"expression"</option>
                    </optgroup>
                    <optgroup label="Aggregate (array as range)">
                        <option value="colorScale" selected=rule_kind_value == "colorScale">"color scale"</option>
                        <option value="dataBar" selected=rule_kind_value == "dataBar">"data bar"</option>
                        <option value="iconSet" selected=rule_kind_value == "iconSet">"icon set"</option>
                        <option value="aboveAverage" selected=rule_kind_value == "aboveAverage">"above average"</option>
                        <option value="belowAverage" selected=rule_kind_value == "belowAverage">"below average"</option>
                        <option value="top" selected=rule_kind_value == "top">"top N"</option>
                        <option value="bottom" selected=rule_kind_value == "bottom">"bottom N"</option>
                        <option value="uniqueValues" selected=rule_kind_value == "uniqueValues">"unique values"</option>
                        <option value="duplicateValues" selected=rule_kind_value == "duplicateValues">"duplicate values"</option>
                    </optgroup>
                </select>
            </label>
            {render_cf_rule_operator_dropdown(rule.rule_kind.clone(), operator_value.clone(), index, rule_for_op, on_update)}
            {render_cf_rule_threshold_control(rule.rule_kind.clone(), threshold_value, index, rule_for_threshold, on_update)}
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"font"</span>
                <input
                    type="color"
                    class="onecalc-home-shell__formatting-color"
                    data-cf-rule-field="font-color"
                    prop:value=normalize_color_for_input(&font_color_value)
                    value=normalize_color_for_input(&font_color_value)
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let mut next = host_cf_rule_from_view(&rule_for_font);
                        next.font_color = Some(target.value());
                        on_update.run((index, next));
                    }
                />
            </label>
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"fill"</span>
                <input
                    type="color"
                    class="onecalc-home-shell__formatting-color"
                    data-cf-rule-field="fill-color"
                    prop:value=normalize_color_for_input(&fill_color_value)
                    value=normalize_color_for_input(&fill_color_value)
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let mut next = host_cf_rule_from_view(&rule_for_fill);
                        next.fill_color = Some(target.value());
                        on_update.run((index, next));
                    }
                />
            </label>
            {seam_badge}
            <button
                type="button"
                class="onecalc-home-shell__cf-rule-remove"
                title="Remove this rule"
                aria-label="remove conditional formatting rule"
                on:click=move |_| on_remove.run(index)
            >
                "✕"
            </button>
            {render_cf_rule_typed_subform(rule.clone(), index, on_update)}
        </div>
    }
    .into_any()
}

/// Render the operator dropdown for a CF rule card. Operator
/// strings are the canonical Excel CF names that OxFml's
/// `evaluate_operator_rule` matches after stripping non-alphanumerics
/// and lowercasing — so `greaterThan` / `greaterThanOrEqual` /
/// `lessThan` / `lessThanOrEqual` / `equal` rather than abbreviated
/// `gt` / `gte` / `lt` / `lte` / `eq` (which OxFml does not match).
///
/// Predicate rule kinds (`blanks` / `noBlanks` / `errors` /
/// `noErrors` / `dates` / `expression`) and visualization rule
/// kinds (`colorScale` / `dataBar` / `iconSet` /
/// `aboveAverage` / `belowAverage` / `uniqueValues` /
/// `duplicateValues`) don't take an operator — the kind itself
/// is the predicate or the aggregate-context computation. The
/// dropdown collapses to a no-op span in those cases.
fn render_cf_rule_operator_dropdown(
    rule_kind: String,
    operator_value: String,
    index: usize,
    rule_for_op: ConditionalFormattingRuleView,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    if matches!(
        rule_kind.to_ascii_lowercase().as_str(),
        "blanks"
            | "noblanks"
            | "errors"
            | "noerrors"
            | "dates"
            | "expression"
            | "colorscale"
            | "databar"
            | "iconset"
            | "aboveaverage"
            | "belowaverage"
            | "top"
            | "bottom"
            | "uniquevalues"
            | "duplicatevalues"
    ) {
        return view! { <span></span> }.into_any();
    }
    view! {
        <label class="onecalc-home-shell__cf-rule-field">
            <span class="onecalc-home-shell__cf-rule-field-label">"op"</span>
            <select
                class="onecalc-home-shell__cf-rule-input"
                data-cf-rule-field="operator"
                on:change=move |ev| {
                    let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                    let mut next = host_cf_rule_from_view(&rule_for_op);
                    let raw = target.value();
                    next.operator = if raw.is_empty() { None } else { Some(raw) };
                    on_update.run((index, next));
                }
            >
                <option value="greaterThan" selected=operator_value == "greaterThan">">"</option>
                <option value="greaterThanOrEqual" selected=operator_value == "greaterThanOrEqual">"≥"</option>
                <option value="lessThan" selected=operator_value == "lessThan">"<"</option>
                <option value="lessThanOrEqual" selected=operator_value == "lessThanOrEqual">"≤"</option>
                <option value="equal" selected=operator_value == "equal">"="</option>
                <option value="notEqual" selected=operator_value == "notEqual">"≠"</option>
                <option value="between" selected=operator_value == "between">"between"</option>
                <option value="notBetween" selected=operator_value == "notBetween">"not between"</option>
                <option value="containsText" selected=operator_value == "containsText">"contains"</option>
                <option value="notContainsText" selected=operator_value == "notContainsText">"not contains"</option>
                <option value="beginsWith" selected=operator_value == "beginsWith">"begins with"</option>
                <option value="endsWith" selected=operator_value == "endsWith">"ends with"</option>
            </select>
        </label>
    }
    .into_any()
}

/// Render the threshold control, adapting to the rule kind:
///
/// - **`dates`** → a relative-date dropdown matching the W070
///   landed predicates (today / yesterday / tomorrow / last 7
///   days / this week / last week / next week / this month /
///   last month / next month). The selected value is stored as
///   `thresholds[0]` so OxFml can dispatch.
/// - **`blanks` / `noBlanks` / `errors` / `noErrors`** → no
///   control (predicate fires from the kind alone).
/// - **`expression`** → free-text input for the formula body.
/// - **`top` / `bottom`** → numeric input for the count or
///   percentage.
/// - **`colorScale` / `dataBar` / `iconSet` / `aboveAverage` /
///   `belowAverage` / `uniqueValues` / `duplicateValues`** →
///   no control (aggregate-context computation; the array
///   itself is the input). SEAM-marked because OxFml hasn't
///   landed the aggregate evaluation yet — see
///   `docs/HANDOFF_OXFML_CF_AGGREGATE_VISUALIZATION_RULES.md`.
/// - **everything else** (`cell_value`, `text`, etc.) → free-text
///   numeric / textual threshold.
fn render_cf_rule_threshold_control(
    rule_kind: String,
    threshold_value: String,
    index: usize,
    rule_for_threshold: ConditionalFormattingRuleView,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let kind_lc = rule_kind.to_ascii_lowercase();
    match kind_lc.as_str() {
        "blanks" | "noblanks" | "errors" | "noerrors" => view! { <span></span> }.into_any(),
        // W073-typed families (`top` / `bottom` included) own their
        // configuration through the per-kind sub-form; the bounded
        // `thresholds` field is upstream-ignored, so the threshold
        // control above the subform would just confuse the user.
        "colorscale"
        | "databar"
        | "iconset"
        | "aboveaverage"
        | "belowaverage"
        | "top"
        | "bottom"
        | "uniquevalues"
        | "duplicatevalues" => view! { <span></span> }.into_any(),
        "dates" => {
            view! {
                <label class="onecalc-home-shell__cf-rule-field">
                    <span class="onecalc-home-shell__cf-rule-field-label">"when"</span>
                    <select
                        class="onecalc-home-shell__cf-rule-input"
                        data-cf-rule-field="threshold"
                        on:change=move |ev| {
                            let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                            let mut next = host_cf_rule_from_view(&rule_for_threshold);
                            next.thresholds = vec![target.value()];
                            on_update.run((index, next));
                        }
                    >
                        <option value="today" selected=threshold_value == "today">"today"</option>
                        <option value="yesterday" selected=threshold_value == "yesterday">"yesterday"</option>
                        <option value="tomorrow" selected=threshold_value == "tomorrow">"tomorrow"</option>
                        <option value="last7Days" selected=threshold_value == "last7Days">"last 7 days"</option>
                        <option value="thisWeek" selected=threshold_value == "thisWeek">"this week"</option>
                        <option value="lastWeek" selected=threshold_value == "lastWeek">"last week"</option>
                        <option value="nextWeek" selected=threshold_value == "nextWeek">"next week"</option>
                        <option value="thisMonth" selected=threshold_value == "thisMonth">"this month"</option>
                        <option value="lastMonth" selected=threshold_value == "lastMonth">"last month"</option>
                        <option value="nextMonth" selected=threshold_value == "nextMonth">"next month"</option>
                    </select>
                </label>
            }
            .into_any()
        }
        _ => {
            let placeholder = if kind_lc == "expression" {
                "=A1>5"
            } else {
                "0"
            };
            let label = if kind_lc == "expression" {
                "formula"
            } else {
                "value"
            };
            view! {
                <label class="onecalc-home-shell__cf-rule-field">
                    <span class="onecalc-home-shell__cf-rule-field-label">{label}</span>
                    <input
                        type="text"
                        class="onecalc-home-shell__cf-rule-input"
                        data-cf-rule-field="threshold"
                        placeholder=placeholder
                        prop:value=threshold_value.clone()
                        value=threshold_value
                        on:input=move |ev| {
                            let target = event_target::<web_sys::HtmlInputElement>(&ev);
                            let mut next = host_cf_rule_from_view(&rule_for_threshold);
                            next.thresholds = vec![target.value()];
                            on_update.run((index, next));
                        }
                    />
                </label>
            }
            .into_any()
        }
    }
}

/// Seed default `typed_rule` and visible-style values when the user
/// picks an aggregate visualization or rank/average rule kind.
/// Existing values are preserved — this only fills in *empty* slots
/// so the rule is immediately functional after a kind switch
/// without forcing the user through a config dialog.
///
/// Per OxFml W073 (`HANDOFF-DNAONECALC-012`, 2026-05-04 update),
/// `typed_rule` is the **only** accepted metadata source for the
/// seven typed families: `colorScale`, `dataBar`, `iconSet`, `top`,
/// `bottom`, `aboveAverage`, `belowAverage`. The W072 bounded-string
/// `thresholds` convention is intentionally ignored upstream for
/// those kinds; the host therefore stops seeding `thresholds` for
/// them (and clears any stale entries on kind switch) and lets the
/// per-kind sub-form populate `typed_rule` directly.
///
/// `thresholds` still carries the real rule input for kinds that
/// need it — `cell_value` / `text` / `dates` / `expression` — and
/// `uniqueValues` / `duplicateValues` use only the kind itself.
fn seed_visualization_rule_defaults(rule: &mut crate::state::FormulaConditionalFormattingRule) {
    use crate::state::{
        FormulaAverageRuleOptions, FormulaColorScaleRuleOptions, FormulaColorScaleStop,
        FormulaConditionalFormattingRank, FormulaConditionalFormattingThreshold,
        FormulaConditionalFormattingTypedRule, FormulaDataBarRuleOptions,
        FormulaIconSetRuleOptions, FormulaRankRuleOptions,
    };

    let kind = rule.rule_kind.to_ascii_lowercase();
    match kind.as_str() {
        "colorscale" => {
            // OxFml W073 ignores bounded `thresholds` for this family.
            // Drop any stale entries so they don't persist and confuse
            // the typed-rule subform.
            rule.thresholds.clear();
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    color_scale: Some(FormulaColorScaleRuleOptions {
                        stops: vec![
                            FormulaColorScaleStop {
                                position: FormulaConditionalFormattingThreshold::Min,
                                color: "#F8696B".to_string(),
                            },
                            FormulaColorScaleStop {
                                position: FormulaConditionalFormattingThreshold::Percentile(50.0),
                                color: "#FFEB84".to_string(),
                            },
                            FormulaColorScaleStop {
                                position: FormulaConditionalFormattingThreshold::Max,
                                color: "#63BE7B".to_string(),
                            },
                        ],
                    }),
                    ..Default::default()
                });
            }
        }
        "databar" => {
            rule.thresholds.clear();
            if rule.fill_color.is_none() {
                rule.fill_color = Some("#638EC6".to_string());
            }
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    data_bar: Some(FormulaDataBarRuleOptions {
                        bar_color: Some("#638EC6".to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
        }
        "iconset" => {
            rule.thresholds.clear();
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    icon_set: Some(FormulaIconSetRuleOptions {
                        set_kind: "3Arrows".to_string(),
                        thresholds: Vec::new(),
                    }),
                    ..Default::default()
                });
            }
        }
        "aboveaverage" | "belowaverage" => {
            rule.thresholds.clear();
            if rule.fill_color.is_none() {
                rule.fill_color = Some("#FFE9B3".to_string());
            }
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    average: Some(FormulaAverageRuleOptions::default()),
                    ..Default::default()
                });
            }
        }
        "top" | "bottom" => {
            rule.thresholds.clear();
            if rule.fill_color.is_none() {
                rule.fill_color = Some("#FFE9B3".to_string());
            }
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    rank: Some(FormulaRankRuleOptions {
                        rank: FormulaConditionalFormattingRank::Count(10),
                    }),
                    ..Default::default()
                });
            }
        }
        "uniquevalues" | "duplicatevalues" => {
            if rule.fill_color.is_none() {
                rule.fill_color = Some("#FFE9B3".to_string());
            }
        }
        _ => {}
    }
}

/// Lift a view-model CF rule back to the host's state shape so the
/// per-field on-change handlers can produce a fresh, fully-populated
/// rule for the reducer. Used inline by `render_cf_rule_card`.
fn host_cf_rule_from_view(
    rule: &ConditionalFormattingRuleView,
) -> crate::state::FormulaConditionalFormattingRule {
    crate::state::FormulaConditionalFormattingRule {
        rule_kind: rule.rule_kind.clone(),
        operator: rule.operator.clone(),
        thresholds: rule.thresholds.clone(),
        font_color: rule.font_color.clone(),
        fill_color: rule.fill_color.clone(),
        typed_rule: rule.typed_rule.clone(),
    }
}

// ---------------------------------------------------------------------------
// Typed CF rule per-kind sub-form
//
// Renders a kind-specific authoring surface below the card header for
// the seven W073-typed families. Per OxFml `HANDOFF-DNAONECALC-012`
// (2026-05-04 update), `typed_rule` is the **only** accepted metadata
// source for these kinds — the bounded-string `thresholds` is
// upstream-ignored, so the sub-form is the rule's only authoring path.
//
// The seven typed kinds: colorScale, dataBar, iconSet, top, bottom,
// aboveAverage, belowAverage. Kinds outside this set render no
// sub-form (the existing top-row threshold control covers them).
// ---------------------------------------------------------------------------

fn render_cf_rule_typed_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let kind = rule.rule_kind.to_ascii_lowercase();
    match kind.as_str() {
        "colorscale" => render_color_scale_subform(rule, index, on_update),
        "databar" => render_data_bar_subform(rule, index, on_update),
        "iconset" => render_icon_set_subform(rule, index, on_update),
        "top" | "bottom" => render_rank_subform(rule, index, on_update),
        "aboveaverage" | "belowaverage" => render_average_subform(rule, index, on_update),
        _ => view! { <span></span> }.into_any(),
    }
}

fn render_color_scale_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{
        FormulaColorScaleRuleOptions, FormulaColorScaleStop, FormulaConditionalFormattingTypedRule,
    };
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed.color_scale.clone().unwrap_or_default();
    let stops = options.stops.clone();
    let stop_rows: Vec<_> = stops
        .iter()
        .enumerate()
        .map(|(stop_index, stop)| {
            let rule_for_kind = rule.clone();
            let rule_for_value = rule.clone();
            let rule_for_color = rule.clone();
            let rule_for_remove = rule.clone();
            let position_kind = threshold_kind_label(&stop.position);
            let position_value = threshold_numeric_value(&stop.position).unwrap_or(0.0);
            let needs_value = matches!(position_kind, "percent" | "percentile" | "num");
            let value_input = if needs_value {
                view! {
                    <input
                        type="number"
                        step="any"
                        class="onecalc-home-shell__cf-rule-input"
                        data-cf-typed-field="color-scale-stop-value"
                        prop:value=position_value.to_string()
                        value=position_value.to_string()
                        on:input=move |ev| {
                            let target = event_target::<web_sys::HtmlInputElement>(&ev);
                            let parsed = target.value().parse::<f64>().unwrap_or(0.0);
                            let mut next = host_cf_rule_from_view(&rule_for_value);
                            update_color_scale_stop(&mut next, stop_index, |stop| {
                                let kind = threshold_kind_label(&stop.position);
                                stop.position = threshold_from_kind_and_value(kind, parsed);
                            });
                            on_update.run((index, next));
                        }
                    />
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            };
            let color_value = normalize_color_for_input(&stop.color);
            view! {
                <div class="onecalc-home-shell__cf-rule-typed-stop">
                    <select
                        class="onecalc-home-shell__cf-rule-input"
                        data-cf-typed-field="color-scale-stop-kind"
                        on:change=move |ev| {
                            let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                            let kind = target.value();
                            let mut next = host_cf_rule_from_view(&rule_for_kind);
                            update_color_scale_stop(&mut next, stop_index, |stop| {
                                let value = threshold_numeric_value(&stop.position).unwrap_or(0.0);
                                stop.position = threshold_from_kind_and_value(&kind, value);
                            });
                            on_update.run((index, next));
                        }
                    >
                        <option value="min" selected=position_kind == "min">"min"</option>
                        <option value="mid" selected=position_kind == "mid">"mid"</option>
                        <option value="max" selected=position_kind == "max">"max"</option>
                        <option value="percent" selected=position_kind == "percent">"%"</option>
                        <option value="percentile" selected=position_kind == "percentile">"pctl"</option>
                        <option value="num" selected=position_kind == "num">"num"</option>
                    </select>
                    {value_input}
                    <input
                        type="color"
                        class="onecalc-home-shell__formatting-color"
                        data-cf-typed-field="color-scale-stop-color"
                        prop:value=color_value.clone()
                        value=color_value
                        on:input=move |ev| {
                            let target = event_target::<web_sys::HtmlInputElement>(&ev);
                            let color = target.value();
                            let mut next = host_cf_rule_from_view(&rule_for_color);
                            update_color_scale_stop(&mut next, stop_index, |stop| {
                                stop.color = color.clone();
                            });
                            on_update.run((index, next));
                        }
                    />
                    <button
                        type="button"
                        class="onecalc-home-shell__cf-rule-typed-stop-remove"
                        title="Remove this stop"
                        aria-label="remove color scale stop"
                        on:click=move |_| {
                            let mut next = host_cf_rule_from_view(&rule_for_remove);
                            remove_color_scale_stop(&mut next, stop_index);
                            on_update.run((index, next));
                        }
                    >
                        "✕"
                    </button>
                </div>
            }
            .into_any()
        })
        .collect();
    let rule_for_add = rule.clone();
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="color-scale">
            {stop_rows}
            <button
                type="button"
                class="onecalc-home-shell__cf-rule-typed-add"
                title="Add a stop to the gradient"
                on:click=move |_| {
                    let mut next = host_cf_rule_from_view(&rule_for_add);
                    let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                    let options = typed
                        .color_scale
                        .get_or_insert_with(FormulaColorScaleRuleOptions::default);
                    options.stops.push(FormulaColorScaleStop {
                        position: crate::state::FormulaConditionalFormattingThreshold::Percentile(50.0),
                        color: "#FFEB84".to_string(),
                    });
                    on_update.run((index, next));
                }
            >
                "+ stop"
            </button>
        </div>
    }
    .into_any()
}

fn render_data_bar_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{
        FormulaConditionalFormattingTypedRule, FormulaDataBarDirection, FormulaDataBarRuleOptions,
    };
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed.data_bar.clone().unwrap_or_default();
    let bar_color = options
        .bar_color
        .clone()
        .unwrap_or_else(|| "#638EC6".to_string());
    let direction_label = match options.direction.unwrap_or(FormulaDataBarDirection::Left) {
        FormulaDataBarDirection::Left => "left",
        FormulaDataBarDirection::Right => "right",
    };
    let show_bar_only = options.show_bar_only;
    let rule_for_color = rule.clone();
    let rule_for_dir = rule.clone();
    let rule_for_show = rule.clone();
    let bar_color_for_input = normalize_color_for_input(&bar_color);
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="data-bar">
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"bar"</span>
                <input
                    type="color"
                    class="onecalc-home-shell__formatting-color"
                    data-cf-typed-field="data-bar-color"
                    prop:value=bar_color_for_input.clone()
                    value=bar_color_for_input
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let value = target.value();
                        let mut next = host_cf_rule_from_view(&rule_for_color);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .data_bar
                            .get_or_insert_with(FormulaDataBarRuleOptions::default);
                        options.bar_color = Some(value);
                        on_update.run((index, next));
                    }
                />
            </label>
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"dir"</span>
                <select
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="data-bar-direction"
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                        let direction = match target.value().as_str() {
                            "right" => FormulaDataBarDirection::Right,
                            _ => FormulaDataBarDirection::Left,
                        };
                        let mut next = host_cf_rule_from_view(&rule_for_dir);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .data_bar
                            .get_or_insert_with(FormulaDataBarRuleOptions::default);
                        options.direction = Some(direction);
                        on_update.run((index, next));
                    }
                >
                    <option value="left" selected=direction_label == "left">"left"</option>
                    <option value="right" selected=direction_label == "right">"right"</option>
                </select>
            </label>
            <label class="onecalc-home-shell__cf-rule-typed-checkbox">
                <input
                    type="checkbox"
                    data-cf-typed-field="data-bar-show-bar-only"
                    prop:checked=show_bar_only
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let checked = target.checked();
                        let mut next = host_cf_rule_from_view(&rule_for_show);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .data_bar
                            .get_or_insert_with(FormulaDataBarRuleOptions::default);
                        options.show_bar_only = checked;
                        on_update.run((index, next));
                    }
                />
                "show bar only"
            </label>
        </div>
    }
    .into_any()
}

fn render_icon_set_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{FormulaConditionalFormattingTypedRule, FormulaIconSetRuleOptions};
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed
        .icon_set
        .clone()
        .unwrap_or_else(|| FormulaIconSetRuleOptions {
            set_kind: "3Arrows".to_string(),
            thresholds: Vec::new(),
        });
    let set_kind = options.set_kind.clone();
    let icon_kinds = [
        "3Arrows",
        "3ArrowsGray",
        "3Flags",
        "3Symbols",
        "3Symbols2",
        "3Stars",
        "3Triangles",
        "4Arrows",
        "4ArrowsGray",
        "4RedToBlack",
        "4Rating",
        "4TrafficLights",
        "5Arrows",
        "5ArrowsGray",
        "5Rating",
        "5Quarters",
    ];
    let options_views: Vec<_> = icon_kinds
        .iter()
        .map(|kind| {
            let selected = *kind == set_kind;
            view! {
                <option value=kind.to_string() selected=selected>{kind.to_string()}</option>
            }
            .into_any()
        })
        .collect();
    let rule_for_kind = rule.clone();
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="icon-set">
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"set"</span>
                <select
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="icon-set-kind"
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                        let value = target.value();
                        let mut next = host_cf_rule_from_view(&rule_for_kind);
                        // OxFml W073 ignores bounded `thresholds` for
                        // iconSet; drop any stale entries on edit so
                        // they don't survive into the saved file.
                        next.thresholds.clear();
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .icon_set
                            .get_or_insert_with(|| FormulaIconSetRuleOptions {
                                set_kind: value.clone(),
                                thresholds: Vec::new(),
                            });
                        options.set_kind = value;
                        on_update.run((index, next));
                    }
                >
                    {options_views}
                </select>
            </label>
        </div>
    }
    .into_any()
}

fn render_rank_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{
        FormulaConditionalFormattingRank, FormulaConditionalFormattingTypedRule,
        FormulaRankRuleOptions,
    };
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed
        .rank
        .clone()
        .unwrap_or_else(|| FormulaRankRuleOptions {
            rank: FormulaConditionalFormattingRank::Count(10),
        });
    let (mode_label, value): (&'static str, f64) = match &options.rank {
        FormulaConditionalFormattingRank::Count(count) => ("count", *count as f64),
        FormulaConditionalFormattingRank::Percent(value) => ("percent", *value),
    };
    let rule_for_mode = rule.clone();
    let rule_for_value = rule.clone();
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="rank">
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"mode"</span>
                <select
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="rank-mode"
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                        let mode = target.value();
                        let mut next = host_cf_rule_from_view(&rule_for_mode);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .rank
                            .get_or_insert_with(|| FormulaRankRuleOptions {
                                rank: FormulaConditionalFormattingRank::Count(10),
                            });
                        let prior_value: f64 = match &options.rank {
                            FormulaConditionalFormattingRank::Count(count) => *count as f64,
                            FormulaConditionalFormattingRank::Percent(value) => *value,
                        };
                        options.rank = match mode.as_str() {
                            "percent" => FormulaConditionalFormattingRank::Percent(prior_value),
                            _ => FormulaConditionalFormattingRank::Count(prior_value.max(0.0) as usize),
                        };
                        on_update.run((index, next));
                    }
                >
                    <option value="count" selected=mode_label == "count">"count"</option>
                    <option value="percent" selected=mode_label == "percent">"percent"</option>
                </select>
            </label>
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"n"</span>
                <input
                    type="number"
                    step="any"
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="rank-value"
                    prop:value=value.to_string()
                    value=value.to_string()
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let parsed = target.value().parse::<f64>().unwrap_or(0.0);
                        let mut next = host_cf_rule_from_view(&rule_for_value);
                        // OxFml W073 ignores bounded `thresholds` for
                        // top/bottom; drop any stale entries so they
                        // don't survive into the saved file.
                        next.thresholds.clear();
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .rank
                            .get_or_insert_with(|| FormulaRankRuleOptions {
                                rank: FormulaConditionalFormattingRank::Count(10),
                            });
                        options.rank = match &options.rank {
                            FormulaConditionalFormattingRank::Count(_) => {
                                FormulaConditionalFormattingRank::Count(parsed.max(0.0) as usize)
                            }
                            FormulaConditionalFormattingRank::Percent(_) => {
                                FormulaConditionalFormattingRank::Percent(parsed)
                            }
                        };
                        on_update.run((index, next));
                    }
                />
            </label>
        </div>
    }
    .into_any()
}

fn render_average_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{FormulaAverageRuleOptions, FormulaConditionalFormattingTypedRule};
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed.average.clone().unwrap_or_default();
    let include_equal = options.include_equal;
    let stddev = options.stddev_multiplier.unwrap_or(0.0);
    let stddev_set = options.stddev_multiplier.is_some();
    let rule_for_equal = rule.clone();
    let rule_for_stddev_toggle = rule.clone();
    let rule_for_stddev_value = rule.clone();
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="average">
            <label class="onecalc-home-shell__cf-rule-typed-checkbox">
                <input
                    type="checkbox"
                    data-cf-typed-field="average-include-equal"
                    prop:checked=include_equal
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let checked = target.checked();
                        let mut next = host_cf_rule_from_view(&rule_for_equal);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .average
                            .get_or_insert_with(FormulaAverageRuleOptions::default);
                        options.include_equal = checked;
                        on_update.run((index, next));
                    }
                />
                "include equal"
            </label>
            <label class="onecalc-home-shell__cf-rule-typed-checkbox">
                <input
                    type="checkbox"
                    data-cf-typed-field="average-stddev-enabled"
                    prop:checked=stddev_set
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let checked = target.checked();
                        let mut next = host_cf_rule_from_view(&rule_for_stddev_toggle);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .average
                            .get_or_insert_with(FormulaAverageRuleOptions::default);
                        options.stddev_multiplier = if checked {
                            Some(options.stddev_multiplier.unwrap_or(1.0))
                        } else {
                            None
                        };
                        on_update.run((index, next));
                    }
                />
                "stddev offset"
            </label>
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"k"</span>
                <input
                    type="number"
                    step="any"
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="average-stddev-value"
                    prop:value=stddev.to_string()
                    value=stddev.to_string()
                    disabled=!stddev_set
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let parsed = target.value().parse::<f64>().unwrap_or(0.0);
                        let mut next = host_cf_rule_from_view(&rule_for_stddev_value);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .average
                            .get_or_insert_with(FormulaAverageRuleOptions::default);
                        options.stddev_multiplier = Some(parsed);
                        on_update.run((index, next));
                    }
                />
            </label>
        </div>
    }
    .into_any()
}

// --- typed-rule helpers ---

fn threshold_kind_label(
    threshold: &crate::state::FormulaConditionalFormattingThreshold,
) -> &'static str {
    use crate::state::FormulaConditionalFormattingThreshold as T;
    match threshold {
        T::Min => "min",
        T::Mid => "mid",
        T::Max => "max",
        T::Percent(_) => "percent",
        T::Percentile(_) => "percentile",
        T::Number(_) => "num",
    }
}

fn threshold_numeric_value(
    threshold: &crate::state::FormulaConditionalFormattingThreshold,
) -> Option<f64> {
    use crate::state::FormulaConditionalFormattingThreshold as T;
    match threshold {
        T::Percent(value) | T::Percentile(value) | T::Number(value) => Some(*value),
        T::Min | T::Mid | T::Max => None,
    }
}

fn threshold_from_kind_and_value(
    kind: &str,
    value: f64,
) -> crate::state::FormulaConditionalFormattingThreshold {
    use crate::state::FormulaConditionalFormattingThreshold as T;
    match kind {
        "min" => T::Min,
        "mid" => T::Mid,
        "max" => T::Max,
        "percent" => T::Percent(value),
        "percentile" => T::Percentile(value),
        _ => T::Number(value),
    }
}

fn update_color_scale_stop(
    rule: &mut crate::state::FormulaConditionalFormattingRule,
    stop_index: usize,
    mutator: impl FnOnce(&mut crate::state::FormulaColorScaleStop),
) {
    use crate::state::{FormulaColorScaleRuleOptions, FormulaConditionalFormattingTypedRule};
    let typed = rule
        .typed_rule
        .get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
    let options = typed
        .color_scale
        .get_or_insert_with(FormulaColorScaleRuleOptions::default);
    if let Some(stop) = options.stops.get_mut(stop_index) {
        mutator(stop);
    }
}

fn remove_color_scale_stop(
    rule: &mut crate::state::FormulaConditionalFormattingRule,
    stop_index: usize,
) {
    if let Some(typed) = rule.typed_rule.as_mut() {
        if let Some(options) = typed.color_scale.as_mut() {
            if stop_index < options.stops.len() {
                options.stops.remove(stop_index);
            }
        }
    }
}

fn render_number_format_presets(
    presets: Vec<NumberFormatPreset>,
    on_set: Callback<String>,
) -> AnyView {
    let chips: Vec<_> = presets
        .into_iter()
        .map(|preset| {
            let label = preset.label;
            let format_code = preset.format_code;
            let seam_id = preset.seam_id.unwrap_or("");
            let seam_attr = seam_id.to_string();
            let seam_badge = preset.seam_id.map(|seam| {
                view! {
                    <span
                        class="onecalc-home-shell__formatting-preset-seam"
                        data-seam-id=seam.to_string()
                        title=format!("<NOT IMPLEMENTED> {seam}")
                    >"⚠"</span>
                }
                .into_any()
            });
            view! {
                <button
                    type="button"
                    class="onecalc-home-shell__formatting-preset"
                    data-format-code=format_code
                    data-seam-id=seam_attr
                    on:click=move |_| {
                        on_set.run(format_code.to_string());
                    }
                >
                    {label}
                    {seam_badge}
                </button>
            }
            .into_any()
        })
        .collect();
    view! { <>{chips}</> }.into_any()
}

/// `<input type="color">` requires a `#RRGGBB` value with a leading
/// hash; an empty string makes Edge / Chrome render the picker as
/// black. Map empty → `#000000` for the control's prop:value while
/// preserving the empty state in the underlying scenario (so an
/// untouched color still serialises as empty / inherit).
fn normalize_color_for_input(raw: &str) -> String {
    if raw.is_empty() {
        "#000000".to_string()
    } else {
        raw.to_string()
    }
}

/// Render persistence-loader warning chips in the status-foot
/// (slice 3). Empty when `load_diagnostics` is empty so the chrome
/// stays minimal. The chip's `data-load-diagnostic` attribute
/// carries the diagnostic slug for browser-test inspection; the
/// `title` carries the human-readable message.
fn render_load_diagnostic_chips(diagnostics: Vec<crate::persistence::LoadDiagnostic>) -> AnyView {
    if diagnostics.is_empty() {
        return view! { <></> }.into_any();
    }
    let chips: Vec<_> = diagnostics
        .into_iter()
        .map(|diagnostic| {
            let slug = diagnostic.slug();
            let message = diagnostic.user_message();
            view! {
                <>
                    <span class="onecalc-home-shell__statusfoot-sep">"·"</span>
                    <span
                        class="onecalc-home-shell__statusfoot-load-warning"
                        data-load-diagnostic=slug
                        title=message
                    >
                        "⚠ imported (Excel-only)"
                    </span>
                </>
            }
            .into_any()
        })
        .collect();
    view! { <>{chips}</> }.into_any()
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
        Some(ResultView::Display {
            text,
            kind,
            applied_font_color,
            applied_fill_color,
        }) => {
            // CF-applied font / fill colours flow through inline
            // `style="color: …; background: …"`. `data-cf-applied`
            // is set when either is present so the corpus can pin
            // the visible-CF state without parsing inline CSS.
            let mut style = String::new();
            if let Some(font) = applied_font_color.as_deref() {
                style.push_str(&format!("color: {}; ", font));
            }
            if let Some(fill) = applied_fill_color.as_deref() {
                style.push_str(&format!("background: {}; ", fill));
            }
            let cf_applied = applied_font_color.is_some() || applied_fill_color.is_some();
            view! {
                <span
                    class="value"
                    data-kind=display_kind_attr(kind)
                    data-cf-applied=if cf_applied { "true" } else { "false" }
                    style=style
                >
                    {text}
                </span>
            }
            .into_any()
        }
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
        Some(ResultView::Array {
            total_rows,
            total_cols,
            label: _,
            cells,
            cell_format,
            truncated,
        }) => render_array_browser(total_rows, total_cols, cells, cell_format, truncated),
    }
}

/// Render the array-result browser (WS-14 §3 item 6). The container
/// is `overflow: auto; resize: both` so the user can scroll and drag-
/// resize the panel. The grid itself uses CSS-grid with sticky row /
/// column headers so the addresses stay visible as the user scrolls.
/// When the bridge truncated the preview window, surface a chip with
/// `+N rows · +M cols hidden` so the user knows the visible cells
/// are a subset.
///
/// `cell_format`, when present, supplies per-cell CF outcomes
/// (W071 + W072): font/fill colour, data bar fill ratio, and / or
/// icon glyph. Each cell renders with the matching style; cells
/// without an outcome render in the default chrome.
fn render_array_browser(
    total_rows: usize,
    total_cols: usize,
    cells: Vec<Vec<String>>,
    cell_format: Option<Vec<Vec<ArrayCellFormatView>>>,
    truncated: bool,
) -> AnyView {
    let preview_rows = cells.len();
    let preview_cols = cells.first().map(|row| row.len()).unwrap_or(0);
    let hidden_rows = total_rows.saturating_sub(preview_rows);
    let hidden_cols = total_cols.saturating_sub(preview_cols);
    // CSS-grid template: 1 header column + N data columns. Each data
    // column is `minmax(4rem, max-content)` so short numbers stay
    // narrow but long strings widen the column up to their natural
    // width, then horizontal scrolling kicks in.
    let grid_template = format!(
        "grid-template-columns: 2.4rem repeat({}, minmax(4rem, max-content));",
        preview_cols.max(1)
    );
    let mut header_cells: Vec<AnyView> = Vec::with_capacity(preview_cols + 1);
    header_cells.push(
        view! {
            <div class="onecalc-array-browser__header onecalc-array-browser__corner" aria-hidden="true"></div>
        }
        .into_any(),
    );
    for col in 0..preview_cols {
        let label = column_index_to_a1_label(col);
        header_cells.push(
            view! {
                <div class="onecalc-array-browser__header onecalc-array-browser__column-header">
                    {label}
                </div>
            }
            .into_any(),
        );
    }
    let mut body_cells: Vec<AnyView> = Vec::with_capacity(preview_rows * (preview_cols + 1));
    for (row_index, row) in cells.into_iter().enumerate() {
        let row_label = (row_index + 1).to_string();
        body_cells.push(
            view! {
                <div class="onecalc-array-browser__header onecalc-array-browser__row-header">
                    {row_label}
                </div>
            }
            .into_any(),
        );
        let row_len = row.len();
        for (col_index, cell_value) in row.into_iter().enumerate() {
            let format_for_cell = cell_format
                .as_ref()
                .and_then(|grid| grid.get(row_index))
                .and_then(|row| row.get(col_index));
            body_cells.push(render_array_browser_cell(
                row_index,
                col_index,
                cell_value,
                format_for_cell,
            ));
        }
        // Pad the final row if it's shorter than the column count
        // (defensive — the bridge already pads, but the cell count
        // attribute only counts cells it emitted).
        for col_pad in row_len..preview_cols {
            body_cells.push(
                view! {
                    <div
                        class="onecalc-array-browser__cell onecalc-array-browser__cell--empty"
                        data-row=row_index.to_string()
                        data-col=col_pad.to_string()
                    ></div>
                }
                .into_any(),
            );
        }
    }
    let truncation_chip = if truncated {
        let mut bits: Vec<String> = Vec::new();
        if hidden_rows > 0 {
            bits.push(format!("+{} rows", hidden_rows));
        }
        if hidden_cols > 0 {
            bits.push(format!("+{} cols", hidden_cols));
        }
        let detail = if bits.is_empty() {
            "more cells hidden".to_string()
        } else {
            format!("{} hidden", bits.join(" · "))
        };
        view! {
            <div class="onecalc-array-browser__truncation" data-truncated="true">
                {detail}
            </div>
        }
        .into_any()
    } else {
        view! { <></> }.into_any()
    };
    view! {
        <div
            class="onecalc-array-browser"
            data-total-rows=total_rows.to_string()
            data-total-cols=total_cols.to_string()
            data-preview-rows=preview_rows.to_string()
            data-preview-cols=preview_cols.to_string()
            data-truncated=if truncated { "true" } else { "false" }
            role="region"
            aria-label="array result browser"
        >
            <div class="onecalc-array-browser__caption">
                {format!("Array[{} × {}]", total_rows, total_cols)}
            </div>
            <div
                class="onecalc-array-browser__scroll"
                style=grid_template
                role="grid"
                aria-rowcount=total_rows.to_string()
                aria-colcount=total_cols.to_string()
            >
                {header_cells}
                {body_cells}
            </div>
            {truncation_chip}
        </div>
    }
    .into_any()
}

/// Render one cell of the array browser, applying CF formatting
/// when the cell carries a per-cell outcome.
///
/// Composition order (for cells with multiple outcomes):
/// 1. Inline `style="background: …; color: …"` for fill / font.
/// 2. Data-bar background overlay via a separate inline-block
///    sized to the bar's `fill_ratio`.
/// 3. Icon glyph rendered ahead of the value.
///
/// A cell with `show_bar_only = true` on its data-bar still emits
/// the value text but hidden via `visibility: hidden` so the bar
/// width remains anchored to the cell's natural width.
fn render_array_browser_cell(
    row_index: usize,
    col_index: usize,
    cell_value: String,
    cell_format: Option<&ArrayCellFormatView>,
) -> AnyView {
    let cell_for_attr = cell_value.clone();
    let mut style = String::new();
    let mut data_attrs: Vec<(&'static str, String)> = Vec::new();

    if let Some(format) = cell_format {
        if let Some(font) = format.effective_font_color.as_deref() {
            style.push_str(&format!("color: {}; ", font));
        }
        if let Some(fill) = format.effective_fill_color.as_deref() {
            style.push_str(&format!("background: {}; ", fill));
        }
        data_attrs.push(("data-cf-applied", "true".to_string()));
    }

    // Icon glyph (if any) rendered ahead of the value.
    let icon_glyph = cell_format
        .and_then(|format| format.icon.as_ref())
        .map(|icon| icon_glyph_for(&icon.set_kind, icon.icon_index));

    // Data bar overlay (if any) rendered as a background layer
    // sized to fill_ratio.
    let data_bar_overlay = cell_format
        .and_then(|format| format.data_bar.as_ref())
        .map(|bar| {
            let percent = (bar.fill_ratio.clamp(0.0, 1.0) * 100.0).round() as u32;
            let direction_attr = match bar.direction {
                DataBarDirectionView::Left => "left",
                DataBarDirectionView::Right => "right",
            };
            let bar_color = bar.bar_color.clone();
            let style = format!("width: {percent}%; background: {bar_color};");
            view! {
                <span
                    class="onecalc-array-browser__data-bar"
                    data-direction=direction_attr
                    style=style
                ></span>
            }
            .into_any()
        });
    let value_visibility_class = cell_format
        .and_then(|format| format.data_bar.as_ref())
        .map(|bar| {
            if bar.show_bar_only {
                "onecalc-array-browser__cell-value--hidden"
            } else {
                ""
            }
        })
        .unwrap_or("");

    let mut cell_classes = String::from("onecalc-array-browser__cell");
    if cell_format.is_some() {
        cell_classes.push_str(" onecalc-array-browser__cell--cf");
    }

    let icon_attr = icon_glyph
        .as_ref()
        .map(|(set_kind, _)| set_kind.clone())
        .unwrap_or_default();
    let icon_view = icon_glyph.map(|(_, glyph)| {
        view! { <span class="onecalc-array-browser__icon">{glyph}</span> }.into_any()
    });

    view! {
        <div
            class=cell_classes
            data-row=row_index.to_string()
            data-col=col_index.to_string()
            data-cf-applied=if cell_format.is_some() { "true" } else { "false" }
            data-icon-set=icon_attr
            title=cell_for_attr
            style=style
        >
            {data_bar_overlay}
            {icon_view}
            <span class={
                if value_visibility_class.is_empty() {
                    "onecalc-array-browser__cell-value".to_string()
                } else {
                    format!("onecalc-array-browser__cell-value {value_visibility_class}")
                }
            }>
                {cell_value}
            </span>
        </div>
    }
    .into_any()
}

/// Map an icon-set kind + index to a Unicode glyph. Excel's icon
/// sets are pixel art in the .xlsx renderer; here we ship the
/// closest representative Unicode glyph per kind. Unknown kinds
/// fall back to the index as a number wrapped in a circle.
fn icon_glyph_for(set_kind: &str, icon_index: usize) -> (String, String) {
    let glyphs: &[&str] = match set_kind {
        // 3-icon sets
        "3Arrows" | "3ArrowsGray" => &["↓", "→", "↑"],
        "3TrafficLights1" | "3TrafficLights2" => &["🔴", "🟡", "🟢"],
        "3Signs" => &["⛔", "⚠", "✅"],
        "3Symbols" | "3Symbols2" => &["✗", "!", "✓"],
        "3Flags" => &["🚩", "🟨", "🟩"],
        // 4-icon sets
        "4Arrows" | "4ArrowsGray" => &["↓", "↘", "↗", "↑"],
        "4Rating" => &["▁", "▃", "▅", "▇"],
        "4RedToBlack" => &["⬛", "🟥", "🟧", "🟩"],
        "4TrafficLights" => &["🔴", "🟠", "🟡", "🟢"],
        // 5-icon sets
        "5Arrows" | "5ArrowsGray" => &["↓", "↘", "→", "↗", "↑"],
        "5Rating" => &["▁", "▂", "▄", "▆", "█"],
        "5Quarters" => &["○", "◔", "◐", "◕", "●"],
        _ => &["•"],
    };
    let glyph = glyphs.get(icon_index).copied().unwrap_or("•").to_string();
    (set_kind.to_string(), glyph)
}

/// True when a key press is a caret-only navigation key — moves
/// the caret without changing the text. The textarea's
/// `on:keyup` filters on these so caret-only navigation triggers
/// a popup-refresh round-trip, while text-input keys fall through
/// to the existing `on:input` path (which already fires the
/// bridge with up-to-date selection).
fn is_caret_navigation_key(key: &str) -> bool {
    matches!(
        key,
        "ArrowLeft"
            | "ArrowRight"
            | "ArrowUp"
            | "ArrowDown"
            | "Home"
            | "End"
            | "PageUp"
            | "PageDown"
    )
}

/// Build a synthetic `EditorInputEvent` from the textarea's current
/// value + selection, used to push a "caret moved, but text didn't
/// change" signal through `apply_live_editor_input`. The reducer
/// updates the editor surface's selection; the bridge refresh
/// re-evaluates signature-help / completion-popup / function-help
/// against the new caret position.
fn synthesize_caret_sync_event(textarea: &HtmlTextAreaElement) -> EditorInputEvent {
    EditorInputEvent {
        text: textarea.value(),
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
        // Caret-sync — the bridge will skip the runtime-evaluation
        // pass. Popups still refresh.
        input_kind: EditorInputKind::CaretSync,
        inserted_text: None,
    }
}

/// Convert a 0-based column index into an Excel-style A1 column
/// label. 0 → "A", 25 → "Z", 26 → "AA", etc. Used by the array
/// browser's column header row so users can read addresses against
/// a familiar mental model.
fn column_index_to_a1_label(index: usize) -> String {
    let mut n = index;
    let mut buf = Vec::new();
    loop {
        let rem = n % 26;
        buf.push(b'A' + rem as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    buf.reverse();
    String::from_utf8(buf).unwrap_or_else(|_| index.to_string())
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
fn render_syntax_overlay(
    runs: Vec<SyntaxRun>,
    fallback_text: String,
    bracket_pair: Option<crate::ui::editor::bracket_matcher::BracketPairHighlight>,
) -> AnyView {
    if runs.is_empty() {
        return view! {
            <span class="syn-text">{fallback_text}{"\n"}</span>
        }
        .into_any();
    }
    // Walk the runs once, computing bracket depth + active flag for
    // every delimiter run that is one of `()[]{}`. Depth wraps modulo
    // the rotating-colour palette in CSS — depth 0 → teal, 1 → rust,
    // 2 → amber, 3 → sage, then rotates. The matching pair under the
    // cursor (open + close) is tagged with `data-bracket-active="true"`
    // and bolded by CSS. Non-bracket delimiters (commas, dots, the
    // leading `=`) are passed through untouched.
    let mut current_depth: usize = 0;
    let spans: Vec<AnyView> = runs
        .into_iter()
        .map(|run| {
            let role_slug = role_slug(run.role);
            let span_start = run.span_start;
            let token_text = run.text.clone();
            let is_bracket = is_bracket_token(&run.text);
            let bracket_depth_attr = if is_bracket {
                let depth_at = if is_open_bracket_text(&run.text) {
                    let d = current_depth;
                    current_depth = current_depth.saturating_add(1);
                    d
                } else {
                    current_depth = current_depth.saturating_sub(1);
                    current_depth
                };
                Some(depth_at)
            } else {
                None
            };
            let bracket_active = match (bracket_depth_attr, &bracket_pair) {
                (Some(_), Some(pair)) => {
                    span_start == pair.open_offset || span_start == pair.close_offset
                }
                _ => false,
            };
            let mut class = format!("syn {}", role_class(run.role));
            if let Some(depth) = bracket_depth_attr {
                let depth_class = depth % BRACKET_DEPTH_COLOR_COUNT;
                class.push_str(&format!(" syn-bracket syn-bracket--depth-{}", depth_class));
                if bracket_active {
                    class.push_str(" syn-bracket--active");
                }
            }
            let depth_attr_value = bracket_depth_attr
                .map(|d| d.to_string())
                .unwrap_or_default();
            let active_attr_value = if bracket_active { "true" } else { "false" };
            view! {
                <span
                    class=class
                    data-token-start=span_start.to_string()
                    data-token-text=token_text.clone()
                    data-token-role=role_slug
                    data-bracket-depth=depth_attr_value
                    data-bracket-active=active_attr_value
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

/// Number of distinct rotating colours offered by the bracket-depth
/// CSS rules. Walking deeper than this wraps back to depth 0 — chosen
/// so the visual signal stays useful even in pathological 10-level
/// nests; four colours is the rainbow-bracket sweet spot the eye can
/// track without the palette feeling noisy.
const BRACKET_DEPTH_COLOR_COUNT: usize = 4;

fn is_bracket_token(text: &str) -> bool {
    matches!(text, "(" | ")" | "[" | "]" | "{" | "}")
}

fn is_open_bracket_text(text: &str) -> bool {
    matches!(text, "(" | "[" | "{")
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
fn render_diagnostic_squiggle_overlay(squiggles: Vec<DiagnosticSquiggle>, text: String) -> AnyView {
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
            // OxFml W067: surface `code`, `stage`, and
            // `worksheet_error_class` as data attributes so browser
            // tests and the eventual UI grouping surface can read
            // them without inference.
            let code_attr = squiggle.code.clone().unwrap_or_default();
            let worksheet_error_class_attr =
                squiggle.worksheet_error_class.clone().unwrap_or_default();
            segments.push(
                view! {
                    <span
                        class=class
                        data-diagnostic-id=squiggle.diagnostic_id
                        data-severity=squiggle.severity.slug()
                        data-stage=squiggle.stage.slug()
                        data-code=code_attr
                        data-worksheet-error-class=worksheet_error_class_attr
                        data-span-start=squiggle.span_start.to_string()
                        data-span-len=squiggle.span_len.to_string()
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
            let end = start.saturating_add(span.len).min(chars.len());
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
/// Editor-foot button that re-runs the active formula through the
/// bridge. Mirrors the F9 keystroke. In Deterministic policy this
/// produces an identical re-render; in LiveRecalc it advances NOW
/// and re-rolls RAND.
///
/// `on:mousedown` (rather than `on:click`) so a click does not pull
/// focus away from the textarea — caret stays where it was.
fn render_recalculate_button(on_recalculate: Callback<()>) -> AnyView {
    view! {
        <button
            type="button"
            class="onecalc-home-shell__recalculate-button"
            data-action="recalculate"
            title="Recalculate formula (F9)"
            aria-label="Recalculate formula"
            on:mousedown=move |ev| {
                ev.prevent_default();
                on_recalculate.run(());
            }
        >
            "↻ Calculate"
        </button>
    }
    .into_any()
}

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
    on_view_mode_toggle: Callback<()>,
) -> AnyView {
    let Some(drill) = drill else {
        return view! { <span></span> }.into_any();
    };
    let aria_hidden = if drill.expanded { "false" } else { "true" };
    let expanded_attr = if drill.expanded { "true" } else { "false" };
    let fresh_attr = if drill.document_is_fresh {
        "true"
    } else {
        "false"
    };
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
            .map(|node| render_formula_drill_row(node.clone(), view_mode, 0))
            .collect();
        let diagnostics_view = render_formula_drill_diagnostics(&drill.diagnostics, view_mode);
        let phase_strip = match view_mode {
            ViewMode::Developer => {
                render_formula_drill_phase_strip_developer(&drill.phase_summaries)
            }
            ViewMode::User => render_formula_drill_phase_strip_user(&drill.phase_summaries),
        };
        let view_toggle = render_drill_view_mode_toggle(view_mode, on_view_mode_toggle);
        view! {
            {view_toggle}
            {diagnostics_view}
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

/// Render one node of the formula drill-down. Uses `<details>`
/// for nodes with children — the user clicks the chevron to
/// collapse / expand each subtree (browser-native).
/// Children are rendered nested inside the details body so the
/// visual hierarchy mirrors the formula's call structure.
fn render_formula_drill_row(node: FormulaDrillNode, view_mode: ViewMode, depth: usize) -> AnyView {
    let has_children = !node.children.is_empty();
    let has_children_attr = if has_children { "true" } else { "false" };
    let state_slug = formula_drill_state_slug(node.state);
    let value_preview_full = node.value_preview.clone();
    let value_preview = value_preview_full.clone().unwrap_or_default();
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
                <span class="onecalc-home-shell__formula-drill-label">{node.label.clone()}</span>
                <span
                    class="onecalc-home-shell__formula-drill-value"
                    title=value_preview.clone()
                >
                    {truncate_for_drill(value_preview.clone())}
                </span>
            </>
        }
        .into_any(),
        ViewMode::User => {
            render_formula_drill_row_user_mode(node.label.clone(), node.state, value_preview_full)
        }
    };
    if has_children {
        let children_view: Vec<AnyView> = node
            .children
            .into_iter()
            .map(|child| render_formula_drill_row(child, view_mode, depth + 1))
            .collect();
        view! {
            <details
                class="onecalc-home-shell__formula-drill-row onecalc-home-shell__formula-drill-row--branch"
                data-depth=depth.to_string()
                data-has-children=has_children_attr
                data-state=state_slug
                data-node-id=node.node_id
                data-aria-level=aria_level
                data-mode=mode_attr
                open
            >
                <summary class="onecalc-home-shell__formula-drill-row-summary">
                    {row_inner}
                </summary>
                <div class="onecalc-home-shell__formula-drill-row-children">
                    {children_view}
                </div>
            </details>
        }
        .into_any()
    } else {
        view! {
            <div
                class="onecalc-home-shell__formula-drill-row onecalc-home-shell__formula-drill-row--leaf"
                role="treeitem"
                data-depth=depth.to_string()
                data-has-children=has_children_attr
                data-state=state_slug
                data-node-id=node.node_id
                data-aria-level=aria_level
                data-mode=mode_attr
            >
                {row_inner}
            </div>
        }
        .into_any()
    }
}

/// Render the view-mode toggle inside the drill-down panel
/// header. The drill-down is the only surface that meaningfully
/// branches on view mode (User mode hides phase chips, state
/// slugs, and SEAM markers; Developer mode surfaces them all),
/// so the toggle lives where it has effect — top-right of the
/// panel body, small and quiet.
fn render_drill_view_mode_toggle(mode: ViewMode, on_toggle: Callback<()>) -> AnyView {
    let mode_attr = mode.slug();
    let label = match mode {
        ViewMode::User => "▸ developer view",
        ViewMode::Developer => "▾ developer view",
    };
    let pressed = matches!(mode, ViewMode::Developer);
    view! {
        <div class="onecalc-home-shell__formula-drill-mode-toggle">
            <button
                type="button"
                class="onecalc-home-shell__formula-drill-mode-button"
                data-view-mode=mode_attr
                aria-pressed=if pressed { "true" } else { "false" }
                title="Toggle Developer view (shows state chips, phase strip, SEAM markers)"
                on:mousedown=move |ev| {
                    ev.prevent_default();
                    on_toggle.run(());
                }
            >
                {label}
            </button>
        </div>
    }
    .into_any()
}

/// Render the diagnostics list inside the drill-down panel.
/// Empty when there are no diagnostics; otherwise emits one row
/// per diagnostic with its severity, message, and (in Developer
/// mode) the diagnostic id / stage / span. Click a row to
/// (eventually) scroll the editor to the span — for now the row
/// is read-only but the `data-span-start` / `data-span-len`
/// attributes are emitted so the click handler is a single-line
/// follow-up when it lands.
fn render_formula_drill_diagnostics(
    diagnostics: &[FormulaDrillDiagnosticRow],
    view_mode: ViewMode,
) -> AnyView {
    if diagnostics.is_empty() {
        return view! { <></> }.into_any();
    }
    let mode_attr = view_mode.slug();
    let rows: Vec<AnyView> = diagnostics
        .iter()
        .map(|diag| {
            let severity_slug = diag.severity.slug();
            let detail = match view_mode {
                ViewMode::Developer => {
                    let code = diag.code.clone().unwrap_or_default();
                    format!(
                        "[{stage}] {code}{sep}{msg}",
                        stage = diag.stage.slug(),
                        code = code,
                        sep = if code.is_empty() { "" } else { " · " },
                        msg = diag.message,
                    )
                }
                ViewMode::User => diag.message.clone(),
            };
            view! {
                <li
                    class="onecalc-home-shell__formula-drill-diagnostic"
                    data-severity=severity_slug
                    data-stage=diag.stage.slug()
                    data-span-start=diag.span_start.to_string()
                    data-span-len=diag.span_len.to_string()
                    data-mode=mode_attr
                >
                    <span
                        class="onecalc-home-shell__formula-drill-diagnostic-severity"
                        data-severity=severity_slug
                    >
                        {severity_slug}
                    </span>
                    <span class="onecalc-home-shell__formula-drill-diagnostic-message">
                        {detail}
                    </span>
                </li>
            }
            .into_any()
        })
        .collect();
    view! {
        <ul
            class="onecalc-home-shell__formula-drill-diagnostics"
            role="list"
            aria-label="formula diagnostics"
            data-mode=mode_attr
        >
            {rows}
        </ul>
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
fn render_formula_drill_phase_strip_developer(chips: &[FormulaDrillPhaseChip]) -> AnyView {
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
fn render_formula_drill_phase_strip_user(chips: &[FormulaDrillPhaseChip]) -> AnyView {
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
    // Anchor at the caret-line TOP (in editor-frame coordinates,
    // i.e. metric-space y plus the textarea padding). The CSS
    // transform `translateY(-100% - 6px)` then lifts the help
    // tooltip's bottom edge 6 px above that line, putting it
    // immediately over the line without the actual rendered
    // height needing to be guessed in pixels. Without the
    // padding offset the top would be 0 → the help renders
    // ABOVE the editor frame, far from the caret (the bug the
    // user reported as "placed very high").
    let style = format!(
        "left: {}px; top: {}px;",
        help.anchor_left_px.saturating_add(EDITOR_FRAME_PAD_PX),
        help.anchor_top_px.saturating_add(EDITOR_FRAME_PAD_PX),
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

/// Editor-frame inner padding, in pixels. Matches the
/// `padding: var(--oc-space-4)` rule on the textarea + overlay
/// in `theme.rs` at the default 16 px html font-size. The
/// caret-box geometry layer reports coordinates in metric-space
/// (line N starts at `N * line_height_px` from y=0); the actual
/// text renders at that y plus this padding offset because the
/// textarea / overlay have padding inside the editor-frame box.
/// All caret-anchored popovers (completion popup, signature
/// help, future hover-help) MUST add this offset to their top
/// value or they will land inside the line of text rather than
/// above / below it.
const EDITOR_FRAME_PAD_PX: usize = 16;

fn render_completion_popup(
    popup: Option<CompletionPopupView>,
    on_click: Callback<String>,
) -> AnyView {
    let Some(popup) = popup else {
        return view! { <span></span> }.into_any();
    };
    // Anchor 4 px below the bottom of the caret line so the
    // popup never overlaps the typed text. The caret line
    // bottom = padding-top + caret_top_px + line_height_px.
    let style = format!(
        "left: {}px; top: {}px;",
        popup.anchor_left_px.saturating_add(EDITOR_FRAME_PAD_PX),
        popup
            .anchor_top_px
            .saturating_add(popup.line_height_px)
            .saturating_add(EDITOR_FRAME_PAD_PX)
            .saturating_add(4),
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
fn render_editor_metrics_chip(metrics: Option<EditorMetricsChip>, view_mode: ViewMode) -> AnyView {
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
            let message = metrics.first_diagnostic_message.clone().unwrap_or_default();
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
fn render_result_context_chip(chip: Option<ResultContextChip>, view_mode: ViewMode) -> AnyView {
    let Some(chip) = chip else {
        return view! { <span></span> }.into_any();
    };
    let mode_attr = view_mode.slug();
    view! {
        <span
            class="onecalc-home-shell__chip onecalc-home-shell__chip--context"
            data-mode=mode_attr
        >
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

#[cfg(target_arch = "wasm32")]
fn build_save_payload(state: &OneCalcHostState) -> Option<(String, String)> {
    let active_id = state.workspace_shell.active_formula_space_id.as_ref()?;
    let formula_space = state.formula_spaces.get(active_id)?;
    let now = current_iso8601_utc();
    let scenario = crate::persistence::formula_space_to_scenario(formula_space, now.clone(), now);
    let stem =
        crate::persistence::suggested_filename_stem(&scenario.identity.name, &scenario.identity.id);
    let xml = crate::persistence::write_formula_xml(&scenario);
    Some((format!("{stem}.dnafml"), xml))
}

#[cfg(target_arch = "wasm32")]
fn current_iso8601_utc() -> String {
    let date = js_sys::Date::new_0();
    date.to_iso_string().as_string().unwrap_or_default()
}
