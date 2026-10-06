#![allow(unused)]

use crate::PAYLOAD_MAX;
use crate::error::{DecodeError, EncodeError};

//this points to the coming data , it only points and does not own the data so zero allocation
pub(crate) struct Cursor<'a> {
    rest: &'a [u8],
}

impl<'a> Cursor<'a> {
    //now we need to assign that buffer to the cursor to point
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { rest: buf }
    }

    //now we need to return how many bytes are remaining
    pub(crate) fn remaining(&self) -> usize {
        self.rest.len()
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        match self.rest.split_first_chunk::<N>() {
            Some((head, tail)) => {
                self.rest = tail;
                Ok(*head)
            }
            None => Err(DecodeError::Truncated {
                need: N as u8,
                have: self.rest.len().min(u8::MAX as usize) as u8,
            }),
        }
    }

    pub(crate) fn u8(&mut self) -> Result<u8, DecodeError> {
        let [b] = self.take::<1>()?;
        Ok(b)
    }

    pub(crate) fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.take()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take()?))
    }
}

pub(crate) struct Writer<'a> {
    buf: &'a mut [u8; PAYLOAD_MAX],
    pos: usize,
}

impl<'a> Writer<'a> {
    pub(crate) fn new(buf: &'a mut [u8; PAYLOAD_MAX]) -> Self {
        buf.fill(0);

        Self { buf, pos: 0 }
    }

    fn put(&mut self, bytes: &[u8]) {
        let end = self.pos.saturating_add(bytes.len());

        if let Some(dst) = self.buf.get_mut(self.pos..end) {
            dst.copy_from_slice(bytes);
            self.pos = end;
        }
    }

    pub(crate) fn u8(&mut self, v: u8) {
        self.put(&[v]);
    }

