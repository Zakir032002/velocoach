use proptest::prelude::*;
use vc_gatt::*;

fn arb_contact() -> impl Strategy<Value = Contact> {
    prop_oneof![
        Just(Contact::NotSupported),
        Just(Contact::NotDetected),
        Just(Contact::Detected),
    ]
}

fn arb_hrm() -> impl Strategy<Value = Hrm> {
    (
        any::<u16>(),
        arb_contact(),
        proptest::option::of(any::<u16>()),
        proptest::collection::vec(any::<u16>(), 0..=RR_CAP),
    )
        .prop_map(|(bpm, contact, energy_kj, rr)| Hrm {
            bpm,
            contact,
            energy_kj,
            rr_1024: heapless::Vec::from_slice(&rr).unwrap(),
        })
}

fn arb_csc() -> impl Strategy<Value = Csc> {
    (
        proptest::option::of((any::<u32>(), any::<u16>())),
        proptest::option::of((any::<u16>(), any::<u16>())),
    )
        .prop_map(|(w, c)| Csc {
            wheel: w.map(|(revs, time_1024)| CscWheel { revs, time_1024 }),
            crank: c.map(|(revs, time_1024)| CrankData { revs, time_1024 }),
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// decode(encode(x)) == x, or encode refuses because x needs > 20 bytes.
    #[test]
    fn hrm_roundtrip(m in arb_hrm()) {
        let mut buf = [0u8; MAX_PAYLOAD];
        match encode_hrm(&m, &mut buf) {
            Ok(n) => { prop_assert_eq!(decode_hrm(&buf[..n]).unwrap(), m); }
            Err(EncodeError::BufferTooSmall { need, .. }) => { prop_assert!(need > MAX_PAYLOAD); }
        }
    }

    /// Anything we accept off the air re-encodes canonically, never longer.
    #[test]
    fn hrm_reencode_is_canonical(b in proptest::collection::vec(any::<u8>(), 0..=MAX_PAYLOAD)) {
        if let Ok(m) = decode_hrm(&b) {
            let mut buf = [0u8; MAX_PAYLOAD];
            let n = encode_hrm(&m, &mut buf).unwrap();
            prop_assert!(n <= b.len());
            prop_assert_eq!(decode_hrm(&buf[..n]).unwrap(), m);
        }
    }

    #[test]
    fn csc_roundtrip(m in arb_csc()) {
        let mut buf = [0u8; MAX_PAYLOAD];
        let n = encode_csc(&m, &mut buf).unwrap(); // at most 11 bytes
        prop_assert_eq!(decode_csc(&buf[..n]).unwrap(), m);
    }

    /// Byte-level round trip: Reserved(5) is not a thing, 5 is LeftCrank.
    #[test]
    fn location_byte_roundtrip(v in any::<u8>()) {
        prop_assert_eq!(SensorLocation::from_u8(v).to_u8(), v);
    }

    /// Garbage of any length up to 64 bytes: errors are fine, panics are not.
    #[test]
    fn decoders_never_panic(b in proptest::collection::vec(any::<u8>(), 0..64)) {
        let _ = decode_hrm(&b);
        let _ = decode_csc(&b);
        let _ = decode_sensor_location(&b);
    }
}
