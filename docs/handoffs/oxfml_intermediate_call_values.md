# Handoff to OxFml — expose per-call intermediate values in the runtime trace

*Posted by Codex agent on behalf of @govert*

## 1. What DnaOneCalc wants

The home shell's formula drill-down (`Ctrl+D`) renders the upstream
`RuntimeFormulaResult.evaluation.trace.prepared_calls` as a tree of
function calls + arguments. The user-mode rendering reads as an
Excel-style `label = value` evaluation list, mirroring Excel's
"Evaluate Formula" dialog. To make that genuinely useful, each row
should show the COMPUTED VALUE of that subexpression: e.g. for
`=SUM(IF(1,2,3),4)` the user should see something like

```
SUM = 6
  IF = 2
    1 = TRUE
    2 = 2
    3 = 3
  4 = 4
```

The drill is the user's clearest path from "the result is wrong" to
"this subexpression is the one that produced the wrong value".

## 2. What the trace exposes today

Looking at `oxfml_core::eval::PreparedCall`:

```rust
pub struct PreparedCall {
    pub function_name: String,
    pub function_id: &'static str,
    pub arg_preparation_profile: ArgPreparationProfile,
    pub prepared_arguments: Vec<PreparedArgument>,
    pub register_id_request: Option<RegisterIdRequest>,
    pub registered_external_call_request: Option<RegisteredExternalCallRequest>,
    pub locale_profile_id: Option<String>,
    pub date_system: Option<String>,
    pub host_query_enabled: bool,
}

pub struct PreparedArgument {
    pub ordinal: usize,
    pub structure_class: PreparedStructureClass,
    pub source_class: PreparedSourceClass,
    pub evaluation_mode: PreparedEvaluationMode,
    pub blankness_class: PreparedBlanknessClass,
    pub caller_context_sensitive: bool,
    pub reference_target: Option<String>,
    pub opaque_reason: Option<String>,
}
```

Neither carries the call's RETURN VALUE or each argument's RESOLVED
VALUE. The host can render the `function_name`, the arg `ordinal` /
`reference_target`, and the prep-class debug attributes, but cannot
show the actual numeric / text / logical value at each level of the
tree.

## 3. What we'd like added upstream

A typed value field on each `PreparedCall` and (where applicable)
each `PreparedArgument`:

```rust
pub struct PreparedCall {
    // ... existing fields ...
    /// Value this call evaluated to. `Some` when the call ran to
    /// completion; `None` when the call short-circuited, errored
    /// early, or sits on a path that wasn't reached (rare).
    pub returned_value: Option<EvalValue>,
}

pub struct PreparedArgument {
    // ... existing fields ...
    /// Value this argument resolved to before being passed to the
    /// function. `Some` for ResolveAndEvaluate args; `None` for
    /// LazyExpression / ReferenceOnly args where the function
    /// receives the raw expression rather than a value.
    pub resolved_value: Option<EvalValue>,
}
```

Both should use the existing `oxfunc_core::value::EvalValue` type
(already used everywhere else in the typed-value layer; the host
already consumes it for the result hero).

The `Option<EvalValue>` shape leaves room for the legitimate cases
where there is no value to report (lazy / reference / unreached),
without forcing the host to invent one or render `None` ambiguously.

## 4. Use cases on the host side

* Walk-tree drill-down rows render `label = value` per subexpression.
  Today they render `label = ⟨debug-string⟩` which is unhelpful.
* Future "evaluate selection as subexpression" feature
  (`SEAM-OXFML-PARTIAL-EVAL` in the WS-14 plan §11) builds on the
  same machinery — knowing each subexpression's value is the
  prerequisite.
* Compare-with-Excel mismatch explainer can point at the first
  subexpression where DnaCalc's value diverges from Excel's, given
  per-call values on both sides.

## 5. What the host will NOT do

* No host-side re-evaluation of subexpressions to fill in the values
  upstream didn't expose. That would duplicate engine logic, drift
  from upstream semantics, and violate `docs/OPERATIONS.md` §9
  (Root-cause Discipline). The host renders what upstream provides;
  if upstream doesn't expose intermediate values, the rows show
  `…` muted in the value column (today's User-mode behaviour for
  `value_preview = None`).

## 6. Coordination

After upstream lands the new fields:

1. Bump the path-dep (no version change needed today; cargo will
   pick up the new struct shape automatically).
2. Update `live_bridge.rs::map_formula_walk` to consume
   `PreparedCall.returned_value` and `PreparedArgument.resolved_value`
   when populating `FormulaWalkNode.value_preview`. The current
   debug-string fillers (`"args: N · profile: X"` / `"eval={mode:?}"`)
   become fallback text only when the value is `None`.
3. Add a browser invariant that types `=SUM(IF(1,2,3),4)`, opens
   the drill, and asserts each row's value column carries the
   expected computed value (`6` for SUM, `2` for IF, `TRUE` for 1
   in the IF condition slot, etc.).
4. Delete this handoff file.

The host side of (2) and (3) lives in DnaOneCalc bead `dno-xcq.33`
(planned, not yet open) — the bead exists to be created once
upstream signals the fields are landing or landed.

## 7. Reasonable interim shape

If the full `Option<EvalValue>` is too invasive for an initial pass,
a smaller-bite alternative: expose just a stringified
`returned_value_display: Option<String>` on `PreparedCall` (rendered
the same way the host already renders `EvalValue` via
`format_eval_value_for_display`). The host gets the user-visible
value without the typed dispatch surface. The full typed-value
field can land later when `SEAM-OXFML-PARTIAL-EVAL` needs the typed
discriminator.
