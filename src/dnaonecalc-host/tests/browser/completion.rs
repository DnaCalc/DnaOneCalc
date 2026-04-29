//! Browser invariants for the completion popup (bead dno-xcq.24).
//!
//! State-side coverage of the popup lifecycle lives in
//! `tests/scenarios/completion.rs`. This file pins DOM-level contracts:
//! the popup mounts when the bridge returns proposals, anchors at the
//! caret, surfaces `data-selected` / `data-proposal-id` attributes the
//! upcoming keyboard layer (bead .25) needs, and accepts on click.
//!
//! Keyboard-driven navigation tests will join this file as part of
//! bead .25.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{
    dispatch_focusout, dispatch_input, dispatch_keydown, mount_home_shell, popup_item_count,
    popup_selected_index, wait_for, wait_for_text,
};

wasm_bindgen_test_configure!(run_in_browser);

/// Typing `=SU` triggers the bridge to return SUM-family proposals;
/// the popup attaches to the DOM with at least one `.onecalc-completion-popup__item`
/// row and `data-item-count >= 1`.
#[wasm_bindgen_test(async)]
async fn typing_partial_function_opens_popup_in_dom() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let item_count = wait_for(&shell, ".onecalc-completion-popup", |element| {
        element
            .get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    assert!(
        item_count.is_some_and(|n| n >= 1),
        "popup should mount with at least one item; got {item_count:?}",
    );

    shell.tear_down();
}

/// First item carries `data-selected="true"`; the rest carry
/// `data-selected="false"`. Pins the contract bead .25's keyboard
/// navigation will toggle.
#[wasm_bindgen_test(async)]
async fn first_popup_item_is_selected_by_default() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    let items = shell.select_all(".onecalc-completion-popup__item");
    assert!(items.length() >= 1, "popup should have at least one row");

    let first = items
        .item(0)
        .expect("first item")
        .dyn_into::<web_sys::Element>()
        .expect("element");
    assert_eq!(
        first.get_attribute("data-selected").as_deref(),
        Some("true"),
        "first item starts selected",
    );

    if items.length() >= 2 {
        let second = items
            .item(1)
            .expect("second item")
            .dyn_into::<web_sys::Element>()
            .expect("element");
        assert_eq!(
            second.get_attribute("data-selected").as_deref(),
            Some("false"),
            "non-first items start unselected",
        );
    }

    shell.tear_down();
}

/// Each row exposes `data-proposal-id` + `data-kind` + glyph + label
/// so the keyboard layer (bead .25) and the future seam-status board
/// (bead .19+) can enumerate them without scraping inner HTML.
#[wasm_bindgen_test(async)]
async fn popup_items_carry_proposal_id_kind_and_glyph() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    let items = shell.select_all(".onecalc-completion-popup__item");
    assert!(items.length() >= 1);
    let first = items
        .item(0)
        .expect("first item")
        .dyn_into::<web_sys::Element>()
        .expect("element");
    assert!(
        first
            .get_attribute("data-proposal-id")
            .is_some_and(|s| !s.is_empty()),
        "data-proposal-id present and non-empty",
    );
    assert_eq!(
        first.get_attribute("data-kind").as_deref(),
        Some("function"),
        "SUM family items are functions",
    );
    let glyph = first
        .query_selector(".onecalc-completion-popup__glyph")
        .ok()
        .flatten()
        .and_then(|el| el.text_content());
    assert!(
        glyph.as_deref().is_some_and(|s| !s.trim().is_empty()),
        "kind glyph rendered",
    );
}

/// Popup is anchored within the editor frame's bounding box. The
/// browser test suite trusts the `style="left: ...; top: ..."`
/// attribute to position it; here we just assert the popup lives
/// inside `.onecalc-home-shell__editor-frame`.
#[wasm_bindgen_test(async)]
async fn popup_is_a_descendant_of_editor_frame_for_anchoring() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    let frame = shell
        .select(".onecalc-home-shell__editor-frame")
        .expect("editor frame mounted");
    let popup_inside_frame = frame.query_selector(".onecalc-completion-popup").ok().flatten();
    assert!(
        popup_inside_frame.is_some(),
        "popup must be a descendant of the editor frame so absolute positioning anchors correctly",
    );

    shell.tear_down();
}

