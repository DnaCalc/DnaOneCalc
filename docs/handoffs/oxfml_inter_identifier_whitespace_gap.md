# Handoff to OxFml — inter-identifier whitespace gap in token snapshot

*Posted by Codex agent on behalf of @govert*

## 1. Symptom (DnaOneCalc surface)

Typing `= a a` (equals, space, `a`, space, `a`) into the home shell's
formula editor leaves the textarea with the correct 5-character value
but the syntax-coloring overlay renders `= aa` (4 characters). The
space between the two `a`s is dropped on screen, so the visible caret
drifts past the rendered text in the same way the leading-whitespace
class did before commit `162f224` ("Fix editor token snapshot
truncation").

Reproduction: `tests/browser/repro_eq_space_a_space_a.rs::typing_eq_space_a_space_a_renders_all_chars_in_overlay`,
currently `#[ignore]`d pending the upstream fix.

## 2. Diagnosis

Headless-browser dump of the spans the host emits for `= a a`:

```
spans rendered (4 total):
  [role="operator"   start="0" text="="]
  [role="trivia"     start="1" text=" "]
  [role="identifier" start="2" text="a"]
  [role="identifier" start="4" text="a"]
```

The host renderer concatenates these and produces `= aa` (4 chars).
The space at offset 3 — between the two `a`s — is unaccounted for
in any token's `leading_trivia` or `trailing_trivia`.

Compare to `= SUM`, which works:

```
spans rendered (3 total):
  [role="operator"   start="0" text="="]
  [role="trivia"     start="1" text=" "]   <- SUM's leading_trivia
  [role="function"   start="2" text="SUM"]
```

For the `SUM` case the inter-token space is correctly attached as
`leading_trivia` on the following token. For `a a` it is dropped on
the floor.

Reproducer in `oxfml_core`:

```rust
let source = FormulaSourceRecord::new("f-1".to_string(), 1, "= a a".to_string())
    .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
let env = EditorEnvironment::new(BindContext::default());
let service = EditorEditService::new(env);
let interaction = service.apply_edit(source, None, EditorAnalysisStage::SyntaxOnly, None);
let tokens = &interaction.document.editor_syntax_snapshot.tokens;
// Today: token at offset 4 has empty leading_trivia. The space at
// offset 3 is missing from the snapshot entirely.
// Expected: token at offset 4 has leading_trivia = [" "], OR
// the token at offset 2 has trailing_trivia = [" "]. Either is
// fine; the rule is that snapshot.tokens + their trivia must
// tile the entire source_text character-for-character.
```

## 3. Root-level contract (the rule the snapshot should obey)

The DnaOneCalc-side syntax overlay relies on a contract that has been
implicit so far:

> Concatenating, in source-text order, every token's `leading_trivia`
> text + `text` field (and the LAST token's `trailing_trivia`) must
> reproduce the original source text character-for-character.

This is the only contract the host needs to render the textarea
content faithfully. If the snapshot ever drops a character (as in the
`\n=aaa` case fixed in commit 162f224, and now in the `= a a` case
documented here), the host has no way to recover the missing
character without reintroducing a forbidden host-side gap-fill (see
DnaOneCalc `docs/OPERATIONS.md` §9 — Root-cause Discipline).

It would be useful for OxFml to add this contract as an explicit
documented invariant on `EditorSyntaxSnapshot` and back it with a
property test: for any source text, a round-trip through the
snapshot reproduces the source. That's the strongest defence against
this class of bug.

## 4. Other inputs to check while you're in there

These are likely to share a code path. Worth a unit test each:

| Input | Likely shape today | Expected shape |
|---|---|---|
| `= a a` | spaces between identifiers dropped | full coverage |
| `= a b c d` | multiple inter-identifier spaces | full coverage |
| `=A1 B1` | cell ref + identifier with space | full coverage |
| `= ABC DEF` | multi-letter identifiers with whitespace | full coverage |
| `=A1   B1` | multiple-space whitespace | full coverage |
| `=a\tb` | tab whitespace | full coverage |

The DnaOneCalc-side buffer-integrity matrix in
`tests/browser/buffer_integrity.rs` (added alongside this handoff)
exercises a range of these from the host side. Each entry is its own
`#[wasm_bindgen_test]` — once the upstream fix lands the entries will
flip from `#[ignore]` to enabled in a single sweep.

## 5. What NOT to do

DnaOneCalc does NOT (and will not) plug these gaps host-side. The
short-lived attempt to do exactly that for the `\n=aaa` case was
reverted; see `docs/OPERATIONS.md` §9 (Root-cause Discipline) and the
revert commit `92378b3` for the discussion. The host's contract is
"render the snapshot faithfully"; if the snapshot is incomplete, that
is upstream's responsibility to fix.

## 6. Coordination

After the upstream fix is on a tagged release that DnaOneCalc consumes:

1. Bump the `oxfml_core` path / version in `src/dnaonecalc-host/Cargo.toml`
   if a version bump is required (today it is a path-dep so no bump
   is needed).
2. Remove `#[ignore = "..."]` from the relevant entries in:
   - `src/dnaonecalc-host/tests/browser/repro_eq_space_a_space_a.rs`
   - `src/dnaonecalc-host/tests/browser/buffer_integrity.rs` (any
     entries marked with the matching pending-upstream reason).
3. Run `scripts/run-browser-tests.ps1` and confirm the inputs pass.
4. Delete this handoff file.
