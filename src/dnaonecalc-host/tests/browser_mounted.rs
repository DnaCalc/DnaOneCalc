#![cfg(target_arch = "wasm32")]

use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::app::preview_state::preview_host_state;
use dnaonecalc_host::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, ProgrammaticComparisonStatus,
};
use dnaonecalc_host::services::retained_artifacts::{
    import_programmatic_artifact, RetainedArtifactImportRequest,
};
use dnaonecalc_host::state::{FormulaSpaceState, OneCalcHostState};
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::components::app_shell::OneCalcShellApp;
use dnaonecalc_host::ui::editor::state::EditorSurfaceState;
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

fn prepare_host_root(document: &web_sys::Document) -> web_sys::Element {
    if let Some(existing) = document.get_element_by_id("onecalc-mounted-test-root") {
        existing.remove();
    }

    let host = document.create_element("div").expect("host element");
    host.set_id("onecalc-mounted-test-root");
    document
        .body()
        .expect("body")
        .append_child(&host)
        .expect("append host");
    host
}

fn two_proposal_editor_document(source_text: &str) -> dnaonecalc_host::adapters::oxfml::EditorDocument {
    use dnaonecalc_host::adapters::oxfml::{
        CompletionProposal, CompletionProposalKind, FormulaTextSpan,
    };

    let mut document = sample_editor_document(source_text);
    document.completion_proposals = vec![
        CompletionProposal {
            proposal_id: "proposal-1".to_string(),
            proposal_kind: CompletionProposalKind::Function,
            display_text: "SUM".to_string(),
            insert_text: "SUM(".to_string(),
            replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
            documentation_ref: Some("preview:function:SUM".to_string()),
            requires_revalidation: true,
        },
        CompletionProposal {
            proposal_id: "proposal-2".to_string(),
            proposal_kind: CompletionProposalKind::Function,
            display_text: "AVERAGE".to_string(),
            insert_text: "AVERAGE(".to_string(),
            replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
            documentation_ref: Some("preview:function:AVERAGE".to_string()),
            requires_revalidation: false,
        },
    ];
    document.function_help = Some(dnaonecalc_host::adapters::oxfml::FunctionHelpPacket {
        lookup_key: "SUM".to_string(),
        display_name: "SUM".to_string(),
        signature_forms: vec![dnaonecalc_host::adapters::oxfml::FunctionHelpSignatureForm {
            display_signature: "SUM(number1, number2, ...)".to_string(),
            min_arity: 1,
            max_arity: None,
        }],
        argument_help: vec!["number1".to_string(), "number2".to_string()],
        short_description: Some("Adds numbers together.".to_string()),
        availability_summary: Some("supported".to_string()),
        deferred_or_profile_limited: false,
    });
    document
}

