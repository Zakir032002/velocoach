use vc_core::directive::{Candidate, Candidates};
use vc_core::policy::Policy;
use vc_core::{Arm, Directive, Engine, EngineConfig, Mode, Profile, WorkoutPlan};

const S: u64 = 1_000_000_000;

fn study(seed: u64) -> Policy {
    let mut p = Policy::new(&EngineConfig::default());
    p.set_mode(Mode::Study, seed);
    p
}

#[test]
fn p1_is_never_randomised_and_every_p2_has_a_draw() {
    let mut p = study(5);
    let mut c = Candidates::default();
    for k in 0..200u64 {
        let t = k * 60 * S; // far apart: always eligible
        let d = if k % 3 == 0 {
            Directive::SensorDropout
        } else {
            Directive::WprimeLow
        };
        c.offer(Candidate::new(d, -1, 20.0, t));
        let dec = p.decide(t, &mut c).unwrap();
        if d == Directive::SensorDropout {
            assert_eq!((dec.arm, dec.draw_index), (Arm::None, None)); // M3
        } else {
            assert!(dec.draw_index.is_some());
            assert_ne!(dec.arm, Arm::None);
        }
    }
}

#[test]
fn a_sham_plays_nothing_but_uses_up_the_cooldown() {
    // Find a seed whose first draw is a sham, then check the cooldown.
    let seed = (0..1_000)
        .find(|s| vc_core::study::StudyAllocator::replay(*s, 0).arm == Arm::Sham)
        .unwrap();
    let mut p = study(seed);
    let mut c = Candidates::default();
    c.offer(Candidate::new(Directive::WprimeLow, -1, 20.0, 0));
    let d = p.decide(0, &mut c).unwrap();
    assert!(d.is_sham() && d.earcon.is_none());
    c.offer(Candidate::new(Directive::SafetyHr, -1, 0.0, S));
    assert!(!p.decide(S, &mut c).unwrap().interrupt); // nothing was playing
    c.offer(Candidate::new(Directive::GreyZoneDrift, -1, 20.0, 10 * S));
    assert!(p.decide(10 * S, &mut c).is_none()); // still inside the sham's cooldown
}

#[test]
fn study_fixes_solo_only_between_sessions_and_the_hash_is_stable() {
    let cfg = EngineConfig {
        profile: Profile::Rolling,
        ..EngineConfig::default()
    };
    let (mut a, mut b) = (
        Engine::new(cfg, WorkoutPlan::default()).unwrap(),
        Engine::new(cfg, WorkoutPlan::default()).unwrap(),
    );
    a.set_mode(Mode::Study, 1).unwrap();
    b.set_mode(Mode::Study, 2).unwrap();
    assert_eq!(a.policy_hash(), b.policy_hash()); // M4: the seed is not part of the policy
    assert_eq!(a.config().profile, Profile::Solo);
    let coach = Engine::new(cfg, WorkoutPlan::default()).unwrap();
    assert_ne!(coach.policy_hash(), a.policy_hash()); // Rolling differs from Solo
    a.start(0);
    assert!(a.set_mode(Mode::Coach, 1).is_err());
}
