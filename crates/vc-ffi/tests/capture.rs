use vc_ffi::{Engine, EngineConfig, Profile, Source};
use vc_sim::Simulator;
use vc_sim::sim::MAX_OUT;

#[test]
fn a_captured_ride_replays_identically() {
    let cfg = EngineConfig {
        cp_w: 250.0,
        wprime_j: 20_000.0,
        hr_max: 190.0,
        profile: Profile::Solo,
        cooldown_between_alerts_s: 20.0,
        prompt_surge_countdown_s: 5.0,
        steps: vec![],
    };
    let e = Engine::for_scenario(cfg, 1).unwrap();
    e.set_capture(true);
    let mut sim = Simulator::new(1, 0, None, false).unwrap();
    sim.start(0);
    e.start(0);
    let mut out = heapless::Vec::<_, MAX_OUT>::new();
    let mut file = Vec::new();
    for k in 0..(700 * 20u64) {
        let t = k * 50_000_000;
        sim.step_into(t, &mut out);
        for p in &out {
            let src = match p.channel {
                vc_gatt::Channel::Hrm => Source::Hr,
                vc_gatt::Channel::Cpm => Source::Power,
                vc_gatt::Channel::Csc => Source::Csc,
            };
            if let Some(d) = e.ingest(t, src, p.bytes.to_vec()) {
                e.on_cue_played(d.id, t + 50_000_000, t + 2_450_000_000, t + 2_460_000_000);
            }
        }
        if k % 5 == 0 {
            e.tick(t);
        }
        if k % 20 == 0 {
            file.extend(e.take_capture());
        }
    }
    file.extend(e.take_capture());
    let r = vc_core::capture::replay(&file).unwrap();
    assert!(
        r.phone.len() >= 3,
        "the ride should decide something: {}",
        r.phone.len()
    );
    assert_eq!(r.phone, r.replayed);
}