    pub(crate) fn u16(&mut self, v: u16) {
        self.put(&v.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, v: u32) {
        self.put(&v.to_le_bytes());
    }

    pub(crate) fn written(&self) -> usize {
        self.pos
    }
}

// THE WHOLE PROJECT FIRST
// Forget Cursor and Writer for a minute.
// Your system is basically:
//             REAL / FAKE CYCLING SENSOR
//                        │
//                        │ Bluetooth
//                        ▼
//               ┌─────────────────┐
//               │   RAW BYTES     │
//               │  06 48 ...      │
//               └────────┬────────┘
//                        │
//                        ▼
//                 ┌──────────────┐
//                 │   vc-gatt    │
//                 │              │
//                 │ decode HRM   │
//                 │ decode CSC   │
//                 └──────┬───────┘
//                        │
//                        ▼
//              ┌──────────────────┐
//              │ NORMAL RUST DATA │
//              │                  │
//              │ bpm = 72         │
//              │ contact = yes    │
//              │ etc.              │
//              └─────────┬────────┘
//                        │
//                        ▼
//                 vc-physio
//                        │
//                        ▼
//              physiology / coaching
//                        │
//                        ▼
//               recommendations/UI

// And later, when you simulate a sensor:
//           Rust simulator
//                │
//                ▼
//        Hrm { bpm: 72, ... }
//                │
//                ▼
//           vc-gatt encode
//                │
//                ▼
//         RAW BLE BYTES
//                │
//                ▼
//         simulated device

// So there are two directions:
// REAL SENSOR
//    │
//    ▼
// bytes
//    │
//    ▼
// DECODE
//    │
//    ▼
// Rust struct

// Rust struct
//    │
//    ▼
// ENCODE
//    │
//    ▼
// bytes
//    │
//    ▼
// SIMULATED SENSOR

// That is the entire reason Cursor and Writer exist.
// 2. WHY CAN'T WE JUST SEND 72?
// This is the key thing.
// Your Rust program understands:
// bpm = 72

// Bluetooth does not send:
// "bpm = 72"

// It sends bytes.
// For example:
// 06 48

// Those are just bytes.
// The computer initially sees:
// 0x06
// 0x48

// It does not automatically know:
// 0x06 = contact detected
// 0x48 = 72 BPM

// That interpretation comes from the Bluetooth specification.
// For the Heart Rate Measurement characteristic, the packet has a specific layout: a flags byte, then heart rate as either u8 or u16, optionally energy, optionally RR intervals. The project document uses that exact layout for characteristic 0x2A37.
// 3. AN ACTUAL EXAMPLE
// Let's make this concrete.
// Suppose the cyclist's heart rate is:
// 72 BPM

// and the heart-rate strap says:
// sensor contact detected

// A simplified HRM packet could look like:
// 06 48

// Let's break that apart.
// First byte
// 06

// Binary:
// 00000110

// The HRM specification uses bits 1–2 for sensor contact.
// bit:  7 6 5 4 3 2 1 0
//       0 0 0 0 0 1 1 0
//                     ↑
//                 contact

// The value indicates:
// contact detected

// Second byte
// 48

// Hex 48 = decimal 72.
// So:
// 06 48
// │  │
// │  └──── 72 BPM
// └─────── flags

// The computer has only received:
// [0x06, 0x48]

// Your Rust code needs to turn that into something like:
// Hrm {
//     bpm: 72,
//     contact: Detected,
//     ...
// }

// 4. THIS IS WHERE Cursor COMES IN
// Think of Cursor as your finger pointing at the next unread byte.
// Imagine this packet:
// [06] [48] [E8] [03]
//   ↑
//  cursor

// The cursor starts at the beginning.
// You say:
// c.u8()

// Meaning:
// “Give me the next 1 byte.”

// It returns:
// 06

// and moves forward:
// [06] [48] [E8] [03]
//       ↑

// Then:
// c.u8()

// returns:
// 48

// and moves:
// [06] [48] [E8] [03]
//            ↑

// That's basically what a Cursor is doing.
// 5. IGNORE THE RUST SYNTAX — THINK OF THIS
// Your code says:
// pub(crate) struct Cursor<'a> {
//     rest: &'a [u8],
// }

// Conceptually:
// Cursor
//   │
//   └── points to the bytes that are still unread

// For example:
// Original packet:

// [06] [48] [E8] [03]

// Initially:
// rest
//  ↓
// [06] [48] [E8] [03]

// After reading one byte:
// rest
//       ↓
// [48] [E8] [03]

// After reading another:
// rest
//            ↓
// [E8] [03]

// So rest simply means:
// the remaining bytes

// 6. WHAT IS [u8]?
// You need this Rust concept first.
// [u8]

// means:
// a sequence of bytes.

// u8:
// one byte
// 0 → 255

// So:
// [u8]

// could represent:
// 06 48 E8 03

// A slice:
// &[u8]

// means:
// “I am borrowing a view of some bytes.”

// So:
// let packet = [0x06, 0x48, 0xE8, 0x03];

// let view = &packet;

// view lets you look at those bytes without owning/copying them.
// 7. WHAT DOES 'a MEAN?
// This part:
// Cursor<'a>

// looks scary, but the idea is straightforward.
// It says:
// “This Cursor is borrowing bytes, and the borrowed bytes must remain valid for as long as the Cursor uses them.”

// For example:
// let packet = [0x06, 0x48];

// let cursor = Cursor::new(&packet);

// The cursor does not own packet.
// It is borrowing it.
// packet
// ┌──────────────┐
// │ 06    48     │
// └──────────────┘
//        ▲
//        │ borrowed
//        │
// Cursor ─────────

// Rust needs to make sure packet doesn't disappear while Cursor is still using it.
// That's what the lifetime 'a is about.
// Don't worry about mastering lifetimes from this code yet.
// At this stage, remember:
// 'a = “the lifetime of the borrowed byte data.”

// 8. WHAT DOES Cursor::new() DO?
// pub(crate) fn new(buf: &'a [u8]) -> Self {
//     Self { rest: buf }
// }

// This is simply:
// “Create a Cursor pointing at the beginning of these bytes.”

// Example:
// let packet = [0x06, 0x48];

// let mut c = Cursor::new(&packet);

// Now:
// packet
// [06] [48]

// cursor
//   ↓
// [06] [48]

// 9. WHY mut c?
// Because reading moves the cursor.
// This:
// let mut c = Cursor::new(&packet);

// means:
// c is allowed to change.

// Because after reading:
// before:
// [06] [48]
//  ↑

// after:
// [06] [48]
//       ↑

// The cursor's internal state changed.
// 10. WHAT DOES remaining() DO?
// pub(crate) fn remaining(&self) -> usize {
//     self.rest.len()
// }

// This is simply:
// “How many unread bytes are left?”

// Example:
// packet:
// [06] [48] [E8] [03]

// remaining = 4

// After reading one byte:
// [48] [E8] [03]

// remaining = 3

// After reading another:
// [E8] [03]

// remaining = 2

// Very simple.
// 11. NOW THE IMPORTANT FUNCTION: take()
// This is the heart of the reader.
// fn take<const N: usize>(
//     &mut self
// ) -> Result<[u8; N], DecodeError>

// Read it as:
// “Give me the next N bytes. If they don't exist, return an error.”

// That's all.
// 12. WHAT IS const N?
// Suppose you want:
// 1 byte

// You call:
// take::<1>()

// If you want:
// 2 bytes

// you call:
// take::<2>()

// If you want:
// 4 bytes

// you call:
// take::<4>()

// So:
// take::<1>() → [u8; 1]
// take::<2>() → [u8; 2]
// take::<4>() → [u8; 4]

// This is called a const generic.
// You don't need to understand the advanced Rust mechanics yet.
// Just think:
// take<N> = give me N bytes

// 13. WHY Result?
// This is extremely important.
// Suppose the packet is:
// [06]

// But you try:
// c.u16()

// A u16 requires 2 bytes.
// You only have:
// 1 byte

// So what should happen?
// Bad design:
// 💥 crash

// Good design:
// Error:
// need 2 bytes
// have 1

// That's what:
// Result<T, DecodeError>

// means.
// It can return:
// Ok(value)

// or:
// Err(error)

// 14. THE take() FUNCTION IN HUMAN LANGUAGE
// Your code:
// fn take<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {

// means:
// I want N bytes.

// If you have enough:
//     give them to me

// If you don't:
//     return DecodeError

// That's the core idea.
// 15. WHAT IS split_first_chunk() DOING?
// This:
// self.rest.split_first_chunk::<N>()

// is basically asking:
// “Can you split off the first N bytes?”

// Suppose:
// rest:

// [06] [48] [E8] [03]

// For:
// split_first_chunk::<2>()

// you conceptually get:
// head              tail

// [06] [48]         [E8] [03]

// So:
// head = bytes I just consumed
// tail = bytes still remaining

// Then:
// self.rest = tail;

// moves the cursor forward.
// 16. VISUALIZE THE CURSOR
// This is the easiest way to remember it.
// Starting:
// rest
//  ↓
// ┌────┬────┬────┬────┐
// │ 06 │ 48 │ E8 │ 03 │
// └────┴────┴────┴────┘

// Read 2:
// head              tail
// ┌────┬────┐       ┌────┬────┐
// │ 06 │ 48 │       │ E8 │ 03 │
// └────┴────┘       └────┴────┘
//                   ↑
//               self.rest

// Read 1 again:
// head       tail
// ┌────┐     ┌────┐
// │ E8 │     │ 03 │
// └────┘     └────┘
//             ↑
//         self.rest

// That's basically the entire Cursor.
// 17. NOW u8()
// pub(crate) fn u8(&mut self) -> Result<u8, DecodeError> {
//     let [b] = self.take::<1>()?;
//     Ok(b)
// }

// Ignore syntax initially.
// This means:
// 1. Take 1 byte
// 2. If that fails → return the error
// 3. Otherwise give me that byte

// So:
// c.u8()

// might return:
// Ok(0x06)

// or:
// Err(DecodeError::Truncated { ... })

// 18. WHAT DOES ? MEAN?
// This is another thing that makes Rust code look terrifying.
// Here:
// self.take::<1>()?

// ? means:
// “If this returned an error, stop here and return that error to my caller. Otherwise, give me the successful value.”

// So:
// let [b] = self.take::<1>()?;

// roughly means:
// take 1 byte

// if error:
//     return error

// if success:
//     continue with the byte

// It's basically error propagation.
// 19. WHAT IS [b]?
// If:
// take::<1>()

// returns:
// [u8; 1]

// that's an array containing exactly one byte:
// [06]

// So:
// let [b] = ...

// means:
// take the one item out of the one-element array

// So:
// [06]
//  ↓
//  b = 06

// 20. NOW u16()
// pub(crate) fn u16(&mut self) -> Result<u16, DecodeError> {
//     Ok(u16::from_le_bytes(self.take()?))
// }

// This does:
// Take 2 bytes
//        ↓
// Interpret them as a little-endian u16
//        ↓
// Return the number

// 21. WHAT IS LITTLE-ENDIAN?
// This is crucial for binary protocols.
// Suppose the bytes are:
// E8 03

// Because this protocol stores the number in little-endian:
// E8 03

// means:
// 0x03E8

// which equals:
// 1000

// So:
// u16::from_le_bytes([0xE8, 0x03])

// produces:
// 1000

// So:
// RAW BYTES

// E8 03
//  │  │
//  └──┴── little-endian u16
//          ↓
//         1000

// 22. u32() IS THE SAME IDEA
// pub(crate) fn u32(&mut self) -> Result<u32, DecodeError> {
//     Ok(u32::from_le_bytes(self.take()?))
// }

// It just reads:
// 4 bytes

// instead of:
// 2 bytes

// So:
// u8  → 1 byte
// u16 → 2 bytes
// u32 → 4 bytes

// 23. SO THE WHOLE Cursor IS JUST THIS
// Think of it as a machine:
//               RAW BYTES
//                   │
//                   ▼
//         ┌─────────────────┐
//         │     CURSOR      │
//         │                 │
//         │ take 1 byte     │
//         │ take 2 bytes    │
//         │ take 4 bytes    │
//         └────────┬────────┘
//                  │
//                  ▼
//              Rust value

// Examples:
// [48]
//  │
//  └── u8() ──────→ 72

// [E8 03]
//  │
//  └── u16() ─────→ 1000

// [78 56 34 12]
//  │
//  └── u32() ─────→ 0x12345678

// 24. NOW WHY DO WE NEED Writer?
// Because sometimes we need to go in the opposite direction.
// Remember:
// Cursor = bytes → Rust values

// Writer is:
// Writer = Rust values → bytes

// Imagine the simulator has:
// bpm = 72

// But Bluetooth requires:
// 48

// So Writer creates those bytes.
// 25. THINK OF WRITER AS A PEN
// Suppose you have an empty packet:
// [00] [00] [00] [00] [00] ...

// Writer has a position:
// [00] [00] [00] [00]
//  ↑
//  pos = 0

// You tell it:
// w.u8(72);

// Now:
// [48] [00] [00] [00]
//       ↑
//     pos = 1

// Then:
// w.u16(1000);

// 1000 is:
// E8 03

// Now:
// [48] [E8] [03] [00]
//             ↑
//           pos = 3

// So Writer is basically:
// “Write bytes here, then move the position forward.”

// 26. WHAT IS buf?
// buf: &'a mut [u8; PAYLOAD_MAX],

// This means Writer has a mutable reference to a byte array.
// The project uses a maximum payload of 20 bytes for the default ATT payload.     velocoach-rs — Day 1 & Day 2 Im…
// So conceptually:
// buf

// ┌────┬────┬────┬────┬────┬──── ... ────┐
// │ 00 │ 00 │ 00 │ 00 │ 00 │             │
// └────┴────┴────┴────┴────┴─────────────┘
//   ↑
//  pos

// Writer fills that array.
// 27. WHY &mut?
// Because Writer needs to modify the original buffer.
// Normal reference:
// &buffer

// means:
// “I can look at it.”

// Mutable reference:
// &mut buffer

// means:
// “I can change it.”

// Writer needs:
// change bytes

// so:
// &mut

// 28. pos
// pos: usize,

// means:
// “Where should I write the next bytes?”

// Example:
// buffer:

// [06] [48] [00] [00]
//           ↑
//          pos=2

// So the next write happens at position 2.
// 29. Writer::new()
// pub(crate) fn new(buf: &'a mut [u8; PAYLOAD_MAX]) -> Self {
//     buf.fill(0);

//     Self { buf, pos: 0 }
// }

// Human language:
// 1. Give me a byte buffer.
// 2. Clear it.
// 3. Start writing at position 0.

// So:
// before:

// [AB] [CD] [91] [77]

// after new():

// [00] [00] [00] [00]

// pos = 0

// The project document similarly initializes/uses a fixed buffer and emphasizes avoiding stale bytes.     Day 1 and 2 implementation docu…
// 30. Writer::u8()
// pub(crate) fn u8(&mut self, value: u8) -> Result<(), EncodeError> {
//     self.write(&[value])
// }

// Human translation:
// Take this one-byte value
// and write it into the packet.

// Example:
// w.u8(72);

// causes:
// 72 decimal
//    ↓
// 0x48
//    ↓
// write 48

// 31. Writer::u16()
// pub(crate) fn u16(&mut self, value: u16) -> Result<(), EncodeError> {
//     self.write(&value.to_le_bytes())
// }

// Meaning:
// Rust number
//    ↓
// convert to little-endian bytes
//    ↓
// write them

// Example:
// w.u16(1000);

// becomes:
// 1000
//  ↓
// E8 03
//  ↓
// write E8 03

// 32. THE MOST IMPORTANT WRITER FUNCTION
// fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {

// This is basically the Writer's engine.
// Suppose:
// pos = 2

// and:
// bytes = [E8, 03]

// Then it needs to write at:
// positions 2 and 3

// 33. WHAT IS end?
// let end = self.pos + bytes.len();

// Suppose:
// pos = 2
// bytes.len() = 2

// Then:
// end = 4

// Meaning:
// write from position 2
// up to position 4

// Visually:
// index:  0    1    2    3    4
//        ┌────┬────┬────┬────┐
// buf:   │ 06 │ 48 │    │    │
//        └────┴────┴────┴────┘
//                   └────┘
//                  write here

// 34. WHY CHECK BUFFER SIZE?
// Suppose buffer only has 20 bytes.
// You already wrote:
// 19 bytes

// and now try writing:
// 4 bytes

// You need:
// 19 + 4 = 23

// But only:
// 20

// exist.
// Without checking:
// 💥 memory safety problem / panic

// With the check:
// Err(BufferTooSmall)

// The project specifically requires encoding to fail safely rather than panic when a payload doesn't fit.
// 35. WHAT DOES copy_from_slice() DO?
// Suppose:
// source:

// [E8] [03]

// and destination is:
// buf[2..4]

// Then:
// copy_from_slice(bytes)

// means:
// copy these bytes into that location

// Result:
// before:

// [06] [48] [00] [00]

// after:

// [06] [48] [E8] [03]

// 36. THEN pos = end
// self.pos = end;

// After writing:
// [06] [48] [E8] [03]
//                   ↑
//                 pos=4

// This tells the Writer:
// “The next write begins after the bytes I just wrote.”

// 37. WHAT DOES len() MEAN?
// pub(crate) fn len(&self) -> usize {
//     self.pos
// }

// It tells you:
// “How many bytes have I written?”

// Example:
// packet:

// [06] [48] [E8] [03]

// pos = 4

// len() = 4

// So the valid packet is:
// first 4 bytes

// not necessarily the entire 20-byte buffer.
// 38. PUT CURSOR AND WRITER NEXT TO EACH OTHER
// This is the part I want you to memorize.
//                     vc-gatt
//                        │
//              ┌─────────┴─────────┐
//              │                   │
//              ▼                   ▼
//           Cursor               Writer
//              │                   │
//           READ                  WRITE
//              │                   │
//              ▼                   ▼
//        bytes → values       values → bytes

// Or:
//              DECODE
//                ↑
//                │
// [06 48 ...] ── Cursor ──→ Hrm { bpm: 72, ... }

//              ENCODE
//                │
//                ▼
// Hrm { bpm: 72, ... } ── Writer ──→ [06 48 ...]

// 39. NOW CONNECT IT TO hrm.rs
// This is where the whole thing finally makes sense.
// The project document says HRM is:
// flags
//    ↓
// heart rate
//    ↓
// optional energy
//    ↓
// optional RR intervals

// for characteristic 0x2A37.
// So hrm.rs can write:
// let mut c = Cursor::new(b);

// let flags = c.u8()?;

// let bpm = if flags & FLAG_HR_U16 != 0 {
//     c.u16_le()?
// } else {
//     u16::from(c.u8()?)
// };

// Look at what is happening.
// Step 1
// let flags = c.u8()?;

// Packet:
// [06] [48]
//  ↑

// Read:
// flags = 06

// Cursor moves:
// [06] [48]
//       ↑

// Step 2
// c.u8()?

// Read:
// 48

// Convert:
// 0x48 → 72

// Result:
// bpm = 72

// Now:
// RAW:

// 06 48

// becomes:

// flags = 06
// bpm   = 72

// 40. THEN HRM INTERPRETS THE FLAGS
// The Cursor does not understand:
// heart rate
// sensor contact
// RR interval

// Very important.
// Cursor is dumb.
// It only knows:
// "give me 1 byte"
// "give me 2 bytes"
// "give me 4 bytes"

// hrm.rs knows what those bytes mean.
// Think:
// Cursor = screwdriver
// HRM decoder = mechanic

// Cursor:
// "Here are 2 bytes."

// HRM decoder:
// "Those 2 bytes represent heart rate."

// 41. THIS SEPARATION IS REALLY GOOD DESIGN
// Instead of writing this everywhere:
// packet[0]
// packet[1]
// packet[2]
// packet[3]

// you create one safe low-level tool:
// Cursor

// Then HRM code becomes:
// flags = c.u8()?
// bpm   = c.u8()?
// energy = c.u16()?
// rr     = c.u16()?

// CSC can do:
// flags = c.u8()?
// wheel_revolutions = c.u32()?
// wheel_time = c.u16()?

// And later other protocols can reuse the same Cursor.
// The implementation guide explicitly says cursor.rs is the place that touches raw offsets, so higher-level protocol modules don't have to repeatedly manipulate byte positions themselves.     Day 1 and 2 implementation docu…
// 42. HERE'S A COMPLETE REALISTIC FLOW
// Imagine the fake sensor says:
// Heart rate = 72 BPM
// Contact = detected

// The simulator has:
// Hrm {
//     bpm: 72,
//     contact: Contact::Detected,
//     ...
// }

// It calls:
// encode_hrm()

// which uses:
// Writer

// The Writer creates:
// 06 48

// Then Bluetooth sends:
// 06 48

// On the receiving side:
// 06 48
//    │
//    ▼
// Cursor
//    │
//    ├── u8() → 06
//    │
//    └── u8() → 48
//                ↓
//               72

// Then HRM decoder produces:
// Hrm {
//     bpm: 72,
//     contact: Detected,
// }

// Then:
// Hrm
//  │
//  ▼
// vc-physio
//  │
//  ▼
// physiological calculations
//  │
//  ▼
// coach

// 43. THE ERROR CASE
// This is equally important.
// Suppose the sensor sends:
// 06

// Only one byte.
// But the decoder expects:
// flags + HR

// If HR is supposed to be two bytes, the Cursor tries to read two bytes.
// It discovers:
// need = 2
// have = 0

// and returns:
// Err(DecodeError::Truncated { ... })

// Then:
// bad packet
//     │
//     ▼
//  Cursor
//     │
//     ▼
//  not enough bytes
//     │
//     ▼
//  DecodeError
//     │
//     ▼
//  caller handles it

// No:
// 💥 crash

// No:
// panic!

// The Day 2 design explicitly calls out this “any input length/content must not panic” requirement.     velocoach-rs — Day 1 & Day 2 Im…
// 44. SO WHAT DOES no_std HAVE TO DO WITH THIS?
// Another thing that probably made this seem complicated.
// The project wants vc-gatt to work without depending on the normal Rust standard library. The document describes vc-gatt as no_std and uses heapless for fixed-capacity data.
// So instead of thinking:
// Why are they making this so complicated?

// think:
// This library may eventually run in
// embedded / constrained environments.

// Therefore:

// fixed buffers
// +
// predictable memory
// +
// no unnecessary heap allocation
// +
// no panic

// That's why you're seeing this style.
// 45. ONE IMPORTANT CORRECTION TO WHAT WE WERE BUILDING
// The code you pasted earlier is not exactly the same implementation as the PDF's current implementation.
// The PDF's version uses:
// Cursor {
//     buf,
//     pos
// }

// and accesses data through:
// get()

// while the code you pasted uses:
// Cursor {
//     rest
// }

// and:
// split_first_chunk()

// The PDF's intended design also has:
// use crate::error::{DecodeError, EncodeError};

// and Writer returns EncodeError when a write does not fit.
// So don't worry about memorizing the exact rest implementation yet. Conceptually they do the same thing:
// PDF implementation:

// buffer + position
//        │
//        ▼
//     Cursor
//        │
//        ▼
//  read N bytes

// your pasted implementation:

// remaining bytes
//        │
//        ▼
//     Cursor
//        │
//        ▼
//  read N bytes

// Same job.
// 46. THE FOUR THINGS YOU ACTUALLY NEED TO UNDERSTAND RIGHT NOW
// Don't try to memorize the whole file.
// Just understand these four:
// 1. &[u8]
// borrowed bytes

// Example:
// 06 48 E8 03

// 2. Cursor
// reads bytes sequentially

// bytes → values

// 3. Writer
// writes values sequentially

// values → bytes

// 4. Result
// success OR error

// Ok(value)

// or
// Err(...)

// so bad sensor packets don't crash the program.
// 47. THE WHOLE THING IN ONE PICTURE
//                     velocoach-rs
//                          │
//                          ▼
//                   ┌─────────────┐
//                   │   vc-gatt   │
//                   └──────┬──────┘
//                          │
//             ┌────────────┴────────────┐
//             │                         │
//             ▼                         ▼

//          DECODE                     ENCODE

//       BLE bytes                  Rust struct
//           │                          │
//           ▼                          ▼
//        Cursor                     Writer
//           │                          │
//           │                          │
//           ▼                          ▼
//      read u8/u16/u32           write u8/u16/u32
//           │                          │
//           ▼                          ▼
//      Rust values                 BLE bytes
//           │                          │
//           ▼                          │
//       HRM / CSC                     │
//           │                          │
//           ▼                          │
//     typed structs                   │
//           │                          │
//           ▼                          │
//      vc-physio                      │
//           │                          │
//           ▼                          │
//       coaching                       │
//                                      │
//                                      └── simulator

// 48. THE SIMPLEST POSSIBLE ANALOGY
// Imagine a letter.
// Bluetooth sends:
// "06 48 E8 03"

// But that's just encoded information.
// Cursor
// is the person reading the letter:
// Read byte 1
// Read byte 2
// Read byte 3...

// HRM decoder
// is the person saying:
// "These first bytes mean heart rate."
// "Those bits mean contact."
// "These bytes mean RR interval."

// Writer
// is the person writing the letter in the encoded format.
// So:
// Cursor = reader
// Writer = writer
// HRM/CSC = interpreter of what the bytes mean

// 49. WHAT YOU SHOULD DO NEXT
// Don't code HRM yet.
// Before moving forward, make sure this mental model is clear:
// BLE gives us:
//     bytes

// Cursor:
//     safely reads those bytes

// HRM decoder:
//     decides what those bytes mean

// Result:
//     bad bytes → error instead of crash

// Writer:
//     takes Rust data and produces bytes

// Once that clicks, hrm.rs will become much easier, because you'll be able to literally watch it say:
// take flags
//       ↓
// take heart rate
//       ↓
// interpret flags
//       ↓
// build Hrm struct

// And that is exactly the next layer the project document adds after Cursor/Writer.
