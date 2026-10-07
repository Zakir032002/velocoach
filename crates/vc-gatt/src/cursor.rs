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
    fn take<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
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
}
