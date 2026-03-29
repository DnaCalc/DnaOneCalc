# DNA OneCalc Workset Register

Status: `active_register`
Date: 2026-03-29

## 1. Purpose
This is the living ordered workset register for `DnaOneCalc`.

It translates the engineering obligations in [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md)
into one coherent sequence of repo-local worksets that can be rolled into epics and
beads.

This file is not an execution-status board.
It defines the high-level work themes and their default expansion order.

## 2. Execution-State Clarification
Execution truth in this repo is split as follows:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) owns scope, design, artifact, and product truth.
2. this register owns the ordered set of worksets and their dependency shape.
3. `.beads/` owns execution state, including readiness, in-progress work, blockers,
   and closure.

Working rule:
1. worksets do not carry `active`, `ready`, `queued`, `blocked`, or `complete`
   status fields,
2. epics and leaf beads are the units that become ready, active, blocked, and closed,
3. a workset is practically incomplete while any rolled-out epic or leaf bead under it
   remains open in `.beads/`,
4. the register should therefore describe workset meaning and rollout intent, not
   duplicate bead-graph execution state.

Interpretation note:
1. some inherited Foundation and local template wording still speaks of `umbrella`
   or `active` worksets,
2. this register interprets that wording as lineage and rollout structure rather than
   as a second execution-status system maintained in the register itself.

## 3. Use Rule
Use this document as:
1. the repo-local workset authority,
2. the source for `workset -> epic -> bead` rollout,
3. the default sequencing guide for broad execution themes,
4. the bridge between [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) and `.beads/`.

Do not use this document as:
1. semantic authority over the OneCalc scope,
2. a substitute for the live bead graph,
3. a second blocker or readiness tracker,
4. a reason to create one document per workset.

## 4. Register Contract
Each workset in this register carries:
1. stable workset id,
2. title,
3. purpose,
4. depends_on,
5. parent spec sections,
6. primary upstream repo dependencies,
7. closure condition,
8. initial epic lanes,
9. execution notes where needed.

## 5. Sequencing Rule
The sequence below is the default expansion order for the repo.

It does not mean:
1. only one workset may exist in the bead graph at a time,
2. later worksets can never be partially explored early,
3. the register itself owns runtime execution status.

It does mean:
1. earlier worksets establish the control surfaces that later work depends on,
2. bead rollout should preserve the dependency logic expressed here,
3. if execution intentionally violates the default sequence, the reason should be
   explicit in the bead graph rather than hidden in chat.

## 6. Workset Sequence

### WS-01 Repo Bootstrap And Local Execution Doctrine
1. purpose:
   establish the repo-local operating model, serialized bead mutation discipline,
   validation basis, and minimum bootstrap surfaces required to execute OneCalc work
   coherently.
2. depends_on: none
3. parent_spec_sections:
   `13.1`, `13.2`, `18`
4. upstream_dependencies:
   `Foundation` doctrine, plus awareness of `OxFml`, `OxFunc`, `OxReplay`,
   `OxXlObs`, and the `OxCalc` seam-reference slice
5. closure_condition:
   local docs, scripts, and execution rules are aligned enough that further work can
   proceed without reconstructing repo doctrine from scattered notes.
6. initial_epic_lanes:
   doctrine sync, bootstrap tooling, validation discipline, repo control-surface
   publication
7. notes:
   this workset should also capture any local-vs-Foundation wording mismatch that
   affects execution method.

### WS-02 Seam Manifest, Dependency Pins, And Host-Profile Matrix
1. purpose:
   define what `DnaOneCalc` actually consumes from upstream and freeze the initial
   host-profile, packet-kind, seam-pin, and minimum capability-ledger basis for the
   repo.
2. depends_on:
   `WS-01`
3. parent_spec_sections:
   `4`, `7.0`, `13.2`, `14`, `19`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, `OxXlObs`, `OxCalc` seam-reference slice,
   later `OxVba`
5. closure_condition:
   the seam manifest, dependency pin set, host-profile ladder, and packet-kind
   register exist in repo-owned form, and the minimum governing capability facts that
   later runs and handoffs must point to are explicit enough to govern downstream
   implementation.
6. initial_epic_lanes:
   upstream seam intake, dependency pin capture, host-profile matrix, packet-kind
   register, baseline seam-evidence map, minimum capability-ledger basis

### WS-03 Artifact Spine And Control Schema Set
1. purpose:
   freeze the retained artifact basis for `Scenario`, `ScenarioRun`, `Observation`,
   `Comparison`, `Witness`, `HandoffPacket`, `Document`, `CapabilityLedgerSnapshot`,
   and `ScenarioCapsule`.
2. depends_on:
   `WS-01`, `WS-02`
3. parent_spec_sections:
   `5.3`, `6.0` through `6.10`, `13.2`
4. upstream_dependencies:
   `OxFml`, `OxReplay`, `OxXlObs`, and `OxFunc` where returned-value or replay
   surfaces shape the retained artifacts
