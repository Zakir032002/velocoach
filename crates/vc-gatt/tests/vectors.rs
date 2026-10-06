use proptest::prelude::*;
use vc_gatt::{
    Contact, Csc, DecodeError, Hrm, MAX_RR, PAYLOAD_MAX, SensorLocation, decode_csc, decode_hrm,
    decode_sensor_location, encode_csc, encode_hrm, rr_capacity,
};
fn hrm(bpm: u16, contact: Contact, energy_kj: Option<u16>, rr: &[u16]) -> Hrm {
    let mut h = Hrm {
        bpm,
        contact,
        energy_kj,
        ..Hrm::default()
    };

    for (slot, v) in h.rr.iter_mut().zip(rr) {
        *slot = *v;
    }

    h.rr_len = rr.len() as u8;
    h
}

fn hrm_cases() -> Vec<(Vec<u8>, Hrm)> {
    vec![
        // 158 bpm, no contact/energy/RR.
        (vec![0x00, 0x9E], hrm(158, Contact::NotSupported, None, &[])),
        // Simulator packet:
        // 158 bpm, contact detected, RR = 1024 = 1.0 s.
        (
            vec![0x16, 0x9E, 0x00, 0x04],
            hrm(158, Contact::Contact, None, &[1024]),
        ),
        // 300 bpm using u16 heart-rate format.
        (
            vec![0x01, 0x2C, 0x01],
            hrm(300, Contact::NotSupported, None, &[]),
        ),
        // 255 bpm using u16 heart-rate format.
        (
            vec![0x01, 0xFF, 0x00],
            hrm(255, Contact::NotSupported, None, &[]),
        ),
        // 120 bpm + 16 kJ energy.
        (
            vec![0x08, 0x78, 0x10, 0x00],
            hrm(120, Contact::NotSupported, Some(16), &[]),
        ),
        // 100 bpm + 5 kJ + RR 768, 896.
        (
            vec![0x18, 0x64, 0x05, 0x00, 0x00, 0x03, 0x80, 0x03],
            hrm(100, Contact::NotSupported, Some(5), &[768, 896]),
        ),
        // NoContact.
        (vec![0x04, 0x00], hrm(0, Contact::NoContact, None, &[])),
        // Contact value 1 => NotSupported.
        (vec![0x02, 0x50], hrm(80, Contact::NotSupported, None, &[])),
        // Reserved flag bits ignored.
        (vec![0xE0, 0x46], hrm(70, Contact::NotSupported, None, &[])),
        // 300 bpm + RR 1024.
        (
            vec![0x11, 0x2C, 0x01, 0x00, 0x04],
            hrm(300, Contact::NotSupported, None, &[1024]),
        ),
    ]
}

#[test]
fn hrm_vectors() {
    for (bytes, want) in hrm_cases() {
        assert_eq!(decode_hrm(&bytes), Ok(want), "bytes {bytes:02X?}");
    }
}

#[test]
fn encoders_match_known_bytes() {
    let mut buf = [0u8; PAYLOAD_MAX];

    let n = encode_hrm(&hrm(158, Contact::Contact, None, &[1024]), &mut buf);

    assert_eq!(&buf[..n], &[0x16, 0x9E, 0x00, 0x04]);
}

fn csc_cases() -> Vec<(Vec<u8>, Csc)> {
    vec![
        // Crank only:
        // 1234 revolutions, event time 5000.
        (
            vec![0x02, 0xD2, 0x04, 0x88, 0x13],
            Csc {
                wheel: None,
                crank: Some((1234, 5000)),
            },
        ),
        // Wheel only:
        // 0x12345678 revolutions, event time 1024.
        (
            vec![0x01, 0x78, 0x56, 0x34, 0x12, 0x00, 0x04],
            Csc {
                wheel: Some((0x1234_5678, 1024)),
                crank: None,
            },
        ),
        // Wheel + crank:
        // wheel: 1 rev, time 1024
        // crank: 10 revs, time 2048
        //
        // 0x0800 = 2048, little-endian => 00 08.
        (
            vec![
                0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x0A, 0x00, 0x00, 0x08,
            ],
            Csc {
                wheel: Some((1, 1024)),
                crank: Some((10, 2048)),
            },
        ),
    ]
}

#[test]
fn csc_vectors() {
    for (bytes, want) in csc_cases() {
        assert_eq!(decode_csc(&bytes), Ok(want), "bytes {bytes:02X?}");
    }
}

#[test]
fn csc_encoder_matches_known_bytes() {
    let mut buf = [0u8; PAYLOAD_MAX];

    let csc = Csc {
        wheel: None,
        crank: Some((1234, 5000)),
    };

    let n = encode_csc(&csc, &mut buf);

    assert_eq!(&buf[..n], &[0x02, 0xD2, 0x04, 0x88, 0x13]);
}

fn contact() -> impl Strategy<Value = Contact> {
    prop_oneof![
        Just(Contact::NotSupported),
        Just(Contact::NoContact),
        Just(Contact::Contact),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    #[test]
    fn hrm_round_trip(
        bpm in any::<u16>(),
        c in contact(),
        energy in any::<Option<u16>>(),
        rr in proptest::collection::vec(any::<u16>(), 0..=MAX_RR),
    ) {
        let m = hrm(bpm, c, energy, &rr);

        let mut buf = [0u8; PAYLOAD_MAX];

        let n = encode_hrm(&m, &mut buf);

        prop_assert!(n <= PAYLOAD_MAX);

        let d = decode_hrm(&buf[..n]).expect("own output must decode");

        let keep = rr
            .len()
            .min(rr_capacity(bpm > 255, energy.is_some()));

        prop_assert_eq!(
            d,
            hrm(bpm, c, energy, &rr[..keep])
        );
    }

    #[test]
    fn csc_round_trip(
        wheel in proptest::option::of(any::<(u32, u16)>()),
        crank in proptest::option::of(any::<(u16, u16)>()),
    ) {
        let m = Csc { wheel, crank };

        let mut buf = [0u8; PAYLOAD_MAX];

        let n = encode_csc(&m, &mut buf);

        prop_assert_eq!(decode_csc(&buf[..n]), Ok(m));
    }

    #[test]
    fn decoders_never_panic(
        b in proptest::collection::vec(any::<u8>(), 0..64)
    ) {
        let _ = decode_hrm(&b);
        let _ = decode_csc(&b);
    }
}

#[test]
fn sensor_location() {
    assert_eq!(decode_sensor_location(&[5]), Ok(SensorLocation::LEFT_CRANK));

    assert!(
        decode_sensor_location(&[0x20])
            .expect("one byte")
            .is_reserved()
    );

    assert_eq!(decode_sensor_location(&[]), Err(DecodeError::Empty));
}
