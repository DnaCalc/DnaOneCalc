# DnaOneCalc Beads Working Method

## 1. Purpose
This file is the complete local beads story for `DnaOneCalc`.

It covers:
1. local execution method,
2. `br` and `bv` usage,
3. the serialized mutation rule,
4. bead quality expectations,
5. the workset -> epic -> bead rollout pattern,
6. a compact rollout template,
7. a compact worked example.

## 2. Core Model
Execution in this repo runs through:
1. `docs/WORKSET_REGISTER.md`
2. `workset -> epic -> bead`
3. `.beads/` as the detailed execution truth

Worksets are the high-level planning and scope-partition unit.
Epics are the main execution lanes under a chosen workset lineage.
Beads are the unit of executable progress.

Interpretation rule:
1. worksets do not carry ready/in-progress/blocked/closed execution status in the
   register,
2. `.beads/` is the sole owner of execution-state truth,
3. a workset is practically incomplete while any epic or bead rolled from it remains
   open.

## 3. Tool Split
`br` is the mutation tool.

Use it to:
1. inspect ready work,
2. create beads,
3. update bead status,
4. add dependencies,
5. close completed beads.

Typical commands:

```powershell
br ready
br show <id>
br create --title "..." --type task --priority 2
br update <id> --status in_progress
br close <id> --reason "Completed"
br dep add <issue> <depends-on>
```

`bv` is the graph-aware triage and analysis tool.

Use it to:
1. inspect the ready path,
2. identify blockers to clear,
3. inspect graph shape and priority pressure.

Agent rule:
1. use only non-interactive robot-style inspection calls from agent sessions,
2. for `bv` and read-only `br` inspection commands, prefer machine-readable or robot
   output modes where available,
3. do not launch blocking interactive views from unattended sessions.

## 4. Mutation Rule
Do not edit `.beads/` files directly.

Serialize all `br` mutations through:
- `scripts/invoke-br-serialized.ps1`

Examples:

```powershell
./scripts/invoke-br-serialized.ps1 create --title "Roll out WS-05 child beads" --type task --priority 2
./scripts/invoke-br-serialized.ps1 update dno-1 --status in_progress
./scripts/invoke-br-serialized.ps1 close dno-1 --reason "Completed"
```

## 5. Bead Quality Bar
Every executable bead should state:
1. one reviewable outcome,
2. completion evidence,
3. its parent epic,
4. any real dependency relationship,
5. canonical evidence or truth surfaces touched where that matters.

Bad beads:
1. vague activity,
2. ongoing theme,
3. mini-worksets disguised as one issue,
4. hidden follow-up work left only in chat or commit messages.

## 6. Rollout Rule
Any workset chosen for execution should be rolled out into one or more epics.

Each epic should normally begin with a rollout bead when the child path still needs to be created or refreshed.

Rollout pattern rule:
1. some epics may be expanded into child beads immediately during initial rollout,
2. some epics should begin with a rollout bead whose job is to create or refresh the
   next child beads once enough context exists,
3. both patterns are normal as long as the bead graph stays explicit and reviewable.

A rollout bead is complete only when:
1. the epic has a believable ready path,
2. the next child beads exist explicitly,
3. the work no longer depends on narrative memory alone.

## 7. Closure Rule
A bead closes only when:
1. the stated outcome exists,
2. the stated evidence exists,
3. any newly discovered required work has already been added back into the bead graph.

Do not close a bead because “enough progress” happened.

## 8. Compact Rollout Template
When a workset is chosen for rollout, capture this:

1. Workset:
   - id
   - title
   - scope
   - terminal condition
2. Execution epics:
   - rollout epic
   - first implementation lane
   - second implementation or integration lane
   - validation or evidence lane
   - cleanup or upstream-pressure lane where needed
3. First rollout bead per epic:
   - title
   - one reviewable outcome
   - completion evidence
4. First execution child beads:
   - one clear outcome each
   - explicit dependencies
   - explicit evidence

## 9. Compact Example
Example workset:
- `WS-05 Editor Viability And OxFml Language-Service Baseline`

Example epic set:
1. roll out host-shell child beads
2. desktop and browser proof-of-life lane
3. editor input and IME viability lane
4. OxFml immutable-edit integration lane
5. validation and evidence lane

Example first child beads:
1. create desktop and browser host shell skeleton
2. prove formula-buffer editing and cursor behavior
3. wire immutable edit request/result through the editor
4. record proof-of-life and viability evidence

## 10. Validator
Use:
- `scripts/check-worksets.ps1`

This is a minimal shape checker for the register, not an execution-status validator.

It should at least confirm:
1. the workset register exists,
2. workset ids are unique,
3. the register exposes a coherent workset sequence suitable for rollout into the bead
   graph.
