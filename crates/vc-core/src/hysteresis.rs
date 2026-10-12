//! Fire once when a value crosses a line; re-arm only after it returns
//! beyond a separate threshold.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hysteresis {
    fire: f64,
    rearm: f64,
    dir: Direction,
    armed: bool,
}

impl Hysteresis {
    /// Fires when value >= fire; re-arms when value < rearm.
    pub const fn above(fire: f64, rearm: f64) -> Self {
        Self {
            fire,
            rearm,
            dir: Direction::Above,
            armed: true,
        }
    }

    /// Fires when value < fire; re-arms when value > rearm.
    /// W′ uses (0.20, 0.30).
    pub const fn below(fire: f64, rearm: f64) -> Self {
        Self {
            fire,
            rearm,
            dir: Direction::Below,
            armed: true,
        }
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Returns true only when the cue should fire.
    /// NaN never fires and never re-arms.
    pub fn update(&mut self, v: f64) -> bool {
        let (crossed, back) = match self.dir {
            Direction::Above => (v >= self.fire, v < self.rearm),
            Direction::Below => (v < self.fire, v > self.rearm),
        };

        if self.armed && crossed {
            self.armed = false;
            return true;
        }

        if !self.armed && back {
            self.armed = true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wprime_style_fires_below_20_rearms_above_30() {
        let mut h = Hysteresis::below(0.20, 0.30);

        let fired: std::vec::Vec<bool> = [0.50, 0.25, 0.19, 0.10, 0.22, 0.29, 0.31, 0.19]
            .iter()
            .map(|&v| h.update(v))
            .collect();

        assert_eq!(
            fired,
            [false, false, true, false, false, false, false, true]
        );
    }
}
