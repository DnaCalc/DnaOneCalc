use leptos::prelude::*;

use crate::ui::panels::value_panel_model::{
    ValuePanelArrayCell, ValuePanelPipelineStepStatus, ValuePanelPresentation, ValuePanelRichValue,
    ValuePanelRichValueData, ValuePanelValue, ValuePanelViewModel,
};

#[component]
pub fn ValuePanel(panel: ValuePanelViewModel) -> impl IntoView {
    let slug = panel.value.slug();
    let header_label = panel.value.header_label();
    let live_state_slug = panel.live_state.slug();

    view! {
        <section
            class="onecalc-value-panel"
            data-component="value-panel"
            data-value-kind=slug
            data-live-state=live_state_slug
        >
            <header class="onecalc-value-panel__header" data-role="value-panel-header">
                <span class="onecalc-value-panel__header-label" data-role="value-panel-header-label">
                    {header_label.clone()}
                </span>
                <span class="onecalc-value-panel__header-kind-tag" data-role="value-panel-kind-tag">
                    {slug}
                </span>
            </header>
            <div class="onecalc-value-panel__primary" data-role="value-panel-primary">
                {render_primary_section(&panel.value)}
            </div>
            {panel.presentation.as_ref().map(|presentation| {
                let presentation = presentation.clone();
                render_presentation_section(presentation).into_any()
            })}
            {render_effective_display_section(
                panel.effective_display_text.clone(),
                panel.display_pipeline.clone(),
            )}
            {panel.provenance.as_ref().map(|provenance| {
                view! {
                    <footer class="onecalc-value-panel__provenance" data-role="value-panel-provenance">
                        {provenance
                            .green_tree_key
                            .clone()
                            .map(|key| view! {
                                <div data-role="value-panel-provenance-green-tree">
                                    <span>"Green tree: "</span>
                                    <code>{key}</code>
                                </div>
                            })}
                        {provenance
                            .walk_node_id
                            .clone()
                            .map(|id| view! {
                                <div data-role="value-panel-provenance-walk-node">
                                    <span>"Walk node: "</span>
                                    <code>{id}</code>
                                </div>
                            })}
                    </footer>
                }
            })}
        </section>
    }
}

