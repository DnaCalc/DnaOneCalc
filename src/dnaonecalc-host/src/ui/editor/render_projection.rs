use crate::adapters::oxfml::EditorSyntaxSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxTokenRole {
    Operator,
    Function,
    Number,
    Delimiter,
    Identifier,
    Text,
    /// Trivia between tokens (whitespace, comments). Rendered with no
    /// special colour but emitted as its own run so the overlay's text
    /// content equals the textarea's value character-for-character —
    /// without this the caret drifts past the trivia run width as soon
    /// as the user types `= SUM` (with a space) or any other input
    /// containing inter-token whitespace.
    Trivia,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxRun {
    pub text: String,
    pub span_start: usize,
    pub span_len: usize,
    pub role: SyntaxTokenRole,
}

/// Build syntax runs from an editor snapshot, preserving trivia
/// (whitespace, comments) so the rendered overlay's character count
/// equals the textarea's content character-for-character.
///
/// Trivia attachment rule: each upstream token carries both
/// `leading_trivia` and `trailing_trivia`, and the same whitespace can
/// appear on BOTH sides of adjacent tokens (the prior token's
/// trailing-trivia equals the next token's leading-trivia). To avoid
/// double-rendering, this function:
///
/// * emits `leading_trivia` of EVERY token (covers the gap before the
///   token, which for the first token is also any leading whitespace
///   at the start of input),
/// * emits `trailing_trivia` of ONLY the LAST token (covers any
///   trailing whitespace at the end of input that has no successor
///   token to claim it as leading-trivia).
///
/// `source_text` is used to FILL GAPS where the upstream tokenizer
/// produces a snapshot that's shorter than the user's input. The
/// upstream OxFml tokenizer can drop trailing characters when the
/// formula starts with certain whitespace (e.g. `\n=aaa` only
/// tokenizes as `\n` + `=`, dropping `aaa` entirely). Without
/// gap-filling the syntax overlay would be shorter than the
/// textarea, leaving the visible caret floating past where the
/// rendered text ends. Gaps are filled with `SyntaxTokenRole::Text`
/// runs sourced from the textarea content directly — the user
/// keeps their text on screen even when the parser can't classify
/// it.
///
/// Pinned by the browser invariant
/// `syntax_overlay_text_must_match_textarea_value_exactly` and the
/// `\n=aaa` reproduction in `tests/browser/repro_enter_eq_aaa.rs`.
pub fn syntax_runs_from_snapshot(
    snapshot: &EditorSyntaxSnapshot,
    source_text: &str,
) -> Vec<SyntaxRun> {
    let source_chars: Vec<char> = source_text.chars().collect();
    let source_len = source_chars.len();
    let mut runs = Vec::with_capacity(snapshot.tokens.len() * 2 + 1);
    let mut expected_offset: usize = 0;
    let last_index = snapshot.tokens.len().saturating_sub(1);
    for (token_index, token) in snapshot.tokens.iter().enumerate() {
        for trivia in &token.leading_trivia {
            push_trivia_run(&mut runs, &mut expected_offset, &trivia.text);
        }
        // Pre-token gap fill: if the trivia didn't bring us up to
        // the token's declared span_start, plug the gap with the
        // raw source-text characters in that range. Defends
        // against snapshots whose declared spans don't tile.
        if token.span.start > expected_offset && token.span.start <= source_len {
            let gap: String = source_chars[expected_offset..token.span.start]
                .iter()
                .collect();
            push_text_run(&mut runs, &mut expected_offset, &gap);
        }
        runs.push(SyntaxRun {
            text: token.text.clone(),
            span_start: token.span.start,
            span_len: token.span.len,
            role: classify_token_role(&token.text),
        });
        expected_offset = token.span.start.saturating_add(token.span.len);
        if token_index == last_index {
            for trivia in &token.trailing_trivia {
                push_trivia_run(&mut runs, &mut expected_offset, &trivia.text);
            }
        }
    }
    // Tail fill: if the last token (plus trivia) did not reach the
    // end of `source_text`, plug the remainder with a Text run.
    // Critical for the `\n=aaa` upstream-tokenizer-dropped-tail
    // case — without this the user types characters that never
    // appear on screen.
    if expected_offset < source_len {
        let tail: String = source_chars[expected_offset..].iter().collect();
        push_text_run(&mut runs, &mut expected_offset, &tail);
    }
    runs
}

/// Emit a trivia run at `expected_offset`, advancing the cursor by
/// the trivia's character count. Empty-text trivia entries are
/// skipped so the run list never carries zero-width segments.
fn push_trivia_run(runs: &mut Vec<SyntaxRun>, expected_offset: &mut usize, text: &str) {
    if text.is_empty() {
        return;
    }
    let len = text.chars().count();
    runs.push(SyntaxRun {
        text: text.to_string(),
        span_start: *expected_offset,
        span_len: len,
        role: SyntaxTokenRole::Trivia,
    });
    *expected_offset = (*expected_offset).saturating_add(len);
}

/// Emit a `Text`-role gap-fill run sourced directly from
/// `raw_entered_cell_text`. Used when the upstream tokenizer's
/// snapshot is shorter than the source text — preserves the
/// user's visible text even for inputs the parser can't classify.
fn push_text_run(runs: &mut Vec<SyntaxRun>, expected_offset: &mut usize, text: &str) {
    if text.is_empty() {
        return;
    }
    let len = text.chars().count();
    runs.push(SyntaxRun {
        text: text.to_string(),
        span_start: *expected_offset,
        span_len: len,
        role: SyntaxTokenRole::Text,
    });
    *expected_offset = (*expected_offset).saturating_add(len);
}

pub fn syntax_runs_from_text(text: &str) -> Vec<SyntaxRun> {
    let mut runs = Vec::new();
    let mut current = String::new();
    let mut current_start = 0usize;
    let mut chars = text.chars().enumerate().peekable();

    while let Some((idx, ch)) = chars.next() {
        if ch.is_whitespace() {
            flush_token(&mut runs, &mut current, current_start);
            continue;
        }

        if ch == '=' || matches!(ch, '(' | ')' | ',') {
            flush_token(&mut runs, &mut current, current_start);
            runs.push(SyntaxRun {
                text: ch.to_string(),
                span_start: idx,
                span_len: 1,
                role: classify_token_role(&ch.to_string()),
            });
            continue;
        }

        if current.is_empty() {
            current_start = idx;
        }
        current.push(ch);

        if chars
            .peek()
            .map(|(_, next)| next.is_whitespace() || matches!(next, '=' | '(' | ')' | ','))
            .unwrap_or(true)
        {
            flush_token(&mut runs, &mut current, current_start);
        }
    }

    runs
}

fn flush_token(runs: &mut Vec<SyntaxRun>, current: &mut String, current_start: usize) {
    if current.is_empty() {
        return;
    }

    let text = std::mem::take(current);
    runs.push(SyntaxRun {
        span_len: text.chars().count(),
        span_start: current_start,
        role: classify_token_role(&text),
        text,
    });
}

fn classify_token_role(text: &str) -> SyntaxTokenRole {
    if text == "=" {
        SyntaxTokenRole::Operator
    } else if matches!(text, "(" | ")" | ",") {
        SyntaxTokenRole::Delimiter
    } else if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        SyntaxTokenRole::Number
    } else if !text.is_empty() && text.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
        SyntaxTokenRole::Function
    } else if !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        SyntaxTokenRole::Identifier
    } else {
        SyntaxTokenRole::Text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_runs_follow_snapshot_tokens() {
        let snapshot = crate::test_support::make_editor_syntax_snapshot(
            "formula-1",
            "green-1",
            vec![
                crate::test_support::make_editor_token("=", 0),
                crate::test_support::make_editor_token("SUM", 1),
            ],
        );

        let runs = syntax_runs_from_snapshot(&snapshot, "=SUM");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[1].text, "SUM");
        assert_eq!(runs[1].span_start, 1);
        assert_eq!(runs[0].role, SyntaxTokenRole::Operator);
        assert_eq!(runs[1].role, SyntaxTokenRole::Function);
    }

    #[test]
    fn syntax_runs_preserve_inter_token_whitespace_trivia() {
        // Simulate "= SUM" — the upstream parser attaches inter-token
        // whitespace as `leading_trivia` of the FOLLOWING token (the
        // browser-corpus invariant `syntax_overlay_text_must_match_textarea_value_exactly`
        // pins the attachment convention). Without trivia preservation
        // the overlay would emit "=SUM" (4 chars) instead of "= SUM"
        // (5 chars), causing the caret to drift one character-width
        // past the visible "M" — the user-reported "caret offset
        // from insertion point" bug.
        use crate::adapters::oxfml::{EditorSyntaxSnapshot, EditorToken, FormulaTextSpan};
        use oxfml_core::consumer::editor::{EditorTrivia, EditorTriviaKind};
        use oxfml_core::source::FormulaChannelKind;
        use oxfml_core::syntax::token::TokenKind;

        let snapshot = EditorSyntaxSnapshot {
            formula_stable_id: "f".to_string(),
            formula_channel_kind: FormulaChannelKind::WorksheetA1,
            green_tree_key: "g".to_string(),
            tokens: vec![
                EditorToken {
                    kind: TokenKind::Equals,
                    text: "=".to_string(),
                    leading_trivia: Vec::new(),
                    trailing_trivia: Vec::new(),
                    span: FormulaTextSpan { start: 0, len: 1 },
                },
                EditorToken {
                    kind: TokenKind::Identifier,
                    text: "SUM".to_string(),
                    leading_trivia: vec![EditorTrivia {
                        kind: EditorTriviaKind::Whitespace,
                        text: " ".to_string(),
                    }],
                    trailing_trivia: Vec::new(),
                    span: FormulaTextSpan { start: 2, len: 3 },
                },
            ],
        };

        let runs = syntax_runs_from_snapshot(&snapshot, "= SUM");
        let total_text: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(
            total_text, "= SUM",
            "concatenated run text must equal the textarea value (5 chars)",
        );
        // Three runs: `=`, ` `, `SUM`.
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].role, SyntaxTokenRole::Operator);
        assert_eq!(runs[1].role, SyntaxTokenRole::Trivia);
        assert_eq!(runs[1].text, " ");
        assert_eq!(runs[1].span_start, 1);
        assert_eq!(runs[1].span_len, 1);
        assert_eq!(runs[2].role, SyntaxTokenRole::Function);
        assert_eq!(runs[2].span_start, 2);
    }

    #[test]
    fn syntax_runs_preserve_leading_trivia_at_start_of_input() {
        // Simulate "  =" — two-space leading trivia on the `=` token.
        use crate::adapters::oxfml::{EditorSyntaxSnapshot, EditorToken, FormulaTextSpan};
        use oxfml_core::consumer::editor::{EditorTrivia, EditorTriviaKind};
        use oxfml_core::source::FormulaChannelKind;
        use oxfml_core::syntax::token::TokenKind;

        let snapshot = EditorSyntaxSnapshot {
            formula_stable_id: "f".to_string(),
            formula_channel_kind: FormulaChannelKind::WorksheetA1,
            green_tree_key: "g".to_string(),
            tokens: vec![EditorToken {
                kind: TokenKind::Equals,
                text: "=".to_string(),
                leading_trivia: vec![EditorTrivia {
                    kind: EditorTriviaKind::Whitespace,
                    text: "  ".to_string(),
                }],
                trailing_trivia: Vec::new(),
                span: FormulaTextSpan { start: 2, len: 1 },
            }],
        };
        let runs = syntax_runs_from_snapshot(&snapshot, "  =");
        let total_text: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(total_text, "  =");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].role, SyntaxTokenRole::Trivia);
        assert_eq!(runs[0].text, "  ");
        assert_eq!(runs[1].role, SyntaxTokenRole::Operator);
    }

    #[test]
    fn syntax_runs_fill_tail_when_snapshot_drops_trailing_chars() {
        // Reproduces the `\n=aaa` upstream-tokenizer-truncation bug
        // at the unit level. Snapshot reports only the `\n` trivia +
        // `=` operator (the tokenizer dropped the `aaa` tail). The
        // projector must plug the gap with a `Text`-role run sourced
        // from the raw text — otherwise the user's `aaa` keystrokes
        // never appear on the syntax-overlay layer and the visible
        // caret floats past the rendered text.
        use crate::adapters::oxfml::{EditorSyntaxSnapshot, EditorToken, FormulaTextSpan};
        use oxfml_core::consumer::editor::{EditorTrivia, EditorTriviaKind};
        use oxfml_core::source::FormulaChannelKind;
        use oxfml_core::syntax::token::TokenKind;

        let snapshot = EditorSyntaxSnapshot {
            formula_stable_id: "f".to_string(),
            formula_channel_kind: FormulaChannelKind::WorksheetA1,
            green_tree_key: "g".to_string(),
            tokens: vec![EditorToken {
                kind: TokenKind::Equals,
                text: "=".to_string(),
                leading_trivia: vec![EditorTrivia {
                    kind: EditorTriviaKind::Whitespace,
                    text: "\n".to_string(),
                }],
                trailing_trivia: Vec::new(),
                span: FormulaTextSpan { start: 1, len: 1 },
            }],
        };
        let runs = syntax_runs_from_snapshot(&snapshot, "\n=aaa");
        let total_text: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(
            total_text, "\n=aaa",
            "concatenated runs must cover the full source text even when \
             the snapshot's tokens don't",
        );
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].role, SyntaxTokenRole::Trivia);
        assert_eq!(runs[0].text, "\n");
        assert_eq!(runs[1].role, SyntaxTokenRole::Operator);
        assert_eq!(runs[1].text, "=");
        // The tail-fill run carries the dropped `aaa` chars.
        assert_eq!(runs[2].role, SyntaxTokenRole::Text);
        assert_eq!(runs[2].text, "aaa");
        assert_eq!(runs[2].span_start, 2);
        assert_eq!(runs[2].span_len, 3);
    }

    #[test]
    fn syntax_runs_fill_gap_between_non_contiguous_tokens() {
        // Defensive: if the upstream snapshot's declared spans
        // don't tile (e.g. token 1 ends at offset 1 but token 2
        // starts at offset 4 with no covering trivia), the
        // projector plugs the gap with a Text run sourced from
        // the raw source text. Same machinery as the tail fill.
        use crate::adapters::oxfml::{EditorSyntaxSnapshot, EditorToken, FormulaTextSpan};
        use oxfml_core::source::FormulaChannelKind;
        use oxfml_core::syntax::token::TokenKind;

        let snapshot = EditorSyntaxSnapshot {
            formula_stable_id: "f".to_string(),
            formula_channel_kind: FormulaChannelKind::WorksheetA1,
            green_tree_key: "g".to_string(),
            tokens: vec![
                EditorToken {
                    kind: TokenKind::Equals,
                    text: "=".to_string(),
                    leading_trivia: Vec::new(),
                    trailing_trivia: Vec::new(),
                    span: FormulaTextSpan { start: 0, len: 1 },
                },
                EditorToken {
                    kind: TokenKind::Identifier,
                    text: "X".to_string(),
                    leading_trivia: Vec::new(),
                    trailing_trivia: Vec::new(),
                    // Note: starts at offset 4, leaving a 3-char gap
                    // that no trivia accounts for.
                    span: FormulaTextSpan { start: 4, len: 1 },
                },
            ],
        };
        let runs = syntax_runs_from_snapshot(&snapshot, "=ZZZX");
        let total_text: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(total_text, "=ZZZX");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[1].role, SyntaxTokenRole::Text);
        assert_eq!(runs[1].text, "ZZZ");
        assert_eq!(runs[1].span_start, 1);
    }

    #[test]
    fn syntax_runs_from_text_splits_formula_like_input() {
        let runs = syntax_runs_from_text("=LET(x,1,x)");
        assert_eq!(runs.len(), 9);
        assert_eq!(runs[1].text, "LET");
        assert_eq!(runs[1].role, SyntaxTokenRole::Function);
        assert_eq!(runs[3].role, SyntaxTokenRole::Identifier);
    }
}
