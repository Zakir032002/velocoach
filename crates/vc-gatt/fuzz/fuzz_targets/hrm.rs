#![no_main]

use libfuzzer_sys::fuzz_target;
use vc_gatt::{MAX_PAYLOAD, decode_hrm, encode_hrm};

fuzz_target!(|data: &[u8]| {
    // Anything accepted that fits in 20 bytes must survive a round trip.
    if let Ok(m) = decode_hrm(data) {
        let mut buf = [0u8; MAX_PAYLOAD];

        if let Ok(n) = encode_hrm(&m, &mut buf) {
            assert_eq!(decode_hrm(&buf[..n]), Ok(m));
        }
    }
});
