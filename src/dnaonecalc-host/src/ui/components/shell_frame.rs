use leptos::prelude::*;

use crate::services::shell_composition::{
    mode_accent_slug, ShellFormulaSpaceListItemViewModel, ShellFrameViewModel, ShellRailSection,
};
use crate::state::AppMode;
use crate::ui::components::shell_drawer::ShellDrawer;

fn encode_data_uri_payload(payload: &str) -> String {
    let mut encoded = String::with_capacity(payload.len());
    for byte in payload.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

#[cfg(target_arch = "wasm32")]
fn copy_payload_to_clipboard(payload: &str) {
    if let Some(window) = web_sys::window() {
        let clipboard = window.navigator().clipboard();
        let _ = clipboard.write_text(payload);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_payload_to_clipboard(_payload: &str) {}

fn render_rail_section(
    section: ShellRailSection,
    formula_spaces: &[ShellFormulaSpaceListItemViewModel],
    on_formula_space_select: Option<Callback<String>>,
    on_reopen_formula_space: Option<Callback<String>>,
    on_close_formula_space: Option<Callback<String>>,
    on_toggle_pin_formula_space: Option<Callback<String>>,
) -> impl IntoView {
    let rows: Vec<_> = formula_spaces
        .iter()
        .filter(|item| item.section == section)
        .cloned()
        .collect();
    let section_slug = section.slug();
    let section_label = section.label();
    let count = rows.len();
    let is_empty = rows.is_empty();
    view! {
        <section
            class="onecalc-shell-frame__rail-section"
            data-role="shell-rail-section"
            data-section=section_slug
        >
            <header class="onecalc-shell-frame__rail-section-title">
                <span>{section_label}</span>
                <span
                    class="onecalc-shell-frame__rail-section-count"
                    data-role="shell-rail-section-count"
                >
                    {count}
                </span>
            </header>
            {if is_empty {
                let placeholder_label = match section {
                    ShellRailSection::Pinned => "No pinned spaces",
                    ShellRailSection::Open => "No open spaces",
                    ShellRailSection::Recent => "No recent documents",
                };
                view! {
                    <p
                        class="onecalc-shell-frame__rail-section-empty"
                        data-role="shell-rail-section-empty"
                    >
                        {placeholder_label}
                    </p>
                }
                .into_any()
            } else {
                view! {
                    <ul class="onecalc-shell-frame__space-list">
                        {rows
                            .into_iter()
                            .map(|formula_space| render_rail_row(
                                formula_space,
                                on_formula_space_select.clone(),
                                on_reopen_formula_space.clone(),
                                on_close_formula_space.clone(),
                                on_toggle_pin_formula_space.clone(),
                            ))
                            .collect_view()}
                    </ul>
                }
                .into_any()
            }}
        </section>
    }
}

fn render_rail_row(
    formula_space: ShellFormulaSpaceListItemViewModel,
    on_formula_space_select: Option<Callback<String>>,
    on_reopen_formula_space: Option<Callback<String>>,
    on_close_formula_space: Option<Callback<String>>,
    on_toggle_pin_formula_space: Option<Callback<String>>,
) -> impl IntoView {
    let can_reopen = formula_space.can_reopen;
    let item_class = if formula_space.is_active {
        "onecalc-shell-frame__space-item onecalc-shell-frame__space-item--active"
    } else {
        "onecalc-shell-frame__space-item"
    };
    let data_state = if can_reopen {
        "recent"
    } else if formula_space.is_active {
        "active"
    } else {
        "idle"
    };
    let formula_space_id = formula_space.formula_space_id.clone();
    let formula_space_id_for_close = formula_space.formula_space_id.clone();
    let formula_space_id_for_pin = formula_space.formula_space_id.clone();
    let verdicts = formula_space.retained_verdicts.clone();
    let meta_summary = format!(
        "{} · {}",
        formula_space.truth_source_label.clone(),
        formula_space.mode_label
    );
    view! {
        <li
            class=item_class
            data-state=data_state
            data-pinned=if formula_space.is_pinned { "true" } else { "false" }
            data-dirty=if formula_space.is_dirty { "true" } else { "false" }
            data-formula-space-id=formula_space.formula_space_id.clone()
        >
            <button
                type="button"
                class="onecalc-shell-frame__space-button"
                data-role=if can_reopen {
                    "formula-space-reopen"
                } else {
                    "formula-space-select"
                }
                data-formula-space-id=formula_space.formula_space_id.clone()
                on:click=move |_| {
                    if can_reopen {
                        if let Some(callback) = on_reopen_formula_space.as_ref() {
                            callback.run(formula_space_id.clone());
                        }
                    } else if let Some(callback) = on_formula_space_select.as_ref() {
                        callback.run(formula_space_id.clone());
                    }
                }
            >
                <span class="onecalc-shell-frame__space-button-header">
                    {if formula_space.is_dirty {
                        view! {
                            <span
                                class="onecalc-shell-frame__space-dirty-dot"
                                data-role="shell-rail-dirty-dot"
                                aria-label="Unsaved changes"
                            ></span>
                        }
                        .into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                    <span class="onecalc-shell-frame__space-button-label">
                        {formula_space.label.clone()}
                    </span>
                </span>
                <span class="onecalc-shell-frame__space-button-meta">
                    {meta_summary.clone()}
                </span>
                <span class="onecalc-shell-frame__space-button-packet">
                    {formula_space.packet_kind_summary.clone()}
                </span>
                {if can_reopen {
                    view! {
                        <span
                            class="onecalc-shell-frame__space-reopen-tag"
                            data-role="shell-rail-reopen-tag"
                        >
                            "Reopen isolated document"
                        </span>
                    }
                    .into_any()
                } else {
                    view! { <></> }.into_any()
                }}
            </button>
            {verdicts.map(|verdicts| {
                view! {
                    <div
                        class="onecalc-shell-frame__space-verdicts"
                        data-role="shell-rail-verdicts"
                        data-comparison-lane=verdicts.comparison_lane_label
                    >
                        <span
                            class="onecalc-shell-frame__space-verdict"
                            data-role="shell-rail-verdict-value"
                            data-verdict=verdict_slug(verdicts.value_match)
                            title="value_match"
                        >
                            "V"
                        </span>
                        <span
                            class="onecalc-shell-frame__space-verdict"
                            data-role="shell-rail-verdict-display"
                            data-verdict=verdict_slug(verdicts.display_match)
                            title="display_match"
                        >
                            "D"
                        </span>
                        <span
                            class="onecalc-shell-frame__space-verdict"
                            data-role="shell-rail-verdict-replay"
                            data-verdict=verdict_slug(verdicts.replay_equivalent)
                            title="replay_equivalent"
                        >
                            "R"
                        </span>
                        <span
                            class="onecalc-shell-frame__space-verdict-lane"
                            data-role="shell-rail-verdict-lane"
                        >
                            {verdicts.comparison_lane_label}
                        </span>
                    </div>
                }
            })}
            {if can_reopen {
                view! { <></> }.into_any()
            } else {
                view! {
                    <div class="onecalc-shell-frame__space-affordances" data-role="shell-rail-affordances">
                        <button
                            type="button"
                            class="onecalc-shell-frame__space-affordance"
                            data-role="shell-rail-affordance-pin"
                            data-pinned=if formula_space.is_pinned { "true" } else { "false" }
                            aria-label="Toggle pinned"
                            on:click=move |_| {
                                if let Some(callback) = on_toggle_pin_formula_space.as_ref() {
                                    callback.run(formula_space_id_for_pin.clone());
                                }
                            }
                        >
                            {if formula_space.is_pinned { "Unpin" } else { "Pin" }}
                        </button>
                        <button
                            type="button"
                            class="onecalc-shell-frame__space-affordance"
                            data-role="shell-rail-affordance-close"
                            aria-label="Close formula space"
                            on:click=move |_| {
                                if let Some(callback) = on_close_formula_space.as_ref() {
                                    callback.run(formula_space_id_for_close.clone());
                                }
                            }
                        >
                            "×"
                        </button>
                    </div>
                }
                .into_any()
            }}
        </li>
    }
}

fn verdict_slug(verdict: Option<bool>) -> &'static str {
    match verdict {
        Some(true) => "pass",
        Some(false) => "fail",
        None => "unobserved",
    }
}

#[component]
pub fn ShellFrame(
    frame: ShellFrameViewModel,
    on_mode_select: Option<Callback<AppMode>>,
    #[prop(default = None)] on_formula_space_select: Option<Callback<String>>,
    #[prop(default = None)] on_reopen_formula_space: Option<Callback<String>>,
    #[prop(default = None)] on_capability_diff_target_select: Option<Callback<String>>,
    #[prop(default = None)] on_new_formula_space: Option<Callback<()>>,
    #[prop(default = None)] on_close_formula_space: Option<Callback<String>>,
    #[prop(default = None)] on_toggle_pin_formula_space: Option<Callback<String>>,
    #[prop(default = None)] on_configure_toggle: Option<Callback<()>>,
    #[prop(default = false)] configure_drawer_open: bool,
    children: Children,
) -> impl IntoView {
    let accent_slug = mode_accent_slug(frame.active_mode);
    view! {
        <div class="onecalc-shell-frame" data-active-mode=accent_slug>
            <aside class="onecalc-shell-frame__rail">
                <div class="onecalc-shell-frame__brand-block">
                    <div class="onecalc-shell-frame__eyebrow">"DNA Calc"</div>
                    <h1>"DNA OneCalc"</h1>
                    <p class="onecalc-shell-frame__brand-copy">
                        "Explore live OxFml execution, inspect replay evidence, and triage retained Excel comparisons from one shell."
                    </p>
                </div>
                <section class="onecalc-shell-frame__active-card" data-role="active-space-context">
                    <div class="onecalc-shell-frame__eyebrow">"Active space"</div>
                    <strong>{frame.active_formula_space_label.clone()}</strong>
                    <div class="onecalc-shell-frame__active-meta">
                        <span data-role="active-space-truth-source">{frame.active_truth_source_label.clone()}</span>
                        <span data-role="active-space-mode">{frame.active_mode_label}</span>
                    </div>
                    <div
                        class="onecalc-shell-frame__active-capability"
                        data-role="active-space-capability-floor"
                    >
                        {frame.active_capability_floor_summary.clone()}
                    </div>
                    <div class="onecalc-shell-frame__workspace-summary" data-role="workspace-summary">
                        {frame.workspace_summary.clone()}
                    </div>
                </section>
                <section
                    class="onecalc-shell-frame__workspace-manifest"
                    data-role="workspace-manifest"
                >
                    <div class="onecalc-shell-frame__eyebrow">"Workspace manifest"</div>
                    <div class="onecalc-shell-frame__workspace-manifest-grid">
                        <div
                            class="onecalc-shell-frame__workspace-manifest-metric"
                            data-role="workspace-manifest-open"
                        >
                            <span>"Open"</span>
                            <strong>{frame.workspace_manifest.open_count}</strong>
                        </div>
                        <div
                            class="onecalc-shell-frame__workspace-manifest-metric"
                            data-role="workspace-manifest-pinned"
                        >
                            <span>"Pinned"</span>
                            <strong>{frame.workspace_manifest.pinned_count}</strong>
                        </div>
                        <div
                            class="onecalc-shell-frame__workspace-manifest-metric"
                            data-role="workspace-manifest-recent"
                        >
                            <span>"Recent"</span>
                            <strong>{frame.workspace_manifest.recent_count}</strong>
                        </div>
                    </div>
                    <p class="onecalc-shell-frame__workspace-manifest-note" data-role="workspace-isolation-note">
                        {frame.workspace_manifest.isolation_note}
                    </p>
                </section>
                <div class="onecalc-shell-frame__rail-section-header" data-role="shell-rail-actions">
                    <span class="onecalc-shell-frame__eyebrow">"Formula spaces"</span>
                    <div class="onecalc-shell-frame__rail-action-buttons">
                        {{
                            let new_callback = on_new_formula_space.clone();
                            view! {
                                <button
                                    type="button"
                                    class="onecalc-shell-frame__rail-action-button"
                                    data-role="shell-rail-new-space"
                                    aria-label="New formula space"
                                    on:click=move |_| {
                                        if let Some(callback) = new_callback.as_ref() {
                                            callback.run(());
                                        }
                                    }
                                >
                                    "+"
                                </button>
                            }
                        }}
                    </div>
                </div>
                {render_rail_section(
                    ShellRailSection::Pinned,
                    &frame.formula_spaces,
                    on_formula_space_select.clone(),
                    on_reopen_formula_space.clone(),
                    on_close_formula_space.clone(),
                    on_toggle_pin_formula_space.clone(),
                )}
                {render_rail_section(
                    ShellRailSection::Open,
                    &frame.formula_spaces,
                    on_formula_space_select.clone(),
                    on_reopen_formula_space.clone(),
                    on_close_formula_space.clone(),
                    on_toggle_pin_formula_space.clone(),
                )}
                {render_rail_section(
                    ShellRailSection::Recent,
                    &frame.formula_spaces,
                    on_formula_space_select.clone(),
                    on_reopen_formula_space.clone(),
                    on_close_formula_space.clone(),
                    on_toggle_pin_formula_space.clone(),
                )}
            </aside>

            <main class="onecalc-shell-frame__content">
                <header class="onecalc-shell-frame__context-bar">
                    <nav
                        class="onecalc-shell-frame__breadcrumb"
                        data-role="shell-breadcrumb"
                        aria-label="Shell breadcrumb"
                    >
                        <span
                            class="onecalc-shell-frame__breadcrumb-segment"
                            data-role="shell-breadcrumb-workspace"
                        >
                            {frame.breadcrumb.workspace_label.clone()}
                        </span>
                        <span
                            class="onecalc-shell-frame__breadcrumb-separator"
                            data-role="shell-breadcrumb-separator"
                        >
                            "›"
                        </span>
                        <span
                            class="onecalc-shell-frame__breadcrumb-segment onecalc-shell-frame__breadcrumb-segment--space"
                            data-role="shell-breadcrumb-space"
                        >
                            {frame.breadcrumb.space_label.clone()}
                        </span>
                        <span
                            class="onecalc-shell-frame__breadcrumb-separator"
                            data-role="shell-breadcrumb-separator"
                        >
                            "›"
                        </span>
                        <span
                            class="onecalc-shell-frame__breadcrumb-segment onecalc-shell-frame__breadcrumb-segment--mode"
                            data-role="shell-breadcrumb-mode"
                            data-mode=accent_slug
                        >
                            {frame.breadcrumb.mode_label}
                        </span>
                    </nav>
                    <div
                        class="onecalc-shell-frame__scope-strip"
                        data-role="shell-scope-strip"
                    >
                        {frame
                            .scope_strip
                            .iter()
                            .map(|segment| {
                                let status_slug = segment.status.slug();
                                let seam_id = segment.status.seam_id().unwrap_or_default();
                                view! {
                                    <div
                                        class="onecalc-shell-frame__scope-segment"
                                        data-role="shell-scope-segment"
                                        data-segment=segment.slug
                                        data-status=status_slug
                                        data-seam-id=seam_id
                                        title=if seam_id.is_empty() {
                                            format!("{}: {}", segment.label, segment.value)
                                        } else {
                                            format!("{}: {} — {}", segment.label, segment.value, seam_id)
                                        }
                                    >
                                        <span
                                            class="onecalc-shell-frame__scope-segment-label"
                                            data-role="shell-scope-segment-label"
                                        >
                                            {segment.label}
                                        </span>
                                        <strong
                                            class="onecalc-shell-frame__scope-segment-value"
                                            data-role="shell-scope-segment-value"
                                        >
                                            {segment.value.clone()}
                                        </strong>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                    <nav class="onecalc-shell-frame__mode-switch">
                        {frame
                            .mode_tabs
                            .iter()
                            .map(|tab| {
                                let tab_mode = tab.mode;
                                let on_mode_select = on_mode_select.clone();
                                let button_class = if tab.is_active {
                                    "onecalc-shell-frame__mode-button onecalc-shell-frame__mode-button--active"
                                } else {
                                    "onecalc-shell-frame__mode-button"
                                };
                                view! {
                                    <button
                                        type="button"
                                        class=button_class
                                        data-mode=tab.label
                                        data-state=if tab.is_active { "active" } else { "idle" }
                                        on:click=move |_| {
                                            if let Some(callback) = on_mode_select.as_ref() {
                                                callback.run(tab_mode);
                                            }
                                        }
                                    >
                                        {tab.label}
                                    </button>
                                }
                            })
                            .collect_view()}
                        {{
                            let configure_callback = on_configure_toggle.clone();
                            view! {
                                <button
                                    type="button"
                                    class="onecalc-shell-frame__configure-action"
                                    data-role="shell-frame-configure-toggle"
                                    data-open=if configure_drawer_open { "true" } else { "false" }
                                    aria-label="Toggle Capability Center"
                                    aria-expanded=if configure_drawer_open { "true" } else { "false" }
                                    on:click=move |_| {
                                        if let Some(callback) = configure_callback.as_ref() {
                                            callback.run(());
                                        }
                                    }
                                >
                                    {if configure_drawer_open {
                                        "Close capability center"
                                    } else {
                                        "Capability Center"
                                    }}
                                </button>
                            }
                        }}
                    </nav>
                </header>

                <section
                    class="onecalc-shell-frame__content-main"
                    data-role="shell-content-main"
                    data-drawer-open=if configure_drawer_open { "true" } else { "false" }
                >
                    <section class="onecalc-shell-frame__mode-body">
                        {children()}
                    </section>
                    {if configure_drawer_open {
                        let export_href = format!(
                            "data:application/json;charset=utf-8,{}",
                            encode_data_uri_payload(&frame.capability_center.snapshot_json),
                        );
                        view! {
                            <ShellDrawer
                                drawer_kind="capability-center".to_string()
                                title=frame.capability_center.title.clone()
                                subtitle=Some(frame.capability_center.subtitle.clone())
                                is_open=true
                                on_close=on_configure_toggle.clone()
                            >
                                <section class="onecalc-capability-center" data-role="capability-center">
                                    <div class="onecalc-capability-center__summary" data-role="capability-center-summary">
                                        {frame
                                            .capability_center
                                            .summary
                                            .iter()
                                            .map(|fact| view! {
                                                <article
                                                    class="onecalc-capability-center__summary-card"
                                                    data-role="capability-center-summary-card"
                                                    data-tone=fact.tone
                                                >
                                                    <span class="onecalc-capability-center__summary-label">{fact.label}</span>
                                                    <strong class="onecalc-capability-center__summary-value">{fact.value.clone()}</strong>
                                                </article>
                                            })
                                            .collect_view()}
                                    </div>
                                    <div class="onecalc-capability-center__actions" data-role="capability-center-actions">
                                        <button
                                            type="button"
                                            class="onecalc-capability-center__action"
                                            data-role="capability-center-copy"
                                            data-copy-payload=frame.capability_center.snapshot_json.clone()
                                            on:click={
                                                let snapshot_json = frame.capability_center.snapshot_json.clone();
                                                move |_| copy_payload_to_clipboard(&snapshot_json)
                                            }
                                        >
                                            "Copy snapshot"
                                        </button>
                                        <a
                                            class="onecalc-capability-center__action onecalc-capability-center__action--link"
                                            data-role="capability-center-export"
                                            href=export_href
                                            download=frame.capability_center.export_file_name.clone()
                                        >
                                            "Export JSON"
                                        </a>
                                    </div>
                                    <div class="onecalc-capability-center__diff" data-role="capability-center-diff">
                                        <header class="onecalc-capability-center__diff-header">
                                            <div>
                                                <div class="onecalc-shell-frame__eyebrow">"Diff target"</div>
                                                <strong>"Compare capability truth"</strong>
                                            </div>
                                            <select
                                                class="onecalc-capability-center__diff-target"
                                                data-role="capability-center-diff-target"
                                                on:change=move |ev| {
                                                    if let Some(callback) = on_capability_diff_target_select.as_ref() {
                                                        callback.run(event_target_value(&ev));
                                                    }
                                                }
                                                prop:value=frame.capability_center.selected_diff_target_slug.clone()
                                            >
                                                {frame
                                                    .capability_center
                                                    .diff_target_options
                                                    .iter()
                                                    .map(|option| view! {
                                                        <option value=option.slug.clone()>{option.label.clone()}</option>
                                                    })
                                                    .collect_view()}
                                            </select>
                                        </header>
                                        <div class="onecalc-capability-center__diff-rows">
                                            {frame
                                                .capability_center
                                                .diff_rows
                                                .iter()
                                                .map(|row| view! {
                                                    <article
                                                        class="onecalc-capability-center__diff-row"
                                                        data-role="capability-center-diff-row"
                                                        data-status=row.status
                                                    >
                                                        <div class="onecalc-capability-center__diff-row-label">{row.label}</div>
                                                        <div class="onecalc-capability-center__diff-row-values">
                                                            <span data-role="capability-center-diff-current">{row.current_value.clone()}</span>
                                                            <span data-role="capability-center-diff-target-value">{row.target_value.clone()}</span>
                                                        </div>
                                                    </article>
                                                })
                                                .collect_view()}
                                        </div>
                                    </div>
                                    <div class="onecalc-capability-center__payload" data-role="capability-center-payload">
                                        <div class="onecalc-shell-frame__eyebrow">"Snapshot payload"</div>
                                        <textarea
                                            class="onecalc-capability-center__payload-text"
                                            data-role="capability-center-payload-text"
                                            readonly
                                            prop:value=frame.capability_center.snapshot_json.clone()
                                        />
                                    </div>
                                </section>
                            </ShellDrawer>
                        }
                        .into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </section>

                <footer class="onecalc-shell-frame__footer" data-role="shell-footer">
                    {frame
                        .footer_facts
                        .iter()
                        .map(|fact| {
                            view! {
                                <div
                                    class="onecalc-shell-frame__footer-fact"
                                    data-tone=fact.tone
                                    data-label=fact.label
                                >
                                    <span class="onecalc-shell-frame__footer-fact-label">{fact.label}</span>
                                    <strong class="onecalc-shell-frame__footer-fact-value">{fact.value.clone()}</strong>
                                </div>
                            }
                        })
                        .collect_view()}
                </footer>
            </main>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::shell_composition::{
        CapabilityCenterFactViewModel, CapabilityCenterViewModel, CapabilityDiffRowViewModel,
        CapabilityDiffTargetOptionViewModel, ShellBreadcrumbViewModel,
        ShellFormulaSpaceListItemViewModel, ShellModeTabViewModel, ShellRailSection,
        ShellRetainedVerdictsViewModel, ShellScopeSegmentStatus, ShellScopeSegmentViewModel,
    };

    #[test]
    fn shell_frame_renders_space_list_and_mode_tabs() {
        let html = view! {
            <ShellFrame
                frame=ShellFrameViewModel {
                    active_mode: AppMode::Explore,
                    active_formula_space_label: "space-1".to_string(),
                    active_mode_label: "Explore",
                    breadcrumb: ShellBreadcrumbViewModel {
                        workspace_label: "DNA OneCalc".to_string(),
                        space_label: "space-1".to_string(),
                        mode_label: "Explore",
                    },
                    scope_strip: vec![
                        ShellScopeSegmentViewModel {
                            slug: "locale",
                            label: "Locale",
                            value: "en-US".to_string(),
                            status: ShellScopeSegmentStatus::NotImplemented {
                                seam_id: "SEAM-OXFUNC-LOCALE-EXPAND",
                            },
                        },
                        ShellScopeSegmentViewModel {
                            slug: "profile",
                            label: "Profile",
                            value: "windows".to_string(),
                            status: ShellScopeSegmentStatus::Live,
                        },
                    ],
                    active_truth_source_label: "live-backed".to_string(),
                    active_host_profile_summary: "Windows Excel default".to_string(),
                    active_packet_kind_summary: "verification publication".to_string(),
                    active_capability_floor_summary: "Explore + Inspect + Workbench".to_string(),
                    context_facts: vec![
                        crate::services::shell_composition::ShellChromeFactViewModel {
                            label: "Truth",
                            value: "live-backed".to_string(),
                            tone: "accent",
                        },
                        crate::services::shell_composition::ShellChromeFactViewModel {
                            label: "Host",
                            value: "Windows Excel default".to_string(),
                            tone: "default",
                        },
                    ],
                    footer_facts: vec![
                        crate::services::shell_composition::ShellChromeFactViewModel {
                            label: "Capability",
                            value: "Explore + Inspect + Workbench".to_string(),
                            tone: "default",
                        },
                    ],
                    workspace_summary: "1 open · 1 pinned · 1 recent".to_string(),
                    workspace_manifest: crate::services::shell_composition::ShellWorkspaceManifestViewModel {
                        open_count: 1,
                        pinned_count: 1,
                        recent_count: 1,
                        isolation_note: "Documents remain isolated OneCalc instances.",
                    },
                    capability_center: CapabilityCenterViewModel {
                        title: "Capability Center".to_string(),
                        subtitle: "Supporting honesty surface for capability truth, diff targeting, copy, and export.".to_string(),
                        summary: vec![
                            CapabilityCenterFactViewModel {
                                label: "Mode",
                                value: "Explore".to_string(),
                                tone: "accent",
                            },
                            CapabilityCenterFactViewModel {
                                label: "Diff target",
                                value: "Workspace baseline".to_string(),
                                tone: "muted",
                            },
                        ],
                        snapshot_json: "{\"mode\":\"Explore\"}".to_string(),
                        export_file_name: "onecalc-capability-space-1.json".to_string(),
                        diff_target_options: vec![
                            CapabilityDiffTargetOptionViewModel {
                                slug: "workspace-baseline".to_string(),
                                label: "Workspace baseline".to_string(),
                            },
                            CapabilityDiffTargetOptionViewModel {
                                slug: "recent:space-2".to_string(),
                                label: "Recent · space-2".to_string(),
                            },
                        ],
                        selected_diff_target_slug: "workspace-baseline".to_string(),
                        diff_rows: vec![CapabilityDiffRowViewModel {
                            label: "Host profile",
                            current_value: "windows".to_string(),
                            target_value: "Workspace-scoped host truth".to_string(),
                            status: "changed",
                        }],
                    },
                    mode_tabs: vec![
                        ShellModeTabViewModel {
                            mode: AppMode::Explore,
                            label: "Explore",
                            is_active: true,
                        },
                        ShellModeTabViewModel {
                            mode: AppMode::Inspect,
                            label: "Inspect",
                            is_active: false,
                        },
                    ],
                    formula_spaces: vec![ShellFormulaSpaceListItemViewModel {
                        formula_space_id: "space-1".to_string(),
                        label: "space-1".to_string(),
                        truth_source_label: "live-backed".to_string(),
                        packet_kind_summary: "verification publication".to_string(),
                        mode_label: "Explore",
                        is_active: true,
                        is_pinned: true,
                        is_dirty: true,
                        can_reopen: false,
                        section: ShellRailSection::Pinned,
                        retained_verdicts: Some(ShellRetainedVerdictsViewModel {
                            value_match: Some(true),
                            display_match: Some(false),
                            replay_equivalent: None,
                            comparison_lane_label: "Mismatched",
                        }),
                    }, ShellFormulaSpaceListItemViewModel {
                        formula_space_id: "space-2".to_string(),
                        label: "space-2".to_string(),
                        truth_source_label: "local-fallback".to_string(),
                        packet_kind_summary: "scratch snapshot".to_string(),
                        mode_label: "Inspect",
                        is_active: false,
                        is_pinned: false,
                        is_dirty: false,
                        can_reopen: true,
                        section: ShellRailSection::Recent,
                        retained_verdicts: None,
                    }],
                }
                on_mode_select=None
                on_formula_space_select=None
                on_reopen_formula_space=None
                configure_drawer_open=true
            >
                <div>"Body"</div>
            </ShellFrame>
        }
        .to_html();

        assert!(html.contains("DNA OneCalc"));
        assert!(html.contains("data-role=\"active-space-context\""));
        assert!(html.contains("data-role=\"active-space-truth-source\""));
        assert!(html.contains("data-role=\"shell-footer\""));
        assert!(html.contains("space-1"));
        assert!(html.contains("data-mode=\"Explore\""));
        assert!(html.contains("data-role=\"formula-space-select\""));
        assert!(html.contains("data-state=\"active\""));
        assert!(html.contains("Pinned"));
        assert!(html.contains("verification publication"));
        assert!(html.contains("1 open · 1 pinned · 1 recent"));
        assert!(html.contains("Body"));
        assert!(html.contains("data-active-mode=\"explore\""));
        assert!(html.contains("data-role=\"shell-breadcrumb\""));
        assert!(html.contains("data-role=\"shell-breadcrumb-workspace\""));
        assert!(html.contains("DNA OneCalc"));
        assert!(html.contains("data-role=\"shell-breadcrumb-space\""));
        assert!(html.contains("data-role=\"shell-breadcrumb-mode\""));
        assert!(html.contains("data-role=\"shell-scope-strip\""));
        assert!(html.contains("data-role=\"shell-scope-segment\""));
        assert!(html.contains("data-segment=\"locale\""));
        assert!(html.contains("data-segment=\"profile\""));
        assert!(html.contains("data-status=\"not-implemented\""));
        assert!(html.contains("data-status=\"live\""));
        assert!(html.contains("SEAM-OXFUNC-LOCALE-EXPAND"));
        assert!(html.contains("data-role=\"shell-rail-new-space\""));
        assert!(html.contains("data-role=\"shell-rail-section\""));
        assert!(html.contains("data-section=\"pinned\""));
        assert!(html.contains("data-section=\"open\""));
        assert!(html.contains("data-section=\"recent\""));
        assert!(html.contains("data-role=\"shell-rail-dirty-dot\""));
        assert!(html.contains("data-role=\"shell-rail-verdicts\""));
        assert!(html.contains("data-role=\"shell-rail-verdict-value\""));
        assert!(html.contains("data-verdict=\"pass\""));
        assert!(html.contains("data-verdict=\"fail\""));
        assert!(html.contains("data-verdict=\"unobserved\""));
        assert!(html.contains("data-role=\"shell-rail-affordance-pin\""));
        assert!(html.contains("data-role=\"shell-rail-affordance-close\""));
        assert!(html.contains("data-role=\"workspace-manifest\""));
        assert!(html.contains("data-role=\"workspace-isolation-note\""));
        assert!(html.contains("data-role=\"formula-space-reopen\""));
        assert!(html.contains("data-role=\"shell-rail-reopen-tag\""));
        assert!(html.contains("data-role=\"shell-content-main\""));
        assert!(html.contains("data-role=\"capability-center\""));
        assert!(html.contains("data-role=\"capability-center-copy\""));
        assert!(html.contains("data-role=\"capability-center-export\""));
        assert!(html.contains("data-role=\"capability-center-diff-target\""));
        assert!(html.contains("data-role=\"capability-center-diff-row\""));
        assert!(html.contains("data-role=\"capability-center-payload-text\""));
        assert!(html.contains("Capability Center"));
        assert!(html.contains("workspace-baseline"));
    }
}
