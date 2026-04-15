#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use dnaonecalc_host::app::host_mount::{bootstrap_editor_bridge, HostMountTarget};
use dnaonecalc_host::app::preview_state::preview_host_state;
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, default_windows_excel_capability_profile,
    default_windows_excel_host_profile, ProgrammaticArtifactCatalogEntry, ProgrammaticBatchPlan,
    ProgrammaticComparisonLane, ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
};
use dnaonecalc_host::services::retained_artifacts::{
    import_programmatic_artifact, RetainedArtifactImportRequest,
};
use dnaonecalc_host::services::spreadsheet_xml::{
    SpreadsheetXmlCellExtraction, VerificationObservationScope,
};
use dnaonecalc_host::services::verification_bundle::{
    ExcelObservationSummary, OxReplayExplainRecord, OxReplayMismatchRecord,
    OxfmlVerificationSummary, VerificationBundleReport, VerificationCaseReport,
    VerificationObservationGapReport,
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

async fn wait_for_textarea_value(
    document: &web_sys::Document,
    expected_value: &str,
) -> web_sys::HtmlTextAreaElement {
    for _ in 0..10 {
        next_microtask().await;
        if let Some(element) = document
            .query_selector("[data-role='editor-input']")
            .expect("query ok")
        {
            let textarea = element
                .dyn_into::<web_sys::HtmlTextAreaElement>()
                .expect("textarea");
            if textarea.value() == expected_value {
                return textarea;
            }
        }
    }

    document
        .query_selector("[data-role='editor-input']")
        .expect("query ok")
        .expect("editor input")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("textarea")
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

fn two_proposal_editor_document(
    source_text: &str,
) -> dnaonecalc_host::adapters::oxfml::EditorDocument {
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
        signature_forms: vec![
            dnaonecalc_host::adapters::oxfml::FunctionHelpSignatureForm {
                display_signature: "SUM(number1, number2, ...)".to_string(),
                min_arity: 1,
                max_arity: None,
            },
        ],
        argument_help: vec!["number1".to_string(), "number2".to_string()],
        short_description: Some("Adds numbers together.".to_string()),
        availability_summary: Some("supported".to_string()),
        deferred_or_profile_limited: false,
    });
    document
}

