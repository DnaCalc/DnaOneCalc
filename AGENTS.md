# AGENTS.md — DnaOneCalc Agent Instructions

## 1. Context Loading Order
On session start, read in this order:
1. `README.md`
2. `docs/CHARTER.md`
3. `docs/OPERATIONS.md`
4. `docs/SCOPE_AND_SPEC.md`
5. `docs/WORKSET_REGISTER.md`
6. `docs/BEADS.md`

Then load upstream documents only as needed from sibling repos.

## 2. Source-Of-Truth Precedence
When guidance conflicts, precedence is:
1. `../Foundation/CHARTER.md`
2. `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
3. `../Foundation/OPERATIONS.md`
4. `docs/CHARTER.md`
5. `docs/SCOPE_AND_SPEC.md`
6. `docs/OPERATIONS.md`
7. `docs/WORKSET_REGISTER.md`
8. `docs/BEADS.md`

`docs/SCOPE_AND_SPEC.md` is the main repo engineering authority.
`docs/WORKSET_REGISTER.md` owns workset truth.
`.beads/` owns execution state.

## 3. Cross-Repo Read-Only Doctrine
You may inspect sibling repositories under the shared `DnaCalc` root for context, upstream contracts, reference docs, and retained evidence.

You must not modify, create, delete, rename, or reformat files outside this repo.

Do not use shared filesystem access to make opportunistic fixes in another repo.
If another repo needs changes, capture the need and route it through a handoff, prompt, or separate repo-local execution flow.

This is a binding rule, not a convenience guideline.

## 4. Execution Doctrine
Execution truth in this repo is split as follows:
1. `docs/WORKSET_REGISTER.md`
2. `.beads/`

Interpretation:
1. `docs/WORKSET_REGISTER.md` owns the ordered workset set, their meaning, and their
   dependency shape,
2. `.beads/` owns readiness, in-progress state, blockers, and closure,
3. worksets are high-level planning units, not execution-state objects.

Active work executes through `workset -> epic -> bead`.

Use `docs/BEADS.md` for:
1. bead working method,
2. `br` and `bv` usage,
3. serialized mutation rules,
4. rollout and closure rules.

## 5. Bead Mutation Rule
Do not edit `.beads/` files directly.

Use `br` for mutations.
Serialize mutations through:
- `scripts/invoke-br-serialized.ps1`

Do not run parallel `br create`, `br update`, `br close`, or dependency-mutation commands.

## 6. Change Discipline
1. Keep changes minimal, explicit, and testable.
2. Do not quietly widen scope beyond `docs/SCOPE_AND_SPEC.md`.
3. Keep `DnaOneCalc` narrower than `OxCalc`.
4. Preserve replay and comparison as first-class product surfaces.
5. If a change implies upstream seam pressure, capture it explicitly rather than silently normalizing local divergence.

## 7. Public Attribution Doctrine
For any issue, pull request, email response, release note, discussion post, or other external/public-facing message authored by an agent, the first line must be:

*Posted by Codex agent on behalf of @govert*

Do not add this line by default to internal docs, working notes, or local run artifacts unless explicitly requested.
