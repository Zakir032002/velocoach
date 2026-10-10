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

fn arb_cpm() -> impl Strategy<Value = Cpm> {
    use proptest::option::of;

    let head = (
        any::<i16>(),
        of(any::<u8>()),
        any::<bool>(),
        of(any::<u16>()),
        any::<bool>(),
        of((any::<u32>(), any::<u16>())),
        of((any::<u16>(), any::<u16>())),
    );

    let tail = (
        of(any::<[u8; 4]>()),
        of(any::<[u8; 4]>()),
        of(any::<[u8; 3]>()),
        of(any::<u16>()),
        of(any::<u16>()),
        of(any::<u16>()),
        any::<bool>(),
    );

    (head, tail).prop_map(
        |(
            (power_w, bal, bal_left, torque, torque_crank, wheel, crank),
            (force, torque_x, angles, tds, bds, energy, offset),
        )| Cpm {
            power_w,
            pedal_balance_half_pct: bal,
            balance_ref_left: bal_left,
            acc_torque_32nm: torque,
            torque_source_crank: torque_crank,
            wheel: wheel.map(|(revs, time_2048)| CpmWheel { revs, time_2048 }),
            crank: crank.map(|(revs, time_1024)| CrankData { revs, time_1024 }),
            extreme_force_raw: force,
            extreme_torque_raw: torque_x,
            extreme_angles_raw: angles,
            top_dead_spot_deg: tds,
            bottom_dead_spot_deg: bds,
            acc_energy_kj: energy,
            offset_compensation: offset,
        },
    )
}

fn arb_channel() -> impl Strategy<Value = Channel> {
    prop_oneof![Just(Channel::Hrm), Just(Channel::Cpm), Just(Channel::Csc),]
}

fn arb_fault_kind() -> impl Strategy<Value = FaultKind> {
    prop_oneof![
        Just(FaultKind::Dropout),
        Just(FaultKind::Garbage),
        Just(FaultKind::Disconnect),
        Just(FaultKind::Coast),
        Just(FaultKind::HrRedline),
        Just(FaultKind::WprimeMismatch),
    ]
}

fn arb_telem() -> impl Strategy<Value = Telem> {
    prop_oneof![
        (any::<u16>(), arb_channel(), any::<u16>(), any::<u64>(),).prop_map(
            |(seq, channel, char_seq, t0_ns)| Telem::Stamp {
                seq,
                channel,
                char_seq,
                t0_ns,
            }
        ),
        (any::<u16>(), any::<u64>(), any::<u64>(),).prop_map(|(ping_id, t_rx_ns, t_tx_ns)| {
            Telem::Pong {
                ping_id,
                t_rx_ns,
                t_tx_ns,
            }
        }),
        (
            any::<u16>(),
            any::<u8>(),
            any::<u8>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
        )
            .prop_map(|(seq, scenario, state, faults, q_drops, notify_errs)| {
                Telem::Status(Status {
                    seq,
                    scenario,
                    state,
                    faults,
                    q_drops,
                    notify_errs,
                })
            },),
    ]
}

fn arb_ctrl() -> impl Strategy<Value = Ctrl> {
    prop_oneof![
        (any::<u16>(), any::<u64>()).prop_map(|(ping_id, t_client_ns)| Ctrl::Ping {
            ping_id,
            t_client_ns,
        }),
        (any::<u8>(), any::<u32>()).prop_map(|(scenario, seed)| Ctrl::Load { scenario, seed }),
        Just(Ctrl::Start),
        Just(Ctrl::Stop),
        (
            arb_fault_kind(),
            proptest::option::of(arb_channel()),
            any::<u32>(),
            any::<i16>(),
        )
            .prop_map(|(kind, channel, dur_ms, param)| Ctrl::Fault {
                kind,
                channel,
                dur_ms,
                param,
            }),
        any::<u8>().prop_map(|kind| Ctrl::Clear { kind }),
        (proptest::option::of(arb_channel()), any::<u8>())
            .prop_map(|(channel, hz_x10)| Ctrl::Rate { channel, hz_x10 }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    #[test]
    fn cpm_roundtrip(m in arb_cpm()) {
        let mut buf = [0u8; MAX_PAYLOAD];

        match encode_cpm(&m, &mut buf) {
            Ok(n) => {
                prop_assert_eq!(decode_cpm(&buf[..n]).unwrap(), m);
            }
            Err(EncodeError::BufferTooSmall { need, .. }) => {
                prop_assert!(need > MAX_PAYLOAD);
            }
        }
    }

    #[test]
    fn telem_roundtrip(m in arb_telem()) {
        let mut buf = [0u8; MAX_PAYLOAD];
        let n = encode_telem(&m, &mut buf).unwrap();

        prop_assert_eq!(decode_telem(&buf[..n]).unwrap(), m);
    }

    #[test]
    fn ctrl_roundtrip(m in arb_ctrl()) {
        let mut buf = [0u8; MAX_PAYLOAD];
        let n = encode_ctrl(&m, &mut buf).unwrap();

        prop_assert_eq!(decode_ctrl(&buf[..n]).unwrap(), m);
    }

    /// Strict protocol: anything accepted re-encodes to identical bytes.
    #[test]
    fn vcsim_accepted_bytes_are_canonical(
        b in proptest::collection::vec(any::<u8>(), 0..=MAX_PAYLOAD)
    ) {
        let mut buf = [0u8; MAX_PAYLOAD];

        if let Ok(m) = decode_telem(&b) {
            let n = encode_telem(&m, &mut buf).unwrap();
            prop_assert_eq!(&buf[..n], &b[..]);
        }

        if let Ok(m) = decode_ctrl(&b) {
            let n = encode_ctrl(&m, &mut buf).unwrap();
            prop_assert_eq!(&buf[..n], &b[..]);
        }
    }

    #[test]
    fn new_decoders_never_panic(
        b in proptest::collection::vec(any::<u8>(), 0..64)
    ) {
        let _ = decode_cpm(&b);
        let _ = decode_cp_feature(&b);
        let _ = decode_csc_feature(&b);
        let _ = decode_telem(&b);
        let _ = decode_ctrl(&b);
    }
}
