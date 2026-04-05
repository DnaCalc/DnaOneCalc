use leptos::prelude::*;

use crate::state::OneCalcHostState;
use crate::ui::components::app_shell::OneCalcShellApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostMountTarget {
    DesktopTauri,
    WebBrowser,
}

pub fn render_shell_html(target: HostMountTarget, initial_state: OneCalcHostState) -> String {
    let host_label = match target {
        HostMountTarget::DesktopTauri => "desktop-tauri",
        HostMountTarget::WebBrowser => "web-browser",
    };

    let body = view! { <OneCalcShellApp initial_state=initial_state /> }.to_html();
    format!(
        "<div data-host-target=\"{host_label}\" data-shell-root=\"onecalc\">{body}</div>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_shell_html_wraps_shared_app_for_desktop() {
        let html = render_shell_html(HostMountTarget::DesktopTauri, OneCalcHostState::default());
        assert!(html.contains("data-host-target=\"desktop-tauri\""));
        assert!(html.contains("DNA OneCalc"));
    }

    #[test]
    fn render_shell_html_wraps_shared_app_for_web() {
        let html = render_shell_html(HostMountTarget::WebBrowser, OneCalcHostState::default());
        assert!(html.contains("data-host-target=\"web-browser\""));
        assert!(html.contains("DNA OneCalc"));
    }
}
