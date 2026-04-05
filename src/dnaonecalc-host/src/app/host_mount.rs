use leptos::prelude::*;

use crate::state::OneCalcHostState;
use crate::ui::components::app_shell::OneCalcShellApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostMountTarget {
    DesktopTauri,
    WebBrowser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBootstrapSpec {
    pub target: HostMountTarget,
    pub mount_element_id: &'static str,
    pub document_title: &'static str,
}

pub fn bootstrap_spec(target: HostMountTarget) -> HostBootstrapSpec {
    HostBootstrapSpec {
        target,
        mount_element_id: "onecalc-root",
        document_title: "DNA OneCalc",
    }
}

pub fn render_shell_html(target: HostMountTarget, initial_state: OneCalcHostState) -> String {
    let host_label = match target {
        HostMountTarget::DesktopTauri => "desktop-tauri",
        HostMountTarget::WebBrowser => "web-browser",
    };
    let spec = bootstrap_spec(target);

    let body = view! { <OneCalcShellApp initial_state=initial_state /> }.to_html();
    format!(
        "<div id=\"{}\" data-host-target=\"{host_label}\" data-shell-root=\"onecalc\">{body}</div>",
        spec.mount_element_id
    )
}

pub fn render_shell_document(target: HostMountTarget, initial_state: OneCalcHostState) -> String {
    let host_label = match target {
        HostMountTarget::DesktopTauri => "desktop-tauri",
        HostMountTarget::WebBrowser => "web-browser",
    };
    let spec = bootstrap_spec(target);
    let body = render_shell_html(target, initial_state);

    format!(
        "<!doctype html><html data-host-target=\"{host_label}\"><head><meta charset=\"utf-8\"><title>{}</title></head><body>{body}</body></html>",
        spec.document_title
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
        assert!(html.contains("id=\"onecalc-root\""));
        assert!(html.contains("data-host-target=\"desktop-tauri\""));
    }

    #[test]
    fn bootstrap_spec_is_shared_between_desktop_and_web() {
        let desktop = bootstrap_spec(HostMountTarget::DesktopTauri);
        let web = bootstrap_spec(HostMountTarget::WebBrowser);

        assert_eq!(desktop.mount_element_id, "onecalc-root");
        assert_eq!(web.mount_element_id, "onecalc-root");
        assert_eq!(desktop.document_title, "DNA OneCalc");
        assert_eq!(web.document_title, "DNA OneCalc");
    }
}
