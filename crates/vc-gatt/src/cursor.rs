#![allow(unused)]
use crate::error::{DecodeError, EncodeError};

pub(crate) struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}
// Incoming Packet Buffer in Memory (&[u8]):
// ┌──────┬──────┬──────┬──────┬──────┬──────┐
// │ 0x16 │ 0x9E │ 0x00 │ 0x04 │ 0xAA │ 0xBB │  Total Length = 6
// └──────┴──────┴──────┴──────┴──────┴──────┘
//   ▲                    ▲
//   │                    │
// pos = 0               pos = 3
// (Read Head)          (Read Head after consuming 3 bytes)
//                       Remaining = 6 - 3 = 3 bytes
impl<'a> Cursor<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    //how many remaining bytes are there unread
    pub(crate) fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }
    //now we have to takeout the byte and move forword the cursor becoz we dont need that anymore
    pub(crate) fn take<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let end = self.pos.saturating_add(N);

        let truncated = DecodeError::Truncated {
            need: end,
            have: self.buf.len(),
        };

        let bytes = self.buf.get(self.pos..end).ok_or(truncated)?;
        //Try to fit this data into a box of size N. If it fits, great, we can use it! If it doesn't fit, let me know by telling me it was truncated.
        let arr: [u8; N] = bytes.try_into().map_err(|_| truncated)?;

        self.pos = end;

        Ok(arr)
    }
    //take 1 byte
    pub(crate) fn u8(&mut self) -> Result<u8, DecodeError> {
        let [b] = self.take::<1>()?;
        Ok(b)
    }
    //take 2 bytes
    pub(crate) fn u16_le(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.take()?))
    }
    //take 4 bytes
    pub(crate) fn u32_le(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    //it is used beacuse when the pedel goes backwords etc, the power meter may give -ve numbers
    //and can change the whole energy values to extreme if we dont convert it
    pub(crate) fn i16_le(&mut self) -> Result<i16, DecodeError> {
        Ok(i16::from_le_bytes(self.take()?))
    }
    // Why it is needed (Day 3 requirement):
    // Used by `vcsim.rs` for nanosecond timestamps (`t0_ns`, `t_rx_ns`, `t_tx_ns`, `t_client_ns`)[cite: 138, 139, 140].
    // A 32-bit nanosecond counter wraps around to 0 every 4.29 seconds, making latency tracking impossible[cite: 26, 27].
    // A 64-bit integer counts nanoseconds continuously for 584 years[cite: 123].
    pub(crate) fn u64_le(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take()?))
    }
    //when measuring in nano seconds the timer should preserve the time for the entire ride so u64 is used

    // Asserts that no unread bytes remain in the cursor[cite: 123].
    //
    // Why it is needed (Day 3 requirement):
    // Enforces strict length verification on our custom `VC-SIM` protocol[cite: 122, 135].
    // While standard BLE sensors tolerate trailing reserved bytes, a VC-SIM control message
    // that arrives with unexpected trailing bytes indicates memory corruption or a framing bug[cite: 136, 143].
    //
    // How it works:
    // If `remaining() == 0`, returns `Ok(())`[cite: 123]. Otherwise, returns `Err(DecodeError::TooLong)`[cite: 123].
    pub(crate) fn finish(&self) -> Result<(), DecodeError> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(DecodeError::TooLong {
                expected: self.pos,
                have: self.buf.len(),
            })
        }
    }
}
//now we have to implement the writer,,and it does the opposite of cursor,,we will pack the bytes
// Caller-Provided Buffer: [u8; 20]
//  ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐
//  │0x00│0x00│0x00│0x00│0x00│0x00│0x00│0x00│0x00│0x00│0x00│0x00│0x00│0x00│0x00│ ...
//  └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘
//    ▲                   ▲                                                 ▲
//    │                   │                                                 │
//  pos = 0        pos after writes                                     buf.len() = 20
//  (start)        (head moves right)                                  (hard boundary)
pub(crate) struct Writer<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Writer<'a> {
    pub(crate) fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub(crate) fn written(&self) -> usize {
        self.pos
    }

    fn put(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        let end = self.pos.saturating_add(bytes.len());
        let have = self.buf.len();

        let dst = self
            .buf
            .get_mut(self.pos..end)
            .ok_or(EncodeError::BufferTooSmall { need: end, have })?;

        dst.copy_from_slice(bytes);
        self.pos = end;

        Ok(())
    }

    pub(crate) fn u8(&mut self, v: u8) -> Result<(), EncodeError> {
        self.put(&[v])
    }

    pub(crate) fn u16_le(&mut self, v: u16) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn u32_le(&mut self, v: u32) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn i16_le(&mut self, v: i16) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn u64_le(&mut self, v: u64) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn bytes(&mut self, v: &[u8]) -> Result<(), EncodeError> {
        self.put(v)
    }
}
