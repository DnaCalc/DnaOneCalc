//! Titlebar scenario-breadcrumb invariants.
//!
//! Pin the WS-14 §6 surface that lets the user see which scenario
//! they are editing and access the Recent / Pinned / Actions
//! dropdown. This bead's slice is rendering + dropdown shape +
//! Esc + outside-click; persistence (Save as / Open / .dnascenario
//! I/O) lives behind `SEAM-ONECALC-SCENARIO-PERSIST` and is not
//! exercised here.
//!
//! Mockup reference: `docs/ux_artifacts/ws14_progressive_home_mockup.html`
//! lines 1075–1124 (titlebar + dropdown).

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, dispatch_keydown, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

/// Click the breadcrumb button via a real `HTMLElement.click()`
/// invocation. Mirrors `formula_drill::editor_foot_toggle_button_opens_panel`.
fn click_element(element: &web_sys::Element) {
    let html: web_sys::HtmlElement = element.clone().unchecked_into();
    html.click();
}

#[wasm_bindgen_test(async)]
async fn titlebar_renders_brand_and_breadcrumb_button() {
    let shell = mount_home_shell();
    // Wait for the editor to mount; the breadcrumb is part of the
    // titlebar which renders unconditionally with the shell.
    let _textarea = shell.textarea().await;

    let brand = shell
        .select(".onecalc-home-shell__brand")
        .expect("brand mounted");
    assert_eq!(brand.text_content().as_deref(), Some("DnaOneCalc"));

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button mounted");
    assert_eq!(
        button.get_attribute("aria-haspopup").as_deref(),
        Some("menu"),
    );
    assert_eq!(
        button.get_attribute("aria-expanded").as_deref(),
        Some("false"),
        "dropdown is closed by default",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn breadcrumb_label_falls_back_to_unsaved_for_default_scenario() {
    // The preview-state seed uses the default `FormulaSpaceState::new`
    // which sets scenario_label = formula_space_id; the projector
    // renders that as the literal "unsaved" rather than leaking the
    // synthetic id.
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let label = shell
        .select(".onecalc-home-shell__breadcrumb-label")
        .expect("breadcrumb label span")
        .text_content()
        .unwrap_or_default();
    assert_eq!(label, "unsaved");

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn clicking_breadcrumb_button_opens_dropdown() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let menu = shell
        .select(".onecalc-home-shell__scenario-menu")
        .expect("scenario menu rendered (closed initially)");
    assert_eq!(
        menu.get_attribute("data-open").as_deref(),
        Some("false"),
        "menu is closed by default",
    );

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    let menu = shell
        .select(".onecalc-home-shell__scenario-menu")
        .expect("scenario menu rendered");
    assert_eq!(
        menu.get_attribute("data-open").as_deref(),
        Some("true"),
        "click on breadcrumb must open the dropdown",
    );
    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button still rendered");
    assert_eq!(
        button.get_attribute("aria-expanded").as_deref(),
        Some("true"),
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn clicking_breadcrumb_button_again_closes_dropdown() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    let menu = shell
        .select(".onecalc-home-shell__scenario-menu")
        .expect("scenario menu");
    assert_eq!(
        menu.get_attribute("data-open").as_deref(),
        Some("false"),
        "second click on breadcrumb closes the dropdown",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn dropdown_renders_recent_pinned_and_actions_sections() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    let sections = shell.select_all(".onecalc-home-shell__scenario-menu-section");
    assert_eq!(sections.length(), 3, "three sections expected: recent, pinned, actions");

    let recent = shell
        .select(".onecalc-home-shell__scenario-menu-section[data-section=\"recent\"]")
        .expect("recent section");
    assert!(recent.text_content().unwrap_or_default().contains("Recent"));

    let pinned = shell
        .select(".onecalc-home-shell__scenario-menu-section[data-section=\"pinned\"]")
        .expect("pinned section");
    assert!(pinned.text_content().unwrap_or_default().contains("Pinned"));

    let actions = shell
        .select(".onecalc-home-shell__scenario-menu-section[data-section=\"actions\"]")
        .expect("actions section");
    let actions_text = actions.text_content().unwrap_or_default();
    assert!(
        actions_text.contains("Actions"),
        "actions section heading missing in {actions_text:?}",
    );
    // User-facing labels say "formula" (per docs/APP_UX_BRIEF.md
    // §1A); the internal action-id slugs (`new-scenario`,
    // `manage-scenarios`, …) keep `scenario`.
    assert!(actions_text.contains("New formula"));
    assert!(actions_text.contains("Save as…"));
    assert!(actions_text.contains("Open…"));
    assert!(actions_text.contains("Duplicate"));
    assert!(actions_text.contains("Manage formulas…"));

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn recent_section_includes_active_scenario_marked_active() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    let active_rows = shell.select_all(
        ".onecalc-home-shell__scenario-menu-item[data-section=\"recent\"][data-is-active=\"true\"]",
    );
    assert!(
        active_rows.length() >= 1,
        "the active scenario must appear in the Recent list with data-is-active=true",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn save_as_action_carries_seam_id_and_open_action_too() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    let save_as = shell
        .select(".onecalc-home-shell__scenario-menu-item[data-action-id=\"save-as\"]")
        .expect("save-as action");
    assert_eq!(
        save_as.get_attribute("data-seam-id").as_deref(),
        Some("SEAM-ONECALC-SCENARIO-PERSIST"),
    );
    let open = shell
        .select(".onecalc-home-shell__scenario-menu-item[data-action-id=\"open\"]")
        .expect("open action");
    assert_eq!(
        open.get_attribute("data-seam-id").as_deref(),
        Some("SEAM-ONECALC-SCENARIO-PERSIST"),
    );

    let new_scenario = shell
        .select(".onecalc-home-shell__scenario-menu-item[data-action-id=\"new-scenario\"]")
        .expect("new-scenario action");
    let seam = new_scenario.get_attribute("data-seam-id").unwrap_or_default();
    assert!(
        seam.is_empty(),
        "new-scenario must NOT carry a seam id today; got {seam:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn clicking_action_button_closes_the_dropdown() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    let save_as = shell
        .select(".onecalc-home-shell__scenario-menu-item[data-action-id=\"save-as\"]")
        .expect("save-as action");
    click_element(&save_as);
    super::scaffold::flush_microtasks(5).await;

    let menu = shell
        .select(".onecalc-home-shell__scenario-menu")
        .expect("menu element");
    assert_eq!(
        menu.get_attribute("data-open").as_deref(),
        Some("false"),
        "clicking an action item closes the dropdown",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn outside_click_closes_the_dropdown() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    // Click on the textarea — outside the breadcrumb-wrap. The
    // root home-shell on:click delegate should detect the
    // outside-click and close the dropdown.
    let html_textarea: web_sys::HtmlElement = textarea.clone().unchecked_into();
    html_textarea.click();
    super::scaffold::flush_microtasks(5).await;

    let menu = shell
        .select(".onecalc-home-shell__scenario-menu")
        .expect("menu element");
    assert_eq!(
        menu.get_attribute("data-open").as_deref(),
        Some("false"),
        "click on the textarea must close the dropdown",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn escape_on_breadcrumb_button_closes_dropdown() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    // Dispatch Esc on the button (focus is on the button after click).
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("Escape");
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("keydown event");
    button
        .dispatch_event(&event)
        .expect("dispatch keydown on breadcrumb button");
    super::scaffold::flush_microtasks(5).await;

    let menu = shell
        .select(".onecalc-home-shell__scenario-menu")
        .expect("menu element");
    assert_eq!(
        menu.get_attribute("data-open").as_deref(),
        Some("false"),
        "Esc on breadcrumb button must close the dropdown",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn dirty_marker_flips_when_user_starts_editing() {
    // The preview state seeds an empty textarea — committed_cell_text
    // is None, raw_entered_cell_text is empty, live_state == Idle,
    // so dirty=false. Typing into the textarea moves to EditingLive,
    // flipping dirty to true.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    assert_eq!(
        button.get_attribute("data-dirty").as_deref(),
        Some("false"),
        "breadcrumb is clean before any input",
    );

    dispatch_input(&textarea, "=1");
    super::scaffold::flush_microtasks(15).await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button after input");
    assert_eq!(
        button.get_attribute("data-dirty").as_deref(),
        Some("true"),
        "typing into the editor flips the dirty marker on",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn status_foot_shows_scenario_label() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let scenario_name = shell
        .select(".onecalc-home-shell__statusfoot-scenario-name")
        .expect("statusfoot scenario name span");
    let label = scenario_name
        .get_attribute("data-scenario-label")
        .unwrap_or_default();
    assert_eq!(
        label, "unsaved",
        "status-foot scenario chip starts at the breadcrumb's fallback label",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn keydown_other_than_escape_does_not_close_dropdown() {
    // Defensive: only Escape should close. Pressing letter keys
    // while focused on the breadcrumb button must not affect
    // dropdown state. Otherwise typing through the keyboard chord
    // surface could become flaky.
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__breadcrumb-button")
        .expect("breadcrumb button");
    click_element(&button);
    super::scaffold::flush_microtasks(5).await;

    let init = web_sys::KeyboardEventInit::new();
    init.set_key("a");
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("keydown event");
    button.dispatch_event(&event).expect("dispatch a-key");
    super::scaffold::flush_microtasks(5).await;

    let menu = shell
        .select(".onecalc-home-shell__scenario-menu")
        .expect("menu element");
    assert_eq!(
        menu.get_attribute("data-open").as_deref(),
        Some("true"),
        "non-Escape keydown must NOT close the dropdown",
    );

    // Also pin: dispatching keydown into the textarea (well-formed
    // input keys) does not affect the breadcrumb's open state. The
    // user might be typing while the dropdown is open by accident;
    // closing on every keystroke would be hostile.
    let textarea = shell.textarea().await;
    dispatch_keydown(&textarea, "x");
    super::scaffold::flush_microtasks(5).await;

    let menu = shell
        .select(".onecalc-home-shell__scenario-menu")
        .expect("menu element");
    assert_eq!(
        menu.get_attribute("data-open").as_deref(),
        Some("true"),
        "textarea keystroke must NOT close the dropdown",
    );

    shell.tear_down();
}
