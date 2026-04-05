use leptos::prelude::*;

use crate::services::shell_composition::ShellFrameViewModel;
use crate::state::AppMode;

#[component]
pub fn ShellFrame(
    frame: ShellFrameViewModel,
    on_mode_select: Option<Callback<AppMode>>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="onecalc-shell-frame">
            <aside class="onecalc-shell-frame__rail">
                <h1>"DNA OneCalc"</h1>
                <div class="onecalc-shell-frame__active-space">
                    "Active space: "
                    {frame.active_formula_space_label.clone()}
                </div>
                <ul class="onecalc-shell-frame__space-list">
                    {frame
                        .formula_spaces
                        .iter()
                        .map(|formula_space| {
                            let item_class = if formula_space.is_active {
                                "onecalc-shell-frame__space-item onecalc-shell-frame__space-item--active"
                            } else {
                                "onecalc-shell-frame__space-item"
                            };
                            view! {
                                <li class=item_class>
                                    {formula_space.label.clone()}
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            </aside>

            <main class="onecalc-shell-frame__content">
                <header class="onecalc-shell-frame__context-bar">
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
                    </nav>
                </header>

                <section class="onecalc-shell-frame__mode-body">
                    {children()}
                </section>
            </main>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::shell_composition::{
        ShellFormulaSpaceListItemViewModel, ShellModeTabViewModel,
    };

    #[test]
    fn shell_frame_renders_space_list_and_mode_tabs() {
        let html = view! {
            <ShellFrame
                frame=ShellFrameViewModel {
                    active_formula_space_label: "space-1".to_string(),
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
                        is_active: true,
                    }],
                }
                on_mode_select=None
            >
                <div>"Body"</div>
            </ShellFrame>
        }
        .to_html();

        assert!(html.contains("DNA OneCalc"));
        assert!(html.contains("Active space: "));
        assert!(html.contains("space-1"));
        assert!(html.contains("data-mode=\"Explore\""));
        assert!(html.contains("Body"));
    }
}
