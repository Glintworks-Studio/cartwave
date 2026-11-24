//! Clean-room NES 2A03 APU + Konami-compatible VRC6 expansion audio emulator,
//! written from the NESdev wiki's hardware documentation (no emulator source).
//!
//! `no_std`, and no allocation after construction. See ENGINE.md.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

pub mod math;
pub mod pulse;
pub mod tables;
pub mod units;