fn sample_verification_bundle_report_json() -> String {
    serde_json::to_string(&VerificationBundleReport {
        bundle_id: "bundle-browser-1".to_string(),
        output_root: "target/onecalc-verification/browser-bundle-1".to_string(),
        host_profile: default_windows_excel_host_profile(),
        capabilities: default_windows_excel_capability_profile(),
        batch_plan: ProgrammaticBatchPlan {
            formula_count: 1,
            comparison_lane: ProgrammaticComparisonLane::OxfmlAndExcel,
            discrepancy_index_required: true,
            retained_artifact_kinds: vec![
                "comparison_outcome".to_string(),
                "replay_bundle".to_string(),
            ],
        },
        retained_artifact_catalog: vec![ProgrammaticArtifactCatalogEntry {
            artifact_id: "artifact-bundle-1".to_string(),
            case_id: "xml-case-browser-1".to_string(),
            comparison_status: ProgrammaticComparisonStatus::Mismatched,
            open_mode_hint: ProgrammaticOpenModeHint::Workbench,
        }],
        case_reports: vec![VerificationCaseReport {
            case_id: "xml-case-browser-1".to_string(),
            entered_cell_text: "=SUM(1,2,3)".to_string(),
            artifact_catalog_entry: ProgrammaticArtifactCatalogEntry {
                artifact_id: "artifact-bundle-1".to_string(),
                case_id: "xml-case-browser-1".to_string(),
                comparison_status: ProgrammaticComparisonStatus::Mismatched,
                open_mode_hint: ProgrammaticOpenModeHint::Workbench,
            },
            comparison_status: ProgrammaticComparisonStatus::Mismatched,
            visible_output_match: Some(false),
            replay_equivalent: Some(false),
            replay_mismatch_kinds: vec![
                "effective_display_text".to_string(),
                "projection_coverage_gap".to_string(),
                "projection_coverage_gap".to_string(),
            ],
            replay_mismatch_records: vec![
                OxReplayMismatchRecord {
                    mismatch_kind: "effective_display_text".to_string(),
                    severity: Some("informational".to_string()),
                    view_family: Some("effective_display_text".to_string()),
                    left_value_repr: Some("6".to_string()),
                    right_value_repr: Some("$6.00".to_string()),
                    detail: Some("comparison view values diverged".to_string()),
                },
                OxReplayMismatchRecord {
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("formatting_view".to_string()),
                    left_value_repr: None,
                    right_value_repr: Some("{\"number_format_code\":\"$#,##0.00\",\"font_color\":\"#112233\",\"fill_color\":\"#ddeeff\"}".to_string()),
                    detail: Some("comparison view family `formatting_view` is missing on `crosslane_xml_view_family_gap_001_left`".to_string()),
                },
                OxReplayMismatchRecord {
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("conditional_formatting_view".to_string()),
                    left_value_repr: None,
                    right_value_repr: Some("[{\"range\":\"A1\",\"rule_kind\":\"expression\",\"formula\":\"=A1>0\",\"font_color\":\"#FF0000\",\"fill_color\":\"#00FF00\"}]".to_string()),
                    detail: Some("comparison view family `conditional_formatting_view` is missing on `crosslane_xml_view_family_gap_001_left`".to_string()),
                },
            ],
            replay_explain_records: vec![
                OxReplayExplainRecord {
                    query_id: Some("explain-crosslane-01".to_string()),
                    summary: Some("comparison diverged on `effective_display_text`".to_string()),
                    mismatch_kind: "effective_display_text".to_string(),
                    severity: Some("informational".to_string()),
                    view_family: Some("effective_display_text".to_string()),
                    left_value_repr: Some("6".to_string()),
                    right_value_repr: Some("$6.00".to_string()),
                    detail: Some("comparison view values diverged".to_string()),
                },
                OxReplayExplainRecord {
                    query_id: Some("explain-crosslane-02".to_string()),
                    summary: Some("comparison view family `formatting_view` is missing on one side".to_string()),
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("formatting_view".to_string()),
                    left_value_repr: None,
                    right_value_repr: Some("{\"number_format_code\":\"$#,##0.00\",\"font_color\":\"#112233\",\"fill_color\":\"#ddeeff\"}".to_string()),
                    detail: Some("comparison view family `formatting_view` is missing on `crosslane_xml_view_family_gap_001_left`".to_string()),
                },
            ],
            discrepancy_summary: Some(
                "Display divergence (effective_display_text): OxFml 6 vs Excel $6.00 | Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on `crosslane_xml_view_family_gap_001_left`"
                    .to_string(),
            ),
            oxfml_summary: OxfmlVerificationSummary {
                evaluation_summary: Some("Number · 6".to_string()),
                effective_display_summary: Some("6".to_string()),
                blocked_reason: None,
                parse_status: Some("Valid".to_string()),
                green_tree_key: Some("green-browser-1".to_string()),
            },
            excel_summary: Some(ExcelObservationSummary {
                observed_value_repr: Some("$6.00".to_string()),
                effective_display_text: Some("$6.00".to_string()),
                observed_formula_repr: Some("=SUM(1,2,3)".to_string()),
                capture_status: "captured".to_string(),
            }),
            spreadsheet_xml_extraction: Some(SpreadsheetXmlCellExtraction {
                workbook_path: "C:/tmp/browser-workbook.xml".to_string(),
                locator: "Input!A1".to_string(),
                worksheet_name: "Input".to_string(),
                workbook_format_profile_hint: "excel-spreadsheetml-2003-default".to_string(),
                formula_text: Some("=SUM(1,2,3)".to_string()),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                data_type: Some("Number".to_string()),
                style_id: Some("calc".to_string()),
                style_hierarchy: vec!["calcBase".to_string(), "calc".to_string()],
                number_format_code: Some("$#,##0.00".to_string()),
                font_color: Some("#112233".to_string()),
                fill_color: Some("#ddeeff".to_string()),
                conditional_formats: vec![],
                date1904: Some(false),
                observation_scope: VerificationObservationScope {
                    oxfml_required_scope: vec!["format_profile".to_string()],
                    oxxlplay_required_surfaces: vec!["effective_display_text".to_string()],
                    oxreplay_required_views: vec![
                        "formatting_view".to_string(),
                        "conditional_formatting_view".to_string(),
                    ],
                },
            }),
            upstream_gap_report: Some(VerificationObservationGapReport {
                oxfml_scope_required: vec!["format_profile".to_string()],
                oxxlplay_supported_surfaces: vec!["cell_value".to_string()],
                oxxlplay_missing_surfaces: vec!["effective_display_text".to_string()],
                oxreplay_required_views: vec![
                    "formatting_view".to_string(),
                    "conditional_formatting_view".to_string(),
                ],
                oxreplay_current_bundle_views: vec!["visible_value".to_string()],
                oxreplay_missing_views: vec![
                    "formatting_view".to_string(),
                    "conditional_formatting_view".to_string(),
                ],
            }),
            case_output_dir: "target/onecalc-verification/browser-bundle-1/cases/xml-case-browser-1"
                .to_string(),
            scenario_path:
                "target/onecalc-verification/browser-bundle-1/cases/xml-case-browser-1/scenario.json"
                    .to_string(),
        }],
    })
    .expect("verification bundle report json")
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

    assert!(
        html.contains("data-screen=\"explore\""),
        "mounted html: {html}"
    );
    assert!(
        html.contains("data-role=\"editor-input\""),
        "mounted html: {html}"
    );
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
    assert!(
        initial_html.contains("Success · SUM result"),
        "initial mounted html: {initial_html}"
    );

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
    assert!(
        html.contains("Array · Dynamic spill"),
        "mounted html after switch: {html}"
    );
    assert!(
        html.contains("2x2 spill preview"),
        "mounted html after switch: {html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn preview_host_initial_state_does_not_auto_open_assist_popups() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);
    let state = preview_host_state();

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! {
            <OneCalcShellApp
                initial_state=state.clone()
                editor_bridge=Some(bootstrap_editor_bridge(HostMountTarget::WebBrowser))
            />
        }
    });

    let html = wait_for_host_html(&document, |html| {
        html.contains("Success · SUM result") && html.contains("data-screen=\"explore\"")
    })
    .await;

    assert!(
        !html.contains("data-role=\"completion-popup\""),
        "mounted html: {html}"
    );
    assert!(
        !html.contains("data-role=\"signature-help-popup\""),
        "mounted html: {html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn live_bridge_typing_updates_visible_calculated_result() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let host = prepare_host_root(&document);

    let formula_space_id = FormulaSpaceId::new("space-live-preview");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state
        .formula_spaces
        .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let editor_bridge = bootstrap_editor_bridge(HostMountTarget::WebBrowser);
    let mount_handle = mount_to(host_element, move || {
        view! {
            <OneCalcShellApp
                initial_state=state.clone()
                editor_bridge=Some(editor_bridge.clone())
            />
        }
    });

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
    textarea
        .dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
        .expect("dispatch input event");

    let html = wait_for_host_html(&document, |html| {
        html.contains("Calculated value")
            && html.contains("Number · 6")
            && html.contains(">6</strong>")
    })
    .await;

    assert!(
        html.contains("Calculated value"),
        "mounted html after typing: {html}"
    );
    assert!(
        html.contains("Number · 6"),
        "mounted html after typing: {html}"
    );
    assert!(
        html.contains(">6</strong>"),
        "mounted html after typing: {html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn typing_in_editor_refreshes_result_state_through_live_bridge() {
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
    let editor_bridge = bootstrap_editor_bridge(HostMountTarget::WebBrowser);
    let mount_handle = mount_to(host_element, move || {
        view! {
            <OneCalcShellApp
                initial_state=state.clone()
                editor_bridge=Some(editor_bridge.clone())
            />
        }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("Calculated value") && html.contains("Number")
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
        html.contains("Calculated value")
            && html.contains("Number · 6")
            && html.contains(">6</strong>")
    })
    .await;

    assert!(
        html.contains("Number · 6"),
        "mounted html after typing: {html}"
    );
    assert!(
        html.contains(">6</strong>"),
        "mounted html after typing: {html}"
    );
    assert!(
        html.contains("Function target"),
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
    let editor_bridge = bootstrap_editor_bridge(HostMountTarget::WebBrowser);
    let mount_handle = mount_to(host_element, move || {
        view! {
            <OneCalcShellApp
                initial_state=state.clone()
                editor_bridge=Some(editor_bridge.clone())
            />
        }
    });

    let initial_html = wait_for_host_html(&document, |html| {
        html.contains("data-role=\"editor-toolbar-state\"")
            && html.contains(">3</strong>")
            && html.contains("Function target")
    })
    .await;
    assert!(
        initial_html.contains("data-role=\"editor-toolbar-state\""),
        "initial mounted html: {initial_html}"
    );

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
        html.contains("expected ')'")
            && html.contains("Input incomplete")
            && html.contains("data-selection-start=\"8\"")
    })
    .await;

    assert!(
        html.contains("data-selection-start=\"8\""),
        "mounted html after keydown: {html}"
    );
    assert!(
        html.contains("expected ')'"),
        "mounted html after keydown: {html}"
    );
    assert!(
        html.contains("Input incomplete"),
        "mounted html after keydown: {html}"
    );
    assert!(
        html.contains("Function target"),
        "mounted html after keydown: {html}"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn tab_and_shift_tab_stay_in_editor_focus_and_prevent_browser_focus_escape() {
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

    let _ = wait_for_host_html(&document, |html| {
        html.contains("data-role=\"editor-input\"")
            && html.contains("data-role=\"editor-toolbar-state\"")
    })
    .await;

    let textarea = document
        .query_selector("[data-role='editor-input']")
        .expect("query ok")
        .expect("editor input")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("textarea");
    textarea.focus().expect("focus textarea");

    let tab_init = web_sys::KeyboardEventInit::new();
    tab_init.set_key("Tab");
    tab_init.set_bubbles(true);
    tab_init.set_cancelable(true);
    let tab_event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &tab_init)
        .expect("tab event");
    let tab_dispatch_result = textarea.dispatch_event(&tab_event).expect("dispatch tab");
    assert!(!tab_dispatch_result, "tab should be prevented");

    let textarea_after_tab = wait_for_textarea_value(&document, "    =SUM(1,2)").await;
    let html_after_tab = wait_for_host_html(&document, |html| {
        html.contains("data-role=\"explore-effective-display\"")
            && html.contains("data-role=\"editor-diagnostic-band\"")
    })
    .await;
    assert!(
        textarea_after_tab.value() == "    =SUM(1,2)",
        "mounted html after tab: {html_after_tab}"
    );
    assert_eq!(
        document
            .active_element()
            .expect("active element after tab")
            .dyn_into::<web_sys::HtmlTextAreaElement>()
            .expect("textarea remains focused")
            .value(),
        "    =SUM(1,2)"
    );

    let shift_tab_init = web_sys::KeyboardEventInit::new();
    shift_tab_init.set_key("Tab");
    shift_tab_init.set_shift_key(true);
    shift_tab_init.set_bubbles(true);
    shift_tab_init.set_cancelable(true);
    let shift_tab_event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &shift_tab_init)
            .expect("shift+tab event");
    let shift_tab_dispatch_result = textarea
        .dispatch_event(&shift_tab_event)
        .expect("dispatch shift+tab");
    assert!(!shift_tab_dispatch_result, "shift+tab should be prevented");

    let textarea_after_shift_tab = wait_for_textarea_value(&document, "=SUM(1,2)").await;
    let html_after_shift_tab = wait_for_host_html(&document, |html| {
        html.contains("data-role=\"explore-effective-display\"")
            && html.contains("data-role=\"editor-diagnostic-band\"")
    })
    .await;
    assert!(
        textarea_after_shift_tab.value() == "=SUM(1,2)",
        "mounted html after shift+tab: {html_after_shift_tab}"
    );
    assert_eq!(
        document
            .active_element()
            .expect("active element after shift+tab")
            .dyn_into::<web_sys::HtmlTextAreaElement>()
            .expect("textarea remains focused")
            .value(),
        "=SUM(1,2)"
    );

    drop(mount_handle);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn ctrl_x_cuts_selection_without_leaving_the_editor_surface() {
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
    formula_space.editor_surface_state =
        EditorSurfaceState::for_text_with_selection("=SUM(1,2)", 1, 4);
    formula_space.effective_display_summary = Some("3".to_string());
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let _ = wait_for_host_html(&document, |html| {
        html.contains("data-role=\"editor-input\"")
            && html.contains("data-role=\"editor-toolbar-state\"")
    })
    .await;

    let textarea = document
        .query_selector("[data-role='editor-input']")
        .expect("query ok")
        .expect("editor input")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("textarea");
    textarea.focus().expect("focus textarea");
    textarea
        .set_selection_range(1, 4)
        .expect("set selection range");

    let cut_init = web_sys::KeyboardEventInit::new();
    cut_init.set_key("x");
    cut_init.set_ctrl_key(true);
    cut_init.set_bubbles(true);
    cut_init.set_cancelable(true);
    let cut_event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &cut_init)
        .expect("ctrl+x event");
    let cut_dispatch_result = textarea
        .dispatch_event(&cut_event)
        .expect("dispatch ctrl+x");
    assert!(!cut_dispatch_result, "ctrl+x should be prevented");

    let textarea_after_cut = wait_for_textarea_value(&document, "=(1,2)").await;
    let html = wait_for_host_html(&document, |html| {
        html.contains("data-role=\"editor-input\"")
            && html.contains("data-role=\"explore-evaluation-summary\"")
    })
    .await;
    assert!(
        textarea_after_cut.value() == "=(1,2)",
        "mounted html after ctrl+x: {html}"
    );
    assert_eq!(
        document
            .active_element()
            .expect("active element after ctrl+x")
            .dyn_into::<web_sys::HtmlTextAreaElement>()
            .expect("textarea remains focused")
            .value(),
        "=(1,2)"
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
    formula_space
        .editor_surface_state
        .signature_help_anchor_offset = Some(1);
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

    let accepted_textarea = wait_for_textarea_value(&document, "=AVERAGE(").await;
    let accepted_html = wait_for_host_html(&document, |html| {
        !html.contains("data-role=\"completion-popup\"")
            && html.contains("data-role=\"editor-input\"")
    })
    .await;
    assert!(
        accepted_textarea.value() == "=AVERAGE(",
        "mounted html after acceptance: {accepted_html}"
    );
    let updated_textarea = document
        .query_selector("[data-role='editor-input']")
        .expect("query ok")
        .expect("editor input after acceptance")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("textarea");
    assert_eq!(updated_textarea.value(), "=AVERAGE(");

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
async fn verification_bundle_import_surface_imports_xml_case_and_opens_inspect_context() {
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
    state
        .formula_spaces
        .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! { <OneCalcShellApp initial_state=state.clone() /> }
    });

    let _ = wait_for_host_html(&document, |html| {
        html.contains("data-screen=\"workbench\"")
            && html.contains("data-role=\"verification-bundle-import-surface\"")
    })
    .await;

    let bundle_textarea = document
        .query_selector("[data-role='verification-bundle-import-json']")
        .expect("query ok")
        .expect("bundle import textarea")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("bundle textarea");
    bundle_textarea.set_value(&sample_verification_bundle_report_json());
    bundle_textarea
        .dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
        .expect("dispatch input");

    let import_button = document
        .query_selector("[data-role='verification-bundle-import-submit']")
        .expect("query ok")
        .expect("bundle import submit");
    import_button
        .dispatch_event(&web_sys::Event::new("click").expect("click event"))
        .expect("dispatch click");

    let imported_html = wait_for_host_html(&document, |html| {
        html.contains("artifact-bundle-1")
            && html.contains("Input!A1")
            && html.contains("OxFml 6 vs Excel $6.00")
    })
    .await;
    assert!(
        imported_html.contains("artifact-bundle-1"),
        "mounted html after bundle import: {imported_html}"
    );
    assert!(
        imported_html.contains("Input!A1"),
        "mounted html after bundle import: {imported_html}"
    );
    assert!(
        imported_html
            .contains("Display divergence (effective_display_text): OxFml 6 vs Excel $6.00"),
        "mounted html after bundle import: {imported_html}"
    );

    let inspect_button = document
        .query_selector(
            "[data-artifact-id='artifact-bundle-1'] [data-role='retained-catalog-open-inspect']",
        )
        .expect("query ok")
        .expect("inspect open button");
    inspect_button
        .dispatch_event(&web_sys::Event::new("click").expect("click event"))
        .expect("dispatch click");

    let inspect_html = wait_for_host_html(&document, |html| {
        html.contains("data-screen=\"inspect\"")
            && html.contains("artifact-bundle-1")
            && html.contains("C:/tmp/browser-workbook.xml @ Input!A1")
            && html.contains("Projection coverage gap (formatting_view)")
    })
    .await;
    assert!(
        inspect_html.contains("data-screen=\"inspect\""),
        "mounted html after inspect open: {inspect_html}"
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
        .query_selector("[data-artifact-id='artifact-2'] [data-role='retained-catalog-open']")
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
    assert!(
        html.contains("Artifact: artifact-2"),
        "mounted html after open: {html}"
    );
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
    assert!(
        html.contains("Artifact: artifact-ui-1"),
        "mounted html after import: {html}"
    );
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