/// Clicking a popup item splices its `insert_text` into the textarea
/// and dismisses the popup. Pins the click-to-accept path that the
/// bead's "no keyboard yet" scope makes the only acceptance route.
#[wasm_bindgen_test(async)]
async fn clicking_popup_item_replaces_text_and_closes_popup() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    // Click the first item via mousedown (the popup uses mousedown to
    // accept so the textarea retains focus).
    let first = shell
        .select(".onecalc-completion-popup__item")
        .expect("first item present");
    let mousedown_event_init = web_sys::MouseEventInit::new();
    mousedown_event_init.set_bubbles(true);
    mousedown_event_init.set_cancelable(true);
    let mousedown_event =
        web_sys::MouseEvent::new_with_mouse_event_init_dict("mousedown", &mousedown_event_init)
            .expect("create mousedown event");
    first
        .dispatch_event(&mousedown_event)
        .expect("dispatch mousedown");

    // After acceptance the textarea value should contain the
    // proposal's insert_text. For SUM family the first proposal
    // typically inserts "SUM(", so the textarea now reads "=SUM("
    // (or another SUM-prefixed form).
    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        // After acceptance the partial 'SU' has been replaced by a
        // full function name. Wait until the textarea reads
        // '=<NAME>' for any NAME of length >= 3 (SUM is 3 chars).
        // The upstream proposal inserts only the function name (the
        // trailing `(` is a UX-side choice we'll add in a future
        // bead alongside argument-aware completion).
        let value = textarea_for_value.value();
        if value.starts_with('=') && value.len() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await
    .expect("textarea value updated after acceptance");
    assert!(
        value_after.starts_with('='),
        "acceptance should preserve the leading `=`; got {value_after:?}",
    );
    // The '=SU' partial was 3 chars; any accepted proposal makes it
    // longer (real function names start with at least SU + N).
    assert!(
        value_after.chars().count() > 3,
        "acceptance should expand the partial; got {value_after:?}",
    );

    // After bead .25 landed the suppression-after-accept rule, the
    // popup is Hidden right after click acceptance and STAYS hidden
    // through the synthetic input event the click handler dispatches.
    // Pin this contract.
    let popup_after = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(
        popup_after.is_some(),
        "popup should be hidden after click acceptance (suppression-after-accept rule)",
    );

    shell.tear_down();
}

// ---------------------------------------------------------------------
// Keyboard policy (bead dno-xcq.25)
// ---------------------------------------------------------------------

/// ArrowDown advances `selected_index`; the popup re-renders with
/// the new selection.
#[wasm_bindgen_test(async)]
async fn arrowdown_advances_popup_selected_index() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    // Wait for popup to mount, with at least 2 items so ArrowDown
    // has somewhere to go.
    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 2)
    })
    .await;

    let initial = popup_selected_index(&shell).expect("initial selected index");
    assert_eq!(initial, 0);

    dispatch_keydown(&textarea, "ArrowDown");

    let advanced = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-selected-index")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n == 1)
    })
    .await;
    assert!(
        advanced.is_some(),
        "ArrowDown should move selected_index from 0 to 1",
    );

    shell.tear_down();
}

/// ArrowUp from index 0 wraps to the last index. Pins the wrap-around
/// behaviour the state machine implements.
#[wasm_bindgen_test(async)]
async fn arrowup_at_first_wraps_to_last_index() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let item_count = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 2)
    })
    .await
    .expect("at least 2 popup items");

    dispatch_keydown(&textarea, "ArrowUp");

    let wrapped = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-selected-index")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n == item_count - 1)
    })
    .await;
    assert!(
        wrapped.is_some(),
        "ArrowUp at index 0 should wrap to item_count - 1",
    );

    shell.tear_down();
}

/// Tab accepts the selected proposal and dismisses the popup. The
/// suppression-after-accept rule keeps the popup hidden through the
/// synthetic input event the acceptance dispatches.
#[wasm_bindgen_test(async)]
async fn tab_accepts_selected_proposal_and_closes_popup() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    dispatch_keydown(&textarea, "Tab");

    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        let value = textarea_for_value.value();
        if value.starts_with('=') && value.chars().count() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await
    .expect("textarea spliced after Tab");
    assert!(value_after.starts_with('='));

    let popup_gone = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(popup_gone.is_some(), "popup should be hidden after Tab");

    shell.tear_down();
}

