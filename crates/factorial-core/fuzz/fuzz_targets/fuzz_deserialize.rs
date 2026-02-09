#![no_main]
use factorial_core::engine::Engine;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Feed arbitrary bytes to Engine::deserialize.
    // Must not panic -- returning Err is fine.
    let _ = Engine::deserialize(data);
});
