*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Lambda invocation hot-loop overhead in `OxFmlCallableInvoker`

Status: filed
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / Performance investigation
Filed date: 2026-05-05
Related:
  `OxFml/crates/oxfml_core/src/eval/mod.rs::OxFmlCallableInvoker`,
  `OxFml/crates/oxfml_core/src/eval/mod.rs::evaluate_expr_value`,
  sibling: `docs/HANDOFF_OXFUNC_REDUCE_HOTLOOP_PERF.md`,
  probe:   `DnaOneCalc/src/dnaonecalc-host/tests/mandelbrot_perf_probe.rs`.

## Symptom

A Mandelbrot formula evaluating 100×60 cells with 30 iterations
each runs in **6.4 s on a native release build** of the host crate
(~13–25 s in wasm). Per inner REDUCE iteration: ~26–35 µs. The
arithmetic is trivial; the dominant cost is per-call dispatch
through `OxFmlCallableInvoker::invoke` plus
`evaluate_expr_value` over the lambda body.

The companion OxFunc handoff
(`HANDOFF_OXFUNC_REDUCE_HOTLOOP_PERF.md`) covers the helper-side
allocator pressure (lazy iterables, inline-storage `EvalArray`
small-rows). This handoff is the **call-side** half: each REDUCE
step routes through OxFml's invoker, which does work that is
constant per evaluation but currently re-done on every step.

## Root causes inside OxFml

The `LocalCallable` lane in
`crates/oxfml_core/src/eval/mod.rs::OxFmlCallableInvoker::invoke`
is the hot path for user-supplied `LAMBDA(...)`:

```rust
fn invoke(&self, callable: &OxLambdaValue, args: &[PreparedArgValue])
    -> Result<PreparedArgValue, CallableInvocationError>
{
    if callable.origin_kind == OxCallableOriginKind::BuiltInCallable { … }

    let binding = self
        .callable_registry
        .borrow()                                   // (1) RefCell borrow per call
        .get(&callable.callable_token)              // (2) HashMap lookup per call
        .cloned()                                   // (3) clone the LambdaBinding
        .ok_or_else(|| { … })?;
    let mut local_bindings = binding.lambda.closure;// (4) clone the closure env
    for (param, arg) in binding.lambda.params.iter().zip(args.iter()) {
        insert_helper_binding(
            &mut local_bindings,
            param.name.clone(),                     // (5) clone param name per call
            HelperBinding::Arg(call_arg_from_prepared(arg, self.callable_registry)),
        );
    }
    for param in binding.lambda.params.iter().skip(args.len()) {
        insert_helper_binding(…);                   //     missing-arg fill-in
    }
    let recursion_cost_units = … ;                  // (6) iter args to flag lambdas
    let Some(_recursion_guard) = try_enter_callable_recursion(…) else { … };

    let mut trace = EvaluationTrace { prepared_calls: Vec::new() }; // (7) per-call alloc
    let mut resolver = LocalReferenceResolver { … };                // (8) per-call alloc
    let value = maybe_grow(…, || {
        evaluate_expr_value(&binding.lambda.body, …)                // (9) body walk
    })?;
    …
}
```

Per Mandelbrot REDUCE iteration each of (1)–(9) runs once; (9) is
the bulk of the body cost (evaluating the `LET` + `INDEX`s + `IF`
+ `HSTACK` against a fresh `local_bindings` map). (1)–(8) are
`O(1)` per call but each adds tens-to-hundreds of nanoseconds of
constant overhead, and they happen 180 000 times for the probe.

### Specific costs

1. **Registry borrow + lookup + clone per step.** The
   `callable_registry` is a `RefCell<HashMap<token, LambdaBinding>>`.
   Every REDUCE step re-borrows, re-looks-up by token, and clones
   the entire binding (which contains the parsed lambda body).
   `LambdaBinding` is large; the clone is not free. The token is
   constant for the duration of the REDUCE.
2. **`local_bindings = binding.lambda.closure` clones the closure
   env.** For a non-trivial enclosing scope (e.g. our outer LET
   with 14 named bindings), that's cloning 14 `HelperBinding`s on
   every step.
3. **`param.name.clone()` per param per step.** The two params of
   the inner LAMBDA are cloned 30 × 6 000 = 180 000 times.
4. **`call_arg_from_prepared(arg, registry)` walks the registry
   again** per arg per step.
