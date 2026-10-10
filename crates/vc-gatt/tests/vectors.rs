use vc_gatt::{
    Channel, Contact, CpFeature, Cpm, CpmWheel, CrankData, Csc, CscFeature, CscWheel, Ctrl,
    DecodeError, EncodeError, FaultKind, Hrm, MAX_PAYLOAD, SensorLocation, Status, Telem,
    decode_cp_feature, decode_cpm, decode_csc, decode_csc_feature, decode_ctrl, decode_hrm,
    decode_sensor_location, decode_telem, encode_cp_feature, encode_cpm, encode_csc,
    encode_csc_feature, encode_ctrl, encode_hrm, encode_sensor_location, encode_telem,
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

// --------------------------------------------------------------------------
// CPM — Cycling Power Measurement
// --------------------------------------------------------------------------

fn cpm_case(power_w: i16) -> Cpm {
    Cpm {
        power_w,
        ..Cpm::default()
    }
}

const CPM_CRANK: CrankData = CrankData {
    revs: 1234,
    time_1024: 5000,
};

const CPM_WHEEL: CpmWheel = CpmWheel {
    revs: 10_000,
    time_2048: 4096,
};

const CPM_DOC_EXAMPLE: [u8; 8] = [0x20, 0x00, 0x38, 0x01, 0xD2, 0x04, 0x88, 0x13];

#[test]
fn cpm_valid_vectors() {
    let cases: &[(&[u8], Cpm)] = &[
        (
            &CPM_DOC_EXAMPLE,
            Cpm {
                crank: Some(CPM_CRANK),
                ..cpm_case(312)
            },
        ),
        (&[0x00, 0x00, 0x38, 0x01], cpm_case(312)),
        (&[0x00, 0x00, 0xF6, 0xFF], cpm_case(-10)),
        (
            &[
                0x21, 0x08, 0x38, 0x01, 0x64, 0xD2, 0x04, 0x88, 0x13, 0x10, 0x00,
            ],
            Cpm {
                pedal_balance_half_pct: Some(100),
                crank: Some(CPM_CRANK),
                acc_energy_kj: Some(16),
                ..cpm_case(312)
            },
        ),
        // Trap: bits 1, 3 and 12 add no bytes.
        (
            &[0x0A, 0x10, 0x38, 0x01],
            Cpm {
                balance_ref_left: true,
                torque_source_crank: true,
                offset_compensation: true,
                ..cpm_case(312)
            },
        ),
        (
            &[0x10, 0x00, 0x38, 0x01, 0x10, 0x27, 0x00, 0x00, 0x00, 0x10],
            Cpm {
                wheel: Some(CPM_WHEEL),
                ..cpm_case(312)
            },
        ),
        (
            &[
                0x34, 0x00, 0x38, 0x01, 0x40, 0x01, 0x10, 0x27, 0x00, 0x00, 0x00, 0x10, 0xD2, 0x04,
                0x88, 0x13,
            ],
            Cpm {
                acc_torque_32nm: Some(320),
                wheel: Some(CPM_WHEEL),
                crank: Some(CPM_CRANK),
                ..cpm_case(312)
            },
        ),
        (
            &[
                0x40, 0x07, 0x38, 0x01, 0x2C, 0x01, 0x9C, 0xFF, 0x5A, 0x40, 0x0B, 0x0F, 0x00, 0xB4,
                0x00,
            ],
            Cpm {
                extreme_force_raw: Some([0x2C, 0x01, 0x9C, 0xFF]),
                extreme_angles_raw: Some([0x5A, 0x40, 0x0B]),
                top_dead_spot_deg: Some(15),
                bottom_dead_spot_deg: Some(180),
                ..cpm_case(312)
            },
        ),
        // Reserved bits 13–15 are ignored.
        (&[0x00, 0xE0, 0x38, 0x01], cpm_case(312)),
    ];

    assert_eq!(cases.len(), 9);

    for (bytes, expected) in cases {
        assert_eq!(decode_cpm(bytes), Ok(*expected), "bytes {bytes:02X?}");
    }
}

#[test]
fn cpm_error_vectors() {
    assert_eq!(decode_cpm(&[]), Err(DecodeError::Empty));

    assert_eq!(
        decode_cpm(&[0x00]),
        Err(DecodeError::Truncated { need: 2, have: 1 })
    );

    assert_eq!(
        decode_cpm(&[0x00, 0x00, 0x38]),
        Err(DecodeError::Truncated { need: 4, have: 3 })
    );

    assert_eq!(
        decode_cpm(&CPM_DOC_EXAMPLE[..7]),
        Err(DecodeError::Truncated { need: 8, have: 7 })
    );
}

#[test]
fn cpm_truncation_sweep() {
    for n in 1..CPM_DOC_EXAMPLE.len() {
        assert!(
            matches!(
                decode_cpm(&CPM_DOC_EXAMPLE[..n]),
                Err(DecodeError::Truncated { .. })
            ),
            "n={n}"
        );
    }
}

#[test]
fn cpm_sim_packet_encodes_byte_exact() {
    let mut buf = [0u8; MAX_PAYLOAD];

    let n = encode_cpm(
        &Cpm {
            crank: Some(CPM_CRANK),
            ..cpm_case(312)
        },
        &mut buf,
    )
    .unwrap();

    assert_eq!(&buf[..n], &CPM_DOC_EXAMPLE);
}

#[test]
fn cpm_every_field_present_in_34_byte_packet() {
    let mut bytes = vec![0xFF, 0x1F, 0x38, 0x01];
    bytes.extend_from_slice(&[0xAB; 30]);

    assert_eq!(bytes.len(), 34);

    let message = decode_cpm(&bytes).unwrap();

    assert_eq!(message.acc_energy_kj, Some(0xABAB));
    assert!(message.balance_ref_left);
    assert!(message.torque_source_crank);
    assert!(message.offset_compensation);

    assert!(matches!(
        encode_cpm(&message, &mut [0u8; MAX_PAYLOAD]),
        Err(EncodeError::BufferTooSmall { .. })
    ));
}

// --------------------------------------------------------------------------
// CP Feature and CSC Feature
// --------------------------------------------------------------------------

#[test]
fn feature_valid_vectors_and_byte_exact_encoding() {
    assert_eq!(
        decode_cp_feature(&[0x89, 0x00, 0x00, 0x00]),
        Ok(CpFeature::SIM)
    );

    assert!(CpFeature::SIM.supports(CpFeature::CRANK_REV));
    assert!(!CpFeature::SIM.supports(CpFeature::WHEEL_REV));

    assert_eq!(decode_csc_feature(&[0x02, 0x00]), Ok(CscFeature::SIM));

    let mut buf = [0u8; MAX_PAYLOAD];

    let n = encode_cp_feature(CpFeature::SIM, &mut buf).unwrap();
    assert_eq!(&buf[..n], &[0x89, 0x00, 0x00, 0x00]);

    let n = encode_csc_feature(CscFeature::SIM, &mut buf).unwrap();
    assert_eq!(&buf[..n], &[0x02, 0x00]);
}

#[test]
fn feature_short_reads_fail() {
    assert_eq!(decode_cp_feature(&[]), Err(DecodeError::Empty));
    assert_eq!(decode_csc_feature(&[]), Err(DecodeError::Empty));

    assert_eq!(
        decode_cp_feature(&[0x89, 0x00, 0x00]),
        Err(DecodeError::Truncated { need: 4, have: 3 })
    );

    assert_eq!(
        decode_csc_feature(&[0x02]),
        Err(DecodeError::Truncated { need: 2, have: 1 })
    );
}

// --------------------------------------------------------------------------
// VC-SIM — TELEM and CTRL
// --------------------------------------------------------------------------

fn telem_bytes(message: &Telem) -> Vec<u8> {
    let mut buf = [0u8; MAX_PAYLOAD];
    let n = encode_telem(message, &mut buf).unwrap();
    buf[..n].to_vec()
}

fn ctrl_bytes(message: &Ctrl) -> Vec<u8> {
    let mut buf = [0u8; MAX_PAYLOAD];
    let n = encode_ctrl(message, &mut buf).unwrap();
    buf[..n].to_vec()
}

#[test]
fn vcsim_stamp_bytes_exact() {
    let message = Telem::Stamp {
        seq: 0x0102,
        channel: Channel::Cpm,
        char_seq: 0x0304,
        t0_ns: 0x1122_3344_5566_7788,
    };

    let expected = [
        0x01, 0x01, 0x02, 0x01, 0x02, 0x04, 0x03, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11,
    ];

    assert_eq!(telem_bytes(&message), expected);
    assert_eq!(decode_telem(&expected), Ok(message));
}

#[test]
fn vcsim_fault_bytes_exact() {
    let message = Ctrl::Fault {
        kind: FaultKind::Dropout,
        channel: Some(Channel::Hrm),
        dur_ms: 12_000,
        param: 0,
    };

    let expected = [0x01, 0x30, 0x01, 0x01, 0xE0, 0x2E, 0x00, 0x00, 0x00, 0x00];

    assert_eq!(ctrl_bytes(&message), expected);
    assert_eq!(decode_ctrl(&expected), Ok(message));
}

#[test]
fn vcsim_all_message_sizes_match_spec() {
    let status = Status {
        seq: 0,
        scenario: 0,
        state: 0,
        faults: 0,
        q_drops: 0,
        notify_errs: 0,
    };

    assert_eq!(
        telem_bytes(&Telem::Stamp {
            seq: 0,
            channel: Channel::Hrm,
            char_seq: 0,
            t0_ns: 0,
        })
        .len(),
        15
    );

    assert_eq!(
        telem_bytes(&Telem::Pong {
            ping_id: 0,
            t_rx_ns: 0,
            t_tx_ns: 0,
        })
        .len(),
        20
    );

    assert_eq!(telem_bytes(&Telem::Status(status)).len(), 12);

    assert_eq!(
        ctrl_bytes(&Ctrl::Ping {
            ping_id: 0,
            t_client_ns: 0,
        })
        .len(),
        12
    );

    assert_eq!(
        ctrl_bytes(&Ctrl::Load {
            scenario: 0,
            seed: 0,
        })
        .len(),
        7
    );

    assert_eq!(ctrl_bytes(&Ctrl::Start).len(), 2);
    assert_eq!(ctrl_bytes(&Ctrl::Stop).len(), 2);

    assert_eq!(
        ctrl_bytes(&Ctrl::Fault {
            kind: FaultKind::Coast,
            channel: None,
            dur_ms: 0,
            param: 0,
        })
        .len(),
        10
    );

    assert_eq!(ctrl_bytes(&Ctrl::Clear { kind: 0xFF }).len(), 3);

    assert_eq!(
        ctrl_bytes(&Ctrl::Rate {
            channel: Some(Channel::Cpm),
            hz_x10: 40,
        })
        .len(),
        4
    );
}

#[test]
fn vcsim_rejects_bad_input() {
    use DecodeError::*;

    assert_eq!(decode_ctrl(&[]), Err(Empty));

    assert_eq!(decode_ctrl(&[0x01]), Err(Truncated { need: 2, have: 1 }));

    assert_eq!(decode_ctrl(&[0x02, 0x21]), Err(BadVersion(2)));

    assert_eq!(decode_ctrl(&[0x01, 0x99]), Err(BadOpcode(0x99)));

    // A CTRL opcode received on TELEM, and vice versa.
    assert_eq!(decode_telem(&[0x01, 0x21]), Err(BadOpcode(0x21)));

    assert_eq!(decode_ctrl(&[0x01, 0x01]), Err(BadOpcode(0x01)));

    assert_eq!(
        decode_ctrl(&[0x01, 0x21, 0x00]),
        Err(TooLong {
            expected: 2,
            have: 3
        })
    );

    assert_eq!(
        decode_ctrl(&[0x01, 0x30, 0x09, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,]),
        Err(BadValue {
            field: "fault.kind",
            value: 9,
        })
    );

    let mut stamp = [0u8; 15];
    stamp[..2].copy_from_slice(&[0x01, 0x01]);
    stamp[4] = 7;

    assert_eq!(
        decode_telem(&stamp),
        Err(BadValue {
            field: "stamp.channel",
            value: 7,
        })
    );
}
