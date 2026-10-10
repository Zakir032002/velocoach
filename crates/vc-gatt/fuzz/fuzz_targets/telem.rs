#![no_main]

use libfuzzer_sys::fuzz_target;
use vc_gatt::{MAX_PAYLOAD, decode_telem, encode_telem};

fuzz_target!(|data: &[u8]| {
    // Strict protocol: accepted input must re-encode to identical bytes.
    if let Ok(m) = decode_telem(data) {
        let mut buf = [0u8; MAX_PAYLOAD];
        let n = encode_telem(&m, &mut buf).expect("accepted messages always fit");

        assert_eq!(&buf[..n], data);
    }
});
