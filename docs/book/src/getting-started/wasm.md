# WASM Quick Start

## Status

WebAssembly support is **planned but not yet available** as a first-class build target. The sections below describe the intended architecture and explain why Factorial is well-suited for WASM once the integration work is complete.

If you need WASM today, the core crate compiles to `wasm32-unknown-unknown` with `cargo build --target wasm32-unknown-unknown`, but there are no official `wasm-bindgen` bindings, JavaScript glue, or tested deployment examples yet.

## Architecture

The planned WASM integration will work as follows:

1. **Compile target** -- `wasm32-unknown-unknown`. The core engine has no system dependencies (no filesystem, no networking, no threads) and compiles cleanly to WASM.
2. **JS bindings** -- `wasm-bindgen` will expose a high-level API similar to the Rust API. Types like `Engine`, `NodeId`, and `Snapshot` will be available as JavaScript classes.
3. **Packaging** -- `wasm-pack` will produce an npm-ready package with TypeScript type definitions.
4. **Memory** -- the engine will own all simulation state inside the WASM linear memory. Snapshots and event buffers will be copied across the WASM boundary on query.

A typical integration will look like:

```text
Browser / Node.js
  --> wasm-bindgen JS glue
    --> factorial-wasm (Rust, compiled to .wasm)
      --> factorial-core (pure Rust, no_std compatible)
```

## Why it will work

Factorial's design choices make it an unusually good fit for WebAssembly:

- **Fixed-point arithmetic** -- all simulation math uses `Fixed64` (Q32.32) and `Fixed32` (Q16.16) instead of IEEE 754 floats. This eliminates the class of WASM float non-determinism issues that affect other engines. Two browsers running the same inputs will produce identical state hashes.
- **No system dependencies** -- the core crate is pure Rust with no libc calls, no file I/O, and no OS-specific code. Nothing needs to be stubbed or polyfilled.
- **No threads required** -- the engine is single-threaded by design. It runs in the browser's main thread or in a Web Worker without any shared-memory complications.
- **Small binary size** -- the core engine avoids heavy dependencies. Preliminary estimates put the `.wasm` output under 200 KB gzipped.

## Coming soon

WASM support is on the roadmap. Tracked work includes:

- `wasm-bindgen` wrapper crate (`factorial-wasm`)
- JavaScript/TypeScript API surface and type definitions
- `wasm-pack` build and npm publishing pipeline
- Browser-based demo application
- Performance benchmarks comparing native vs. WASM execution

Watch the repository for updates. In the meantime, the [Rust Quick Start](rust.md) and [C/FFI Quick Start](ffi.md) guides cover all currently supported integration paths.
