# DnaOneCalc Charter

`DnaOneCalc` is one repo in the larger `DNA Calc` program.
Read [../../Foundation/CHARTER.md](../../Foundation/CHARTER.md) as the
top-level charter for that wider project.
This local charter defines the narrower role, scope, and ownership of
`DnaOneCalc` within that broader system.

## 1. Mission
`DnaOneCalc` is the single-formula proving host for the DNA Calc stack.

Its job is to:
1. host one isolated formula scenario at a time,
2. drive `OxFml` and `OxFunc` honestly,
3. expose replay, comparison, witness, and handoff flows through a serious product UI,
4. validate behavior against Excel through `OxXlObs` where available,
5. pressure upstream repos with structured evidence rather than local reinvention.

## 2. Product Identity
The primary product identity is:
1. `Twin Oracle Workbench`
2. `Live Formula Semantic X-Ray`

This repo is not:
1. a worksheet engine,
2. a workbook dependency host,
3. a substitute for `OxCalc`,
4. a new semantics lane.

## 3. Dependency Constitution
Primary runtime dependencies:
1. `OxFml`
2. `OxFunc`
3. `OxReplay`

Primary empirical validation dependency:
1. `OxXlObs`

Staged later dependency:
1. `OxVba`

Runtime non-dependency:
1. `OxCalc`

`OxCalc` remains informative seam-reference material only.

## 4. Ownership Boundary
`DnaOneCalc` owns:
1. product shell,
2. host policy,
3. persistence,
4. extension hosting,
5. scenario orchestration,
6. replay and comparison presentation,
7. upstream handoff production.

`DnaOneCalc` does not own:
1. formula semantics,
2. function semantics,
3. replay semantics,
4. VBA semantics,
5. Excel observation semantics.

Those remain in the relevant `Ox*` repos.

## 5. Scope Boundary
The active design takes the single-cell branch of the Charter:
1. `OC-H0` literal-and-function core,
2. `OC-H1` driven single-formula host,
3. `OC-H2` host extensions and add-ins.

It excludes:
1. worksheet-style reference binding,
2. defined-name binding as a public OneCalc model,
3. workbook graph semantics,
4. worksheet `REGISTER.ID` / `CALL` as a product lane.

## 6. Evidence Rule
Every meaningful session should be capable of becoming retained evidence.
Every retained evidence item should be capable of becoming an upstream work request.

## 7. Cross-Repo Boundary Rule
This repo may read sibling repos under the shared `DnaCalc` root for seam consumption, evidence intake, and architectural alignment.

Those sibling repos are read-only from the perspective of this repo.
Required changes outside `DnaOneCalc` must be routed through a handoff, prompt, or separate repo-scoped run.
