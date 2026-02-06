# API Conventions & Safety

This page documents the conventions and safety rules that govern the
Factorial C FFI. Every consumer of `factorial.h` -- whether C, C++,
C#, GDScript, or any other language with C interop -- must follow these
rules to avoid undefined behavior and memory corruption.

---

## Opaque pointer pattern

The FFI exposes a single engine handle: `FactorialEngine*`. This type is
declared as an opaque `struct` in the generated header. You must **never**
dereference it, cast it, or inspect its contents. The only valid
operations on a `FactorialEngine*` are:

1. Receive it from `factorial_create()` or `factorial_create_delta()`.
2. Pass it as the first argument to every other `factorial_*` function.
3. Free it with `factorial_destroy()`.

After `factorial_destroy()`, the pointer is invalid. Using it in any
subsequent call is undefined behavior.

```c
/* Correct */
FactorialEngine *engine = factorial_create();
factorial_step(engine);
factorial_destroy(engine);
engine = NULL;  /* good practice */

/* WRONG -- never dereference */
// engine->inner;  /* compilation error: incomplete type */
```

---

## Result codes

Every FFI function returns a `FactorialResult` enum. You must check the
return value of every call. The values are:

| Value | Name                | Meaning |
|-------|---------------------|---------|
| 0     | `FACTORIAL_RESULT_OK` | Success. |
| 1     | `FACTORIAL_RESULT_NULL_POINTER` | A required pointer argument was null. |
| 2     | `FACTORIAL_RESULT_INVALID_HANDLE` | The engine handle is invalid (null or dangling). |
| 3     | `FACTORIAL_RESULT_SERIALIZE_ERROR` | Serialization failed. |
| 4     | `FACTORIAL_RESULT_DESERIALIZE_ERROR` | Deserialization failed. |
| 5     | `FACTORIAL_RESULT_NODE_NOT_FOUND` | The requested node ID does not exist in the graph. |
| 6     | `FACTORIAL_RESULT_EDGE_NOT_FOUND` | The requested edge ID does not exist in the graph. |
| 7     | `FACTORIAL_RESULT_INTERNAL_ERROR` | A Rust panic was caught at the FFI boundary. |
| 8     | `FACTORIAL_RESULT_POISONED` | The engine is poisoned (see below). |

A typical guard pattern in C:

```c
FactorialResult res = factorial_step(engine);
if (res != FACTORIAL_RESULT_OK) {
    fprintf(stderr, "factorial_step failed: %d\n", res);
    /* handle error */
}
```

---

## Poisoned flag

If a Rust panic occurs inside any FFI function, the panic is caught by
`catch_unwind` at the FFI boundary. The function returns
`FACTORIAL_RESULT_INTERNAL_ERROR` **and** sets an internal `poisoned` flag
on the engine. Once poisoned, **every subsequent call** on that engine
returns `FACTORIAL_RESULT_POISONED` without performing any work.

This is a safety mechanism. A panic may leave the engine's internal data
structures in an inconsistent state. Continuing to use a poisoned engine
would risk silent data corruption.

### Recovery

The recommended recovery path is:

1. Destroy the poisoned engine with `factorial_destroy()`.
2. Recreate the engine from scratch with `factorial_create()`, or
   restore from a serialized snapshot with `factorial_deserialize()`.

An escape hatch exists: `factorial_clear_poison()` resets the poisoned
flag and allows the engine to be used again. Use this only if you are
certain the panic did not corrupt state (for example, if you triggered it
intentionally during testing). You can query the poisoned status at any
time with `factorial_is_poisoned()`.

```c
if (factorial_is_poisoned(engine)) {
    /* Recommended: destroy and recreate */
    factorial_destroy(engine);
    engine = factorial_create();
}
```

---

## Pull-based events

The FFI does **not** use callbacks that cross the language boundary. No
function pointers are passed from C into Rust. Instead, the event model
is pull-based:

1. Call `factorial_step()` or `factorial_advance()`. Events generated
   during the step are buffered internally.
2. Call `factorial_poll_events()` to retrieve a pointer to the buffered
   events and their count.
3. Iterate over the returned `FfiEvent` array.

