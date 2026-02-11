# Contributing to Factorial

Thank you for your interest in contributing to Factorial! This guide covers everything you need to get started.

For detailed architecture information, see [CLAUDE.md](CLAUDE.md).

## Getting Started

1. Clone the repository and ensure you have the Rust 2024 edition toolchain installed.
2. Build the workspace:
   ```bash
   cargo build --workspace
   ```
3. Run all tests:
   ```bash
   cargo test --workspace
   ```

## Build Commands

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test --package factorial-core

# Run a single test by name
cargo test --package factorial-core -- test_name

# Lint (CI runs with -D warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Format check / auto-format
cargo fmt --all -- --check
cargo fmt --all

# Benchmarks
cargo bench --package factorial-core

# Run an example
cargo run --package factorial-core --example minimal_factory

# Coverage (requires cargo-llvm-cov)
cargo llvm-cov --package factorial-core --package factorial-ffi --fail-under-lines 80 --text
```

## CI Requirements

All pull requests must pass the following checks:

- **Tests**: `cargo test --workspace` passes with no failures.
- **Clippy**: `cargo clippy --workspace --all-targets -- -D warnings` with `RUSTFLAGS=-Dwarnings`. Zero warnings allowed.
- **Formatting**: `cargo fmt --all -- --check` passes.
- **Coverage**: 80% minimum line coverage on `factorial-core` and `factorial-ffi`.

## Code Style

### Fixed-Point Arithmetic

All simulation math uses Q32.32 fixed-point (`Fixed64`), never floats. This guarantees cross-platform determinism for multiplayer lockstep. Use `Fixed64::from_num()` for construction and `.to_f64()` for display/debug conversion.

### Struct-of-Arrays Storage

Node and edge state uses `SlotMap` + `SecondaryMap` for cache-friendly iteration. Follow this pattern when adding new per-node or per-edge state.

### No Panics in Simulation Code

Use `Result<T, E>` with explicit `thiserror` error types. Validation happens upstream, not inside the simulation pipeline.

### Registry Immutability

The registry is built once at startup and then frozen. Tech tree unlocks gate access but never mutate the registry.

### Snapshot Versioning

Serialized state includes version numbers. When changing serialization formats, always include forward and backward migration logic.

## Testing

- Unit tests go in `#[cfg(test)] mod tests` at the bottom of each source file.
- Integration tests live in `crates/factorial-core/tests/` and `crates/factorial-integration-tests/`.
- Examples in `crates/factorial-core/examples/` double as documentation.
- Use `test_utils.rs` helpers for common setup (behind `#[cfg(any(test, feature = "test-utils"))]`).
- Mutation testing runs weekly on `factorial-core` and `factorial-ffi`.

## Pull Request Process

1. Create a branch from `main` with a descriptive name.
2. Make your changes, following the code style guidelines above.
3. Ensure all CI checks pass locally before pushing.
4. Write a clear PR description explaining **what** changed and **why**.
5. Keep PRs focused -- one logical change per PR.

## Project Structure

All crates live under `crates/`. The workspace is defined in the root `Cargo.toml`. See [CLAUDE.md](CLAUDE.md) for a full architecture overview of the three-layer design: Core, Framework Modules, and Integration crates.