#[wasm_bindgen_test(async)]
async fn mounts_shell_into_browser_dom_container() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

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

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
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
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn preview_host_allows_switching_between_seeded_demo_scenarios() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);
    let state = preview_host_state();

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("Success · SUM result")
            && html.contains("data-screen=\"explore\"")
            && html.contains("Effective display")
    })
    .await;
    assert!(initial_html.contains("Success · SUM result"), "initial mounted html: {initial_html}");

    let array_space_button = document
        .query_selector("[data-formula-space-id='preview-array']")
        .expect("query ok")
        .expect("array space button");
    array_space_button
        .dispatch_event(&web_sys::Event::new("click").expect("click event"))
        .expect("dispatch click");

    let html = wait_for_host_html(&document, |html| {
        html.contains("Array · Dynamic spill")
            && html.contains("data-role=\"explore-array-preview\"")
            && html.contains("2x2 spill preview")
    })
    .await;
    assert!(html.contains("Array · Dynamic spill"), "mounted html after switch: {html}");
    assert!(html.contains("2x2 spill preview"), "mounted html after switch: {html}");

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn typing_in_editor_clears_stale_result_and_help_state() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

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

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("Effective display: 3") && html.contains("Evaluation summary: Number")
    })
    .await;
    assert!(
        initial_html.contains("preview:function:SUM"),
        "initial mounted html: {initial_html}"
    );

    let textarea = document
        .query_selector("[data-role='editor-input']")
        .expect("query ok")
        .expect("editor input")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("textarea");
    textarea.set_value("=SUM(1,2,3)");
    textarea
        .set_selection_range(11, 11)
        .expect("set selection range");
    let input_event = web_sys::InputEvent::new("input").expect("input event");
    textarea
        .dispatch_event(&input_event)
        .expect("dispatch input event");

    let html = wait_for_host_html(&document, |html| {
        html.contains("Effective display: Unavailable")
            && html.contains("Evaluation summary: Unavailable")
            && html.contains("Function help target: None")
    })
    .await;

    assert!(
        html.contains("Effective display: Unavailable"),
        "mounted html after typing: {html}"
    );
    assert!(
        html.contains("Evaluation summary: Unavailable"),
        "mounted html after typing: {html}"
    );
    assert!(
        html.contains("Function help target: None"),
        "mounted html after typing: {html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn backspace_keydown_updates_editor_state_and_clears_stale_analysis() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

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

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("Chars: 9")
            && html.contains("Effective display: 3")
            && html.contains("Function help target: SUM")
    })
    .await;
    assert!(initial_html.contains("Chars: 9"), "initial mounted html: {initial_html}");

    let textarea = document
        .query_selector("[data-role='editor-input']")
        .expect("query ok")
        .expect("editor input")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("textarea");

    let keyboard_init = web_sys::KeyboardEventInit::new();
    keyboard_init.set_key("Backspace");
    let keyboard_event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &keyboard_init)
            .expect("keyboard event");
    textarea
        .dispatch_event(&keyboard_event)
        .expect("dispatch keydown");

    let html = wait_for_host_html(&document, |html| {
        html.contains("Chars: 8")
            && html.contains("Effective display: Unavailable")
            && html.contains("Function help target: None")
            && html.contains("Evaluation summary: Unavailable")
    })
    .await;

    assert!(html.contains("Chars: 8"), "mounted html after keydown: {html}");
    assert!(
        html.contains("Effective display: Unavailable"),
        "mounted html after keydown: {html}"
    );
    assert!(
        html.contains("Evaluation summary: Unavailable"),
        "mounted html after keydown: {html}"
    );
    assert!(
        html.contains("Function help target: None"),
        "mounted html after keydown: {html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn completion_popup_selection_and_acceptance_work_in_browser_mount() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

    let formula_space_id = FormulaSpaceId::new("space-mounted");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());

    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SU");
    formula_space.editor_document = Some(two_proposal_editor_document("=SU"));
    formula_space.editor_surface_state = EditorSurfaceState::for_text("=SU");
    formula_space.editor_surface_state.completion_selected_index = Some(0);
    formula_space.editor_surface_state.completion_anchor_offset = Some(3);
    formula_space.editor_surface_state.signature_help_anchor_offset = Some(1);
    formula_space.effective_display_summary = Some("Unavailable".to_string());
    formula_space.latest_evaluation_summary = Some("Unavailable".to_string());
    state.formula_spaces.insert(formula_space);

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("data-role=\"completion-popup\"")
            && html.contains("preview:function:SUM")
            && html.contains("data-selected=\"true\"")
    })
    .await;
    assert!(
        initial_html.contains("preview:function:SUM"),
        "initial mounted html: {initial_html}"
    );

    let textarea = document
        .query_selector("[data-role='editor-input']")
        .expect("query ok")
        .expect("editor input")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("textarea");

    let arrow_down = web_sys::KeyboardEventInit::new();
    arrow_down.set_key("ArrowDown");
    let arrow_down_event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &arrow_down)
            .expect("keyboard event");
    textarea
        .dispatch_event(&arrow_down_event)
        .expect("dispatch keydown");

    let selected_html = wait_for_host_html(&document, |html| {
        html.contains("preview:function:AVERAGE")
            && html.contains("data-completion-id=\"proposal-2\"")
            && html.contains("data-selected=\"true\"")
            && html.contains("aria-selected=\"true\"")
    })
    .await;
    assert!(
        selected_html.contains("preview:function:AVERAGE"),
        "mounted html after selection: {selected_html}"
    );

    let second_completion = document
        .query_selector("[data-completion-id='proposal-2']")
        .expect("query ok")
        .expect("completion item");
    let click_event = web_sys::Event::new("click").expect("click event");
    second_completion
        .dispatch_event(&click_event)
        .expect("dispatch click");

    let accepted_html = wait_for_host_html(&document, |html| {
        html.contains("Chars: 9")
            && html.contains("Function help target: None")
            && html.contains("data-role=\"editor-input\"")
    })
    .await;
    assert!(
        accepted_html.contains("Chars: 9"),
        "mounted html after acceptance: {accepted_html}"
    );
    assert!(
        accepted_html.contains("Function help target: None"),
        "mounted html after acceptance: {accepted_html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn workbench_catalog_inspect_open_routes_into_inspect_with_retained_context() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

    let formula_space_id = FormulaSpaceId::new("space-mounted");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state.active_formula_space_view.active_mode = dnaonecalc_host::state::AppMode::Workbench;

    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    import_programmatic_artifact(
        &mut state,
        RetainedArtifactImportRequest {
            formula_space_id,
            catalog_entry: build_programmatic_artifact_catalog_entry(
                "artifact-inspect-1",
                "case-inspect-1",
                ProgrammaticComparisonStatus::Blocked,
            ),
            discrepancy_summary: Some("excel lane unavailable".to_string()),
        },
    );

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("data-screen=\"workbench\"")
            && html.contains("data-artifact-id=\"artifact-inspect-1\"")
    })
    .await;
    assert!(
        initial_html.contains("data-role=\"retained-catalog-open-inspect\""),
        "initial mounted html: {initial_html}"
    );

    let inspect_button = document
        .query_selector(
            "[data-artifact-id='artifact-inspect-1'] [data-role='retained-catalog-open-inspect']",
        )
        .expect("query ok")
        .expect("inspect open button");
    inspect_button
        .dispatch_event(&web_sys::Event::new("click").expect("click event"))
        .expect("dispatch click");

    let html = wait_for_host_html(&document, |html| {
        html.contains("data-screen=\"inspect\"")
            && html.contains("data-role=\"inspect-retained-context\"")
            && html.contains("artifact-inspect-1")
            && html.contains("excel lane unavailable")
    })
    .await;
    assert!(
        html.contains("data-screen=\"inspect\""),
        "mounted html after inspect open: {html}"
    );
    assert!(
        html.contains("artifact-inspect-1"),
        "mounted html after inspect open: {html}"
    );
    assert!(
        html.contains("excel lane unavailable"),
        "mounted html after inspect open: {html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn workbench_catalog_open_updates_active_retained_artifact_in_browser_mount() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

    let formula_space_id = FormulaSpaceId::new("space-mounted");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state.active_formula_space_view.active_mode = dnaonecalc_host::state::AppMode::Workbench;

    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    import_programmatic_artifact(
        &mut state,
        RetainedArtifactImportRequest {
            formula_space_id: formula_space_id.clone(),
            catalog_entry: build_programmatic_artifact_catalog_entry(
                "artifact-1",
                "case-1",
                ProgrammaticComparisonStatus::Mismatched,
            ),
            discrepancy_summary: Some("dna=1 excel=2".to_string()),
        },
    );
    import_programmatic_artifact(
        &mut state,
        RetainedArtifactImportRequest {
            formula_space_id,
            catalog_entry: build_programmatic_artifact_catalog_entry(
                "artifact-2",
                "case-2",
                ProgrammaticComparisonStatus::Blocked,
            ),
            discrepancy_summary: Some("excel lane unavailable".to_string()),
        },
    );
    state.retained_artifacts.open_artifact_id = Some("artifact-1".to_string());

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("Artifact: artifact-1")
            && html.contains("data-artifact-id=\"artifact-1\"")
            && html.contains("data-open=\"true\"")
    })
    .await;
    assert!(
        initial_html.contains("Artifact: artifact-1"),
        "initial mounted html: {initial_html}"
    );

    let second_button = document
        .query_selector(
            "[data-artifact-id='artifact-2'] [data-role='retained-catalog-open']",
        )
        .expect("query ok")
        .expect("second retained artifact open button");
    let click_event = web_sys::Event::new("click").expect("click event");
    second_button
        .dispatch_event(&click_event)
        .expect("dispatch click");

    let html = wait_for_host_html(&document, |html| {
        html.contains("Artifact: artifact-2")
            && html.contains("excel lane unavailable")
            && html.contains("Review blocked comparison and host policy")
    })
    .await;
    assert!(html.contains("Artifact: artifact-2"), "mounted html after open: {html}");
    assert!(
        html.contains("excel lane unavailable"),
        "mounted html after open: {html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn workbench_import_surface_imports_blocked_artifact_into_catalog() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

    let formula_space_id = FormulaSpaceId::new("space-mounted");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state.active_formula_space_view.active_mode = dnaonecalc_host::state::AppMode::Workbench;

    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("data-role=\"retained-import-surface\"")
    })
    .await;
    assert!(
        initial_html.contains("data-role=\"retained-import-surface\""),
        "initial mounted html: {initial_html}"
    );

    let artifact_id_input = document
        .query_selector("[data-role='retained-import-artifact-id']")
        .expect("query ok")
        .expect("artifact id input")
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("input");
    artifact_id_input.set_value("artifact-ui-1");
    artifact_id_input
        .dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
        .expect("dispatch input");

    let case_id_input = document
        .query_selector("[data-role='retained-import-case-id']")
        .expect("query ok")
        .expect("case id input")
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("input");
    case_id_input.set_value("case-ui-1");
    case_id_input
        .dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
        .expect("dispatch input");

    let summary_input = document
        .query_selector("[data-role='retained-import-summary']")
        .expect("query ok")
        .expect("summary input")
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("input");
    summary_input.set_value("imported discrepancy");
    summary_input
        .dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
        .expect("dispatch input");

    let blocked_button = document
        .query_selector("[data-role='retained-import-submit'][data-import-status='blocked']")
        .expect("query ok")
        .expect("blocked import button");
    blocked_button
        .dispatch_event(&web_sys::Event::new("click").expect("click event"))
        .expect("dispatch click");

    let html = wait_for_host_html(&document, |html| {
        html.contains("Artifact: artifact-ui-1")
            && html.contains("imported discrepancy")
            && html.contains("data-artifact-id=\"artifact-ui-1\"")
    })
    .await;
    assert!(html.contains("Artifact: artifact-ui-1"), "mounted html after import: {html}");
    assert!(
        html.contains("imported discrepancy"),
        "mounted html after import: {html}"
    );
    assert!(
        html.contains("data-artifact-id=\"artifact-ui-1\""),
        "mounted html after import: {html}"
    );

    drop(mount_handle);
    host.remove();
}