5. **`EvaluationTrace { prepared_calls: Vec::new() }`** is
   allocated fresh on every call even when nothing is going to be
   appended to it.

The body evaluation itself (step 9) re-binds the local-bindings
map and re-walks the AST. The AST is constant; the
local-bindings-map allocation is per-call.

## Concrete asks

### A. Hoist invariant work out of the per-step path

Add an `invoke_many` API parallel to `invoke` that REDUCE / SCAN /
MAP / BYROW / BYCOL can call:

```rust
pub trait CallableInvoker {
    fn invoke(…) -> Result<…, …>;

    fn invoke_many(
        &self,
        callable: &OxLambdaValue,
        seed_args: &[PreparedArgValue],
        per_iter: &mut dyn FnMut(&mut dyn FnMut(&[PreparedArgValue])
            -> Result<PreparedArgValue, CallableInvocationError>),
    ) -> Result<(), CallableInvocationError>;
}
```

The default impl is a fallback that just calls `invoke` per item.
The `OxFmlCallableInvoker` specialisation does the registry
borrow, binding clone, closure-env clone, param-name resolution,
and resolver setup **once**, then exposes a `&[PreparedArgValue] →
result` closure that the helper invokes per item with only the
varying-arg substitutions touching the local bindings.

This is the single largest win in the call-side half: the body
walk in (9) still runs per item, but (1)–(8) collapse to once.

### B. Reuse `local_bindings` across iterations

Inside `invoke_many`, allocate `local_bindings` once and on each
iter:

1. Restore the param slots back to the closure's pre-call value
   (or use a scope-stack so we can pop the args).
2. Insert the new args.
3. Run `evaluate_expr_value` against the same body.

This drops the `closure.clone()` cost from O(closure_size) per
iter to O(arity) per iter.

### C. Cache resolved param names on `LambdaBinding`

`LambdaBinding.lambda.params: Vec<Param>` already carries
`param.name`. Add a parallel `param_keys: Vec<HelperBindingKey>`
populated once at registration so `insert_helper_binding` doesn't
re-hash the name string on every call.

### D. Skip `EvaluationTrace` allocation when no consumer subscribed

`trace.prepared_calls` is a Vec built on every call. If the host
isn't asking for traces (the runtime path doesn't), skip the
allocation entirely. Plumb a `TraceLevel::None` path.

### E. Specialise `evaluate_expr_value` for the body when args are
known scalar numbers

Optional, high-value for hot loops: a JIT-light path that, for
lambda bodies whose AST contains only arithmetic + INDEX over a
fixed-shape arg accumulator, lowers to a direct-call closure
without re-walking the AST. Out of scope for the first slice;
flag separately. (A) + (B) + (C) + (D) probably bring the probe
into "fast enough" territory without this.

## Suggested test corpus

`OxFml/crates/oxfml_core/tests/lambda_invoker_perf_tests.rs`
(new):

1. `invoke_many_clones_lambda_binding_once_for_n_iterations` —
   instrument `LambdaBinding.clone` to count, run REDUCE over
   `SEQUENCE(1000)`, assert clone count == 1.
2. `invoke_many_does_not_reallocate_local_bindings_per_iter` —
   instrument the bindings map allocator, run 1 000 iters, assert
   alloc count is bounded by a small constant.
3. `invoke_many_skips_evaluation_trace_when_disabled` — assert
   `EvaluationTrace::prepared_calls.capacity() == 0` after the
   call returns.
4. End-to-end timing pin: run a Mandelbrot-shape lambda 5 000
   iterations and assert per-iter time ≤ a target threshold
   (start with current/4 = 6 µs/iter, tighten as fixes land).

## DnaOneCalc-side impact

After (A) + (B) + (C) + (D) the host probe should drop
substantially. Current breakdown is approximately:

* **OxFml call-side overhead (this handoff)**: maybe 60-70% of the
  per-iter cost.
* **OxFunc helper-side allocator pressure (sibling handoff)**: ~20%.
* **Actual arithmetic + body evaluation**: ~10%.

Order matters: this handoff and the OxFunc one are independent and
can land in either order. Together they should drop the per-iter
cost from ~26-35 µs to roughly 5-10 µs, putting the 100×60×30
probe in the 1-2 second range.

The remaining gap to "instant" closes through the host-side fix
that's already scoped: split `SyntaxAndBind` (every event) from
runtime evaluation (debounced + commit-only). That's a host slice;
no upstream change needed.
