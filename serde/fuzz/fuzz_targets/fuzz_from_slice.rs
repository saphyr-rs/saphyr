#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() <= 10240 {
        _ = saphyr_serde::from_slice::<saphyr_serde::Value>(data);
    }
});
