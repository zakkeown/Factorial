# Robustness Audit Design: Tier 1 Coverage & Hardening

**Date:** 2026-02-06
**Status:** Approved
**Scope:** factorial-core, factorial-ffi

## Goal

Bring Factorial to beta-ready robustness with measurable confidence metrics. Developers evaluating this engine should see concrete evidence of thorough testing: line coverage numbers, mutation testing results, and CI gates that enforce standards going forward.

## Approach

Measure first, then fill gaps, then harden, then automate.

## Phase 1: Tooling & Baseline

### Tools

| Tool | Purpose |
|------|---------|
| cargo-llvm-cov | LLVM-based line/branch coverage, HTML reports, lcov output |
| cargo-mutants | Mutation testing — verifies tests assert on behavior, not just execution |
| cargo-nextest | Faster test runner with per-test timing and better output |

### Baseline Process

1. Install all three tools
2. Run `cargo llvm-cov --workspace --html` for initial coverage report
3. Record per-crate and per-module coverage for Tier 1
4. Identify lowest-coverage modules as first targets
5. Commit baseline to `docs/coverage/baseline.md`

## Phase 2: Coverage Gap-Fill — factorial-core

Work module-by-module, lowest coverage first.

### Known Gaps

| Module | Current State | Tests to Add |
|--------|--------------|-------------|
| query.rs | 0 tests | NodeSnapshot/TransportSnapshot aggregation, progress calculations, empty nodes |
| sim.rs | 4 tests | Each simulation strategy, state transitions, zero-delta edge case |
| component.rs | 3 tests | Storage retrieval, missing component, type mismatches |
| id.rs | 3 tests | Uniqueness, equality, hashing, Display |
| module.rs | 8 tests | Registration lifecycle, DeserializeFailed/NotFound errors |
| registry.rs | 8 tests | InvalidItemRef error, duplicate registration, lookup-after-removal |

### Error Path Tests

| Error Type | Untested Variants |
|------------|-------------------|
| DeserializeError | TooShort, FutureVersion, MissingPartition, PartitionDecode |
| GraphError | NodeNotFound, EdgeNotFound |
| DataLoadError | All variants (JsonParse, Registry, UnknownItemRef, UnknownRecipeRef) |

### Principle

Every test asserts on behavior, not just that code runs without panicking. Check return values. Check error variants. Check that error messages are useful.

## Phase 3: Coverage Gap-Fill — factorial-ffi

The FFI boundary requires adversarial thinking. Tests should behave like a hostile caller.

### Test Categories

| Category | What to Test |
|----------|-------------|
| Null/invalid input | Null buffers in serialization, null string args, zero-length buffers |
| Panic recovery | Force panics, verify catch_unwind returns InternalError |
| Handle lifecycle | Double-free, use-after-destroy, operations on destroyed handles |
| Serialization boundary | Truncated buffers, corrupted bytes, oversized payloads |
| State consistency | Operations after poisoned state, interleaved create/destroy |

### Principle

Every public FFI function gets at least one happy-path test and one misuse test.

## Phase 4: Mutation Testing

### When

After Tier 1 hits 80%+ line coverage.

### Process

1. Run `cargo mutants --package factorial-core` for baseline survivorship
2. Categorize surviving mutants:
   - **Worth killing:** Production logic mutations (e.g., `>=` to `>` in throughput) — write targeted tests
   - **Not worth killing:** Logging, Display impls — skip
3. Write tests to kill high-value survivors
4. Re-run to confirm improvement
5. Record final kill rate in mutation report

### Constraints

- Not chasing 100% kill rate
- Not running on integration test crates (too slow)
- Not mutating test utility code

## Phase 5: CI Setup (GitHub Actions)

### Workflow 1: Tests & Lint (every push/PR)

- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- Fast, runs on every push and PR

### Workflow 2: Coverage Gate (every PR)

- Runs `cargo llvm-cov` on Tier 1 crates
- Fails PR if coverage drops below 80%
- Posts coverage summary as PR comment via `gh`

### Workflow 3: Mutation Testing (weekly + manual)

- Scheduled weekly run of `cargo mutants` on Tier 1
- Also triggerable via `workflow_dispatch` for pre-release checks
- Results stored as workflow artifact

## Deliverables

1. **80-120 new tests** in factorial-core and factorial-ffi
2. **docs/coverage/baseline.md** — before/after coverage tables
3. **docs/coverage/mutation-report.md** — mutants summary with survivor analysis
4. **.github/workflows/** — three CI workflows, functional out of the box
5. **Tier 2/3 roadmap** — remaining gaps in spatial, fluid, power, stats, tech-tree

## Definition of Done

- All existing 643 tests still pass
- Tier 1 line coverage at 80%+
- Mutation testing run with high-value survivors addressed
- Three CI workflows committed and functional
- All docs committed
- No new warnings (`cargo clippy` clean)

## Out of Scope (Tier 2/3, future work)

- factorial-spatial (blueprint system, spatial queries)
- factorial-fluid (bridge.rs severely under-tested)
- factorial-power, factorial-stats, factorial-tech-tree
- C/C++ compilation integration tests for FFI
- Property-based testing (proptest)
- Fuzz testing (cargo-fuzz)
