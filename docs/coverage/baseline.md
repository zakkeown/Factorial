# Coverage Baseline

**Date:** 2026-02-06
**Tools:** cargo-llvm-cov 0.8.4 (LLVM-based)
**Test count:** 637 tests (0 failures)

## Tier 1: factorial-core + factorial-ffi

| File | Lines | Missed Lines | Coverage % |
|------|-------|-------------|-----------|
| factorial-core/src/component.rs | 54 | 3 | 94.44% |
| factorial-core/src/data_loader.rs | 206 | 15 | 92.72% |
| factorial-core/src/dirty.rs | 197 | 0 | 100.00% |
| factorial-core/src/engine.rs | 2287 | 161 | 92.96% |
| factorial-core/src/event.rs | 867 | 32 | 96.31% |
| factorial-core/src/fixed.rs | 62 | 0 | 100.00% |
| factorial-core/src/graph.rs | 558 | 7 | 98.75% |
| factorial-core/src/id.rs | 18 | 0 | 100.00% |
| factorial-core/src/item.rs | 210 | 5 | 97.62% |
| factorial-core/src/junction.rs | 104 | 3 | 97.12% |
| factorial-core/src/migration.rs | 236 | 5 | 97.88% |
| factorial-core/src/module.rs | 166 | 6 | 96.39% |
| factorial-core/src/processor.rs | 785 | 23 | 97.07% |
| factorial-core/src/profiling.rs | 218 | 0 | 100.00% |
| factorial-core/src/registry.rs | 214 | 14 | 93.46% |
| factorial-core/src/replay.rs | 232 | 17 | 92.67% |
| factorial-core/src/serialize.rs | 1005 | 93 | 90.75% |
| factorial-core/src/sim.rs | 63 | 6 | 90.48% |
| factorial-core/src/test_utils.rs | 183 | 0 | 100.00% |
| factorial-core/src/transport.rs | 364 | 8 | 97.80% |
| factorial-core/src/validation.rs | 217 | 2 | 99.08% |
| factorial-ffi/src/lib.rs | 1691 | 169 | 90.01% |

**Aggregate Tier 1 Coverage:** 94.27% line coverage (9937 lines, 569 missed)

## Summary by Metric

| Metric | Total | Missed | Coverage |
|--------|-------|--------|----------|
| Regions | 17534 | 844 | 95.19% |
| Functions | 994 | 56 | 94.37% |
| Lines | 9937 | 569 | 94.27% |

## Lowest Coverage Files (improvement targets)

| File | Coverage % | Missed Lines |
|------|-----------|-------------|
| factorial-ffi/src/lib.rs | 90.01% | 169 |
| factorial-core/src/sim.rs | 90.48% | 6 |
| factorial-core/src/serialize.rs | 90.75% | 93 |
| factorial-core/src/replay.rs | 92.67% | 17 |
| factorial-core/src/data_loader.rs | 92.72% | 15 |
| factorial-core/src/engine.rs | 92.96% | 161 |
| factorial-core/src/registry.rs | 93.46% | 14 |
| factorial-core/src/component.rs | 94.44% | 3 |
