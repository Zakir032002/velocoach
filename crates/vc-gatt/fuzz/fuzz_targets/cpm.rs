#![no_main]

use libfuzzer_sys::fuzz_target;
use vc_gatt::{MAX_PAYLOAD, decode_cpm, encode_cpm};

fuzz_target!(|data: &[u8]| {
    // Anything accepted that fits in 20 bytes must survive a round trip.
    if let Ok(m) = decode_cpm(data) {
        let mut buf = [0u8; MAX_PAYLOAD];

        if let Ok(n) = encode_cpm(&m, &mut buf) {
            assert_eq!(decode_cpm(&buf[..n]), Ok(m));
        }
    }
});
