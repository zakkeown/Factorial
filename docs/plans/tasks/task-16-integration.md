# Task 16: Integration Tests & Benchmarks

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 7 — Integration (sequential) |
| **Branch** | `main` (commit directly) |
| **Depends on** | All tasks (1-15b) — everything must be merged to main |
| **Parallel with** | None |
| **Skill** | `subagent-driven-development` |

## Files

- Create: `crates/factorial-core/tests/integration.rs`
- Create: `crates/factorial-core/benches/sim_bench.rs`

## Context

End-to-end tests that exercise the full stack. Benchmarks per design doc §9 "Target Scale".

## Integration Test Scenarios

1. **Builderment-style chain:** Source → FlowTransport → FixedRecipe → FlowTransport → consumer. Verify items flow end-to-end.

2. **Multi-output recipe:** Electrolyzer (water → oxygen + hydrogen). Verify both outputs and stall on either full.

3. **Belt with inserters:** ItemTransport belt → Inserter junction → building. Verify items picked up correctly.

4. **Modifiers:** Speed module on assembler. Verify reduced duration.

5. **Serialize round-trip:** Build a factory, serialize, deserialize, run 100 more ticks, compare state hash with fresh run.

6. **Determinism:** Run same factory twice from same initial state, verify identical tick-by-tick state hashes.

## Benchmarks

- `small_factory`: 200 nodes, 500 edges, FlowTransport — target <2ms/tick
- `medium_factory`: 5000 nodes, 10000 edges, mixed transport — target <5ms/tick
- `belt_heavy`: 1000 ItemTransport belts with 50 slots each — measure belt throughput

## Additional Integration (Post-Module Merge)

If framework modules (T12-14) merged after T15b, add FFI wrappers for module-specific queries:
- Stats: `factorial_get_throughput`, `factorial_get_idle_ratio`
- Tech tree: `factorial_start_research`, `factorial_get_tech_status`
- Power: `factorial_get_satisfaction`, `factorial_get_network_status`

## Run

```bash
cargo test -p factorial-core --test integration
cargo bench -p factorial-core
```

## Commit

```bash
git add -A && git commit -m "feat: integration tests and benchmarks"
```

## Verification

- `cargo test -p factorial-core --test integration` — all 6 scenarios pass
- `cargo bench -p factorial-core` — benchmarks run and print results
- Small factory benchmark < 2ms/tick
- Determinism test produces identical hashes
- Full `cargo test --workspace` — everything green
