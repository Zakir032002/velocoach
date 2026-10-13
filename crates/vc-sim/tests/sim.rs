#![allow(clippy::unwrap_used)]

use heapless::Vec;
use vc_gatt::{Channel, Ctrl, MAX_PAYLOAD, decode_cpm, decode_hrm, encode_ctrl};
use vc_sim::sim::MAX_OUT;
use vc_sim::{Outcome, SimPacket, Simulator};

const S: u64 = 1_000_000_000;
const MS: u64 = 1_000_000;

fn ctrl(m: Ctrl) -> std::vec::Vec<u8> {
    let mut b = [0u8; MAX_PAYLOAD];
    let n = encode_ctrl(&m, &mut b).unwrap();
    b[..n].to_vec()
}

fn cue(id: u32, arm: u8, delta: i16) -> std::vec::Vec<u8> {
    ctrl(Ctrl::CuePlayed {
        cue_id: id,
        kind: 1,
        dir: -1,
        delta_w: delta,
        arm,
        starts_in_ms: 50,
        dur_ms: 2_400,
    })
}

/// Step every 50 ms over [from, to); every packet with the truth at that moment.
fn run(sim: &mut Simulator, from: u64, to: u64, mut each: impl FnMut(&SimPacket, &vc_sim::Truth)) {
    let (mut out, mut t) = (Vec::<SimPacket, MAX_OUT>::new(), from);
    while t < to {
        sim.step_into(t, &mut out);
        if let Some(truth) = sim.truth() {
            out.iter().for_each(|p| each(p, &truth));
        }
        t += 50 * MS;
    }
}

fn started(id: u8, persona: u8) -> Simulator {
    let mut s = Simulator::new(id, persona, None, false).unwrap();
    s.start(0);
    s
}

#[test]
fn decoded_packets_equal_the_truth_at_the_real_rates() {
    let mut sim = started(1, 0);
    let (mut hr, mut cpm, mut csc) = (0, 0, 0);
    run(&mut sim, 0, 60 * S, |p, t| match p.channel {
        Channel::Hrm => {
            hr += 1;
            assert_eq!(decode_hrm(&p.bytes).unwrap().bpm, t.hr_reported);
        }
        Channel::Cpm => {
            cpm += 1;
            assert_eq!(
                decode_cpm(&p.bytes).unwrap().power_w,
                vc_sim::packets::watts_i16(t.power_w)
            );
        }
        Channel::Csc => csc += 1,
    });
    assert_eq!((hr, cpm, csc), (59, 239, 60)); // 1 Hz, 4 Hz, 1 Hz over 59.95 s
}

#[test]
fn dana_complies_and_the_power_shows_it() {
    let mut sim = started(1, 0); // climb_surge, Dana; interval 1 at 320 W from 300 s
    run(&mut sim, 0, 400 * S, |_, _| {});
    sim.apply_ctrl(400 * S, &cue(1, 1, 70)).unwrap();
    run(&mut sim, 400 * S, 420 * S, |_, _| {});
    let r = sim.responses()[0];
    let target = sim.truth().unwrap().target_w;
    match r.outcome {
        Outcome::Complied => {
            assert_eq!(target, 250.0);
            let applied = r.applied_ns.unwrap();
            let due = r.audio_end_ns + u64::from(r.latency_ms.unwrap()) * MS;
            assert!(
                applied >= due.saturating_sub(MS) && applied < due + 250 * MS + MS,
                "{applied} vs {due}"
            );
        }
        Outcome::Ignored => assert_eq!(target, 320.0),
        other => unreachable!("{other:?}"),
    }
}

#[test]
fn reacting_never_shifts_the_ride() {
    let (mut a, mut b) = (started(6, 0), started(6, 0));
    let (mut pa, mut pb) = (std::vec::Vec::new(), std::vec::Vec::new());
    run(&mut a, 0, 300 * S, |p, _| pa.push(p.clone()));
    run(&mut b, 0, 300 * S, |p, _| pb.push(p.clone()));
    a.apply_ctrl(300 * S, &cue(1, 0, 40)).unwrap(); // arm none: a P1 cue, not modelled
    run(&mut a, 300 * S, 900 * S, |p, _| pa.push(p.clone()));
    run(&mut b, 300 * S, 900 * S, |p, _| pb.push(p.clone()));
    assert_eq!(pa, pb);
}

#[test]
fn human_mode_follows_the_slider_and_ignores_the_persona() {
    let mut sim = Simulator::new(6, 1, None, false).unwrap();
    sim.apply_ctrl(
        0,
        &ctrl(Ctrl::Load {
            scenario: 6,
            persona: 1,
            seed: 7,
            human: true,
        }),
    )
    .unwrap();
    sim.set_human_effort(800); // 0.8 x CP = 200 W
    sim.start(0);
    run(&mut sim, 0, 10 * S, |_, _| {});
    assert_eq!(sim.truth().unwrap().target_w, 200.0); // warm-up: no offset or creep
    sim.apply_ctrl(10 * S, &cue(1, 1, 40)).unwrap();
    assert_eq!(sim.responses()[0].outcome, Outcome::NotApplicable);
}

#[test]
fn same_seed_and_cues_give_identical_rides_and_records() {
    let ride = || {
        let mut sim = started(6, 0);
        let mut packets = std::vec::Vec::new();
        for k in 0..20u64 {
            let t = k * 60 * S;
            run(&mut sim, t, t + 60 * S, |p, _| packets.push(p.clone()));
            sim.apply_ctrl(t + 60 * S, &cue(k as u32 + 1, 1 + (k % 2) as u8, 12))
                .unwrap();
        }
        (packets, sim.responses().to_vec())
    };
    assert_eq!(ride(), ride());
}
