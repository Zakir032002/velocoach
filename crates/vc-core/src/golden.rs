//! Golden-log lines: fixed columns, no floating-point text.
use core::fmt::{self, Write};

use crate::decision::{Arm, CueDecision};
use crate::directive::{AkiClass, Priority};
use crate::phrase;

pub const HEADER: &str = "# velocoach golden v1\n\
#     t_ms   id prio aki directive            arm       phrase earcon       dir dw_x10 int\n";

pub const fn arm_name(a: Arm) -> &'static str {
    match a {
        Arm::None => "none",
        Arm::Direct => "direct",
        Arm::Autonomy => "autonomy",
        Arm::Sham => "sham",
    }
}

pub const fn prio_name(p: Priority) -> &'static str {
    match p {
        Priority::P1 => "P1",
        Priority::P2 => "P2",
        Priority::P3 => "P3",
    }
}

pub const fn aki_name(a: AkiClass) -> &'static str {
    match a {
        AkiClass::P0 => "P0",
        AkiClass::P1 => "P1",
        AkiClass::P2 => "P2",
        AkiClass::P3 => "P3",
        AkiClass::P4 => "P4",
        AkiClass::P5 => "P5",
    }
}

pub fn write_line(w: &mut impl Write, d: &CueDecision) -> fmt::Result {
    let phrase = phrase::def(d.phrase).map_or("?", |p| p.key);
    let earcon = d.earcon.map_or("-", |e| e.key());
    let dw_x10 = libm::round(d.delta_star_w * 10.0) as i64;

    writeln!(
        w,
        "{:>10} {:>4} {:<4} {:<3} {:<20} {:<9} {:<6} {:<12} {:>3} {:>6} {}",
        d.decided_ns / 1_000_000,
        d.id,
        prio_name(d.priority),
        aki_name(d.aki_class),
        d.directive.name(),
        arm_name(d.arm),
        phrase,
        earcon,
        d.dir,
        dw_x10,
        if d.interrupt { "yes" } else { "-" },
    )
}
