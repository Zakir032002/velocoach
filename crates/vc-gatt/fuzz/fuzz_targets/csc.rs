#![no_main]

use libfuzzer_sys::fuzz_target;
use vc_gatt::{MAX_PAYLOAD, decode_csc, encode_csc};

fuzz_target!(|data: &[u8]| {
    // Anything accepted that fits in 20 bytes must survive a round trip.
    if let Ok(m) = decode_csc(data) {
        let mut buf = [0u8; MAX_PAYLOAD];

        if let Ok(n) = encode_csc(&m, &mut buf) {
            assert_eq!(decode_csc(&buf[..n]), Ok(m));
        }
    }
});