The event buffer is **owned by the engine**. It is valid until the next
call to `factorial_step()`, `factorial_advance()`, or
`factorial_destroy()`, at which point the buffer is cleared and the
pointer becomes invalid. Do not free the event buffer yourself.

```c
FfiEventBuffer events;
factorial_poll_events(engine, &events);
for (uint32_t i = 0; i < events.count; i++) {
    switch (events.events[i].kind) {
    case FFI_EVENT_KIND_ITEM_PRODUCED:
        /* handle production */
        break;
    /* ... */
    }
}
```

This design avoids the complexity and unsafety of passing function
pointers across the FFI boundary. It also gives the caller full control
over when events are processed (typically once per frame).

---

## Buffer ownership

The FFI uses two distinct buffer ownership models:

### Caller-allocated out-pointers

For scalar and small struct queries, the caller allocates the output
variable and passes a pointer:

```c
uint32_t count;
factorial_node_count(engine, &count);
/* count is written by the library, owned by the caller */
```

The same pattern applies to `FfiEventBuffer`, `FfiMutationResult`,
`FfiProcessorInfo`, and other out-parameters. The library writes into
the caller's memory.

### Library-allocated byte buffers

`factorial_serialize()` returns an `FfiByteBuffer` containing a pointer
and length. This buffer is heap-allocated by the Rust side. The caller
**must** free it with `factorial_free_buffer()` when done:

```c
FfiByteBuffer buf;
FactorialResult res = factorial_serialize(engine, &buf);
if (res == FACTORIAL_RESULT_OK) {
    /* use buf.data / buf.len */
    save_to_disk(buf.data, buf.len);
    factorial_free_buffer(buf);  /* mandatory */
}
```

Failing to call `factorial_free_buffer()` leaks memory. Calling it on a
buffer you did not receive from `factorial_serialize()` is undefined
behavior. Calling it with a null `data` pointer is safe (no-op).

---

## ID types

Node and edge identifiers are represented as plain `uint64_t` values
across the FFI boundary:

```c
typedef uint64_t FfiNodeId;
typedef uint64_t FfiEdgeId;
typedef uint64_t FfiPendingNodeId;
typedef uint64_t FfiPendingEdgeId;
```

These are **not** pointers. They are opaque integer handles derived from
the internal slotmap key representation. It is safe to:

- Store them in arrays, hash maps, or databases.
- Compare them with `==` and `!=`.
- Serialize them to disk or network.
- Pass them between frames.

It is **not** safe to:

- Assume any particular bit layout or ordering.
- Use arithmetic on them (e.g., `node_id + 1` is meaningless).
- Use an ID obtained from one engine instance with a different engine
  instance.

Pending IDs (returned by `factorial_add_node` and `factorial_connect`)
are temporary. They are valid only until `factorial_apply_mutations()`,
which returns the mapping from pending IDs to real IDs via
`FfiMutationResult`.

---

## Thread safety

The `FactorialEngine` is **not** thread-safe. All calls to a given
engine instance must occur from the same thread, or be externally
synchronized by the caller (e.g., with a mutex).

The internal event and mutation caches use thread-local storage. Calling
engine functions from multiple threads simultaneously will corrupt these
caches and produce undefined behavior.

If your game loop runs the simulation on a dedicated thread, ensure that
all `factorial_*` calls for a given engine happen on that thread. Do not
share the engine pointer across threads without synchronization.

```c
/* CORRECT: single-threaded access */
void game_tick(FactorialEngine *engine) {
    factorial_step(engine);
    FfiEventBuffer events;
    factorial_poll_events(engine, &events);
    /* process events */
}

/* WRONG: unsynchronized multi-threaded access */
// Thread A: factorial_step(engine);
// Thread B: factorial_poll_events(engine, &events);  /* data race */
```

---

## Summary of rules

1. Treat `FactorialEngine*` as opaque. Never dereference.
2. Check `FactorialResult` on every call.
3. If you receive `INTERNAL_ERROR`, the engine is poisoned. Destroy and
   recreate it.
4. Poll events after each step. The event buffer is invalidated by the
   next step or destroy.
5. Free serialization buffers with `factorial_free_buffer()`.
6. IDs are `uint64_t` values, not pointers. Safe to store and compare.
7. Access the engine from one thread only, or synchronize externally.
