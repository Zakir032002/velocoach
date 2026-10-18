//! The Simulator object for the iQOO rig.
//! Built only with `--features sim`.
use std::sync::{Arc, Mutex};

use vc_gatt::Channel;
use vc_sim::sim::{CtrlOutcome, MAX_OUT};

use crate::{VcError, lock};

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum SensorChannel {
    Hrm,
    Cpm,
    Csc,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct SimPacket {
    pub channel: SensorChannel,
    pub char_seq: u16,
    pub due_ns: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum CtrlReply {
    Ack,
    /// Kotlin stamps its send time and notification time.
    Pong {
        ping_id: u16,
        t_rx_ns: u64,
    },
    /// The simulated rig drops the link for this duration.
    Disconnect {
        dur_ms: u32,
    },
}

#[derive(uniffi::Object)]
pub struct Simulator {
    inner: Mutex<vc_sim::Simulator>,
    out: Mutex<heapless::Vec<vc_sim::SimPacket, MAX_OUT>>,
}

#[uniffi::export]
impl Simulator {
    /// Persona 0 = scenario default; seed None = scenario seed.
    #[uniffi::constructor]
    pub fn new(
        scenario_id: u8,
        persona: u8,
        seed: Option<u64>,
        human: bool,
    ) -> Result<Arc<Self>, VcError> {
        let simulator = vc_sim::Simulator::new(scenario_id, persona, seed, human)
            .map_err(|e| VcError::Sim(e.to_string()))?;

        Ok(Arc::new(Self {
            inner: Mutex::new(simulator),
            out: Mutex::new(heapless::Vec::new()),
        }))
    }

    pub fn name(&self) -> String {
        lock(&self.inner).name().to_string()
    }

    pub fn is_running(&self) -> bool {
        lock(&self.inner).is_running()
    }

    pub fn start(&self, now_ns: u64) {
        lock(&self.inner).start(now_ns);
    }

    pub fn stop(&self) {
        lock(&self.inner).stop();
    }

    /// Called every 50 ms from the HandlerThread.
    pub fn step(&self, now_ns: u64) -> Vec<SimPacket> {
        let mut out = lock(&self.out);
        lock(&self.inner).step_into(now_ns, &mut out);

        out.iter()
            .map(|p| SimPacket {
                channel: match p.channel {
                    Channel::Hrm => SensorChannel::Hrm,
                    Channel::Cpm => SensorChannel::Cpm,
                    Channel::Csc => SensorChannel::Csc,
                },
                char_seq: p.char_seq,
                due_ns: p.due_ns,
                bytes: p.bytes.to_vec(),
            })
            .collect()
    }

    /// CTRL bytes from the iPhone. Copying is acceptable on this cold path.
    pub fn apply_ctrl(&self, now_ns: u64, bytes: Vec<u8>) -> Result<CtrlReply, VcError> {
        let result = lock(&self.inner)
            .apply_ctrl(now_ns, &bytes)
            .map_err(|e| VcError::Sim(e.to_string()))?;

        Ok(match result {
            CtrlOutcome::Ack => CtrlReply::Ack,
            CtrlOutcome::Pong { ping_id, t_rx_ns } => CtrlReply::Pong { ping_id, t_rx_ns },
            CtrlOutcome::Disconnect { dur_ms } => CtrlReply::Disconnect { dur_ms },
        })
    }

    /// Human mode: slider effort in thousandths of CP.
    /// The rig's "I am the rider" switch; it wins over LOAD's human byte.
    pub fn set_human(&self, on: bool) {
        lock(&self.inner).set_human(on);
    }

    pub fn set_human_effort(&self, permille: i16) {
        lock(&self.inner).set_human_effort(permille);
    }

    pub fn take_disconnect(&self) -> Option<u32> {
        lock(&self.inner).take_disconnect()
    }
}

/// iQOO -> iPhone: the rig's send time for one sensor notification.
#[uniffi::export]
pub fn encode_stamp(seq: u16, channel: SensorChannel, char_seq: u16, t0_ns: u64) -> Vec<u8> {
    let channel = match channel {
        SensorChannel::Hrm => vc_gatt::Channel::Hrm,
        SensorChannel::Cpm => vc_gatt::Channel::Cpm,
        SensorChannel::Csc => vc_gatt::Channel::Csc,
    };
    crate::telem_bytes(&vc_gatt::Telem::Stamp {
        seq,
        channel,
        char_seq,
        t0_ns,
    })
}
