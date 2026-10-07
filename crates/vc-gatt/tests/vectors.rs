use vc_gatt::{
    Contact, CrankData, Csc, CscWheel, DecodeError, EncodeError, Hrm, MAX_PAYLOAD, SensorLocation,
    decode_csc, decode_hrm, decode_sensor_location, encode_csc, encode_hrm, encode_sensor_location,
};

fn hrm(bpm: u16, contact: Contact, energy_kj: Option<u16>, rr: &[u16]) -> Hrm {
    Hrm {
        bpm,
        contact,
        energy_kj,
        rr_1024: vc_gatt::heapless::Vec::from_slice(rr).unwrap(),
    }
}

/* -------------------------------------------------------------------------- */
/* HRM                                                                         */
/* -------------------------------------------------------------------------- */

#[test]
fn hrm_valid_vectors() {
    use Contact::*;

    let cases: &[(&[u8], Hrm)] = &[
        (&[0x00, 0x48], hrm(72, NotSupported, None, &[])),
        (&[0x02, 0x48], hrm(72, NotSupported, None, &[])),
        (&[0x04, 0x48], hrm(72, NotDetected, None, &[])),
        (&[0x06, 0x9E], hrm(158, Detected, None, &[])),
        (&[0x01, 0x48, 0x00], hrm(72, NotSupported, None, &[])),
        (&[0x01, 0x2C, 0x01], hrm(300, NotSupported, None, &[])),
        (
            &[0x08, 0x50, 0x34, 0x12],
            hrm(80, NotSupported, Some(0x1234), &[]),
        ),
        (&[0x16, 0x9E, 0x00, 0x04], hrm(158, Detected, None, &[1024])),
        (
            &[0x1F, 0x9E, 0x00, 0x10, 0x00, 0x00, 0x03, 0x00, 0x04],
            hrm(158, Detected, Some(16), &[768, 1024]),
        ),
        (&[0xE6, 0x9E], hrm(158, Detected, None, &[])),
        (&[0x10, 0x48], hrm(72, NotSupported, None, &[])),
    ];

    for (bytes, want) in cases {
        assert_eq!(decode_hrm(bytes), Ok(want.clone()), "bytes {bytes:02X?}");
    }
}

#[test]
fn hrm_error_vectors() {
    assert_eq!(decode_hrm(&[]), Err(DecodeError::Empty));

    assert_eq!(
        decode_hrm(&[0x01, 0x48]),
        Err(DecodeError::Truncated { need: 3, have: 2 })
    );

    assert_eq!(
        decode_hrm(&[0x08, 0x48, 0x10]),
        Err(DecodeError::Truncated { need: 4, have: 3 })
    );

    assert_eq!(
        decode_hrm(&[0x10, 0x48, 0x00]),
        Err(DecodeError::Truncated { need: 4, have: 3 })
    );
}

#[test]
fn hrm_truncation_sweep() {
    let full = [0x1F, 0x9E, 0x00, 0x10, 0x00, 0x00, 0x03, 0x00, 0x04];

    for n in 0..=full.len() {
        let result = decode_hrm(&full[..n]);

        match n {
            0 => {
                assert_eq!(result, Err(DecodeError::Empty));
            }

            1..=4 | 6 | 8 => {
                assert!(
                    matches!(result, Err(DecodeError::Truncated { .. })),
                    "n={n}"
                );
            }

            _ => {
                assert_eq!(result.unwrap().rr_1024.len(), (n - 5) / 2, "n={n}");
            }
        }
    }
}

#[test]
fn hrm_sim_packet_encodes_byte_exact() {
    let mut buf = [0u8; MAX_PAYLOAD];

    let n = encode_hrm(&hrm(158, Contact::Detected, None, &[1024]), &mut buf).unwrap();

    assert_eq!(&buf[..n], &[0x16, 0x9E, 0x00, 0x04]);
}

