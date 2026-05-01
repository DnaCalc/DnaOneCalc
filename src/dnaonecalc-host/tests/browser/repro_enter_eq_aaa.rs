//! Reproduction for the user-reported bug:
//!
//!   "Press <enter>, then '=' then 'aaa'. The a's don't show on
//!    the screen."
//!
//! Root cause is upstream in OxFml: when the formula source text
//! starts with a leading whitespace character (e.g. `\n`), the
//! editor tokenizer emits a snapshot that ends after the FIRST
//! non-trivia token and silently drops the remaining text. For
//! `\n=aaa` the snapshot is `\n` trivia + `=` operator only;
//! `aaa` is missing entirely. The same input WITHOUT the leading
//! newline (`=aaa`) tokenizes correctly into `=` + `aaa`.
//!
//! Status: PENDING upstream fix in OxFml's editor tokenizer. The
//! fix belongs in `oxfml_core::consumer::editor`'s tokenizer /
//! syntax-snapshot builder, NOT in DnaOneCalc. See
//! `docs/handoffs/oxfml_leading_whitespace_truncation.md` for the
//! prompt routed to OxFml.
//!
//! Both invariants below run real text through the live bridge
//! and assert the syntax-overlay text equals the textarea value.
//! They are #[ignore]d today; once the upstream fix lands they
//! flip back to enabled and act as the regression-prevention
//! pin.
//!
//! IMPORTANT for future agents: do NOT re-enable these tests by
//! adding host-side gap-fill logic that papers over the upstream
//! truncation. That route was tried and reverted — it added a
//! permanent reliance on workaround code, hid the fact that
//! tokens were lost, and would have masked any future upstream
//! tokenizer regression. The right place for the fix is upstream.
//! See `docs/OPERATIONS.md` §9 (Root-cause Discipline).

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
#[ignore = "pending upstream OxFml fix: leading whitespace truncates token snapshot"]
async fn typing_enter_then_eq_then_aaa_renders_all_chars_in_overlay() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_input(&textarea, "\n");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter");

    dispatch_input(&textarea, "\n=");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter +=");

    dispatch_input(&textarea, "\n=aaa");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter += + aaa");

    assert_eq!(textarea.value(), "\n=aaa");
    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn typing_eq_aaa_without_leading_newline_still_renders_correctly() {
    // Control: the same `=aaa` content WITHOUT the leading
    // newline tokenizes correctly. Kept enabled so a regression
    // in the no-leading-whitespace path is caught.
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
    // We do NOT trim — the bug we are reproducing involves the
    // *first* character being a newline, which trim would silently
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
