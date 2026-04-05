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

pub fn render_shell_document(target: HostMountTarget, initial_state: OneCalcHostState) -> String {
    let host_label = match target {
        HostMountTarget::DesktopTauri => "desktop-tauri",
        HostMountTarget::WebBrowser => "web-browser",
    };
    let body = render_shell_html(target, initial_state);

    format!(
        "<!doctype html><html data-host-target=\"{host_label}\"><head><meta charset=\"utf-8\"><title>DNA OneCalc</title></head><body>{body}</body></html>"
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

    #[test]
    fn render_shell_document_wraps_shell_in_html_document() {
        let html = render_shell_document(HostMountTarget::DesktopTauri, OneCalcHostState::default());
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<title>DNA OneCalc</title>"));
        assert!(html.contains("data-shell-root=\"onecalc\""));
        assert!(html.contains("data-host-target=\"desktop-tauri\""));
    }
}
