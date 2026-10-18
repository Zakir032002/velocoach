//! The README's protocol appendix is checked against the encoders: the code is the truth.
//! Every row (`| \`NAME\` | \`0xNN\` | N B | ...`) must name a message whose encoding starts
//! with version 0x01, has that opcode in byte 1 and is exactly N bytes long, and every message
//! must have a row.
use vc_gatt::MAX_PAYLOAD;
use vc_gatt::{Channel, Ctrl, FaultKind, Status, Telem, VCSIM_VERSION, encode_ctrl, encode_telem};

fn encoded() -> Vec<(&'static str, Vec<u8>)> {
    let telem = |m: Telem| {
        let mut b = [0u8; MAX_PAYLOAD];
        let n = encode_telem(&m, &mut b).expect("telem encodes");
        b[..n].to_vec()
    };
    let ctrl = |m: Ctrl| {
        let mut b = [0u8; MAX_PAYLOAD];
        let n = encode_ctrl(&m, &mut b).expect("ctrl encodes");
        b[..n].to_vec()
    };
    vec![
        (
            "STAMP",
            telem(Telem::Stamp {
                seq: 1,
                channel: Channel::Cpm,
                char_seq: 2,
                t0_ns: 3,
            }),
        ),
        (
            "PONG",
            telem(Telem::Pong {
                ping_id: 1,
                t_rx_ns: 2,
                t_tx_ns: 3,
            }),
        ),
        (
            "STATUS",
            telem(Telem::Status(Status {
                seq: 1,
                scenario: 1,
                state: 0,
                faults: 0,
                q_drops: 0,
                notify_errs: 0,
            })),
        ),
        (
            "RIDER_INPUT",
            telem(Telem::RiderInput {
                seq: 1,
                t_ns: 2,
                effort_permille: 700,
            }),
        ),
        (
            "PING",
            ctrl(Ctrl::Ping {
                ping_id: 1,
                t_client_ns: 2,
            }),
        ),
        (
            "LOAD",
            ctrl(Ctrl::Load {
                scenario: 1,
                persona: 0,
                seed: 7,
                human: false,
            }),
        ),
        ("START", ctrl(Ctrl::Start)),
        ("STOP", ctrl(Ctrl::Stop)),
        (
            "FAULT",
            ctrl(Ctrl::Fault {
                kind: FaultKind::Dropout,
                channel: Some(Channel::Cpm),
                dur_ms: 5_000,
                param: 0,
            }),
        ),
        ("CLEAR", ctrl(Ctrl::Clear { kind: 0xFF })),
        (
            "RATE",
            ctrl(Ctrl::Rate {
                channel: None,
                hz_x10: 40,
            }),
        ),
        (
            "CUE_PLAYED",
            ctrl(Ctrl::CuePlayed {
                cue_id: 1,
                kind: 1,
                dir: -1,
                delta_w: 13,
                arm: 1,
                starts_in_ms: 50,
                dur_ms: 2_400,
            }),
        ),
    ]
}

/// (name, opcode, size) for every protocol row of the README.
fn readme_rows() -> Vec<(String, u8, usize)> {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"))
        .expect("README.md");
    readme
        .lines()
        .filter_map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            let name = cells.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
            let op = cells.get(2)?.strip_prefix("`0x")?.strip_suffix('`')?;
            let size = cells.get(3)?.strip_suffix(" B")?;
            Some((
                name.to_string(),
                u8::from_str_radix(op, 16).ok()?,
                size.parse().ok()?,
            ))
        })
        .collect()
}

#[test]
fn the_readme_protocol_table_matches_the_encoders() {
    let rows = readme_rows();
    let msgs = encoded();
    assert_eq!(rows.len(), msgs.len(), "README rows: {rows:?}");
    for (name, bytes) in &msgs {
        let (_, op, size) = rows
            .iter()
            .find(|r| r.0 == *name)
            .unwrap_or_else(|| panic!("README has no row for {name}"));
        assert_eq!(bytes[0], VCSIM_VERSION, "{name}: version byte");
        assert_eq!(
            bytes[1], *op,
            "{name}: README opcode 0x{op:02X}, code 0x{:02X}",
            bytes[1]
        );
        assert_eq!(
            bytes.len(),
            *size,
            "{name}: README {size} B, code {} B",
            bytes.len()
        );
    }
}

#[test]
fn stamp_is_15_bytes_and_rate_is_0x40() {
    let msgs = encoded();
    let get = |n: &str| &msgs.iter().find(|m| m.0 == n).unwrap().1;
    assert_eq!(get("STAMP").len(), 15);
    assert_eq!(get("RATE")[1], 0x40);
}
