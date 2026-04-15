use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{ClipboardEvent, HtmlTextAreaElement, InputEvent as WebInputEvent, KeyboardEvent};

#[cfg(target_arch = "wasm32")]
use crate::ui::editor::browser_measurement::capture_overlay_measurement_event;
use crate::ui::editor::commands::{
    classify_dom_input, keydown_to_command, EditorCommand, EditorInputEvent, EditorKeyContext,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::ui::editor::geometry::derive_overlay_snapshot;
use crate::ui::editor::geometry::{
    resolve_overlay_box, EditorOverlayGeometrySnapshot, EditorOverlayMeasurement,
    EditorOverlayMeasurementEvent, EditorOverlayMeasurementSource,
};
use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};
use crate::ui::panels::explore::ExploreEditorClusterViewModel;

#[component]
pub fn FormulaEditorSurface(
    editor: ExploreEditorClusterViewModel,
    #[prop(default = None)] on_input_event: Option<Callback<EditorInputEvent>>,
    #[prop(default = None)] on_command: Option<Callback<EditorCommand>>,
    #[prop(default = None)] on_overlay_measurement: Option<Callback<EditorOverlayMeasurementEvent>>,
) -> impl IntoView {
    let entry_mode = editor.entry_mode;
    let live_state = editor.live_state;
    let result_class_label = editor
        .result_value_summary
        .clone()
        .unwrap_or_else(|| "Unevaluated".to_string());
    let effective_display_label = editor.effective_display_summary.clone().unwrap_or_default();
    let bracket_pair = editor.bracket_pair;
    let editor_settings = editor.editor_settings;
    let settings_popover_open = editor.editor_settings_popover_open;
    let fallback_mode = editor_settings.reduce_motion;
    let auto_proof_interval_ms = editor_settings.auto_proof_quiet_interval_ms;
    let auto_proof_epoch: StoredValue<u32> = StoredValue::new(0);
    let auto_proof_command = on_command.clone();
    let textarea_ref: NodeRef<leptos::html::Textarea> = NodeRef::new();
    let line_count = editor.raw_entered_cell_text.lines().count().max(1);
    let function_count = editor
        .syntax_runs
        .iter()
        .filter(|run| run.role == SyntaxTokenRole::Function)
        .count();
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
    // Sync the DOM textarea's selectionStart/selectionEnd from logical state
    // after each render. Native keys (arrows, backspace, delete, plain Enter)
    // do not go through the reducer — the textarea owns them — so on:input's
    // selection reporting keeps the DOM and logical selection aligned for
    // those paths. This post-render sync covers the rarer reducer-driven
    // mutations (Tab indent, completion accept, F4 reference cycle,
    // CancelEntry revert) where the reducer rewrites both text and
    // selection; without the sync, the textarea's prop:value reassignment
    // would reset the DOM cursor to offset 0.
    schedule_textarea_selection_sync(textarea_ref, selection_start, selection_end);
    let selection_label = if editor_state.selection.is_collapsed() {
        "collapsed"
    } else {
        "range"
    };
    let overlay_measurement = EditorOverlayMeasurement::derived_grid();
    let overlay_geometry = editor.overlay_geometry.clone().unwrap_or_default();
    let (caret_measurement_source, caret_box) = resolve_overlay_box(
        overlay_geometry.caret_box,
        overlay_measurement.offset_box(&editor.raw_entered_cell_text, editor_state.caret.offset),
    );
    let (selection_measurement_source, selection_box) = resolve_overlay_box(
        overlay_geometry.selection_box,
        overlay_measurement.span_box(
            &editor.raw_entered_cell_text,
            crate::adapters::oxfml::FormulaTextSpan {
                start: selection_start,
                len: selection_end.saturating_sub(selection_start),
            },
        ),
    );
    let selected_completion_proposal_id = editor.selected_completion_proposal_id.clone();
    let completion_anchor_span = editor.completion_anchor_span;
    let measurement_source = if overlay_geometry != EditorOverlayGeometrySnapshot::default() {
        "mixed"
    } else {
        match overlay_measurement.source {
            EditorOverlayMeasurementSource::DerivedGrid => "derived-grid",
            EditorOverlayMeasurementSource::DomMeasured => "dom-measured",
        }
    };
    let editor_for_click_measurement = editor.clone();
    let editor_for_input_measurement = editor.clone();
    let editor_for_keyup_measurement = editor.clone();
    let editor_state_for_keydown = editor_state.clone();
    let diagnostics_state_label = if editor.diagnostics.is_empty() {
        "Ready to evaluate".to_string()
    } else {
        format!("{} diagnostic(s) need review", editor.diagnostics.len())
    };
    let diagnostics_state_detail = if editor.diagnostics.is_empty() {
        "Live OxFml analysis is clean and the editor surface is in-sync."
    } else {
        "Live OxFml analysis reported issues in the current entry."
    };

    view! {
        <section
            class="onecalc-formula-editor-surface"
            data-component="formula-editor-surface"
            data-entry-mode=entry_mode.slug()
            data-live-state=live_state.slug()
            data-expanded-editor=if editor.expanded_editor { "true" } else { "false" }
            data-fallback-mode=if fallback_mode { "true" } else { "false" }
        >
            <header class="onecalc-formula-editor-surface__toolbar">
                <div class="onecalc-formula-editor-surface__toolbar-copy">
                    <div>
                        <div class="onecalc-formula-editor-surface__toolbar-title">"Formula"</div>
                        <div class="onecalc-formula-editor-surface__toolbar-subtitle">
                            "Native input, live syntax, and replay-aware assist in one surface."
                        </div>
                    </div>
                    <div class="onecalc-formula-editor-surface__toolbar-metrics">
                        <span>{line_count} " lines"</span>
                        <span>{editor.syntax_runs.len()} " tokens"</span>
                        <span>{function_count} " functions"</span>
                    </div>
                </div>
                <div class="onecalc-formula-editor-surface__toolbar-pills" data-role="editor-toolbar-pills">
                    <span
                        class="onecalc-formula-editor-surface__entry-mode-pill"
                        data-role="editor-entry-mode-pill"
                        data-entry-mode=entry_mode.slug()
                    >
                        {entry_mode.label()}
                    </span>
                    <span
                        class="onecalc-formula-editor-surface__result-class-pill"
                        data-role="editor-result-class-pill"
                        data-has-result=if editor.result_value_summary.is_some() { "true" } else { "false" }
                    >
                        {result_class_label.clone()}
                    </span>
                    <span
                        class="onecalc-formula-editor-surface__live-state-pill"
                        data-role="editor-live-state-pill"
                        data-live-state=live_state.slug()
                        title=live_state.label()
                    >
                        <span data-role="editor-live-state-glyph">{live_state.glyph()}</span>
                        <span data-role="editor-live-state-label">{live_state.label()}</span>
                    </span>
                </div>
                <div class="onecalc-formula-editor-surface__toolbar-state" data-role="editor-toolbar-state">
                    {if editor.diagnostics.is_empty() { "Clean" } else { "Review" }}
                </div>
                {{
                    let toggle_callback = on_command.clone();
                    view! {
                        <button
                            type="button"
                            class="onecalc-formula-editor-surface__settings-gear"
                            data-role="editor-settings-gear"
                            data-open=if settings_popover_open { "true" } else { "false" }
                            aria-label="Editor settings"
                            aria-expanded=if settings_popover_open { "true" } else { "false" }
                            on:click=move |_| {
                                if let Some(command_callback) = toggle_callback.as_ref() {
                                    command_callback.run(EditorCommand::ToggleEditorSettingsPopover);
                                }
                            }
                        >
                            "⚙"
                        </button>
                    }
                }}
            </header>
            {if settings_popover_open {
                render_editor_settings_popover(editor_settings, on_command.clone()).into_any()
            } else {
                view! { <></> }.into_any()
            }}

            <div class="onecalc-formula-editor-surface__body">
                <div class="onecalc-formula-editor-surface__line-rail" data-role="editor-line-rail">
                    {(1..=line_count)
                        .map(|line_number| {
                            let active = line_number.saturating_sub(1) == caret_box.start.line_index;
                            view! {
                                <div
                                    class=("onecalc-formula-editor-surface__line-number", true)
                                    class=("onecalc-formula-editor-surface__line-number--active", active)
                                    data-line-number=line_number
                                >
                                    {line_number}
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="onecalc-formula-editor-surface__editor-stage">
                <div class="onecalc-formula-editor-surface__native-input-layer" data-role="native-input-layer">
                    <textarea
                        class="onecalc-formula-editor-surface__textarea"
                        data-role="editor-input"
                        node_ref=textarea_ref
                        spellcheck="false"
                        autocomplete="off"
                        prop:value=editor.raw_entered_cell_text.clone()
                        on:click=move |ev| {
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &ev;
                            if let Some(callback) = on_overlay_measurement.as_ref() {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    callback.run(capture_overlay_measurement_event(
                                        &event_target::<HtmlTextAreaElement>(&ev),
                                        &editor_for_click_measurement,
                                    ));
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    callback.run(build_overlay_measurement_event(&editor_for_click_measurement));
                                }
                            }
                        }
                        on:input=move |ev| {
                            let textarea = event_target::<HtmlTextAreaElement>(&ev);
                            schedule_auto_proof(
                                auto_proof_interval_ms,
                                auto_proof_epoch,
                                auto_proof_command.clone(),
                            );
                            if let Some(callback) = on_input_event.as_ref() {
                                let web_input_event = ev.dyn_ref::<WebInputEvent>();
                                callback.run(EditorInputEvent {
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
                                        .map(|input_event| classify_dom_input(&input_event.input_type()))
                                        .unwrap_or(crate::ui::editor::commands::EditorInputKind::Other),
                                    inserted_text: web_input_event.and_then(|input_event| input_event.data()),
                                });
                            }
                            if let Some(callback) = on_overlay_measurement.as_ref() {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    callback.run(capture_overlay_measurement_event(
                                        &textarea,
                                        &editor_for_input_measurement,
                                    ));
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    callback.run(build_overlay_measurement_event(&editor_for_input_measurement));
                                }
                            }
                            let _ = textarea.focus();
                        }
                        on:keyup=move |ev| {
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &ev;
                            if let Some(callback) = on_overlay_measurement.as_ref() {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    callback.run(capture_overlay_measurement_event(
                                        &event_target::<HtmlTextAreaElement>(&ev),
                                        &editor_for_keyup_measurement,
                                    ));
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    callback.run(build_overlay_measurement_event(&editor_for_keyup_measurement));
                                }
                            }
                        }
                        on:cut=move |ev: ClipboardEvent| {
                            ev.prevent_default();
                            ev.stop_propagation();
                            let textarea = event_target::<HtmlTextAreaElement>(&ev);
                            if let Some(callback) = on_input_event.as_ref() {
                                callback.run(EditorInputEvent {
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
                                    input_kind: crate::ui::editor::commands::EditorInputKind::Other,
                                    inserted_text: None,
                                });
                            }
                            if let Some(command_callback) = on_command.as_ref() {
                                command_callback.run(EditorCommand::CutSelection);
                            }
                            let _ = textarea.focus();
                        }
                        on:keydown=move |ev: KeyboardEvent| {
                            let textarea = event_target::<HtmlTextAreaElement>(&ev);
                            let key_context = EditorKeyContext {
                                completion_active: editor_state_for_keydown
                                    .completion_anchor_offset
                                    .is_some(),
                            };
                            if let Some(command) = keydown_to_command(
                                &ev.key(),
                                ev.shift_key(),
                                ev.ctrl_key() || ev.meta_key(),
                                ev.alt_key(),
                                key_context,
                            ) {
                                ev.prevent_default();
                                ev.stop_propagation();
                                if command == EditorCommand::CutSelection {
                                    if let Some(callback) = on_input_event.as_ref() {
                                        callback.run(EditorInputEvent {
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
                                            input_kind: crate::ui::editor::commands::EditorInputKind::Other,
                                            inserted_text: None,
                                        });
                                    }
                                }
                                if let Some(command_callback) = on_command.as_ref() {
                                    command_callback.run(command);
                                }
                                let _ = textarea.focus();
                            }
                        }
                    />
                </div>
                <div
                    class="onecalc-formula-editor-surface__overlay-layer"
                    data-role="overlay-layer"
                    data-measurement-source=measurement_source
                    data-char-width-px=overlay_measurement.char_width_px
                    data-line-height-px=overlay_measurement.line_height_px
                    data-fallback-mode=if fallback_mode { "true" } else { "false" }
                >
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
                    <div class="onecalc-formula-editor-surface__inline-diagnostic-spans" data-role="inline-diagnostic-spans">
                        {editor
                            .diagnostics
                            .iter()
                            .map(|diagnostic| {
                                view! {
                                    <span
                                        class="onecalc-formula-editor-surface__inline-diagnostic"
                                        data-role="inline-diagnostic"
                                        data-diagnostic-id=diagnostic.diagnostic_id.clone()
                                        data-span-start=diagnostic.span_start
                                        data-span-len=diagnostic.span_len
                                    >
                                        {inline_diagnostic_excerpt(
                                            &editor.raw_entered_cell_text,
                                            diagnostic.span_start,
                                            diagnostic.span_len,
                                        )}
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
                        data-selection-line=selection_box.start.line_index
                        data-selection-column=selection_box.start.column_index
                        data-selection-top-px=selection_box.top_px
                        data-selection-left-px=selection_box.left_px
                        data-selection-width-px=selection_box.width_px
                        data-selection-height-px=selection_box.height_px
                        data-selection-measurement-source=measurement_source_label(selection_measurement_source)
                    >
                        "Selection: "
                        {selection_start}
                        ".."
                        {selection_end}
                    </div>
                    {bracket_pair.map(|pair| {
                        let open_box = overlay_measurement.span_box(
                            &editor.raw_entered_cell_text,
                            crate::adapters::oxfml::FormulaTextSpan {
                                start: pair.open_offset,
                                len: 1,
                            },
                        );
                        let close_box = overlay_measurement.span_box(
                            &editor.raw_entered_cell_text,
                            crate::adapters::oxfml::FormulaTextSpan {
                                start: pair.close_offset,
                                len: 1,
                            },
                        );
                        view! {
                            <div
                                class="onecalc-formula-editor-surface__bracket-pair-layer"
                                data-role="bracket-pair-layer"
                            >
                                <span
                                    class="onecalc-formula-editor-surface__bracket-pair"
                                    data-role="bracket-pair-open"
                                    data-bracket-offset=pair.open_offset
                                    style=bracket_pair_style(open_box)
                                />
                                <span
                                    class="onecalc-formula-editor-surface__bracket-pair"
                                    data-role="bracket-pair-close"
                                    data-bracket-offset=pair.close_offset
                                    style=bracket_pair_style(close_box)
                                />
                            </div>
                        }
                    })}
                    <div
                        class="onecalc-formula-editor-surface__caret-indicator"
                        data-role="caret-indicator"
                        data-caret-offset=editor_state.caret.offset
                        data-caret-line=caret_box.start.line_index
                        data-caret-column=caret_box.start.column_index
                        data-caret-top-px=caret_box.top_px
                        data-caret-left-px=caret_box.left_px
                        data-caret-measurement-source=measurement_source_label(caret_measurement_source)
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
                    {editor_state
                        .completion_anchor_offset
                        .map(|offset| {
                            let popup_command = on_command.clone();
                            let (anchor_measurement_source, anchor_box) = resolve_overlay_box(
                                overlay_geometry.completion_anchor_box,
                                completion_anchor_span
                                    .map(|span| overlay_measurement.span_box(&editor.raw_entered_cell_text, span))
                                    .unwrap_or_else(|| overlay_measurement.offset_box(&editor.raw_entered_cell_text, offset)),
                            );
                            let (_, popup_box) = resolve_overlay_box(
                                overlay_geometry.completion_popup_box,
                                completion_anchor_span
                                    .map(|span| overlay_measurement.span_box(&editor.raw_entered_cell_text, span))
                                    .unwrap_or(anchor_box),
                            );
                            view! {
                                <div
                                    class="onecalc-formula-editor-surface__assist-indicator"
                                    data-role="completion-anchor-indicator"
                                    data-anchor-offset=offset
                                    data-anchor-line=anchor_box.start.line_index
                                    data-anchor-column=anchor_box.start.column_index
                                    data-anchor-top-px=anchor_box.top_px
                                    data-anchor-left-px=anchor_box.left_px
                                    data-anchor-width-px=anchor_box.width_px
                                    data-anchor-height-px=anchor_box.height_px
                                    data-anchor-measurement-source=measurement_source_label(anchor_measurement_source)
                                    data-anchor-span-start=completion_anchor_span.map(|span| span.start)
                                    data-anchor-span-len=completion_anchor_span.map(|span| span.len)
                                >
                                    "Completion anchor: "
                                    {offset}
                                </div>
                                <div
                                    class="onecalc-formula-editor-surface__popup-container onecalc-formula-editor-surface__popup-container--completion"
                                    data-role="completion-popup-container"
                                    data-focused-assist="completion"
                                    data-popup-line=popup_box.start.line_index
                                    data-popup-column=popup_box.start.column_index
                                    style=overlay_popup_style(popup_box)
                                >
                                    <div
                                        class="onecalc-formula-editor-surface__completion-popup"
                                        data-role="completion-popup"
                                        role="listbox"
                                        aria-label="Formula completion proposals"
                                    >
                                        {editor
                                            .completion_items
                                            .iter()
                                            .enumerate()
                                            .map(|(index, item)| {
                                                let is_selected = selected_completion_proposal_id
                                                    .as_ref()
                                                    .is_some_and(|proposal_id| proposal_id == &item.proposal_id);
                                                let popup_command = popup_command.clone();
                                                view! {
                                                    <button
                                                        class="onecalc-formula-editor-surface__completion-item"
                                                        type="button"
                                                        data-completion-id=item.proposal_id.clone()
                                                        data-completion-index=index
                                                        data-proposal-kind=match item.proposal_kind {
                                                            crate::services::explore_mode::ExploreCompletionKindView::Function => "function",
                                                            crate::services::explore_mode::ExploreCompletionKindView::DefinedName => "defined-name",
                                                            crate::services::explore_mode::ExploreCompletionKindView::TableName => "table-name",
                                                            crate::services::explore_mode::ExploreCompletionKindView::TableColumn => "table-column",
                                                            crate::services::explore_mode::ExploreCompletionKindView::StructuredSelector => "structured-selector",
                                                            crate::services::explore_mode::ExploreCompletionKindView::SyntaxAssist => "syntax-assist",
                                                        }
                                                        data-doc-ref=item.documentation_ref.clone().unwrap_or_default()
                                                        data-requires-revalidation=if item.requires_revalidation { "true" } else { "false" }
                                                        data-selected=if is_selected { "true" } else { "false" }
                                                        data-active-row=if is_selected { "true" } else { "false" }
                                                        role="option"
                                                        aria-selected=if is_selected { "true" } else { "false" }
                                                        tabindex=if is_selected { "0" } else { "-1" }
                                                        on:click=move |_| {
                                                            if let Some(command_callback) = popup_command.as_ref() {
                                                                command_callback.run(EditorCommand::AcceptCompletionByIndex(index));
                                                            }
                                                        }
                                                    >
                                                        {item.display_text.clone()}
                                                    </button>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            }
                        })}
                    {editor_state
                        .signature_help_anchor_offset
                        .map(|offset| {
                            let (call_measurement_source, call_box) = resolve_overlay_box(
                                overlay_geometry.signature_help_anchor_box,
                                editor
                                    .signature_help
                                    .as_ref()
                                    .map(|help| overlay_measurement.span_box(&editor.raw_entered_cell_text, help.call_span))
                                    .unwrap_or_else(|| overlay_measurement.offset_box(&editor.raw_entered_cell_text, offset)),
                            );
                            let (_, popup_box) = resolve_overlay_box(
                                overlay_geometry.signature_help_popup_box,
                                editor
                                    .signature_help
                                    .as_ref()
                                    .map(|help| overlay_measurement.span_box(&editor.raw_entered_cell_text, help.call_span))
                                    .unwrap_or(call_box),
                            );
                            view! {
                                <div
                                    class="onecalc-formula-editor-surface__assist-indicator"
                                    data-role="signature-help-anchor-indicator"
                                    data-anchor-offset=offset
                                    data-anchor-line=call_box.start.line_index
                                    data-anchor-column=call_box.start.column_index
                                    data-anchor-top-px=call_box.top_px
                                    data-anchor-left-px=call_box.left_px
                                    data-anchor-width-px=call_box.width_px
                                    data-anchor-height-px=call_box.height_px
                                    data-anchor-measurement-source=measurement_source_label(call_measurement_source)
                                >
                                    "Signature help anchor: "
                                    {offset}
                                </div>
                                <div
                                    class="onecalc-formula-editor-surface__popup-container onecalc-formula-editor-surface__popup-container--signature"
                                    data-role="signature-help-popup-container"
                                    data-focused-assist="signature"
                                    data-popup-line=popup_box.start.line_index
                                    data-popup-column=popup_box.start.column_index
                                    style=overlay_signature_popup_style(popup_box)
                                >
                                    <div
                                        class="onecalc-formula-editor-surface__signature-help-popup"
                                        data-role="signature-help-popup"
                                        role="status"
                                        aria-live="polite"
                                    >
                                        {editor
                                            .signature_help
                                            .as_ref()
                                            .map(|help| {
                                                view! {
                                                    <div
                                                        data-role="signature-help-content"
                                                        data-active-argument-index=help.active_argument_index
                                                        data-call-span-start=help.call_span.start
                                                        data-call-span-len=help.call_span.len
                                                        data-call-line=call_box.start.line_index
                                                        data-call-column=call_box.start.column_index
                                                    >
                                                        <span data-role="signature-help-callee">
                                                            {help.callee_text.clone()}
                                                        </span>
                                                        {render_signature_help_signature(
                                                            editor.function_help.as_ref(),
                                                            help.active_argument_index,
                                                        )}
                                                    </div>
                                                }
                                                .into_any()
                                            })
                                            .unwrap_or_else(|| view! { <div>"Unavailable"</div> }.into_any())}
                                    </div>
                                </div>
                            }
                        })}
                </div>
                </div>
            </div>

            <footer class="onecalc-formula-editor-surface__diagnostic-band" data-role="editor-diagnostic-band">
                <div class="onecalc-formula-editor-surface__diagnostic-band-state">
                    <span class="onecalc-formula-editor-surface__diagnostic-icon">
                        {if editor.diagnostics.is_empty() { "OK" } else { "!" }}
                    </span>
                    <div>
                        <strong>{diagnostics_state_label}</strong>
                        <div>{diagnostics_state_detail}</div>
                    </div>
                </div>
                <div
                    class="onecalc-formula-editor-surface__diagnostic-band-effective-display"
                    data-role="editor-effective-display"
                    data-has-display=if editor.effective_display_summary.is_some() { "true" } else { "false" }
                >
                    <span data-role="editor-effective-display-label">"Effective display: "</span>
                    <span data-role="editor-effective-display-value">
                        {if effective_display_label.is_empty() {
                            "—".to_string()
                        } else {
                            effective_display_label.clone()
                        }}
                    </span>
                </div>
                <div class="onecalc-formula-editor-surface__diagnostic-band-action">
                    {if editor.has_signature_help { "Signature help ready" } else { "Assist idle" }}
                </div>
            </footer>

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

fn measurement_source_label(source: EditorOverlayMeasurementSource) -> &'static str {
    match source {
        EditorOverlayMeasurementSource::DerivedGrid => "derived-grid",
        EditorOverlayMeasurementSource::DomMeasured => "dom-measured",
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_overlay_measurement_event(
    editor: &ExploreEditorClusterViewModel,
) -> EditorOverlayMeasurementEvent {
    EditorOverlayMeasurementEvent {
        snapshot: derive_overlay_snapshot(
            &editor.raw_entered_cell_text,
            editor.editor_surface_state.caret.offset,
            crate::adapters::oxfml::FormulaTextSpan {
                start: editor.editor_surface_state.selection.start(),
                len: editor
                    .editor_surface_state
                    .selection
                    .end()
                    .saturating_sub(editor.editor_surface_state.selection.start()),
            },
            editor.completion_anchor_span,
            editor.signature_help.as_ref().map(|help| help.call_span),
        ),
    }
}

fn overlay_popup_style(box_geometry: crate::ui::editor::geometry::EditorOverlayBox) -> String {
    format!(
        "position:absolute;top:{}px;left:{}px;",
        box_geometry.top_px + box_geometry.height_px,
        box_geometry.left_px
    )
}

fn schedule_textarea_selection_sync(
    textarea_ref: NodeRef<leptos::html::Textarea>,
    selection_start: usize,
    selection_end: usize,
) {
    let _ = textarea_ref;
    let _ = selection_start;
    let _ = selection_end;
    #[cfg(target_arch = "wasm32")]
    {
        let start = selection_start as u32;
        let end = selection_end as u32;
        leptos::prelude::set_timeout(
            move || {
                if let Some(element) = textarea_ref.get() {
                    let current_start = element.selection_start().ok().flatten();
                    let current_end = element.selection_end().ok().flatten();
                    if current_start != Some(start) || current_end != Some(end) {
                        let _ = element.set_selection_range(start, end);
                    }
                }
            },
            std::time::Duration::from_millis(0),
        );
    }
}

fn schedule_auto_proof(
    interval_ms: Option<u32>,
    epoch: StoredValue<u32>,
    on_command: Option<Callback<EditorCommand>>,
) {
    let Some(ms) = interval_ms else {
        return;
    };
    let next_epoch = epoch.with_value(|current| current.wrapping_add(1));
    epoch.set_value(next_epoch);
    let _ = on_command;
    let _ = ms;
    #[cfg(target_arch = "wasm32")]
    {
        let command = on_command;
        leptos::prelude::set_timeout(
            move || {
                if epoch.with_value(|current| *current) != next_epoch {
                    return;
                }
                if let Some(callback) = command.as_ref() {
                    callback.run(EditorCommand::RequestProof);
                }
            },
            std::time::Duration::from_millis(ms as u64),
        );
    }
}

fn render_editor_settings_popover(
    settings: crate::ui::editor::state::EditorSettings,
    on_command: Option<Callback<EditorCommand>>,
) -> impl IntoView {
    use crate::ui::editor::state::{CompletionAggressiveness, EditorSettingUpdate, HelpPlacement};
    let toggle = |update: EditorSettingUpdate, on_command: Option<Callback<EditorCommand>>| {
        move |_| {
            if let Some(callback) = on_command.as_ref() {
                callback.run(EditorCommand::UpdateEditorSetting(update));
            }
        }
    };
    let completion_slug = settings.completion_aggressiveness.slug();
    let help_slug = settings.help_placement.slug();
    let auto_proof_enabled = settings.auto_proof_quiet_interval_ms.is_some();
    let auto_proof_value = settings
        .auto_proof_quiet_interval_ms
        .map(|ms| ms.to_string())
        .unwrap_or_else(|| "off".to_string());
    view! {
        <div
            class="onecalc-formula-editor-surface__settings-popover"
            data-role="editor-settings-popover"
            role="dialog"
            aria-label="Editor settings"
        >
            <div class="onecalc-formula-editor-surface__settings-popover-grid">
                <button
                    type="button"
                    class="onecalc-formula-editor-surface__settings-toggle"
                    data-role="setting-toggle-auto-close-brackets"
                    data-checked=if settings.auto_close_brackets { "true" } else { "false" }
                    on:click=toggle(EditorSettingUpdate::ToggleAutoCloseBrackets, on_command.clone())
                >
                    <span>"Auto-close brackets"</span>
                    <span>{if settings.auto_close_brackets { "on" } else { "off" }}</span>
                </button>
                <button
                    type="button"
                    class="onecalc-formula-editor-surface__settings-toggle"
                    data-role="setting-toggle-highlight-bracket-pairs"
                    data-checked=if settings.highlight_bracket_pairs { "true" } else { "false" }
                    on:click=toggle(EditorSettingUpdate::ToggleHighlightBracketPairs, on_command.clone())
                >
                    <span>"Highlight bracket pairs"</span>
                    <span>{if settings.highlight_bracket_pairs { "on" } else { "off" }}</span>
                </button>
                <div
                    class="onecalc-formula-editor-surface__settings-choice"
                    data-role="setting-completion-aggressiveness"
                    data-value=completion_slug
                >
                    <span>"Completion"</span>
                    <div class="onecalc-formula-editor-surface__settings-choice-buttons">
                        {[
                            CompletionAggressiveness::Manual,
                            CompletionAggressiveness::OnIdentifier,
                            CompletionAggressiveness::Always,
                        ]
                        .iter()
                        .map(|choice| {
                            let choice = *choice;
                            let is_active = settings.completion_aggressiveness == choice;
                            let handler = toggle(
                                EditorSettingUpdate::SetCompletionAggressiveness(choice),
                                on_command.clone(),
                            );
                            view! {
                                <button
                                    type="button"
                                    class="onecalc-formula-editor-surface__settings-choice-button"
                                    data-value=choice.slug()
                                    data-active=if is_active { "true" } else { "false" }
                                    on:click=handler
                                >
                                    {choice.label()}
                                </button>
                            }
                        })
                        .collect_view()}
                    </div>
                </div>
                <div
                    class="onecalc-formula-editor-surface__settings-choice"
                    data-role="setting-help-placement"
                    data-value=help_slug
                >
                    <span>"Help placement"</span>
                    <div class="onecalc-formula-editor-surface__settings-choice-buttons">
                        {[HelpPlacement::Inline, HelpPlacement::Sidecar]
                            .iter()
                            .map(|choice| {
                                let choice = *choice;
                                let is_active = settings.help_placement == choice;
                                let handler = toggle(
                                    EditorSettingUpdate::SetHelpPlacement(choice),
                                    on_command.clone(),
                                );
                                view! {
                                    <button
                                        type="button"
                                        class="onecalc-formula-editor-surface__settings-choice-button"
                                        data-value=choice.slug()
                                        data-active=if is_active { "true" } else { "false" }
                                        on:click=handler
                                    >
                                        {choice.label()}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
                <button
                    type="button"
                    class="onecalc-formula-editor-surface__settings-toggle"
                    data-role="setting-toggle-reuse-timing-badge"
                    data-checked=if settings.reuse_timing_badge_visible { "true" } else { "false" }
                    on:click=toggle(EditorSettingUpdate::ToggleReuseTimingBadge, on_command.clone())
                >
                    <span>"Show reuse/timing badge"</span>
                    <span>{if settings.reuse_timing_badge_visible { "on" } else { "off" }}</span>
                </button>
                <button
                    type="button"
                    class="onecalc-formula-editor-surface__settings-toggle"
                    data-role="setting-toggle-reduce-motion"
                    data-checked=if settings.reduce_motion { "true" } else { "false" }
                    on:click=toggle(EditorSettingUpdate::ToggleReduceMotion, on_command.clone())
                >
                    <span>"Reduce motion / overlays"</span>
                    <span>{if settings.reduce_motion { "on" } else { "off" }}</span>
                </button>
                <button
                    type="button"
                    class="onecalc-formula-editor-surface__settings-toggle"
                    data-role="setting-toggle-auto-proof"
                    data-checked=if auto_proof_enabled { "true" } else { "false" }
                    data-value=auto_proof_value
                    on:click=toggle(EditorSettingUpdate::ToggleAutoProofQuietInterval, on_command.clone())
                >
                    <span>"Auto-proof quiet interval"</span>
                    <span>{if auto_proof_enabled { "600 ms" } else { "off" }}</span>
                </button>
            </div>
        </div>
    }
}

fn bracket_pair_style(box_geometry: crate::ui::editor::geometry::EditorOverlayBox) -> String {
    format!(
        "position:absolute;top:{}px;left:{}px;width:{}px;height:{}px;",
        box_geometry.top_px, box_geometry.left_px, box_geometry.width_px, box_geometry.height_px
    )
}

fn overlay_signature_popup_style(
    box_geometry: crate::ui::editor::geometry::EditorOverlayBox,
) -> String {
    // Anchor at the top of the call span; CSS transform shifts the popup above
    // the line so it reads like a classic ScreenTip hovering over the caret.
    format!(
        "position:absolute;top:{}px;left:{}px;transform:translateY(calc(-100% - 6px));",
        box_geometry.top_px, box_geometry.left_px
    )
}

fn render_signature_help_signature(
    function_help: Option<&crate::services::explore_mode::ExploreFunctionHelpView>,
    active_argument_index: usize,
) -> AnyView {
    match function_help.and_then(|help| help.signature_forms.first()) {
        Some(signature) => render_signature_form(
            &signature.display_signature,
            active_argument_index,
            "signature-help",
        ),
        None => view! {
            <span data-role="signature-help-argument">
                {"arg "}
                {active_argument_index}
            </span>
        }
        .into_any(),
    }
}

fn render_signature_form(
    display_signature: &str,
    active_argument_index: usize,
    role_prefix: &'static str,
) -> AnyView {
    let (prefix, arguments, suffix) = split_signature(display_signature);

    view! {
        <span class="onecalc-signature-form" data-role=format!("{role_prefix}-signature")>
            <span data-role=format!("{role_prefix}-signature-prefix")>{prefix}</span>
            {arguments
                .into_iter()
                .enumerate()
                .map(|(index, argument)| {
                    let active = index == active_argument_index;
                    view! {
                        <>
                            {if index > 0 {
                                view! { <span data-role=format!("{role_prefix}-signature-separator")>{", "}</span> }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                            <span
                                class=("onecalc-signature-argument", true)
                                class=("onecalc-signature-argument--active", active)
                                data-role=format!("{role_prefix}-signature-argument")
                                data-active=if active { "true" } else { "false" }
                            >
                                {argument}
                            </span>
                        </>
                    }
                })
                .collect_view()}
            <span data-role=format!("{role_prefix}-signature-suffix")>{suffix}</span>
        </span>
    }
    .into_any()
}

fn split_signature(display_signature: &str) -> (String, Vec<String>, String) {
    let Some(open_index) = display_signature.find('(') else {
        return (display_signature.to_string(), Vec::new(), String::new());
    };
    let Some(close_index) = display_signature.rfind(')') else {
        return (display_signature.to_string(), Vec::new(), String::new());
    };
    if close_index <= open_index {
        return (display_signature.to_string(), Vec::new(), String::new());
    }

    let prefix = display_signature[..=open_index].to_string();
    let inner = &display_signature[(open_index + 1)..close_index];
    let suffix = display_signature[close_index..].to_string();
    let arguments = inner
        .split(',')
        .map(|argument| argument.trim().to_string())
        .filter(|argument| !argument.is_empty())
        .collect();
    (prefix, arguments, suffix)
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

fn inline_diagnostic_excerpt(text: &str, span_start: usize, span_len: usize) -> String {
    text.chars().skip(span_start).take(span_len).collect()
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
                    scenario_label: "Success · SUM result".to_string(),
                    truth_source_label: "preview-backed".to_string(),
                    host_profile_summary: "Windows desktop preview".to_string(),
                    packet_kind_summary: "preview edit packet".to_string(),
                    capability_floor_summary: "Explore + Inspect".to_string(),
                    mode_availability_summary: "Explore / Inspect / Workbench".to_string(),
                    trace_summary: Some("Preview packet reused green=false, bind complete".to_string()),
                    blocked_reason: None,
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
                    completion_items: vec![crate::services::explore_mode::ExploreCompletionItemView {
                        proposal_id: "proposal-1".to_string(),
                        proposal_kind: crate::services::explore_mode::ExploreCompletionKindView::Function,
                        display_text: "SUM".to_string(),
                        insert_text: "SUM(".to_string(),
                        replacement_span: Some(crate::adapters::oxfml::FormulaTextSpan { start: 1, len: 3 }),
                        documentation_ref: Some("preview:function:SUM".to_string()),
                        requires_revalidation: true,
                    }],
                    selected_completion_proposal_id: Some("proposal-1".to_string()),
                    selected_completion_item: Some(crate::services::explore_mode::ExploreCompletionItemView {
                        proposal_id: "proposal-1".to_string(),
                        proposal_kind: crate::services::explore_mode::ExploreCompletionKindView::Function,
                        display_text: "SUM".to_string(),
                        insert_text: "SUM(".to_string(),
                        replacement_span: Some(crate::adapters::oxfml::FormulaTextSpan { start: 1, len: 3 }),
                        documentation_ref: Some("preview:function:SUM".to_string()),
                        requires_revalidation: true,
                    }),
                    help_sync_lookup_key: Some("SUM".to_string()),
                    has_signature_help: true,
                    signature_help: Some(crate::services::explore_mode::ExploreSignatureHelpView {
                        callee_text: "SUM".to_string(),
                        call_span: crate::adapters::oxfml::FormulaTextSpan { start: 0, len: 9 },
                        active_argument_index: 1,
                    }),
                    function_help: Some(crate::services::explore_mode::ExploreFunctionHelpView {
                        lookup_key: "SUM".to_string(),
                        display_name: "SUM".to_string(),
                        signature_forms: vec![crate::services::explore_mode::ExploreFunctionHelpSignatureView {
                            display_signature: "SUM(number1, number2, ...)".to_string(),
                            min_arity: 1,
                            max_arity: None,
                        }],
                        argument_help: vec!["number1".to_string(), "number2".to_string()],
                        short_description: Some("Adds numbers".to_string()),
                        availability_summary: Some("supported".to_string()),
                        deferred_or_profile_limited: false,
                    }),
                    function_help_lookup_key: Some("SUM".to_string()),
                    completion_anchor_span: Some(crate::adapters::oxfml::FormulaTextSpan { start: 1, len: 3 }),
                    overlay_geometry: Some(EditorOverlayGeometrySnapshot {
                        caret_box: Some(crate::ui::editor::geometry::EditorMeasuredOverlayBox {
                            top_px: 42,
                            left_px: 64,
                            width_px: 2,
                            height_px: 22,
                            line_index: 0,
                            column_index: 4,
                        }),
                        selection_box: Some(crate::ui::editor::geometry::EditorMeasuredOverlayBox {
                            top_px: 42,
                            left_px: 24,
                            width_px: 40,
                            height_px: 22,
                            line_index: 0,
                            column_index: 1,
                        }),
                        completion_anchor_box: Some(crate::ui::editor::geometry::EditorMeasuredOverlayBox {
                            top_px: 64,
                            left_px: 24,
                            width_px: 40,
                            height_px: 22,
                            line_index: 0,
                            column_index: 1,
                        }),
                        signature_help_anchor_box: Some(crate::ui::editor::geometry::EditorMeasuredOverlayBox {
                            top_px: 86,
                            left_px: 0,
                            width_px: 72,
                            height_px: 22,
                            line_index: 0,
                            column_index: 0,
                        }),
                        completion_popup_box: Some(crate::ui::editor::geometry::EditorMeasuredOverlayBox {
                            top_px: 64,
                            left_px: 24,
                            width_px: 40,
                            height_px: 22,
                            line_index: 0,
                            column_index: 1,
                        }),
                        signature_help_popup_box: Some(crate::ui::editor::geometry::EditorMeasuredOverlayBox {
                            top_px: 86,
                            left_px: 0,
                            width_px: 72,
                            height_px: 22,
                            line_index: 0,
                            column_index: 0,
                        }),
                    }),
                    green_tree_key: Some("green-1".to_string()),
                    reused_green_tree: false,
                    entry_mode: crate::ui::editor::state::EditorEntryMode::Formula,
                    live_state: crate::ui::editor::state::EditorLiveState::EditingLive,
                    expanded_editor: false,
                    result_value_summary: Some("Number".to_string()),
                    effective_display_summary: Some("3".to_string()),
                    bracket_pair: Some(crate::ui::editor::bracket_matcher::BracketPairHighlight {
                        open_offset: 4,
                        close_offset: 8,
                    }),
                    editor_settings: crate::ui::editor::state::EditorSettings::default(),
                    editor_settings_popover_open: true,
                    configure_drawer_open: false,
                    editor_surface_state: EditorSurfaceState {
                        caret: EditorCaret { offset: 4 },
                        selection: EditorSelection { anchor: 1, focus: 4 },
                        scroll_window: EditorScrollWindow {
                            first_visible_line: 0,
                            visible_line_count: 6,
                        },
                        completion_anchor_offset: Some(4),
                        completion_selected_index: Some(0),
                        signature_help_anchor_offset: Some(4),
                    },
                }
            />
        }
        .to_html();

        assert!(html.contains("data-component=\"formula-editor-surface\""));
        assert!(html.contains("data-role=\"editor-input\""));
        assert!(html.contains("spellcheck=\"false\""));
        assert!(html.contains("autocomplete=\"off\""));
        assert!(html.contains("data-role=\"native-input-layer\""));
        assert!(html.contains("data-role=\"overlay-layer\""));
        assert!(html.contains("data-measurement-source=\"mixed\""));
        assert!(html.contains("data-role=\"syntax-layer\""));
        assert!(html.contains("data-role=\"diagnostic-markers\""));
        assert!(html.contains("data-role=\"inline-diagnostic-spans\""));
        assert!(html.contains("data-role=\"inline-diagnostic\""));
        assert!(html.contains("data-diagnostic-id=\"diag-1\""));
        assert!(html.contains("data-token-role=\"function\""));
        assert!(html.contains("data-role=\"caret-indicator\""));
        assert!(html.contains("data-caret-line=\"0\""));
        assert!(html.contains("data-caret-top-px=\"42\""));
        assert!(html.contains("data-caret-measurement-source=\"dom-measured\""));
        assert!(html.contains("data-role=\"selection-indicator\""));
        assert!(html.contains("data-selection-kind=\"range\""));
        assert!(html.contains("data-selection-line=\"0\""));
        assert!(html.contains("data-selection-left-px=\"24\""));
        assert!(html.contains("data-selection-measurement-source=\"dom-measured\""));
        assert!(html.contains("data-role=\"completion-anchor-indicator\""));
        assert!(html.contains("data-anchor-line=\"0\""));
        assert!(html.contains("data-anchor-measurement-source=\"dom-measured\""));
        assert!(html.contains("position:absolute;"));
        assert!(html.contains("top:86px;"));
        assert!(html.contains("left:24px;"));
        assert!(html.contains("transform:translateY(calc(-100% - 6px))"));
        assert!(html.contains("data-role=\"completion-popup-container\""));
        assert!(html.contains("data-role=\"signature-help-anchor-indicator\""));
        assert!(html.contains("data-role=\"signature-help-popup-container\""));
        assert!(html.contains("data-role=\"completion-popup\""));
        assert!(html.contains("role=\"listbox\""));
        assert!(html.contains("data-role=\"signature-help-popup\""));
        assert!(html.contains("data-completion-id=\"proposal-1\""));
        assert!(html.contains("data-completion-index=\"0\""));
        assert!(html.contains("data-proposal-kind=\"function\""));
        assert!(html.contains("data-doc-ref=\"preview:function:SUM\""));
        assert!(html.contains("data-requires-revalidation=\"true\""));
        assert!(html.contains("data-selected=\"true\""));
        assert!(html.contains("data-active-row=\"true\""));
        assert!(html.contains("aria-selected=\"true\""));
        assert!(html.contains("data-active-argument-index=\"1\""));
        assert!(html.contains("data-call-span-start=\"0\""));
        assert!(html.contains("data-call-span-len=\"9\""));
        assert!(html.contains("data-role=\"signature-help-signature-argument\""));
        assert!(html.contains("data-active=\"true\""));
        assert!(html.contains("data-anchor-span-start=\"1\""));
        assert!(html.contains("data-anchor-span-len=\"3\""));
        assert!(html.contains("data-role=\"editor-entry-mode-pill\""));
        assert!(html.contains("data-entry-mode=\"formula\""));
        assert!(html.contains("data-role=\"editor-result-class-pill\""));
        assert!(html.contains("data-has-result=\"true\""));
        assert!(html.contains("data-role=\"editor-live-state-pill\""));
        assert!(html.contains("data-live-state=\"editing-live\""));
        assert!(html.contains("data-role=\"editor-effective-display\""));
        assert!(html.contains("data-role=\"bracket-pair-layer\""));
        assert!(html.contains("data-role=\"bracket-pair-open\""));
        assert!(html.contains("data-bracket-offset=\"4\""));
        assert!(html.contains("data-role=\"bracket-pair-close\""));
        assert!(html.contains("data-bracket-offset=\"8\""));
        assert!(html.contains("data-role=\"editor-settings-gear\""));
        assert!(html.contains("data-role=\"editor-settings-popover\""));
        assert!(html.contains("data-role=\"setting-toggle-auto-close-brackets\""));
        assert!(html.contains("data-role=\"setting-completion-aggressiveness\""));
        assert!(html.contains("data-role=\"setting-help-placement\""));
        assert!(html.contains("data-role=\"setting-toggle-auto-proof\""));
        assert!(html.contains("data-fallback-mode=\"false\""));
    }

    #[test]
    fn formula_editor_surface_renders_fallback_mode_when_reduce_motion_enabled() {
        let fallback_settings = crate::ui::editor::state::EditorSettings {
            reduce_motion: true,
            ..crate::ui::editor::state::EditorSettings::default()
        };
        let html = view! {
            <FormulaEditorSurface
                editor=ExploreEditorClusterViewModel {
                    raw_entered_cell_text: "=1+1".to_string(),
                    scenario_label: "fallback".to_string(),
                    truth_source_label: "local-fallback".to_string(),
                    host_profile_summary: "headless".to_string(),
                    packet_kind_summary: "preview".to_string(),
                    capability_floor_summary: "Explore".to_string(),
                    mode_availability_summary: "Explore".to_string(),
                    trace_summary: None,
                    blocked_reason: None,
                    syntax_runs: vec![],
                    diagnostics: vec![],
                    completion_count: 0,
                    completion_items: vec![],
                    selected_completion_proposal_id: None,
                    selected_completion_item: None,
                    help_sync_lookup_key: None,
                    has_signature_help: false,
                    signature_help: None,
                    function_help: None,
                    function_help_lookup_key: None,
                    completion_anchor_span: None,
                    overlay_geometry: None,
                    green_tree_key: None,
                    reused_green_tree: false,
                    entry_mode: crate::ui::editor::state::EditorEntryMode::Formula,
                    live_state: crate::ui::editor::state::EditorLiveState::Idle,
                    expanded_editor: false,
                    result_value_summary: None,
                    effective_display_summary: None,
                    bracket_pair: None,
                    editor_settings: fallback_settings,
                    editor_settings_popover_open: false,
                    configure_drawer_open: false,
                    editor_surface_state: EditorSurfaceState::for_text("=1+1"),
                }
            />
        }
        .to_html();

        assert!(html.contains("data-fallback-mode=\"true\""));
        assert!(html.contains("data-component=\"formula-editor-surface\""));
        assert!(html.contains("data-role=\"editor-input\""));
    }
}
