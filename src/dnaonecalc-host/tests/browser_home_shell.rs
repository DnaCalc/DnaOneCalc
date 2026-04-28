//! WS-14 Pre-MVP P8 — wasm-bindgen browser test for the live engine
//! round-trip through the new minimal HomeShell.
//!
//! Runs only on `wasm32-unknown-unknown` via `wasm-bindgen-test`. The test
//! mounts the `HomeShell` into a detached DOM container, types
//! `=SUM(1,2,3)` into the textarea via a real DOM input event, and waits
//! for the result block to settle on `6`. End-to-end coverage that the
//! `LiveOxfmlBridge` is reachable from the WS-14 mount path.

#![cfg(target_arch = "wasm32")]

use dnaonecalc_host::app::host_mount::{bootstrap_editor_bridge, HostMountTarget};
use dnaonecalc_host::app::preview_state::preview_minimal_host_state;
use dnaonecalc_host::ui::components::home_shell::HomeShell;
use leptos::mount::mount_to;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const HOST_ID: &str = "onecalc-home-shell-test-root";

async fn next_microtask() {
    JsFuture::from(web_sys::js_sys::Promise::resolve(&JsValue::UNDEFINED))
        .await
        .expect("microtask tick");
}

fn prepare_host_root(document: &web_sys::Document) -> web_sys::Element {
    if let Some(existing) = document.get_element_by_id(HOST_ID) {
        existing.remove();
    }
    let host = document.create_element("div").expect("host element");
    host.set_id(HOST_ID);
    document
        .body()
        .expect("body")
        .append_child(&host)
        .expect("append host");
    host
}

async fn wait_for_result_text(
    document: &web_sys::Document,
    expected: &str,
) -> Option<String> {
    for _ in 0..30 {
        next_microtask().await;
        if let Some(element) = document
            .query_selector(".onecalc-home-shell__result-block .value")
            .expect("query ok")
        {
            let text = element.text_content().unwrap_or_default();
            let trimmed = text.trim();
            if trimmed == expected {
                return Some(trimmed.to_string());
            }
        }
    }
    document
        .query_selector(".onecalc-home-shell__result-block .value")
        .expect("query ok")
        .map(|el| el.text_content().unwrap_or_default().trim().to_string())
}

#[wasm_bindgen_test(async)]
async fn home_shell_typing_sum_shows_result() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

    let initial_state = preview_minimal_host_state();
    let editor_bridge = bootstrap_editor_bridge(HostMountTarget::WebBrowser);
    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! {
            <HomeShell
                initial_state=initial_state.clone()
                editor_bridge=Some(editor_bridge.clone())
            />
        }
    });

    // The textarea should be present after the first render.
    let textarea = {
        let mut found: Option<web_sys::HtmlTextAreaElement> = None;
        for _ in 0..10 {
            next_microtask().await;
            if let Some(element) = document
                .query_selector(".onecalc-home-shell__textarea")
                .expect("query ok")
            {
                found = Some(
                    element
                        .dyn_into::<web_sys::HtmlTextAreaElement>()
                        .expect("textarea"),
                );
                break;
            }
        }
        found.expect("textarea mounted")
    };

    // Type =SUM(1,2,3) and dispatch a real input event so the on:input
    // handler fires through services::live_edit::apply_live_editor_input.
    textarea.set_value("=SUM(1,2,3)");
    textarea
        .set_selection_range(11, 11)
        .expect("set selection range");
    textarea
        .dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
        .expect("dispatch input event");

    let result_text = wait_for_result_text(&document, "6").await;
    assert_eq!(
        result_text.as_deref(),
        Some("6"),
        "result block should show 6 after typing =SUM(1,2,3); got {:?}",
        result_text,
    );

    drop(mount_handle);
    host.remove();
}
