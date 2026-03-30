# DNA OneCalc Workset Register

Status: `active_register`
Date: 2026-03-31

## 1. Purpose
This is the living ordered workset register for `DnaOneCalc`.

It translates [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) into one coherent
implementation sequence for the whole scoped product.

This file is not an execution-status board.
It defines the high-level work themes, their dependency order, and their intended
rollout shape.

## 2. Planning-Surface Clarification
Planning and execution truth in this repo is split as follows:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) owns scope, design, artifact, and product truth.
2. this register owns the ordered set of worksets and their dependency shape.
3. `.beads/` owns epics, beads, readiness, blockers, in-progress state, and closure.

Working rule:
1. worksets do not carry `active`, `ready`, `blocked`, or `complete` status fields,
2. epics and leaf beads are the units that become ready, in progress, blocked, and
   closed,
3. this register and `.beads/` are the planning surfaces for implementation work,
4. after planning is in place, default execution outputs should be code, tests, and
   narrowly-scoped spec or seam corrections rather than local documentation rollout.

## 3. Use Rule
Use this document as:
1. the repo-local workset authority,
2. the source for `workset -> epic -> bead` rollout,
3. the full-scope implementation map for the product,
4. the sequencing guide for broad engineering themes.

Do not use this document as:
1. semantic authority over the OneCalc scope,
2. a substitute for the live bead graph,
3. a second blocker or readiness tracker,
4. a reason to create one document per workset or per bead.

## 4. Register Contract
Each workset in this register carries:
1. stable workset id,
2. title,
3. purpose,
4. depends_on,
5. parent spec sections,
6. primary upstream repo dependencies,
7. closure condition,
8. initial epic lanes.

## 5. Sequencing Rule
The sequence below is the default expansion order for the repo.

It does mean:
1. earlier worksets establish the product spine and implementation substrate that
   later work depends on,
2. early rollout should bias toward the shortest honest path to real dependency-backed
   formula entry and evaluation,
3. later supporting surfaces should not crowd out the first working slice unless they
   are true prerequisites.

## 6. Workset Sequence

### WS-01 Host Runtime Integration And Product Shell
1. purpose:
   establish the desktop-first OneCalc shell, the runtime boundary between host and
   upstream libraries, the host-profile and packet-kind substrate, and real
   code-level integration of the primary runtime dependencies.
2. depends_on: none
3. parent_spec_sections:
   `3`, `4`, `7.0`, `9.1` through `9.4`, `13.2`, `14.1`, `15`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, and the `OxCalc` seam-reference slice
5. closure_condition:
   a desktop-first host shell builds against real upstream dependencies, exposes the
   current host profile and packet kind honestly, and provides a runnable shell path
   that can support formula entry and evaluation work without locally redefining
   upstream semantics.
6. initial_epic_lanes:
   dependency pin and package integration, host-profile and packet-kind substrate,
   desktop shell bootstrap, platform-honesty and secondary-host gating

### WS-02 Formula Editing And Language-Service Integration
1. purpose:
   implement real formula entry and editor behavior in the shell and integrate the
   OxFml edit and diagnostics path as the authoritative language-service substrate.
2. depends_on:
   `WS-01`
3. parent_spec_sections:
   `4.2`, `5`, `9.3`, `9.4`, `16`
4. upstream_dependencies:
   `OxFml`, with `OxFunc` metadata pressure where relevant
5. closure_condition:
   a user can type and edit formulas in the real shell, OxFml edit packets are wired
   into host state, and trustworthy diagnostics are visible with runnable proof of
   the editing flow.
6. initial_epic_lanes:
   editor shell and interaction model, OxFml edit-packet integration, diagnostics
   projection, keyboard-first verification

### WS-03 Function Surface And First Evaluation Slice
1. purpose:
   wire the admitted OxFunc surface into OneCalc and deliver the first real
   dependency-backed formula-evaluation slice through OxFml and OxFunc.
2. depends_on:
   `WS-01`, `WS-02`
3. parent_spec_sections:
   `3`, `4.3`, `7.1`, `7.4`, `14.2`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`
5. closure_condition:
   OneCalc derives the admitted function surface from OxFunc in code, a user can
   enter an admitted formula and see a real evaluated result plus honest diagnostics,
   and the vertical slice has runnable verification.
6. initial_epic_lanes:
   library-context and function-surface integration, evaluate action and result
   surface, completion and current help path, vertical-slice verification

### WS-04 Driven Single-Formula Host And Retained Runs
1. purpose:
   realize the H1 driven single-formula host model, implement retained scenario and
   run handling, and support version-to-version comparison without drifting toward
   worksheet semantics.
2. depends_on:
   `WS-03`
3. parent_spec_sections:
   `6.1`, `6.2`, `7.2`, `7.2.1`, `9.1.4`, `10.3`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`
5. closure_condition:
   the driven single-formula host model exists in code, scenarios and scenario runs
   can be retained and reopened honestly, and retained version-to-version comparison
   exists for the admitted H1 scope.
6. initial_epic_lanes:
   driven host model and recalc context, retained scenario and scenario-run handling,
   retained version comparison, H1 execution verification