fn render_primary_section(value: &ValuePanelValue) -> AnyView {
    match value {
        ValuePanelValue::Number { raw, detail_rows } => {
            let raw_label = if raw.is_nan() {
                "unresolved".to_string()
            } else {
                format!("{raw}")
            };
            view! {
                <div class="onecalc-value-panel__number" data-role="value-panel-number">
                    <div class="onecalc-value-panel__number-main" data-role="value-panel-number-raw">
                        {raw_label}
                    </div>
                    {if detail_rows.is_empty() {
                        view! { <></> }.into_any()
                    } else {
                        let rows = detail_rows.clone();
                        view! {
                            <dl class="onecalc-value-panel__kv-list" data-role="value-panel-number-details">
                                {rows
                                    .into_iter()
                                    .map(|row| view! {
                                        <div class="onecalc-value-panel__kv-row">
                                            <dt>{row.label}</dt>
                                            <dd>{row.value}</dd>
                                        </div>
                                    })
                                    .collect_view()}
                            </dl>
                        }
                        .into_any()
                    }}
                </div>
            }
            .into_any()
        }
        ValuePanelValue::Text {
            content,
            utf16_code_units,
            utf16_code_unit_limit,
            has_dangling_surrogate,
        } => view! {
            <div class="onecalc-value-panel__text" data-role="value-panel-text">
                <div class="onecalc-value-panel__text-content" data-role="value-panel-text-content">
                    {content.clone()}
                </div>
                <div class="onecalc-value-panel__text-meta">
                    <span data-role="value-panel-text-utf16-count">
                        {*utf16_code_units}
                        " / "
                        {*utf16_code_unit_limit}
                        " UTF-16 code units"
                    </span>
                    {if *has_dangling_surrogate {
                        view! {
                            <span
                                class="onecalc-value-panel__text-warning"
                                data-role="value-panel-text-dangling-surrogate"
                            >
                                "Dangling surrogate detected"
                            </span>
                        }
                        .into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </div>
            </div>
        }
        .into_any(),
        ValuePanelValue::Logical(value) => view! {
            <div class="onecalc-value-panel__logical" data-role="value-panel-logical">
                {if *value { "TRUE" } else { "FALSE" }}
            </div>
        }
        .into_any(),
        ValuePanelValue::Error {
            code_label,
            code_numeric,
            surface,
            detail,
        } => view! {
            <div class="onecalc-value-panel__error" data-role="value-panel-error">
                <div class="onecalc-value-panel__error-code" data-role="value-panel-error-code">
                    {code_label.clone()}
                </div>
                {code_numeric.map(|code| view! {
                    <div data-role="value-panel-error-numeric">"code "{code}</div>
                })}
                {surface.map(|surface| view! {
                    <div
                        class="onecalc-value-panel__error-surface"
                        data-role="value-panel-error-surface"
                        data-error-surface=surface.slug()
                    >
                        {surface.label()}
                    </div>
                })}
                {detail.clone().map(|detail| view! {
                    <p data-role="value-panel-error-detail">{detail}</p>
                })}
            </div>
        }
        .into_any(),
        ValuePanelValue::Array {
            shape,
            rows,
            truncated,
        } => {
            let rows = rows.clone();
            let shape = *shape;
            let truncated = *truncated;
            view! {
                <div class="onecalc-value-panel__array" data-role="value-panel-array">
                    <div class="onecalc-value-panel__array-header" data-role="value-panel-array-shape">
                        {shape.rows}
                        " rows × "
                        {shape.cols}
                        " cols"
                        {if truncated {
                            view! { <span data-role="value-panel-array-truncated">" · truncated"</span> }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }}
                    </div>
                    {if rows.is_empty() {
                        view! {
                            <div
                                class="onecalc-value-panel__array-empty"
                                data-role="value-panel-array-empty"
                            >
                                "No cells projected"
                            </div>
                        }
                        .into_any()
                    } else {
                        view! {
                            <table class="onecalc-value-panel__array-grid" data-role="value-panel-array-grid">
                                <tbody>
                                    {rows
                                        .into_iter()
                                        .enumerate()
                                        .map(|(row_index, row)| view! {
                                            <tr data-role="value-panel-array-row" data-row-index=row_index>
                                                {row
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(col_index, cell)| view! {
                                                        <td
                                                            class="onecalc-value-panel__array-cell"
                                                            data-role="value-panel-array-cell"
                                                            data-row-index=row_index
                                                            data-col-index=col_index
                                                        >
                                                            {render_array_cell_label(&cell)}
                                                        </td>
                                                    })
                                                    .collect_view()}
                                            </tr>
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        }
                        .into_any()
                    }}
                </div>
            }
            .into_any()
        }
        ValuePanelValue::Reference {
            kind,
            target,
            multi_area_targets,
            normalized_differs,
        } => view! {
            <div class="onecalc-value-panel__reference" data-role="value-panel-reference">
                <div
                    class="onecalc-value-panel__reference-kind"
                    data-role="value-panel-reference-kind"
                    data-reference-kind=kind.slug()
                >
                    {kind.label()}
                </div>
                <code data-role="value-panel-reference-target">{target.clone()}</code>
                {if multi_area_targets.is_empty() {
                    view! { <></> }.into_any()
                } else {
                    let multi_area_targets = multi_area_targets.clone();
                    view! {
                        <ul
                            class="onecalc-value-panel__reference-multi-area"
                            data-role="value-panel-reference-multi-area"
                        >
                            {multi_area_targets
                                .into_iter()
                                .map(|target| view! { <li><code>{target}</code></li> })
                                .collect_view()}
                        </ul>
                    }
                    .into_any()
                }}
                {if *normalized_differs {
                    view! {
                        <div
                            class="onecalc-value-panel__reference-warning"
                            data-role="value-panel-reference-normalized-warning"
                        >
                            "Normalized form differs from target"
                        </div>
                    }
                    .into_any()
                } else {
                    view! { <></> }.into_any()
                }}
            </div>
        }
        .into_any(),
        ValuePanelValue::Lambda {
            callable_token,
            origin_kind,
            arity_min,
            arity_max,
            capture_mode,
            invocation_contract_ref,
        } => view! {
            <dl class="onecalc-value-panel__lambda" data-role="value-panel-lambda">
                <div class="onecalc-value-panel__kv-row">
                    <dt>"Callable token"</dt>
                    <dd><code>{callable_token.clone()}</code></dd>
                </div>
                <div class="onecalc-value-panel__kv-row">
                    <dt>"Origin kind"</dt>
                    <dd>{origin_kind.clone()}</dd>
                </div>
                <div class="onecalc-value-panel__kv-row">
                    <dt>"Arity"</dt>
                    <dd>
                        {*arity_min}
                        ".."
                        {arity_max.map(|v| v.to_string()).unwrap_or_else(|| "∞".to_string())}
                    </dd>
                </div>
                <div class="onecalc-value-panel__kv-row">
                    <dt>"Capture mode"</dt>
                    <dd>{capture_mode.clone()}</dd>
                </div>
                <div class="onecalc-value-panel__kv-row">
                    <dt>"Contract ref"</dt>
                    <dd><code>{invocation_contract_ref.clone()}</code></dd>
                </div>
            </dl>
        }
        .into_any(),
        ValuePanelValue::RichValue(rich) => render_rich_value(rich).into_any(),
        ValuePanelValue::Unevaluated => view! {
            <div
                class="onecalc-value-panel__unevaluated"
                data-role="value-panel-unevaluated"
            >
                "No proved result yet. Commit or request proof to evaluate."
            </div>
        }
        .into_any(),
    }
}

fn render_rich_value(rich: &ValuePanelRichValue) -> AnyView {
    let type_name = rich.type_name.clone();
    let fallback = rich.fallback.clone();
    let kvps = rich.key_value_pairs.clone();
    view! {
        <div class="onecalc-value-panel__rich-value" data-role="value-panel-rich-value">
            <div class="onecalc-value-panel__rich-value-header" data-role="value-panel-rich-value-type">
                {type_name}
            </div>
            <div class="onecalc-value-panel__rich-value-fallback" data-role="value-panel-rich-value-fallback">
                <span>"Fallback: "</span>
                {render_rich_value_data(&fallback)}
            </div>
            {if kvps.is_empty() {
                view! { <></> }.into_any()
            } else {
                view! {
                    <dl class="onecalc-value-panel__rich-value-kvps" data-role="value-panel-rich-value-kvps">
                        {kvps
                            .into_iter()
                            .map(|kvp| view! {
                                <div
                                    class="onecalc-value-panel__kv-row"
                                    data-exclude-from-calc-comparison=if kvp.exclude_from_calc_comparison { "true" } else { "false" }
                                >
                                    <dt>
                                        {kvp.key.clone()}
                                        {if kvp.exclude_from_calc_comparison {
                                            view! {
                                                <span
                                                    class="onecalc-value-panel__rich-kvp-flag"
                                                    data-role="value-panel-rich-kvp-flag"
                                                >
                                                    "not compared"
                                                </span>
                                            }
                                            .into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }}
                                    </dt>
                                    <dd>{render_rich_value_data(&kvp.value)}</dd>
                                </div>
                            })
                            .collect_view()}
                    </dl>
                }
                .into_any()
            }}
        </div>
    }
    .into_any()
}

fn render_rich_value_data(data: &ValuePanelRichValueData) -> AnyView {
    match data {
        ValuePanelRichValueData::Number(value) => {
            view! { <span data-role="rich-scalar">{format!("{value}")}</span> }.into_any()
        }
        ValuePanelRichValueData::Text(value) => {
            view! { <span data-role="rich-scalar">{value.clone()}</span> }.into_any()
        }
        ValuePanelRichValueData::Logical(value) => view! {
            <span data-role="rich-scalar">{if *value { "TRUE" } else { "FALSE" }}</span>
        }
        .into_any(),
        ValuePanelRichValueData::Error(value) => view! {
            <span data-role="rich-scalar" class="onecalc-value-panel__error">{value.clone()}</span>
        }
        .into_any(),
        ValuePanelRichValueData::EmptyCell => {
            view! { <span data-role="rich-scalar">"empty"</span> }.into_any()
        }
        ValuePanelRichValueData::Array { shape, rows } => {
            let shape = *shape;
            let rows = rows.clone();
            view! {
                <div data-role="rich-array">
                    <span>"Array ["{shape.rows}" × "{shape.cols}"]"</span>
                    <ul>
                        {rows
                            .into_iter()
                            .map(|row| view! {
                                <li>
                                    {row
                                        .into_iter()
                                        .map(|cell| view! {
                                            <span>{render_rich_value_data(&cell)}</span>
                                        })
                                        .collect_view()}
                                </li>
                            })
                            .collect_view()}
                    </ul>
                </div>
            }
            .into_any()
        }
        ValuePanelRichValueData::Nested(nested) => render_rich_value(nested).into_any(),
    }
}

fn render_presentation_section(presentation: ValuePanelPresentation) -> AnyView {
    view! {
        <div class="onecalc-value-panel__presentation" data-role="value-panel-presentation">
            {presentation.number_format_hint.map(|hint| view! {
                <span
                    class="onecalc-value-panel__presentation-badge"
                    data-role="value-panel-presentation-number-format"
                    data-presentation-hint=hint.slug()
                >
                    {hint.label()}
                </span>
            })}
            {presentation.style_hint.map(|hint| view! {
                <span
                    class="onecalc-value-panel__presentation-badge"
                    data-role="value-panel-presentation-style"
                    data-presentation-style=hint.slug()
                >
                    {hint.label()}
                </span>
            })}
            {presentation.format_code.map(|code| view! {
                <code
                    class="onecalc-value-panel__presentation-format-code"
                    data-role="value-panel-presentation-format-code"
                >
                    {code}
                </code>
            })}
        </div>
    }
    .into_any()
}

fn render_effective_display_section(
    effective_display_text: Option<String>,
    pipeline: Vec<crate::ui::panels::value_panel_model::ValuePanelPipelineStep>,
) -> AnyView {
    let display_label = effective_display_text
        .clone()
        .unwrap_or_else(|| "—".to_string());
    view! {
        <div class="onecalc-value-panel__effective-display" data-role="value-panel-effective-display">
            <div class="onecalc-value-panel__effective-display-line">
                <span class="onecalc-value-panel__effective-display-label">"Effective display"</span>
                <strong
                    class="onecalc-value-panel__effective-display-value"
                    data-has-display=if effective_display_text.is_some() { "true" } else { "false" }
                >
                    {display_label}
                </strong>
            </div>
            <div
                class="onecalc-value-panel__effective-display-pipeline"
                data-role="value-panel-effective-display-pipeline"
            >
                {pipeline
                    .into_iter()
                    .map(|step| {
                        let status_slug = step.status.slug();
                        let seam_id = match &step.status {
                            ValuePanelPipelineStepStatus::NotImplemented { seam_id } => {
                                Some(seam_id.clone())
                            }
                            ValuePanelPipelineStepStatus::Live => None,
                        };
                        view! {
                            <span
                                class="onecalc-value-panel__pipeline-chip"
                                data-role="value-panel-pipeline-chip"
                                data-status=status_slug
                                data-seam-id=seam_id.clone().unwrap_or_default()
                                title=seam_id.unwrap_or_default()
                            >
                                <span data-role="pipeline-chip-label">{step.label}</span>
                                {step.value_preview.map(|preview| view! {
                                    <span data-role="pipeline-chip-preview">
                                        " · "
                                        {preview}
                                    </span>
                                })}
                            </span>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
    .into_any()
}

fn render_array_cell_label(cell: &ValuePanelArrayCell) -> String {
    match cell {
        ValuePanelArrayCell::Number(value) => format!("{value}"),
        ValuePanelArrayCell::Text(value) => value.clone(),
        ValuePanelArrayCell::Logical(value) => {
            if *value {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        ValuePanelArrayCell::Error(value) => value.clone(),
        ValuePanelArrayCell::EmptyCell => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::editor::state::EditorLiveState;
    use crate::ui::panels::value_panel_model::{
        build_value_panel_from_explore_strings, ValuePanelArrayCell, ValuePanelArrayShape,
        ValuePanelPipelineStep, ValuePanelPipelineStepStatus, ValuePanelValue,
    };

    #[test]
    fn renders_unevaluated_panel_from_empty_strings() {
        let panel = build_value_panel_from_explore_strings(None, None, None, EditorLiveState::Idle);
        let html = view! { <ValuePanel panel=panel /> }.to_html();
        assert!(html.contains("data-component=\"value-panel\""));
        assert!(html.contains("data-value-kind=\"unevaluated\""));
        assert!(html.contains("data-live-state=\"idle\""));
        assert!(html.contains("data-role=\"value-panel-unevaluated\""));
        assert!(html.contains("data-role=\"value-panel-effective-display-pipeline\""));
    }

    #[test]
    fn renders_number_panel_with_effective_display() {
        let panel = build_value_panel_from_explore_strings(
            Some("Number"),
            Some("3"),
            Some("green-1"),
            EditorLiveState::EditingLive,
        );
        let html = view! { <ValuePanel panel=panel /> }.to_html();
        assert!(html.contains("data-value-kind=\"number\""));
        assert!(html.contains("data-role=\"value-panel-number\""));
        assert!(html.contains("data-role=\"value-panel-effective-display\""));
        assert!(html.contains(">3<"));
        assert!(html.contains("data-role=\"value-panel-provenance-green-tree\""));
        // "raw" pipeline chip should be live.
        assert!(html.contains("data-status=\"live\""));
        // "format code" should carry its seam id.
        assert!(html.contains("SEAM-ONECALC-EXTENDED-VALUE-ROUTING"));
        assert!(html.contains("SEAM-OXFUNC-LOCALE-EXPAND"));
        assert!(html.contains("SEAM-OXFML-CF-COLORSCALE"));
    }

    #[test]
    fn renders_array_panel_when_rows_present() {
        let panel = ValuePanelViewModel {
            value: ValuePanelValue::Array {
                shape: ValuePanelArrayShape { rows: 1, cols: 2 },
                rows: vec![vec![
                    ValuePanelArrayCell::Number(1.0),
                    ValuePanelArrayCell::Text("two".to_string()),
                ]],
                truncated: false,
            },
            presentation: None,
            effective_display_text: None,
            display_pipeline: vec![ValuePanelPipelineStep {
                label: "raw".to_string(),
                value_preview: None,
                status: ValuePanelPipelineStepStatus::Live,
            }],
            provenance: None,
            live_state: EditorLiveState::ProofedScratch,
        };
        let html = view! { <ValuePanel panel=panel /> }.to_html();
        assert!(html.contains("data-value-kind=\"array\""));
        assert!(html.contains("data-role=\"value-panel-array-shape\""));
        assert!(html.contains("rows"));
        assert!(html.contains("cols"));
        assert!(html.contains("data-role=\"value-panel-array-cell\""));
        assert!(html.contains("two"));
    }
}
