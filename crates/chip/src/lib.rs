//! Clean-room NES 2A03 APU + VRC6 expansion audio emulator, written from the
//! NESdev wiki's hardware documentation (no emulator source was read).
//!
//! - Channels: [`pulse`], [`triangle`], [`noise`], [`dmc`], [`vrc6`], with the
//!   shared [`units`] and the [`frame_counter`].
//! - [`synth`]: the event-driven system clock, [`mixer`], band-limited
//!   synthesis ([`blep`]) and the analog output stage ([`filter`]).
//!
//! `no_std`, and no allocation after construction. See ENGINE.md.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

pub mod blep;
pub mod dmc;
pub mod driver;
pub mod filter;
pub mod frame_counter;
pub mod math;
pub mod mixer;
pub mod noise;
pub mod pitch;
pub mod player;
pub mod pulse;
pub mod samples;
pub mod synth;
pub mod tables;
pub mod triangle;
pub mod units;
pub mod vrc6;
