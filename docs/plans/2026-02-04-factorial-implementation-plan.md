# Factorial Engine Implementation Plan — Parallel Execution Guide

> **For Claude:** This is the **orchestrator plan**. Each task has its own detailed file in `docs/plans/tasks/`. Use the phase-by-phase workflow below to execute tasks with maximum parallelism via parallel-cc and superpowers skills.

**Goal:** Implement the Factorial factory game engine core library in Rust, from zero code to a working simulation with transport, processors, events, serialization, and framework modules.

**Architecture:** Cargo workspace with `factorial-core` library crate, `factorial-ffi` cdylib crate, and `factorial-{tech-tree,power,stats}` framework module crates. All simulation arithmetic uses `fixed` crate types. SoA component storage via `slotmap`. Serialization via `serde` + `bitcode`. TDD throughout.

**Tech Stack:** Rust 1.93, `fixed` (fixed-point), `slotmap` (generational arenas), `serde` + `bitcode` (serialization), `cbindgen` (C headers), `criterion` (benchmarks)

**Design Doc:** `docs/plans/2026-02-04-factorial-engine-design.md` — the authoritative specification.

---

## Task Index

| Task | Name | File | Phase |
|------|------|------|-------|
| 1 | Workspace Setup & Fixed-Point Types | `tasks/task-01-workspace-setup.md` | 1 |
| 2 | ID Types & Registry | `tasks/task-02-ids-registry.md` | 1 |
| 3 | Item Storage | `tasks/task-03-item-storage.md` | 2 |
| 4 | Component Storage (SoA) | `tasks/task-04-component-storage.md` | 2 |
| 5 | Production Graph | `tasks/task-05-production-graph.md` | 3 |
| 6 | Transport Strategies | `tasks/task-06-transport.md` | 4 |
| 7 | Processor System | `tasks/task-07-processors.md` | 4 |
| 8 | Simulation Loop | `tasks/task-08-simulation-loop.md` | 5a |
| 9 | Event System | `tasks/task-09-events.md` | 5b |
| 10 | Query API | `tasks/task-10-query-api.md` | 5b |
| 11 | Serialization & Snapshots | `tasks/task-11-serialization.md` | 5c |
| 12 | Production Statistics | `tasks/task-12-stats.md` | 6 |
| 13 | Tech Tree | `tasks/task-13-tech-tree.md` | 6 |
| 14 | Power Networks | `tasks/task-14-power.md` | 6 |
| 15a | FFI Crate Skeleton | `tasks/task-15a-ffi-skeleton.md` | 3 (bg) |
| 15b | FFI Implementation | `tasks/task-15b-ffi-implementation.md` | 6 |
| 16 | Integration Tests & Benchmarks | `tasks/task-16-integration.md` | 7 |

---

## Dependency Graph

```
Phase 1 (sequential):     T1 → T2
                                │
Phase 2 (2 parallel):     T3 ──┤── T4
                                │
Phase 3 (1 + background): T5 ──┤── T15a (bg)
                                │
Phase 4 (2 parallel):     T6 ──┤── T7
                                │
Phase 5 (staged):         T8 → T9 ──┤── T10 → T11
                                     │
Phase 6 (4 parallel):    T12 ──┤── T13 ──┤── T14 ──┤── T15b
                                │
Phase 7 (sequential):    T16
```

**Critical path:** T1 → T2 → T3 → T5 → T6 → T8 → T9 → T11 → T12 → T16

**Parallel windows (4):**
| Window | Tasks | Sessions | Shared Files |
|--------|-------|----------|-------------|
| Phase 2 | T3 + T4 | 2 worktrees | `lib.rs` (trivial: each adds `pub mod`) |
| Phase 4 | T6 + T7 | 2 worktrees | `lib.rs` (trivial: each adds `pub mod`) |
| Phase 5b | T9 + T10 | 2 worktrees | `engine.rs`, `lib.rs` (non-trivial — see notes) |
| Phase 6 | T12 + T13 + T14 + T15b | 4 worktrees | workspace `Cargo.toml` (trivial: each adds member) |

---

## Phase-by-Phase Execution

### Phase 1: Foundation (Sequential)

**Session:** Main branch, single Claude session
**Skill:** `subagent-driven-development`
**Duration:** T1 + T2 sequentially

```
1. Read tasks/task-01-workspace-setup.md → execute → commit to main
2. Read tasks/task-02-ids-registry.md → execute → commit to main
```

No worktrees needed. Foundation must exist before anything else.

---

### Phase 2: Core Types (2 Parallel Worktrees)

**Sessions:** 2 Claude instances in separate worktrees
**Skill:** `executing-plans` in each session
**Prerequisite:** Phase 1 complete (T1 + T2 on main)

