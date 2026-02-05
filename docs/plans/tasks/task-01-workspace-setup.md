# Task 1: Workspace Setup & Fixed-Point Types

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 1 — Foundation (sequential) |
| **Branch** | `main` (commit directly) |
| **Depends on** | Nothing — this is the first task |
| **Parallel with** | None |
| **Skill** | `subagent-driven-development` |

## Files

- Create: `Cargo.toml` (workspace root)
- Create: `crates/factorial-core/Cargo.toml`
- Create: `crates/factorial-core/src/lib.rs`
- Create: `crates/factorial-core/src/fixed.rs`

## Context

Design doc §1 "Numeric Representation". All simulation values use Q32.32 (`Fixed64`) and Q16.16 (`Fixed32`). The `fixed` crate provides `FixedI64<U32>` and `FixedI32<U16>` which map exactly.

## Step 1: Create workspace structure

```bash
mkdir -p crates/factorial-core/src
```

Root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/factorial-core",
]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
bitcode = { version = "0.6", features = ["serde", "derive"] }
fixed = "1.28"
slotmap = "1.0"
```

`crates/factorial-core/Cargo.toml`:

```toml
[package]
name = "factorial-core"
version = "0.1.0"
edition = "2024"

[dependencies]
fixed = { workspace = true }
serde = { workspace = true }
bitcode = { workspace = true }
slotmap = { workspace = true }

[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "sim_bench"
harness = false
```

## Step 2: Write failing tests for Fixed64/Fixed32 type aliases and arithmetic

Create `crates/factorial-core/src/fixed.rs`:

```rust
use fixed::types::{I32F32, I16F16};
use serde::{Serialize, Deserialize};

/// Q32.32 fixed-point: 32 integer bits, 32 fractional bits.
/// Range: ±2,147,483,647 with precision to ~0.00000000023.
/// Used for all simulation-critical arithmetic.
pub type Fixed64 = I32F32;

/// Q16.16 fixed-point for compact storage (item properties, etc.).
/// Range: ±32,767 with precision to ~0.000015.
pub type Fixed32 = I16F16;

/// Ticks are the atomic unit of simulation time.
pub type Ticks = u64;

/// Convert an f64 to Fixed64. Use only for initialization, never in sim loop.
#[inline]
pub fn f64_to_fixed64(v: f64) -> Fixed64 {
    Fixed64::from_num(v)
}

/// Convert Fixed64 to f64. Use only for display/FFI, never in sim loop.
#[inline]
pub fn fixed64_to_f64(v: Fixed64) -> f64 {
    v.to_num::<f64>()
}

/// Convert an f64 to Fixed32. Use only for initialization.
#[inline]
pub fn f64_to_fixed32(v: f64) -> Fixed32 {
    Fixed32::from_num(v)
}

/// Convert Fixed32 to f64. Use only for display/FFI.
#[inline]
pub fn fixed32_to_f64(v: Fixed32) -> f64 {
    v.to_num::<f64>()
}

/// Checked multiplication for Fixed64 that returns None on overflow.
#[inline]
pub fn checked_mul_64(a: Fixed64, b: Fixed64) -> Option<Fixed64> {
    a.checked_mul(b)
}

/// Checked division for Fixed64 that returns None on zero divisor.
#[inline]
pub fn checked_div_64(a: Fixed64, b: Fixed64) -> Option<Fixed64> {
    a.checked_div(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed64_basic_arithmetic() {
        let a = f64_to_fixed64(1.5);
        let b = f64_to_fixed64(2.0);
        let sum = a + b;
        assert_eq!(fixed64_to_f64(sum), 3.5);
    }

    #[test]
    fn fixed64_multiplication() {
        let a = f64_to_fixed64(3.0);
        let b = f64_to_fixed64(4.0);
        let product = a * b;
        assert_eq!(fixed64_to_f64(product), 12.0);
    }

    #[test]
    fn fixed64_checked_mul_overflow() {
        let big = Fixed64::MAX;
        let two = f64_to_fixed64(2.0);
        assert!(checked_mul_64(big, two).is_none());
    }

    #[test]
    fn fixed64_checked_div_by_zero() {
        let a = f64_to_fixed64(1.0);
        let zero = f64_to_fixed64(0.0);
        assert!(checked_div_64(a, zero).is_none());
    }

    #[test]
    fn fixed32_basic_arithmetic() {
        let a = f64_to_fixed32(10.5);
        let b = f64_to_fixed32(3.25);
        let diff = a - b;
        assert_eq!(fixed32_to_f64(diff), 7.25);
    }

    #[test]
    fn fixed64_determinism() {
        // Same inputs must always produce same outputs.
        let a = f64_to_fixed64(1.0 / 3.0);
        let b = f64_to_fixed64(1.0 / 3.0);
        assert_eq!(a, b);
        assert_eq!(a * f64_to_fixed64(3.0), b * f64_to_fixed64(3.0));
    }

    #[test]
    fn fixed64_ordering() {
        let a = f64_to_fixed64(1.0);
        let b = f64_to_fixed64(2.0);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn ticks_type() {
        let t: Ticks = 60;
        assert_eq!(t, 60u64);
    }
}
```

## Step 3: Wire up lib.rs and run tests

`crates/factorial-core/src/lib.rs`:

```rust
pub mod fixed;
```

Run:

```bash
cargo test -p factorial-core
```

Expected: All tests PASS.

## Step 4: Commit

```bash
git add -A && git commit -m "feat: workspace setup with fixed-point type aliases and arithmetic"
```

## Verification

- `cargo test -p factorial-core` — all tests pass
- `Fixed64` and `Fixed32` type aliases resolve correctly
- Checked arithmetic functions work