/// Enter behaves identically to Tab for popup acceptance. Mirror
/// invariant so a future regression doesn't accidentally diverge the
/// two key handlers.
#[wasm_bindgen_test(async)]
async fn enter_accepts_selected_proposal_and_closes_popup() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    dispatch_keydown(&textarea, "Enter");

    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        let value = textarea_for_value.value();
        if value.starts_with('=') && value.chars().count() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await;
    assert!(value_after.is_some(), "Enter should splice like Tab");

    shell.tear_down();
}

/// Escape dismisses the popup WITHOUT changing the textarea text.
/// Pinned: the popup is gone, the partial input the user typed is
/// preserved.
#[wasm_bindgen_test(async)]
async fn escape_dismisses_popup_without_changing_text() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    let value_before = textarea.value();

    dispatch_keydown(&textarea, "Escape");

    let popup_gone = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(popup_gone.is_some(), "popup should be hidden after Escape");
    assert_eq!(
        textarea.value(),
        value_before,
        "Escape must not change the textarea text",
    );

    shell.tear_down();
}

/// When the popup is Hidden, ArrowLeft / ArrowRight do NOT trigger
/// any popup-state mutation. Pinned by checking that the editor
/// frame's `data-measure-tick` (which advances on every input
/// dispatch through the bridge) is unchanged after a key-only
/// dispatch — i.e. our keydown handler did not fire any
/// state-mutating callback.
#[wasm_bindgen_test(async)]
async fn arrow_keys_when_popup_hidden_do_not_mutate_state() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    // Dispatch some text WITHOUT triggering the popup. A bare `=` is
    // not a function-name prefix; the bridge returns no useful-prefix
    // proposals, so the popup stays Hidden.
    dispatch_input(&textarea, "=");

    let _ = wait_for(&shell, ".onecalc-home-shell__editor-frame", |el| {
        el.get_attribute("data-measure-tick")
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    let tick_before = shell
        .select(".onecalc-home-shell__editor-frame")
        .and_then(|el| el.get_attribute("data-measure-tick"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Verify popup is NOT mounted before dispatching the arrow key.
    assert!(
        shell.select(".onecalc-completion-popup").is_none(),
        "precondition: popup must be Hidden for this invariant",
    );

    dispatch_keydown(&textarea, "ArrowLeft");
    dispatch_keydown(&textarea, "ArrowRight");

    let tick_after = shell
        .select(".onecalc-home-shell__editor-frame")
        .and_then(|el| el.get_attribute("data-measure-tick"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    assert_eq!(
        tick_before, tick_after,
        "popup-Hidden arrow keys must not trigger any reducer round-trip; \
         measure-tick changing implies a state mutation slipped through",
    );

    shell.tear_down();
}

/// Focus-out on the textarea dismisses the popup so it doesn't sit
/// stale on an unfocused editor.
#[wasm_bindgen_test(async)]
async fn focusout_dismisses_open_popup() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    dispatch_focusout(&textarea);

    let popup_gone = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(
        popup_gone.is_some(),
        "popup should be Hidden after focusout",
    );

    shell.tear_down();
}

/// Suppression-after-accept: a keyboard acceptance (Tab) closes the
/// popup, and the synthetic input event that propagates the new
/// textarea value through the bridge does NOT re-open the popup
/// even though the bridge's proposal list now matches the
/// just-inserted function name. Pinned at the DOM level so a
/// regression in the suppression flag is caught here.
#[wasm_bindgen_test(async)]
async fn suppression_after_accept_keeps_popup_hidden_through_bridge_refresh() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    dispatch_keydown(&textarea, "Tab");

    // Popup gone within reasonable settle window.
    let popup_gone = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(popup_gone.is_some());

    // Crucially: the popup STAYS gone across a few extra microtask
    // ticks (the bridge refresh that the synthetic input dispatched
    // would have re-opened it WITHOUT the suppression flag).
    for _ in 0..10 {
        super::scaffold::next_microtask().await;
    }
    assert!(
        shell.select(".onecalc-completion-popup").is_none(),
        "suppression must keep popup hidden across post-accept bridge refresh",
    );

    let _ = popup_item_count;
    let _ = wait_for_text;
    shell.tear_down();
}