5. closure_condition:
   artifact identities, lineage rules, shared envelope fields, and minimum schema
   floors are explicit enough that UI, replay, persistence, and retained-run or
   handoff flows do not invent private shapes ad hoc.
6. initial_epic_lanes:
   artifact identity rules, lineage and referential integrity, minimum schema fields,
   handoff and capsule contract basis

### WS-04 Host Shell, Runtime Split, And Platform Proof
1. purpose:
   stand up the workbench shell, shared application core, desktop/browser host split,
   and the first platform-honest product shell.
2. depends_on:
   `WS-01`, `WS-02`
3. parent_spec_sections:
   `5`, `9.1` through `9.3`, `9.6`, `16`
4. upstream_dependencies:
   none as semantic owners, but shaped by all runtime dependencies named in `WS-02`
5. closure_condition:
   the shared shell and host split exist with keyboard-usable basic flows, explicit
   platform gating, and visible product-shell truth.
6. initial_epic_lanes:
   shared app-core skeleton, desktop host proof-of-life, browser/WASM proof-of-life,
   context bar and shell skeleton, platform gating

### WS-05 Editor Viability And OxFml Language-Service Baseline
1. purpose:
   prove real editor viability and integrate OxFml edit packets, diagnostics, and the
   minimum language-service baseline into the host shell.
2. depends_on:
   `WS-02`, `WS-04`
3. parent_spec_sections:
   `4.2`, `9.3`, `9.4`, `16`
4. upstream_dependencies:
   `OxFml`, with `OxFunc` metadata pressure where relevant
5. closure_condition:
   a user can edit formulas in the real host shell, receive trustworthy diagnostics,
   and rely on deterministic edit-packet identity and provenance.
6. initial_epic_lanes:
   editor buffer and IME viability, immutable edit packet integration, diagnostics
   surface, keyboard flow and latency evidence

### WS-06 Function Surface Admission, Metadata, Help, And Completion
1. purpose:
   integrate the OxFunc function-surface truth model into OneCalc, including snapshot
   intake, overlay-driven admission labels, completion, and current help/signature
   surfaces.
2. depends_on:
   `WS-02`, `WS-05`
3. parent_spec_sections:
   `4.3`, `7.4`, `14.2`, `16`
4. upstream_dependencies:
   `OxFunc`, plus `OxFml` editor integration points
5. closure_condition:
   OneCalc derives `supported`, `preview`, `experimental`, `deferred`, and
   `catalog_only` honestly from upstream overlays and exposes completion/help without
   inventing private surface taxonomy.
6. initial_epic_lanes:
   library-context snapshot consumption, admission-label derivation, validated
   completion flow, function and signature help integration

### WS-07 OC-H0 Execution, Result Surface, And Retained Runs
1. purpose:
   deliver the first end-to-end `OC-H0` run path through `OxFml` and `OxFunc` with a
   retained `ScenarioRun` and an honest result surface.
2. depends_on:
   `WS-02`, `WS-03`, `WS-04`, `WS-05`, `WS-06`
3. parent_spec_sections:
   `7.1`, `9.1.2` through `9.1.4`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`
5. closure_condition:
   promoted H0 scenarios can be authored, run, retained, and reopened with explicit
   host-profile, packet-kind, and provisionality truth visible in the UI.
6. initial_epic_lanes:
   execution facade, result-surface normalization, retained run identity, H0 scenario
   proof set

### WS-08 Replay, X-Ray, Witness, And Handoff Baseline
1. purpose:
   make retained replay and semantic inspection first-class by wiring replay capture,
   X-Ray views, witness retention, and draft handoff generation into the workbench.
2. depends_on:
   `WS-03`, `WS-07`
3. parent_spec_sections:
   `5.1`, `10.0` through `10.3`, `14.3`, `16`
4. upstream_dependencies:
   `OxReplay`, `OxFml`, and `OxXlObs` where observation-facing comparison boundaries
   affect replay/comparison flow
5. closure_condition:
   retained runs can open replay/X-Ray surfaces and generate the first structured
   witness and handoff artifacts with honest capability-floor labeling.
6. initial_epic_lanes:
   replay capture and open flow, X-Ray inspection, witness retention, handoff draft
   generation, replay-floor labeling

### WS-09 Capability Observatory And Workbench Truth Surfaces
1. purpose:
   implement the machine-readable capability ledger, the human-facing capability
   center, and the always-visible truth surfaces that keep dependency and mode claims
   honest.
2. depends_on:
   `WS-02`, `WS-03`, `WS-04`, `WS-06`, `WS-07`, `WS-08`
3. parent_spec_sections:
   `5.2`, `5.3`, `9.6.5`, `9.6.7`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, `OxXlObs`, later `OxVba`
5. closure_condition:
   the workspace can emit, display, diff, and export the exact dependency, seam,
   observation, replay, extension, and platform truth that governs current behavior.
6. initial_epic_lanes:
   capability snapshot lifecycle hardening, context-bar truth system, capability
   center UI, capability diff and export

### WS-10 OC-H1 Driven Host And Version-To-Version Comparison
1. purpose:
   realize the driven single-formula host model without sliding toward worksheet
   semantics, including recalc controls, formula replacement, and retained
   version-to-version comparison.
2. depends_on:
   `WS-07`, `WS-08`, `WS-09`
3. parent_spec_sections:
   `7.2`, `7.2.1`, `10.1`, `10.3`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`
