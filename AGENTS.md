# AGENTS.md — DnaOneCalc Agent Instructions

## 1. Start Here
Read [README.md](README.md) first.

Then follow the local reading order from the README:
1. `docs/CHARTER.md`
2. `docs/OPERATIONS.md`
3. `docs/SCOPE_AND_SPEC.md`
4. `docs/WORKSET_REGISTER.md`
5. `docs/BEADS.md`

## 2. Local Authority
Within this repo:
1. `docs/SCOPE_AND_SPEC.md` is the main engineering authority,
2. `docs/WORKSET_REGISTER.md` owns workset truth,
3. `.beads/` owns execution state,
4. `docs/BEADS.md` defines the local bead method.

## 3. Cross-Repo Rule
Agents working in this repo may read sibling repositories under `C:\Work\DnaCalc`
for context, upstream contracts, reference docs, and retained evidence.

Agents must not write outside this repo.

Do not modify, create, delete, rename, or reformat files outside `DnaOneCalc`.
If another repo needs changes, capture that through a handoff, prompt, or separate
repo-scoped execution flow.

## 4. Execution Rule
Active work executes through:
1. `workset -> epic -> bead`
2. `docs/WORKSET_REGISTER.md`
3. `.beads/`

Do not edit `.beads/` files directly.
Use `br` directly for bead mutations and inspection.

## 5. Change Rule
1. Keep `DnaOneCalc` narrower than `OxCalc`.
2. Preserve replay and comparison as first-class product surfaces.
3. Prefer implementation code, test code, and narrow spec or seam corrections over
   local documentation rollout.
4. If a change creates upstream seam pressure, capture it explicitly instead of
   silently normalizing local divergence.

## 6. Public Attribution
For any external or public-facing message authored by an agent, the first line must be:

*Posted by Codex agent on behalf of @govert*

This applies to outward-facing authored content such as handoffs, prompts, public
notes, or other messages intended for human readers outside the immediate repo
execution flow.

It does not apply to internal engineering artifacts such as git commit messages,
branch names, local bead updates, or other repo-internal tool metadata unless a
separate instruction explicitly says otherwise.
