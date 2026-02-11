#![no_main]
use factorial_core::serialize::PartitionedSnapshot;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Feed arbitrary bytes to PartitionedSnapshot::from_bytes.
    // Must not panic -- returning Err is fine.
    let _ = PartitionedSnapshot::from_bytes(data);
});
