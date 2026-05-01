# Handoff to OxFml — leading-whitespace token-snapshot truncation

*Posted by Codex agent on behalf of @govert*

## 1. Symptom (DnaOneCalc surface)

In the DnaOneCalc home shell's formula editor, typing `<Enter>` then
`=` then `aaa` produces a textarea value of `\n=aaa` (5 chars) but the
syntax-coloring overlay (which renders the upstream
`EditorSyntaxSnapshot.tokens` as character-aligned spans) shows only
`\n=` (2 visible chars + the leading newline trivia). The user-typed
`aaa` characters are dropped from the rendered formula entirely. The
reproduction is `tests/browser/repro_enter_eq_aaa.rs::typing_enter_then_eq_then_aaa_renders_all_chars_in_overlay`
in the DnaOneCalc repo, currently `#[ignore]`d pending the fix
described here.

## 2. Diagnosis (where the truncation actually lives)

DnaOneCalc's host renders syntax tokens character-by-character from
the snapshot returned by
`oxfml_core::consumer::editor::EditorEditService::apply_edit`. A debug
dump from the headless browser run shows what the upstream snapshot
contains for the two inputs:

WITH leading newline (`\n=aaa`, 5 chars):
```
spans rendered (2 total):
  [role="trivia"     start="0" text="\n"]
  [role="operator"   start="1" text="="]
```

WITHOUT leading newline (`=aaa`, 4 chars):
```
spans rendered (2 total):
  [role="operator"   start="0" text="="]
  [role="identifier" start="1" text="aaa"]
```

Two observations:

1. The `aaa` identifier token IS present in the no-leading-newline
   case but ABSENT from the leading-newline case. The host renderer
   processes both snapshots identically; the difference is entirely
   upstream.
2. The `\n` is correctly emitted as `Whitespace` trivia attached to
   the `=` token. The leading newline itself is handled. The bug is
   that some part of the tokenizer / snapshot-builder bails out
   after emitting the `=` token and never tokenizes the rest of the
   input.

The reproducer is small enough to run inside a unit test in
`oxfml_core`:

```rust
let source = FormulaSourceRecord::new("f-1".to_string(), 1, "\n=aaa".to_string())
    .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
let env = EditorEnvironment::new(BindContext::default());
let service = EditorEditService::new(env);
let interaction = service.apply_edit(source, None, EditorAnalysisStage::SyntaxOnly, None);
let tokens = &interaction.document.editor_syntax_snapshot.tokens;
// Today: tokens.len() == 1 (only `=`), `aaa` is missing.
// Expected: tokens.len() == 2 (`=`, `aaa`) — same as for `=aaa` without leading newline.
```

## 3. Expected behaviour

`EditorSyntaxSnapshot.tokens` for `\n=aaa` should match
`EditorSyntaxSnapshot.tokens` for `=aaa` plus a `\n` trivia run
attached as `leading_trivia` on the first non-trivia token. Concretely:

```
EditorToken { text: "=",   span: 1..2, leading_trivia: ["\n"], trailing_trivia: [] }
EditorToken { text: "aaa", span: 2..5, leading_trivia: [],     trailing_trivia: [] }
```

The same shape should hold for any leading whitespace prefix
(spaces, tabs, multiple newlines) and any number of subsequent
identifier / number / operator tokens.

## 4. Other cases worth checking once the fix lands

These are likely to share a code path with the leading-newline case;
fixing one should fix all four. Worth a unit test each.

| Input | Today's likely snapshot | Expected snapshot |
|---|---|---|
| `\n=aaa` | `[\n trivia] [=]` (`aaa` dropped) | `[\n trivia] [=] [aaa]` |
| `\n\n=aaa` | likely same truncation | `[\n\n trivia] [=] [aaa]` |
| ` =aaa` (leading space) | possibly truncates | `[" " trivia] [=] [aaa]` |
| `\t=aaa` (leading tab) | possibly truncates | `[\t trivia] [=] [aaa]` |
| `\n=SUM(1,2)` | likely truncates after `=` | full token list with the leading `\n` as trivia on `=` |

## 5. Where to look

The most likely culprits, by familiarity with the upstream layout:

* `oxfml_core::consumer::editor::EditorEditService::apply_edit` —
  walks the tokenizer output and packages it into `EditorToken`s.
  Probably fine.
* `oxfml_core` tokenizer — wherever it produces the raw token
  stream from the source text. The truncation must live here or in
  the trivia-attachment pass. Look for code that returns early
  after consuming the FIRST non-trivia token when there's leading
  trivia.
* `oxfml_core::syntax::token` and the trivia-attachment logic
  (`leading_trivia` / `trailing_trivia` accumulator) — possibly the
  offending state machine resets / bails when it sees the first
  trivia run before any token.

## 6. What NOT to do

DnaOneCalc tried a host-side workaround (gap-fill of any unseen
tail of `source_text` with a generic `Text`-role run) and reverted
it. The workaround masked the symptom but kept upstream bugs
visible only as faded-color text in the overlay, which would have
hidden any future tokenizer regression. The reproduction tests
in DnaOneCalc are now `#[ignore]`d with a reason string referencing
this handoff; once the upstream fix lands, removing the `#[ignore]`
is the regression gate.

## 7. Coordination

After the upstream fix is on a tagged release that DnaOneCalc consumes:

1. Bump the `oxfml_core` path / version in `src/dnaonecalc-host/Cargo.toml`.
2. Remove `#[ignore = "..."]` from the two reproductions in
   `src/dnaonecalc-host/tests/browser/repro_enter_eq_aaa.rs`.
3. Run `scripts/run-browser-tests.ps1` and confirm the
   `typing_enter_then_eq_then_aaa_renders_all_chars_in_overlay`
   invariant passes.
4. Delete this handoff file.

## 8. Reproduction checklist for the upstream maintainer

```bash
# Inside the OxFml repo:
cargo test -p oxfml_core
# Add a unit test (suggested in §2) that asserts the token shape
# for `\n=aaa` matches the shape for `=aaa` plus a `\n` trivia run.
# Make it pass.
```

The DnaOneCalc-side regression test will validate end-to-end once
DnaOneCalc consumes the upstream change.
