#![no_main]

use libfuzzer_sys::fuzz_target;
use vc_sim::Simulator;
use vc_sim::sim::MAX_OUT;

// Input: [scenario][persona][human], followed by
// records of [dt][len][CTRL bytes].
fuzz_target!(|data: &[u8]| {
    let [id, persona, human, rest @ ..] = data else {
        return;
    };

    let Ok(mut sim) = Simulator::new(id % 8, persona % 6, Some(1), human & 1 == 1) else {
        return;
    };

    sim.start(0);

    let mut out = heapless::Vec::<_, MAX_OUT>::new();
    let (mut t, mut rest) = (0u64, rest);

    while let [dt, len, tail @ ..] = rest {
        t = t.saturating_add(u64::from(*dt) * 50_000_000);

        let n = usize::from(*len).min(tail.len()).min(20);
        let (bytes, next) = tail.split_at(n);

        let _ = sim.apply_ctrl(t, bytes);
        sim.step_into(t, &mut out);
        let _ = sim.take_disconnect();
        let _ = sim.responses().len();

        rest = next;
    }
});
