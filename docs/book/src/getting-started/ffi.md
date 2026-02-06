# C/FFI Quick Start

This guide walks through using the Factorial engine from C or C++ via the FFI layer. You will build the same minimal factory as the [Rust Quick Start](rust.md) -- an iron mine feeding an assembler -- but using the C-compatible API exposed by `factorial-ffi`.

## 1. Build the library

Build the FFI crate as a shared library (or static library, depending on your needs):

```bash
cargo build -p factorial-ffi --release
```

The output is `target/release/libfactorial_ffi.so` (Linux), `libfactorial_ffi.dylib` (macOS), or `factorial_ffi.dll` (Windows).

## 2. Generate the header

Use [cbindgen](https://github.com/mozilla/cbindgen) to generate a C header from the Rust source:

```bash
cbindgen --crate factorial-ffi -o factorial.h
```

This produces declarations for all `extern "C"` functions, `repr(C)` structs, and enums.

## 3. Link in your project

Add the shared library and header to your build system. For example, with GCC:

```bash
gcc -o my_game main.c -L target/release -lfactorial_ffi -Wl,-rpath,target/release
```

Or with CMake:

```cmake
target_link_libraries(my_game PRIVATE factorial_ffi)
target_include_directories(my_game PRIVATE ${CMAKE_SOURCE_DIR}/include)
```

## 4. Create an engine

All interaction goes through an opaque `FactorialEngine` handle. Every FFI function returns a `FactorialResult` status code -- always check it.

```c
#include "factorial.h"
#include <stdio.h>

int main(void) {
    FactorialEngine *engine = factorial_create();
    if (engine == NULL) {
        fprintf(stderr, "Failed to create engine\n");
        return 1;
    }
```

`FactorialResult` codes:

| Code | Meaning |
|------|---------|
| `Ok` (0) | Success |
| `NullPointer` (1) | A required pointer argument was null |
| `InvalidHandle` (2) | The engine handle is invalid |
| `NodeNotFound` (5) | The requested [node](../introduction/glossary.md#node) was not found |
| `EdgeNotFound` (6) | The requested [edge](../introduction/glossary.md#edge) was not found |
| `InternalError` (7) | A Rust panic was caught at the FFI boundary |
| `Poisoned` (8) | A previous panic left the engine in an inconsistent state |

## 5. Add nodes and connect

Graph mutations use the same queue-apply-resolve pattern as the Rust API. Pending IDs are written through output pointers and resolved after `factorial_apply_mutations`:

```c
    /* Queue two nodes */
    FfiPendingNodeId pending_mine, pending_assembler;
    FactorialResult r;

    r = factorial_add_node(engine, 0, &pending_mine);      /* BuildingTypeId 0 = mine */
    if (r != Ok) { fprintf(stderr, "add_node failed: %d\n", r); return 1; }

    r = factorial_add_node(engine, 1, &pending_assembler);  /* BuildingTypeId 1 = assembler */
    if (r != Ok) { fprintf(stderr, "add_node failed: %d\n", r); return 1; }

    /* Apply mutations to get real IDs */
    FfiMutationResult mut_result;
    r = factorial_apply_mutations(engine, &mut_result);
    if (r != Ok) { fprintf(stderr, "apply_mutations failed: %d\n", r); return 1; }

    /* Extract real node IDs from the mutation result */
    FfiNodeId mine_id = mut_result.added_nodes[0].real_id;
    FfiNodeId assembler_id = mut_result.added_nodes[1].real_id;
```

Now connect them with an edge:

```c
    /* Queue a connection */
    FfiPendingEdgeId pending_belt;
    r = factorial_connect(engine, mine_id, assembler_id, &pending_belt);
    if (r != Ok) { fprintf(stderr, "connect failed: %d\n", r); return 1; }

    r = factorial_apply_mutations(engine, &mut_result);
    if (r != Ok) { fprintf(stderr, "apply_mutations failed: %d\n", r); return 1; }

    FfiEdgeId belt_id = mut_result.added_edges[0].real_id;
```

## 6. Configure

Set the mine as a [Source processor](../introduction/glossary.md#processor) and configure [inventories](../introduction/glossary.md#inventory) and a [transport](../introduction/glossary.md#transport-strategy).

[`Fixed64`](../introduction/glossary.md#fixed-point) values are passed as raw Q32.32 bits: shift an integer left by 32. For example, `2` becomes `2LL << 32`.

```c
    /* Mine: Source producing item type 0 at rate 2/tick */
    int64_t rate_2 = 2LL << 32;  /* Fixed64 representation of 2 */
    r = factorial_set_source(engine, mine_id, 0, rate_2);
    if (r != Ok) { fprintf(stderr, "set_source failed: %d\n", r); return 1; }

    /* Assembler: Fixed recipe, 2x item 0 -> 1x item 1, duration 5 ticks */
    FfiItemStack inputs[]  = { { .item_type = 0, .quantity = 2 } };
    FfiItemStack outputs[] = { { .item_type = 1, .quantity = 1 } };
    FfiRecipe recipe = {
        .input_count  = 1,
        .inputs       = inputs,
        .output_count = 1,
        .outputs      = outputs,
        .duration     = 5,
    };
    r = factorial_set_fixed_processor(engine, assembler_id, &recipe);
    if (r != Ok) { fprintf(stderr, "set_fixed_processor failed: %d\n", r); return 1; }

    /* Set inventories (capacity 100 per node) */
    factorial_set_input_capacity(engine, mine_id, 100);
    factorial_set_output_capacity(engine, mine_id, 100);
    factorial_set_input_capacity(engine, assembler_id, 100);
    factorial_set_output_capacity(engine, assembler_id, 100);

    /* Set flow transport on the belt: rate 5/tick */
    int64_t rate_5 = 5LL << 32;
    r = factorial_set_flow_transport(engine, belt_id, rate_5);
    if (r != Ok) { fprintf(stderr, "set_flow_transport failed: %d\n", r); return 1; }
```

## 7. Step the simulation

Each call to `factorial_step` advances the engine by one [tick](../introduction/glossary.md#tick):

```c
    for (int tick = 0; tick < 10; tick++) {
        r = factorial_step(engine);
        if (r != Ok) {
            fprintf(stderr, "step failed at tick %d: %d\n", tick, r);
            break;
        }
```

## 8. Poll events

After each step, poll the event buffer to react to production changes. The buffer is engine-owned and valid until the next `factorial_step` or `factorial_destroy`:

```c
        FfiEventBuffer events;
        r = factorial_poll_events(engine, &events);
        if (r == Ok) {
            for (uint32_t i = 0; i < events.count; i++) {
                printf("  Event: kind=%d node=%llu tick=%llu\n",
                       events.events[i].kind,
                       events.events[i].node,
                       events.events[i].tick);
            }
        }
```

Event kinds include `ItemProduced`, `RecipeCompleted`, `BuildingStalled`, and others. See the `FfiEventKind` enum in the generated header.

## 9. Query state

Query [processor state](../introduction/glossary.md#processor), node count, and other properties at any time:

```c
        FfiProcessorInfo info;
        r = factorial_get_processor_state(engine, assembler_id, &info);
        if (r == Ok) {
            printf("  Assembler: state=%d progress=%u\n", info.state, info.progress);
        }

        uint32_t count;
        factorial_node_count(engine, &count);
        printf("  Total nodes: %u\n", count);
    }
```

The `FfiProcessorState` enum maps to: `Idle` (0), `Working` (1), `StalledMissingInputs` (2), `StalledOutputFull` (3), `StalledNoPower` (4), `StalledDepleted` (5).

You can also retrieve a deterministic [state hash](../introduction/glossary.md#state-hash) for desync detection:

```c
    uint64_t hash;
    factorial_get_state_hash(engine, &hash);
    printf("State hash: %llu\n", hash);
```

## 10. Cleanup

Destroy the engine to free all memory. After this call, the handle is invalid:

```c
    factorial_destroy(engine);
    return 0;
}
```

## Error handling summary

Every FFI function wraps its body in `catch_unwind` to prevent Rust panics from crossing the FFI boundary. If a panic is caught, the function returns `InternalError` and marks the engine as **poisoned**. A poisoned engine rejects all subsequent calls with `Poisoned`. Always check `FactorialResult` return values, and treat `Poisoned` as an unrecoverable error that requires creating a new engine.

## What to explore next

- **Full FFI reference** -- all functions, structs, and enums. See [Function Reference](../ffi/reference.md).
- **API conventions** -- safety rules, memory ownership, thread safety. See [API Conventions & Safety](../ffi/conventions.md).
- **Language bindings** -- higher-level wrappers for GDScript, C#, and others. See [Language-Specific Bindings](../ffi/bindings.md).
