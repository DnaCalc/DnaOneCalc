# DnaOneCalc Operations

## 1. Purpose
This document defines how `DnaOneCalc` is operated day to day.

It is intentionally concise.
For the detailed bead method, tooling usage, rollout template, and worked example, see [BEADS.md](BEADS.md).

## 2. Source-Of-Truth Order
Within this repo, precedence is:
1. `docs/CHARTER.md`
2. `docs/SCOPE_AND_SPEC.md`
3. this file
4. `docs/WORKSET_REGISTER.md`
5. `.beads/`

Interpretation:
1. `docs/SCOPE_AND_SPEC.md` owns scope, design, and artifact truth,
2. `docs/WORKSET_REGISTER.md` owns workset truth,
3. `.beads/` owns execution state,
4. this file owns the operating model and behavioral rules for working in the repo.

## 3. Execution Surfaces
Workset and execution surfaces live in:
1. `docs/WORKSET_REGISTER.md`
2. `.beads/`

The register holds the ordered workset set, their meaning, and their dependency shape.
The bead graph holds epics, beads, dependencies, blockers, readiness, in-progress
state, and closure.

These are the planning documents for repo execution.
Once doctrine and planning setup exist, default execution outputs should be
implementation code, test code, and narrowly-scoped spec or seam corrections,
not a growing body of local per-bead notes.

Interpretation rule:
1. worksets are high-level work themes, not execution-state objects,
2. the register does not track `active`, `ready`, `blocked`, or `complete` status per
   workset,
3. a workset is incomplete while it still has open epics or leaf beads in `.beads/`,
4. neither the register nor the bead graph should be treated as a mandate to
   create one document per work item.

Default execution model:
1. use the register to choose the next workset(s) to expand,
2. roll chosen worksets into epics,
3. create some execution beads directly during rollout when the path is already clear,
4. use rollout epics where the child bead set still needs to be discovered or staged
   during execution,
5. let the bead graph own the resulting ready set and dependency tracking,
6. expand early or well-understood work directly into executable child beads rather
   than hiding obvious implementation behind rollout placeholders,
7. close beads only with visible outcome and evidence.

## 4. Cross-Repo Read-Only Doctrine
Agents working from the `DnaOneCalc` repo may read files in sibling repositories under the shared `DnaCalc` root when needed for seam consumption, integration, evidence intake, or architectural alignment.

Those sibling repositories are read-only from the perspective of this repo.
Required changes outside `DnaOneCalc` must be routed through an explicit handoff, prompt, or separate repo-scoped run.

Cross-repo visibility is permission for understanding, not for opportunistic cleanup or silent fixes.

## 5. Bead Mutation Rule
Use `br` to mutate bead state.
Do not edit `.beads/` files directly.

`bv` is supported for graph-aware triage and analysis.
Use only non-interactive robot-style invocations from agent sessions.

## 6. Validation Discipline
Minimum local expectations before claiming meaningful progress:
1. touched docs reflect the new truth,
2. `docs/WORKSET_REGISTER.md` still matches the intended workset sequence and scope
   partitioning,
3. bead state is synchronized with the actual execution state,
4. relevant local checks for the touched area have been run where available.

Bootstrap validator:
- `scripts/check-worksets.ps1`

Interpretation:
1. this script is only a register shape check,
2. it does not report bead readiness, blockers, progress, or closure truth.

## 7. Change Discipline
1. Keep changes minimal, explicit, and reviewable.
2. Do not silently widen OneCalc toward `OxCalc`.
3. Do not claim replay, compare, formatting, conditional-formatting, or extension breadth beyond the retained evidence and admitted scope.
4. Do not substitute documentation rollout for implementation progress.
5. Capability-bearing work should normally land meaningful code plus verification.
6. Local documentation after planning setup should be limited to spec corrections,
   upstream seam handoffs, or necessary reference for behavior that now exists in code.
7. When upstream seams need to change, produce a handoff rather than normalizing local drift.

## 8. Document Count Discipline
This repo intentionally keeps a small top-level doc set.

Default rule:
1. do not create one document per workset,
2. do not multiply status documents when `WORKSET_REGISTER.md` and `.beads/` already hold the needed truth,
3. do not create bead-sized local notes as a default execution output,
4. do not split the bead method across many files unless the repo later grows enough complexity to justify it.
