use leptos::prelude::*;

/// Uniform right-side drawer primitive used across every WS-13 surface that
/// needs progressive disclosure without leaving the current mode — Configure,
/// Witness chain, Handoff history, Provenance, Host context, Node detail,
/// Editor settings, Workspace settings, Seam status board.
///
/// The primitive owns: frame, header with title + close affordance, scrollable
/// body that hosts children, and a data attribute carrying a drawer-kind slug
/// so individual mounts can style themselves.
///
/// Consumers decide when the drawer is visible via the `is_open` prop and
/// where to mount it in their panel layout; ShellDrawer does not manage its
/// own open/close state or portal rendering.
#[component]
pub fn ShellDrawer(
    drawer_kind: String,
    title: String,
    #[prop(default = None)] subtitle: Option<String>,
    #[prop(default = false)] is_open: bool,
    #[prop(default = None)] on_close: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    if !is_open {
        return view! { <></> }.into_any();
    }

    let close_callback = on_close.clone();
    let has_subtitle = subtitle.is_some();
    let subtitle_text = subtitle.unwrap_or_default();
    let aria_label = title.clone();
    let title_text = title;
    let drawer_kind_attr = drawer_kind.clone();
    let drawer_kind_label = drawer_kind;

    view! {
        <aside
            class="onecalc-shell-drawer"
            data-component="shell-drawer"
            data-drawer-kind=drawer_kind_attr
            data-open="true"
            role="dialog"
            aria-label=aria_label
        >
            <header class="onecalc-shell-drawer__header" data-role="shell-drawer-header">
                <div class="onecalc-shell-drawer__titles">
                    <div class="onecalc-shell-drawer__eyebrow" data-role="shell-drawer-eyebrow">
                        {drawer_kind_label}
                    </div>
                    <strong class="onecalc-shell-drawer__title" data-role="shell-drawer-title">
                        {title_text}
                    </strong>
                    {if has_subtitle {
                        view! {
                            <div
                                class="onecalc-shell-drawer__subtitle"
                                data-role="shell-drawer-subtitle"
                            >
                                {subtitle_text}
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </div>
                <button
                    type="button"
                    class="onecalc-shell-drawer__close"
                    data-role="shell-drawer-close"
                    aria-label="Close drawer"
                    on:click=move |_| {
                        if let Some(callback) = close_callback.as_ref() {
                            callback.run(());
                        }
                    }
                >
                    "×"
                </button>
            </header>
            <div class="onecalc-shell-drawer__body" data-role="shell-drawer-body">
                {children()}
            </div>
        </aside>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_drawer_renders_when_open_with_title_and_close_button() {
        let html = view! {
            <ShellDrawer
                drawer_kind="configure".to_string()
                title="Configure".to_string()
                is_open=true
            >
                <div>"Drawer body content"</div>
            </ShellDrawer>
        }
        .to_html();

        assert!(html.contains("data-component=\"shell-drawer\""));
        assert!(html.contains("data-drawer-kind=\"configure\""));
        assert!(html.contains("data-open=\"true\""));
        assert!(html.contains("role=\"dialog\""));
        assert!(html.contains("data-role=\"shell-drawer-header\""));
        assert!(html.contains("data-role=\"shell-drawer-title\""));
        assert!(html.contains("data-role=\"shell-drawer-close\""));
        assert!(html.contains("data-role=\"shell-drawer-body\""));
        assert!(html.contains("Drawer body content"));
    }

    #[test]
    fn shell_drawer_renders_nothing_when_closed() {
        let html = view! {
            <ShellDrawer
                drawer_kind="configure".to_string()
                title="Configure".to_string()
                is_open=false
            >
                <div>"Should not appear"</div>
            </ShellDrawer>
        }
        .to_html();

        assert!(!html.contains("data-component=\"shell-drawer\""));
        assert!(!html.contains("Should not appear"));
    }

    #[test]
    fn shell_drawer_renders_optional_subtitle_when_provided() {
        let html = view! {
            <ShellDrawer
                drawer_kind="witness-chain".to_string()
                title="Witness chain".to_string()
                subtitle=Some("3 verified".to_string())
                is_open=true
            >
                <div>"Body"</div>
            </ShellDrawer>
        }
        .to_html();

        assert!(html.contains("data-role=\"shell-drawer-subtitle\""));
        assert!(html.contains("3 verified"));
    }
}