### WS-05 Artifact Spine, Persistence, Capability Snapshots, And ScenarioCapsule
1. purpose:
   implement the retained artifact backbone, immutable capability snapshots, document
   persistence, and portable scenario transport needed to make OneCalc work durable.
2. depends_on:
   `WS-03`, `WS-04`
3. parent_spec_sections:
   `5.3`, `6.0` through `6.10`, `10.4`, `11`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, `OxXlObs`
5. closure_condition:
   core artifact identities and envelopes are implemented in code, one isolated
   document round-trips through the declared persistence format with its required
   invariants intact, immutable capability snapshots govern retained work, and one
   scenario can be exported and re-imported as a `ScenarioCapsule` without losing
   lineage or capability truth.
6. initial_epic_lanes:
   core artifact model and identities, capability snapshot emission, document
   persistence round-trip, ScenarioCapsule export and intake

### WS-06 Replay, X-Ray, Witness, Explain, And Handoff
1. purpose:
   make retained replay, semantic inspection, witness generation, and handoff output
   first-class parts of the product once real runs and retained artifacts exist.
2. depends_on:
   `WS-04`, `WS-05`
3. parent_spec_sections:
   `5`, `10.0` through `10.4`, `16`
4. upstream_dependencies:
   `OxReplay`, `OxFml`, `OxFunc`, `OxXlObs`
5. closure_condition:
   retained runs can open replay and X-Ray surfaces, diff and explain are surfaced
   honestly for the current replay floor, and witness plus handoff artifacts can be
   produced from real retained work rather than ad hoc notes.
6. initial_epic_lanes:
   replay capture and open flow, X-Ray and diff surfaces, witness and explain flow,
   handoff generation and mode gating

### WS-07 Formatting, Effective Display, And Conditional Formatting
1. purpose:
   implement the formatting and effective-display plane, keep host style state and
   returned presentation hints explicit, and admit the isolated conditional-formatting
   subset without overclaiming parity.
2. depends_on:
   `WS-03`, `WS-05`, `WS-06`
3. parent_spec_sections:
   `8`, `10.1`, `11.3`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxXlObs`
5. closure_condition:
   base formatting and effective display are rendered and retained honestly, the
   admitted isolated conditional-formatting subset is implemented with explicit
   labeling, and the relevant planes remain distinct in product and compare surfaces.
6. initial_epic_lanes:
   presentation-hint and style-state composition, effective-display rendering,
   conditional-formatting subset integration, formatting verification families

### WS-08 Windows Observation And Twin-Oracle Comparison
1. purpose:
   establish the Windows-only live Excel observation lane and make OneCalc's twin
   compare story real through retained `Observation` and `Comparison` artifacts.
2. depends_on:
   `WS-05`, `WS-06`, `WS-07`
3. parent_spec_sections:
   `5.1`, `10.1` through `10.3`, `14.3`, `16`, `17.1`
4. upstream_dependencies:
   `OxXlObs`, `OxReplay`
5. closure_condition:
   the Windows desktop host can capture Excel observations, persist observation and
   comparison artifacts with provenance and lossiness surfaced explicitly, and drive a
   twin compare workflow with reliability badges and widening-pressure output.
6. initial_epic_lanes:
   Windows observation capture integration, observation and comparison persistence,
   twin compare view, widening-request and compare-pressure workflow

### WS-09 Extension ABI, Add-Ins, And RTD
1. purpose:
   implement the admitted desktop extension ABI, provider loading, invocation, and
   the constrained RTD subset while keeping unsupported hosts and later VBA-backed
   expansion visibly gated.
2. depends_on:
   `WS-03`, `WS-05`
3. parent_spec_sections:
   `7.3`, `12`, `14.4`, `16`, `17.2`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, later `OxVba`
5. closure_condition:
   declared desktop hosts can load and execute the admitted extension subset
   honestly, RTD support exists only within the admitted platform and semantic gate,
   and unsupported hosts are visibly blocked rather than implied.
6. initial_epic_lanes:
   extension ABI and exclusion register, provider loading and invocation, RTD subset
   and lifecycle, platform honesty and later-VBA pressure

### WS-10 Workspace Management, Capability Center, Scenario Library, And Acceptance
1. purpose:
   complete the user-facing workspace and honesty surfaces, grow the promoted
   scenario library, and make acceptance evidence and upstream pressure routine
   product operations.
2. depends_on:
   `WS-05`, `WS-06`, `WS-07`, `WS-08`, `WS-09`
3. parent_spec_sections:
   `5.2`, `5.3`, `9.1.1`, `9.1.5`, `9.1.6`, `9.5`, `9.6`, `10.3`, `16`, `17`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, `OxXlObs`, later `OxVba`
5. closure_condition:
   multi-file workspace management exists without introducing cross-instance
   semantics, the capability center and diff flows surface actual executable truth,
   promoted scenarios and acceptance evidence are managed in-product, and upstream
   pressure or widening output is generated as an ordinary consequence of real runs.
6. initial_epic_lanes:
   workspace management, capability center and diff flows, scenario library and
   promotion, host acceptance matrix and regression evidence, upstream-pressure
   workflows
