#![allow(clippy::unwrap_used)]

use vc_core::decision::Arm;
use vc_core::golden::write_line;
use vc_core::phrase::{Earcon, PhraseId};
use vc_core::{AkiClass, CueDecision, Directive, Priority};

#[test]
fn one_line_is_byte_exact() {
    let d = CueDecision {
        id: 3,
        directive: Directive::WprimeLow,
        priority: Priority::P2,
        aki_class: AkiClass::P2,
        dir: -1,
        delta_star_w: 150.04,
        surge: None,
        decided_ns: 166_750_000_000,
        arm: Arm::Direct,
        phrase: PhraseId::WprimeLowDirect,
        earcon: Some(Earcon::P2DualTone),
        lead_ms: 50,
        interrupt: false,
    };

    let mut s = String::new();
    write_line(&mut s, &d).unwrap();

    assert_eq!(
        s,
        "    166750    3 P2   P2  WPRIME_LOW           direct    WPL_D  ear_p2        -1   1500 -\n"
    );
}
