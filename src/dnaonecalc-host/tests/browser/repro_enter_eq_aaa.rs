//! Reproduction (and now regression) for the user-reported bug:
//!
//!   "Press <enter>, then '=' then 'aaa'. The a's don't show on
//!    the screen."
//!
//! Root cause: the upstream OxFml tokenizer DROPS the trailing
//! `aaa` from `\n=aaa`, producing a snapshot of just (`\n`
//! trivia + `=` operator). The same input WITHOUT the leading
//! newline tokenizes correctly into `=` + `aaa`. The host now
//! defends against this by filling any gap between the
//! snapshot's declared coverage and the raw source text with a
//! `Text`-role run sourced from `raw_entered_cell_text`. The
//! user keeps every character on screen even when the parser
//! can't classify it.
//!
//! See the `syntax_runs_fill_tail_when_snapshot_drops_trailing_chars`
//! and `syntax_runs_fill_gap_between_non_contiguous_tokens` unit
//! tests in `ui/editor/render_projection.rs` for the projector-
//! side coverage; this file pins the end-to-end browser contract.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn typing_enter_then_eq_then_aaa_renders_all_chars_in_overlay() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    // Step 1: Enter alone.
    dispatch_input(&textarea, "\n");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter");

    // Step 2: Enter + `=`.
    dispatch_input(&textarea, "\n=");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter +=");

    // Step 3: the user's full sequence — Enter + `=` + `aaa`.
    // Without the gap-fill defence the upstream tokenizer drops
    // `aaa` and the overlay shows only `\n=`.
    dispatch_input(&textarea, "\n=aaa");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter += + aaa");

    // The textarea value itself must still be the user's full input —
    // not silently truncated by Leptos's prop:value or any input-event
    // handling. Pinning this separately so a regression in the value
    // pipeline is distinguished from a regression in the overlay.
    assert_eq!(textarea.value(), "\n=aaa");

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn typing_eq_aaa_without_leading_newline_still_renders_correctly() {
    // Control: the same `=aaa` content WITHOUT the leading
    // newline must keep working as before. If this regresses
    // it means the gap-fill defence is too aggressive.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=aaa");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after =aaa");
    shell.tear_down();
}

fn assert_overlay_matches_textarea(
    shell: &super::scaffold::MountedShell,
    textarea: &web_sys::HtmlTextAreaElement,
    label: &str,
) {
    let textarea_value = textarea.value();
    let overlay_text_raw = shell
        .select(".onecalc-home-shell__editor-overlay")
        .and_then(|el| el.text_content())
        .unwrap_or_default();
    // The overlay appends one trailing newline to keep its line-box
    // height when the last line is empty; strip exactly that one.
    // Crucially we do NOT trim — the bug we are reproducing involves
    // the *first* character being a newline, which trim would silently
    // swallow.
    let overlay_stripped = overlay_text_raw
        .strip_suffix('\n')
        .map(|s| s.to_string())
        .unwrap_or(overlay_text_raw.clone());

    assert_eq!(
        overlay_stripped, textarea_value,
        "[{label}] overlay text must match textarea value character-for-character. \
         textarea = {textarea_value:?} ({} chars), \
         overlay  = {overlay_stripped:?} ({} chars).",
        textarea_value.chars().count(),
        overlay_stripped.chars().count(),
    );
}