#### Setup

```bash
# Terminal 1
git worktree add .worktrees/feat-items -b feat/items
cd .worktrees/feat-items && claude
# → "Execute tasks/task-03-item-storage.md"

# Terminal 2
git worktree add .worktrees/feat-components -b feat/components
cd .worktrees/feat-components && claude
# → "Execute tasks/task-04-component-storage.md"
```

#### parallel-cc Coordination

```
# Before starting (in each session):
get_parallel_status                    # See other active sessions

# When first task finishes and is ready to merge:
claim_file("crates/factorial-core/src/lib.rs")    # Lock lib.rs
check_conflicts(currentBranch, "main")             # Verify clean merge
# → merge to main
# → release_file

# Second task rebases and merges:
rebase_assist(targetBranch: "main")                # Rebase onto updated main
claim_file("crates/factorial-core/src/lib.rs")
# → merge to main
# → release_file
```

#### Cleanup

```bash
git worktree remove .worktrees/feat-items
git worktree remove .worktrees/feat-components
```

**Note:** Task 4 imports from `crate::item::Inventory` (Task 3). If T3 hasn't merged yet when T4 needs to compile, rebase T4's branch onto T3's branch first.

---

### Phase 3: Graph + FFI Skeleton (Sequential + Background)

**Session:** Main branch, single Claude session
**Skill:** `subagent-driven-development`
**Prerequisite:** Phase 2 complete (T3 + T4 merged to main)

```
1. Read tasks/task-05-production-graph.md → execute → commit to main
2. (Background) Read tasks/task-15a-ffi-skeleton.md → execute → commit to main
```

T15a is a ~5 minute scaffolding task that creates the FFI crate structure. Run it after T5 or dispatch as a background subagent while T5 is being implemented (it only depends on T2).

---

### Phase 4: Strategies (2 Parallel Worktrees)

**Sessions:** 2 Claude instances in separate worktrees
**Skill:** `executing-plans` in each session
**Prerequisite:** Phase 3 complete (T5 on main)

#### Setup

```bash
# Terminal 1
git worktree add .worktrees/feat-transport -b feat/transport
cd .worktrees/feat-transport && claude
# → "Execute tasks/task-06-transport.md"

# Terminal 2
git worktree add .worktrees/feat-processors -b feat/processors
cd .worktrees/feat-processors && claude
# → "Execute tasks/task-07-processors.md"
```

#### Merge

Same pattern as Phase 2 — `claim_file` on `lib.rs`, merge sequentially, `release_file`.

---

### Phase 5: Engine + Systems (Staged)

Three sub-phases. Can be run in one long session or split across sessions.

**Prerequisite:** Phase 4 complete (T6 + T7 merged to main)

#### 5a: Simulation Loop (Sequential)

**Session:** Main branch
**Skill:** `subagent-driven-development`

```
Read tasks/task-08-simulation-loop.md → execute → commit to main
```

#### 5b: Engine Systems (2 Parallel Worktrees)

**Sessions:** 2 Claude instances
**Prerequisite:** T8 on main

```bash
# Terminal 1
git worktree add .worktrees/feat-events -b feat/events
cd .worktrees/feat-events && claude
# → "Execute tasks/task-09-events.md"

# Terminal 2
git worktree add .worktrees/feat-query -b feat/query
cd .worktrees/feat-query && claude
# → "Execute tasks/task-10-query-api.md"
```

**WARNING — Non-trivial merge conflict:** Both T9 and T10 modify `engine.rs` (adding fields to `Engine` struct and methods). Strategy:

1. Merge whichever finishes first to main
2. Second branch: `rebase_assist(targetBranch: "main")`
3. Before merging second: `detect_advanced_conflicts(currentBranch, "main")`
4. If semantic conflicts, use `get_auto_fix_suggestions` or resolve manually

**Alternative:** Run T9 → T10 sequentially in one session to avoid the `engine.rs` conflict entirely. This is simpler and only slightly slower.

#### 5c: Serialization (Sequential)

**Session:** Main branch
**Prerequisite:** T9 + T10 merged to main

```
Read tasks/task-11-serialization.md → execute → commit to main
```

---

### Phase 6: Framework Modules + FFI (4 Parallel Worktrees)

**Sessions:** Up to 4 Claude instances in separate worktrees
**Skill:** `executing-plans` in each session
**Prerequisite:** Phase 5 complete (T11 on main)

This is the **biggest parallel window** — 4 independent tasks in 4 worktrees.

#### Setup

