use vc_ffi::{Engine, EngineConfig, Profile, Source};

fn cfg() -> EngineConfig {
    EngineConfig {
        cp_w: 250.0,
        wprime_j: 20_000.0,
        hr_max: 190.0,
        profile: Profile::Solo,
        cooldown_between_alerts_s: 20.0,
        prompt_surge_countdown_s: 5.0,
        steps: vec![],
    }
}

#[test]
fn the_engine_wrapper_works_and_never_throws_on_bad_bytes() {
    let engine = Engine::for_scenario(cfg(), 1).unwrap();
    engine.start(0);

    // Invalid input is counted, not propagated as an exception.
    assert!(engine.ingest(0, Source::Power, vec![]).is_none());
    assert_eq!(engine.snapshot().decode_errors, 1);
    assert_eq!(engine.snapshot().policy_hash.len(), 16);

    assert!(
        Engine::new(EngineConfig {
            cooldown_between_alerts_s: 60.0,
            ..cfg()
        })
        .is_err()
    );

    assert_eq!(vc_ffi::scenarios().len(), 6);

    assert_eq!(
        vc_ffi::encode_load(1, 2, 42, false),
        [0x01, 0x20, 1, 2, 42, 0, 0, 0, 0]
    );
}

#[cfg(feature = "sim")]
#[test]
fn the_simulator_wrapper_streams_packets() {
    let simulator = vc_ffi::Simulator::new(1, 0, None, false).unwrap();
    simulator.start(0);

    let n: usize = (1..=40_u64)
        .map(|k| simulator.step(k * 50_000_000).len())
        .sum();

    // Two seconds: CPM x8, HRM x2, CSC x2.
    assert_eq!(n, 8 + 2 + 2);
}
