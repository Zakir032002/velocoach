//! Deterministic fault windows for simulator scenarios and CTRL commands.
use heapless::Vec;
use vc_gatt::{Channel, FaultKind};

const MS: u64 = 1_000_000;
const MAX_FAULTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoastMode {
    Stale,
    Silent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FaultWindow {
    kind: FaultKind,
    channel: Option<Channel>,
    start_ns: u64,
    end_ns: u64,
    param: i16,
}

impl FaultWindow {
    fn active(&self, t_ns: u64) -> bool {
        self.start_ns <= t_ns && t_ns < self.end_ns
    }

    fn matches_channel(&self, ch: Channel) -> bool {
        self.channel.is_none_or(|target| target == ch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Injected {
    Done,
    SetTrueWprime(f64),
    Disconnect(u32),
}

#[derive(Debug, Clone, Default)]
pub struct Faults {
    windows: Vec<FaultWindow, MAX_FAULTS>,
    garbage_left: [u16; 3],
}

impl Faults {
    /// Inject a fault at an absolute ride time.
    ///
    /// For `Garbage`, `param` specifies the number of malformed packets.
    /// For `Coast`, `param == 1` means silent; other values mean stale.
    /// For `WprimeMismatch`, `param` is the true W′ in units of 100 J.
    pub fn inject(
        &mut self,
        kind: FaultKind,
        channel: Option<Channel>,
        at_ns: u64,
        dur_ms: u32,
        param: i16,
    ) -> Injected {
        match kind {
            FaultKind::Garbage => {
                let count = if param > 0 { param as u16 } else { 1 };

                match channel {
                    Some(ch) => {
                        let index = crate::packets::ch_index(ch);
                        self.garbage_left[index] = self.garbage_left[index].saturating_add(count);
                    }
                    None => {
                        for remaining in &mut self.garbage_left {
                            *remaining = remaining.saturating_add(count);
                        }
                    }
                }

                Injected::Done
            }
            FaultKind::WprimeMismatch => {
                let joules = f64::from(param) * 100.0;
                if joules > 0.0 {
                    Injected::SetTrueWprime(joules)
                } else {
                    Injected::Done
                }
            }
            _ => {
                let end_ns = at_ns.saturating_add(u64::from(dur_ms) * MS);
                let window = FaultWindow {
                    kind,
                    channel,
                    start_ns: at_ns,
                    end_ns,
                    param,
                };

                if self.windows.push(window).is_err() {
                    // Fixed capacity: do not panic if the queue is full.
                    return Injected::Done;
                }

                if kind == FaultKind::Disconnect {
                    Injected::Disconnect(dur_ms)
                } else {
                    Injected::Done
                }
            }
        }
    }

    /// Clear the specified fault kind; 0xFF clears all faults.
    pub fn clear(&mut self, kind: u8) {
        if kind == 0xFF {
            self.windows.clear();
            self.garbage_left = [0; 3];
            return;
        }

        self.windows.retain(|f| f.kind as u8 != kind);

        if kind == FaultKind::Garbage as u8 {
            self.garbage_left = [0; 3];
        }
    }

    /// A dropout suppresses its selected channel; a disconnect suppresses all
    /// selected channels for the duration of its window.
    pub fn silenced(&self, ch: Channel, t_ns: u64) -> bool {
        self.windows.iter().any(|f| {
            f.active(t_ns)
                && f.matches_channel(ch)
                && matches!(f.kind, FaultKind::Dropout | FaultKind::Disconnect)
        })
    }

    pub fn coast_at(&self, t_ns: u64) -> Option<CoastMode> {
        self.windows.iter().rev().find_map(|f| {
            if f.kind == FaultKind::Coast && f.active(t_ns) {
                Some(if f.param == 1 {
                    CoastMode::Silent
                } else {
                    CoastMode::Stale
                })
            } else {
                None
            }
        })
    }

    pub fn redline_at(&self, t_ns: u64) -> bool {
        self.windows
            .iter()
            .any(|f| f.kind == FaultKind::HrRedline && f.active(t_ns))
    }

    pub fn take_garbage(&mut self, ch: Channel) -> bool {
        let index = crate::packets::ch_index(ch);
        let Some(remaining) = self.garbage_left.get_mut(index) else {
            return false;
        };

        if *remaining == 0 {
            return false;
        }

        *remaining -= 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropout_silences_only_its_channel() {
        let mut faults = Faults::default();
        let _ = faults.inject(FaultKind::Dropout, Some(Channel::Cpm), 100, 1_000, 0);

        assert!(faults.silenced(Channel::Cpm, 500));
        assert!(!faults.silenced(Channel::Hrm, 500));
        assert!(!faults.silenced(Channel::Cpm, 1_000_000_100));
    }

    #[test]
    fn garbage_counter_is_deterministic_and_finite() {
        let mut faults = Faults::default();
        let _ = faults.inject(FaultKind::Garbage, Some(Channel::Cpm), 0, 0, 2);

        assert!(faults.take_garbage(Channel::Cpm));
        assert!(faults.take_garbage(Channel::Cpm));
        assert!(!faults.take_garbage(Channel::Cpm));
    }

    #[test]
    fn clear_all_resets_windows_and_garbage() {
        let mut faults = Faults::default();
        let _ = faults.inject(FaultKind::Garbage, Some(Channel::Cpm), 0, 0, 2);
        faults.clear(0xFF);

        assert!(!faults.take_garbage(Channel::Cpm));
    }
}