5. closure_condition:
   OneCalc can run and compare retained H1 scenario families using the admitted
   driven-host model while remaining visibly narrower than `OxCalc`.
6. initial_epic_lanes:
   driven recalc model, formula-replacement flows, retained version comparison,
   H1 scenario-family growth

### WS-11 Formatting, Effective Display, And Isolated Conditional Formatting
1. purpose:
   add the formatting plane, effective-display composition, and the admitted
   isolated-instance conditional-formatting subset as first-class host scope.
2. depends_on:
   `WS-07`, `WS-08`, `WS-10`
3. parent_spec_sections:
   `8`, `10.3`, `11.3`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxXlObs`
5. closure_condition:
   promoted formatting and admitted conditional-formatting subsets are rendered,
   retained, inspectable, and compared honestly, with the relevant planes kept
   separate in X-Ray and comparison views.
6. initial_epic_lanes:
   base formatting and effective display, style-state versus presentation-hint
   composition, CF carrier model, retained formatting-sensitive scenario families

### WS-12 Persistence And ScenarioCapsule Transport
1. purpose:
   implement the first real persistence story through `SpreadsheetML 2003` plus the
   separate evidence-transport story through `ScenarioCapsule`, while giving
   multi-instance and workspace file management an explicit first home.
2. depends_on:
   `WS-03`, `WS-09`, `WS-10`, `WS-11`
3. parent_spec_sections:
   `6.7`, `6.7.1` through `6.7.3`, `10.4`, `11`, `16`
4. upstream_dependencies:
   local container mapping is OneCalc-owned, with replay and observation dependencies
   shaping retained attachments
5. closure_condition:
   one isolated OneCalc instance round-trips through the declared persistence format
   with retained identity and formatting truth intact, and capsule export/intake
   preserves lineage and capability refs honestly, with workspace-level management of
   multiple isolated instances made explicit without introducing cross-instance
   semantics.
6. initial_epic_lanes:
   document mapping, attachment and retained-artifact indexing, capsule manifest and
   export, capsule intake validation, multi-instance and workspace file-management
   baseline

### WS-13 Windows Twin-Oracle Comparison Through OxXlObs
1. purpose:
   establish the Windows-only live Excel compare lane and the corresponding retained
   comparison and widening-pressure workflow.
2. depends_on:
   `WS-08`, `WS-09`, `WS-10`, `WS-11`, `WS-12`
3. parent_spec_sections:
   `5.2`, `9.1.1`, `10`, `14.3`, `16`, `17.1`
4. upstream_dependencies:
   `OxXlObs`, `OxReplay`
5. closure_condition:
   the Windows desktop host can emit retained `Observation` and `Comparison`
   artifacts with `direct`, `derived`, `lossy`, and `unavailable` truth surfaced
   explicitly and can generate widening requests when the comparison envelope is too
   narrow.
6. initial_epic_lanes:
   live capture integration, compare view, reliability and provenance labeling,
   widening-request handoff flow

### WS-14 Extension ABI, Desktop Add-In Loading, And RTD
1. purpose:
   define and implement the admitted native-extension ABI, desktop add-in loading,
   function registration, invocation, and RTD lifecycle.
2. depends_on:
   `WS-06`, `WS-09`, `WS-10`
3. parent_spec_sections:
   `7.3`, `12`, `14.4`, `16`, `17.2`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, later `OxVba`
5. closure_condition:
   declared desktop hosts can load and run admitted extensions honestly, with
   unsupported platforms visibly gated and the RTD story kept within the admitted
   subset.
6. initial_epic_lanes:
   portable ABI contract, extension discovery and validation, registration and
   invocation flow, RTD activation model and platform honesty

### WS-15 Corpus Hardening, Host Acceptance, And Upstream Pressure
1. purpose:
   harden the retained scenario corpus, host acceptance evidence, capability diffs,
   and routine upstream pressure and handoff flows once the main product slices exist.
2. depends_on:
   `WS-08`, `WS-09`, `WS-10`, `WS-11`, `WS-12`, `WS-13`, `WS-14`
3. parent_spec_sections:
   `9.1.5`, `9.1.6`, `10.3`, `13.4`, `16`, `17`, `19`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, `OxXlObs`, later `OxVba`
5. closure_condition:
   promoted scenario families, host-acceptance evidence, capability-diff workflows,
   and evidence-backed upstream handoff generation are ordinary product operations
   rather than ad hoc cleanup.
6. initial_epic_lanes:
   promoted scenario spines, host acceptance matrix and regression evidence, corpus
   governance, capability-diff workflows, upstream-pressure and handoff refinement
