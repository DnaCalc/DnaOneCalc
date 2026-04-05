#![cfg(target_arch = "wasm32")]

use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::{FormulaSpaceState, OneCalcHostState};
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::components::app_shell::OneCalcShellApp;
use leptos::mount::mount_to;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

async fn next_microtask() {
    JsFuture::from(web_sys::js_sys::Promise::resolve(&JsValue::UNDEFINED))
        .await
        .expect("microtask tick");
}

async fn wait_for_host_html(
    document: &web_sys::Document,
    predicate: impl Fn(&str) -> bool,
) -> String {
    for _ in 0..10 {
        next_microtask().await;
        let html = document
            .get_element_by_id("onecalc-mounted-test-root")
            .expect("mounted root")
            .inner_html();
        if predicate(&html) {
            return html;
        }
    }

    document
        .get_element_by_id("onecalc-mounted-test-root")
        .expect("mounted root")
        .inner_html()
}

#[wasm_bindgen_test(async)]
async fn mounts_shell_into_browser_dom_container() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = document.create_element("div").expect("host element");
    host.set_id("onecalc-mounted-test-root");
    document
        .body()
        .expect("body")
        .append_child(&host)
        .expect("append host");

    let formula_space_id = FormulaSpaceId::new("space-mounted");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());

    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.effective_display_summary = Some("3".to_string());
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    let host_element: web_sys::HtmlElement = host.unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let html = wait_for_host_html(&document, |html| {
        html.contains("data-screen=\"explore\"")
            && html.contains("data-role=\"editor-input\"")
            && html.contains("Formula Explorer")
    })
    .await;

    assert!(html.contains("data-screen=\"explore\""), "mounted html: {html}");
    assert!(html.contains("data-role=\"editor-input\""), "mounted html: {html}");
    assert!(html.contains("Formula Explorer"), "mounted html: {html}");

    drop(mount_handle);
}