```bash
# Terminal 1
git worktree add .worktrees/feat-stats -b feat/stats
cd .worktrees/feat-stats && claude
# → "Execute tasks/task-12-stats.md"

# Terminal 2
git worktree add .worktrees/feat-tech-tree -b feat/tech-tree
cd .worktrees/feat-tech-tree && claude
# → "Execute tasks/task-13-tech-tree.md"

# Terminal 3
git worktree add .worktrees/feat-power -b feat/power
cd .worktrees/feat-power && claude
# → "Execute tasks/task-14-power.md"

# Terminal 4
git worktree add .worktrees/feat-ffi -b feat/ffi
cd .worktrees/feat-ffi && claude
# → "Execute tasks/task-15b-ffi-implementation.md"
```

#### Merge Strategy

T12, T13, T14 each modify workspace `Cargo.toml` (adding their crate to members). Merge them sequentially:

```
1. First to finish: claim_file("Cargo.toml") → merge → release
2. Second: rebase_assist("main") → claim_file("Cargo.toml") → merge → release
3. Third: rebase_assist("main") → claim_file("Cargo.toml") → merge → release
4. T15b: no Cargo.toml conflict (already added in T15a) → merge directly
```

#### Cleanup

```bash
git worktree remove .worktrees/feat-stats
git worktree remove .worktrees/feat-tech-tree
git worktree remove .worktrees/feat-power
git worktree remove .worktrees/feat-ffi
```

---

### Phase 7: Integration (Sequential)

**Session:** Main branch
**Skill:** `subagent-driven-development`
**Prerequisite:** All tasks merged to main

```
Read tasks/task-16-integration.md → execute → commit to main
```

Final verification:

```bash
cargo test --workspace
cargo bench -p factorial-core
```

Then: `superpowers:finishing-a-development-branch`

---

## Parallel-CC Quick Reference

### Session Coordination

| Tool | When to Use |
|------|------------|
| `get_parallel_status` | Start of each session — see who else is working |
| `get_my_session` | Identify your session for file claims |
| `claim_file(path)` | Before merging — lock shared files |
| `release_file(claimId)` | After merging — unlock shared files |
| `list_file_claims` | Check what's locked before editing |

### Merge Workflow

| Tool | When to Use |
|------|------------|
| `check_conflicts(current, target)` | Before any merge — quick conflict check |
| `detect_advanced_conflicts(current, target)` | Before merging to engine.rs — semantic analysis |
| `rebase_assist(targetBranch)` | When main has moved forward — rebase your branch |
| `get_auto_fix_suggestions(file, current, target)` | When conflicts found — get AI resolution |
| `apply_auto_fix(suggestionId)` | Apply a suggested resolution |
| `notify_when_merged(branch)` | Wait for a dependency branch to merge |
| `check_merge_status(branch)` | Poll whether a branch has merged |

### Shared File Conflict Strategy

| File | Conflict Type | Resolution |
|------|--------------|------------|
| `crates/factorial-core/src/lib.rs` | Each task adds `pub mod X;` | Trivial additive — keep all lines |
| `Cargo.toml` (workspace) | Each module adds member | Trivial additive — keep all entries |
| `crates/factorial-core/Cargo.toml` | Rare (deps set in T1-T2) | `claim_file` if needed |
| `crates/factorial-core/src/engine.rs` | T9 + T10 both add fields/methods | Non-trivial — use `detect_advanced_conflicts` |

---

## Skill Assignment Summary

| Context | Skill |
|---------|-------|
| Sequential phases on main (T1, T2, T5, T8, T11, T16) | `subagent-driven-development` |
| Parallel worktree sessions (T3/T4, T6/T7, T9/T10, T12-15b) | `executing-plans` |
| Research within any session | `dispatching-parallel-agents` |
| TDD within any task | `test-driven-development` (used by subagents) |
| Before claiming work is done | `verification-before-completion` |
| After all tasks complete | `finishing-a-development-branch` |

---

## Estimated Execution Timeline

| Phase | Tasks | Parallelism | Relative Time |
|-------|-------|-------------|---------------|
| 1 | T1 → T2 | Sequential | ██ |
| 2 | T3 ‖ T4 | 2-way | █ |
| 3 | T5 + T15a(bg) | 1 + background | ███ |
| 4 | T6 ‖ T7 | 2-way | ██ |
| 5a | T8 | Sequential | ██ |
| 5b | T9 ‖ T10 | 2-way | █ |
| 5c | T11 | Sequential | ██ |
| 6 | T12 ‖ T13 ‖ T14 ‖ T15b | **4-way** | ██ |
| 7 | T16 | Sequential | ██ |

**Original plan:** 11 sequential levels, 3 parallel windows (max 3-way)
**This plan:** 9 steps, 4 parallel windows (max **4-way**), FFI pulled 2 phases earlier