#[test]
fn hrm_keeps_oldest_rr_when_packet_is_bigger_than_default_mtu() {
    let mut bytes = vec![0x10, 0x48];

    for i in 1u16..=11 {
        bytes.extend_from_slice(&i.to_le_bytes());
    }

    let m = decode_hrm(&bytes).unwrap();

    assert_eq!(m.rr_1024.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn hrm_encode_refuses_what_does_not_fit() {
    // u16 HR + energy + 9 RR
    // = 1 + 2 + 2 + 18 = 23 bytes > 20.
    let m = hrm(
        300,
        Contact::Detected,
        Some(1),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    );

    let result = encode_hrm(&m, &mut [0u8; MAX_PAYLOAD]);

    assert!(matches!(result, Err(EncodeError::BufferTooSmall { .. })));
}

/* -------------------------------------------------------------------------- */
/* CSC                                                                         */
/* -------------------------------------------------------------------------- */

const WHEEL: CscWheel = CscWheel {
    revs: 10_000,
    time_1024: 2048,
};

const CRANK: CrankData = CrankData {
    revs: 1234,
    time_1024: 5120,
};

const BOTH: [u8; 11] = [
    0x03, 0x10, 0x27, 0x00, 0x00, 0x00, 0x08, 0xD2, 0x04, 0x00, 0x14,
];

#[test]
fn csc_valid_vectors() {
    let crank_only = Csc {
        wheel: None,
        crank: Some(CRANK),
    };

    let cases: &[(&[u8], Csc)] = &[
        (&[0x02, 0xD2, 0x04, 0x00, 0x14], crank_only),
        (
            &[0x01, 0x10, 0x27, 0x00, 0x00, 0x00, 0x08],
            Csc {
                wheel: Some(WHEEL),
                crank: None,
            },
        ),
        (
            &BOTH,
            Csc {
                wheel: Some(WHEEL),
                crank: Some(CRANK),
            },
        ),
        (&[0x00], Csc::default()),
        (&[0xFE, 0xD2, 0x04, 0x00, 0x14], crank_only),
        (
            &[0x02, 0xFF, 0xFF, 0xFF, 0xFF],
            Csc {
                wheel: None,
                crank: Some(CrankData {
                    revs: 65535,
                    time_1024: 65535,
                }),
            },
        ),
    ];

    for (bytes, want) in cases {
        assert_eq!(decode_csc(bytes), Ok(*want), "bytes {bytes:02X?}");
    }
}

#[test]
fn csc_error_vectors() {
    assert_eq!(decode_csc(&[]), Err(DecodeError::Empty));

    assert_eq!(
        decode_csc(&[0x02, 0xD2, 0x04, 0x00]),
        Err(DecodeError::Truncated { need: 5, have: 4 })
    );

    assert_eq!(
        decode_csc(&[0x01, 0x10, 0x27]),
        Err(DecodeError::Truncated { need: 5, have: 3 })
    );
}

#[test]
fn csc_truncation_sweep() {
    for n in 1..BOTH.len() {
        assert!(
            matches!(decode_csc(&BOTH[..n]), Err(DecodeError::Truncated { .. })),
            "n={n}"
        );
    }
}

#[test]
fn csc_sim_packet_encodes_byte_exact() {
    let mut buf = [0u8; MAX_PAYLOAD];

    let n = encode_csc(
        &Csc {
            wheel: None,
            crank: Some(CRANK),
        },
        &mut buf,
    )
    .unwrap();

    assert_eq!(&buf[..n], &[0x02, 0xD2, 0x04, 0x00, 0x14]);
}

/* -------------------------------------------------------------------------- */
/* Sensor Location                                                             */
/* -------------------------------------------------------------------------- */

#[test]
fn sensor_location_vectors() {
    assert_eq!(
        decode_sensor_location(&[0x05]),
        Ok(SensorLocation::LeftCrank)
    );

    assert_eq!(
        decode_sensor_location(&[0x06]),
        Ok(SensorLocation::RightCrank)
    );

    assert_eq!(
        decode_sensor_location(&[0x11]),
        Ok(SensorLocation::Reserved(17))
    );

    assert_eq!(
        decode_sensor_location(&[0x05, 0xFF]),
        Ok(SensorLocation::LeftCrank)
    );

    assert_eq!(decode_sensor_location(&[]), Err(DecodeError::Empty));
}

#[test]
fn sensor_location_encode_roundtrip() {
    let locations = [
        SensorLocation::Other,
        SensorLocation::LeftCrank,
        SensorLocation::RightCrank,
        SensorLocation::Chest,
        SensorLocation::Reserved(17),
        SensorLocation::Reserved(255),
    ];

    for location in locations {
        let mut buf = [0u8; MAX_PAYLOAD];

        let n = encode_sensor_location(location, &mut buf).unwrap();

        assert_eq!(n, 1);
        assert_eq!(buf[0], location.to_u8());

        assert_eq!(decode_sensor_location(&buf[..n]), Ok(location));
    }
}

/* -------------------------------------------------------------------------- */
/* Workspace constant                                                          */
/* -------------------------------------------------------------------------- */

#[test]
fn payload_is_mtu_minus_header() {
    assert_eq!(MAX_PAYLOAD, 23 - 3);
}
