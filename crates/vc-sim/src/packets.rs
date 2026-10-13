//! Fixed-capacity sensor packets for the simulated rider.
use heapless::Vec;
use vc_gatt::{
    Channel, Contact, Cpm, CrankData, Csc, Hrm, MAX_PAYLOAD, encode_cpm, encode_csc, encode_hrm,
};

use crate::rider::Truth;

pub type Payload = Vec<u8, 20>;

pub const fn ch_index(ch: Channel) -> usize {
    match ch {
        Channel::Hrm => 0,
        Channel::Cpm => 1,
        Channel::Csc => 2,
    }
}

/// Convert simulated power into an integer watt reading.
pub fn watts_i16(watts: f64) -> i16 {
    libm::round(watts).clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

fn payload<F>(encode: F) -> Option<Payload>
where
    F: FnOnce(&mut [u8; MAX_PAYLOAD]) -> Result<usize, vc_gatt::EncodeError>,
{
    let mut buffer = [0u8; MAX_PAYLOAD];
    let n = encode(&mut buffer).ok()?;
    let bytes = buffer.get(..n)?;

    let mut out = Payload::new();
    out.extend_from_slice(bytes).ok()?;
    Some(out)
}

/// Build the selected sensor's real GATT measurement payload.
///
/// HRM includes one RR value. CPM includes power and crank data.
/// CSC is omitted when the crank is silent.
pub fn sensor_packet(ch: Channel, truth: &Truth, crank: Option<CrankData>) -> Option<Payload> {
    match ch {
        Channel::Hrm => {
            let mut rr = heapless::Vec::new();
            let _ = rr.push(1024);

            let measurement = Hrm {
                bpm: truth.hr_reported,
                contact: Contact::Detected,
                energy_kj: None,
                rr_1024: rr,
            };

            payload(|out| encode_hrm(&measurement, out))
        }
        Channel::Cpm => {
            let measurement = Cpm {
                power_w: watts_i16(truth.power_w),
                crank,
                ..Cpm::default()
            };

            payload(|out| encode_cpm(&measurement, out))
        }
        Channel::Csc => {
            let crank = crank?;
            let measurement = Csc {
                wheel: None,
                crank: Some(crank),
            };

            payload(|out| encode_csc(&measurement, out))
        }
    }
}

/// Return one of four malformed CPM-shaped payloads.
///
/// The counter, not the ride's PRNG, chooses the pattern so fault injection
/// never shifts the rider's random stream.
pub fn garbage(k: u32) -> Payload {
    const BAD: [&[u8]; 4] = [
        &[0x01],
        &[0x02, 0x00],
        &[0x02, 0x00, 0xFA],
        &[0x02, 0x00, 0xFA, 0x00, 0x01],
    ];

    let mut out = Payload::new();
    let pattern = BAD[((k.saturating_sub(1)) % BAD.len() as u32) as usize];
    let _ = out.extend_from_slice(pattern);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rider::RiderParams;

    #[test]
    fn csc_is_omitted_when_crank_is_silent() {
        let truth = Truth {
            t_ns: 250_000_000,
            power_w: 200.0,
            hr_bpm: 100.0,
            hr_reported: 100,
            cadence_rpm: 0.0,
            wprime_true_j: RiderParams::DEFAULT.wprime_j,
            blown: false,
            target_w: 200.0,
            grade_pct: 0.0,
        };

        assert!(sensor_packet(Channel::Csc, &truth, None).is_none());
    }

    #[test]
    fn garbage_cycles_through_four_patterns() {
        assert_eq!(garbage(1), garbage(5));
        assert_ne!(garbage(1), garbage(2));
        assert_ne!(garbage(2), garbage(3));
        assert_ne!(garbage(3), garbage(4));
    }
}
