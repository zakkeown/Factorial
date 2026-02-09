# WASM Build Target Design

**Date:** 2026-02-08
**Status:** Approved

## Goal

Add a `factorial-wasm` crate that compiles the engine to `wasm32-unknown-unknown` and exports a C-style API callable from any WASM runtime (browser, Node, Godot, Wasmtime, etc.). No JS glue, no wasm-bindgen — a standalone `.wasm` module.

## Architecture

### Approach: Thin WASM Core + Future JS Wrapper

- `factorial-wasm` — compiles the engine to a standalone `.wasm` file with exported C-style functions. Runtime-agnostic.
- `factorial-js` (future, out of scope) — TypeScript wrapper that imports the `.wasm` and provides ergonomic bindings for browser/Node consumers.

### Why Not Reuse factorial-ffi Directly

The FFI crate is tuned for native C interop: `repr(C)` structs, raw pointers, cbindgen. WASM needs different mechanics:

- **No raw pointers.** WASM linear memory uses integer offsets. The engine lives in a handle table; callers reference it by integer handle.
- **No cbindgen.** WASM exports are numbered functions, not C headers.
- **Allocator exports.** WASM consumers need `factorial_alloc` / `factorial_free` to pass byte buffers across the boundary. Native FFI doesn't need this.

What stays the same: function names, semantics, error codes, handle-based opaque engine pattern, pull-based event polling.

## WASM Compatibility

The codebase is almost WASM-ready. One blocker:

**`std::time::Instant`** is used in profiling code (`engine.rs`, `profiling.rs`). `Instant` doesn't exist on `wasm32-unknown-unknown`.

**Fix:** Gate timing behind the `profiling` feature flag. `factorial-wasm` simply doesn't enable the feature. No `cfg(target_arch)` scattered through the codebase.

Everything else — fixed-point math, slotmap, serde/bitcode, graph, processors, transport, logic — is pure computation with no OS dependencies.

## API Surface

### Memory Management (WASM-specific)

- `factorial_alloc(size: i32) -> i32` — allocate bytes in WASM linear memory, returns pointer offset.
- `factorial_free(ptr: i32, size: i32)` — free a previous allocation.

### Engine Lifecycle (mirrors FFI)

- `factorial_create() -> i32` — returns engine handle (integer, not pointer).
- `factorial_create_delta(fixed_timestep: i64) -> i32`
- `factorial_destroy(handle: i32) -> i32`
- `factorial_step(handle: i32) -> i32`
- `factorial_advance(handle: i32, delta_us: i64) -> i32`

### Graph, Processors, Transport, Queries, Logic

All ~40 existing FFI functions get WASM equivalents. Signatures change only in that `*mut FactorialEngine` becomes `handle: i32`, and `*mut T` out-params become pre-allocated WASM memory offsets.

### Data Exchange Pattern

For variable-length data (serialization, events):

1. Caller allocates a buffer via `factorial_alloc`.
2. Calls e.g. `factorial_serialize(handle, buf_ptr, buf_len, out_written_ptr)`.
3. Reads the result from linear memory.
4. Frees with `factorial_free`.

For events, same pull-based model: `factorial_poll_events` writes into an engine-owned buffer, returns pointer + length. Valid until next `factorial_step`.

### Handle Table

Internally, a `Vec<Option<Engine>>` stores up to N engines. Handles are indices. This avoids raw pointers entirely and supports multiple simultaneous engine instances.

## Crate Structure

```
crates/factorial-wasm/
  Cargo.toml
  src/
    lib.rs        -- handle table, alloc/free, result codes
    engine.rs     -- lifecycle: create, destroy, step, advance
    graph.rs      -- add_node, remove_node, connect, disconnect, apply_mutations
    query.rs      -- node_count, edge_count, tick, state_hash, inventories, processor state
    processor.rs  -- set_source, set_fixed_processor
    transport.rs  -- set_flow/item/batch/vehicle_transport
    serialize.rs  -- serialize, deserialize, free_buffer
    event.rs      -- poll_events
    logic.rs      -- logic network functions
```

### Cargo.toml

- `crate-type = ["cdylib"]` — produces a `.wasm` file.
- Depends on `factorial-core` (without `profiling` feature) and `factorial-logic`.
- No `wasm-bindgen`, no `js-sys`, no `web-sys`.

### Build

```bash
cargo build --package factorial-wasm --target wasm32-unknown-unknown --release
```

Output: `target/wasm32-unknown-unknown/release/factorial_wasm.wasm`

### Workspace Integration

Add `"crates/factorial-wasm"` to root `Cargo.toml` workspace members. CI builds WASM target in a separate step.

## Core Engine Changes

Only one change to `factorial-core`: gate `std::time::Instant` usage behind the `profiling` feature flag. Six `Instant::now()` calls in `engine.rs` and the `TickProfile` struct in `profiling.rs`.

No other core changes needed.

## Testing Strategy

### Layer 1: Compile Check (CI)

`cargo build --package factorial-wasm --target wasm32-unknown-unknown`. Catches WASM-incompatible deps or API mismatches.

### Layer 2: Native Unit Tests

Handle table, argument validation, and error code mapping are platform-independent. Test with `cargo test --package factorial-wasm` (compiles as native, runs `#[cfg(test)]` modules).

### Layer 3: WASM Integration Tests (fast follow)

Use `wasmtime` crate to load the `.wasm` and call exported functions end-to-end. No browser needed, runs in CI. Added after the basic crate compiles and native tests pass.

## Out of Scope

- **JS/TS wrapper package** (`factorial-js`) — future work.
- **wasm-bindgen / wasm-pack** — stays runtime-agnostic.
- **WASI support** — `wasm32-unknown-unknown` only.
- **wasm-opt / size optimization** — document the command, don't automate yet.
- **Profiling in WASM** — `profiling` feature not enabled.
- **Streaming/async APIs** — engine is synchronous. Consumers drive the tick loop.
