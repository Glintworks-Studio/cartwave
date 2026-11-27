//! Batch 4c §4: the authenticity A/B pass (internal tool; docs/ab/AB.md).
//!
//! Feeds the same register writes to our emulator and to external reference
//! players. Everything here is ours: the songs (the demo, the showcase), the
//! test patterns, the register logs, and the NSF container (a 6502 player we
//! wrote, replaying a log at the frame rate). No game ROM, rip or copyrighted
//! music is used.
//!
//!   cargo run --release -p ui --example ab -- <command> ...
//!     log demo|showcase <out.log>    our driver's register writes per frame tick
//!                                    (the console's tracks) and the sample ROM
//!     patterns <dir>                 every targeted test pattern: <name>.log + .seg
//!     check demo|showcase            the log replayed == the app's own render
//!     replay <log> <out.wav> [off|nes|fc] [nsfinit]  our emulator, 48 kHz float
//!     solo <log> <out.log> <group>   the log with one channel's writes (and $4015, $4017)
//!     nsf <log> <out.nsf>            the log in our NSF player
//!
//! The log format (text): `cartwave-reglog 1`, `vrc6 0|1`, `rom <32768 hex>`,
//! then one `f` line per frame tick with its writes as `addr:value` (hex).

use std::fmt::Write as _;
use std::path::Path;

use chip::filter::FilterMode;
use chip::synth::Synth;
use chip::tables::CPU_HZ_NTSC;

const SR: f64 = 48_000.0;
/// A frame tick is 29780.5 CPU cycles (the player's schedule, in half cycles).
const FRAME_X2: u64 = 59_561;

#[derive(Clone)]
struct Log {
    vrc6: bool,
    rom: Vec<u8>,
    frames: Vec<Vec<(u16, u8)>>,
}

impl Log {
    fn save(&self, p: &Path) {
        let mut s = String::new();
        let _ = writeln!(s, "cartwave-reglog 1");
        let _ = writeln!(s, "vrc6 {}", self.vrc6 as u8);
        let _ = write!(s, "rom ");
        for b in &self.rom {
            let _ = write!(s, "{b:02x}");
        }
        s.push('\n');
        for f in &self.frames {
            s.push('f');
            for (a, v) in f {
                let _ = write!(s, " {a:04x}:{v:02x}");
            }
            s.push('\n');
        }
        std::fs::write(p, s).expect("write log");
    }

    fn load(p: &Path) -> Log {
        let t = std::fs::read_to_string(p).expect("read log");
        let mut l = Log { vrc6: false, rom: vec![0; chip::dmc::ROM_SIZE], frames: Vec::new() };
        for line in t.lines() {
            if let Some(v) = line.strip_prefix("vrc6 ") {
                l.vrc6 = v == "1";
            } else if let Some(h) = line.strip_prefix("rom ") {
                l.rom = (0..h.len() / 2).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap()).collect();
            } else if let Some(rest) = line.strip_prefix('f') {
                l.frames.push(
                    rest.split_whitespace()
                        .map(|w| {
                            let (a, v) = w.split_once(':').unwrap();
                            (u16::from_str_radix(a, 16).unwrap(), u8::from_str_radix(v, 16).unwrap())
                        })
                        .collect(),
                );
            }
        }
        l
    }
}

/// The two songs we own, as the app opens them.
fn project(name: &str) -> ui::file::Project {
    match name {
        "demo" => {
            let mut p = ui::file::Project::new_legacy();
            p.arrangement = ui::mock::demo_arrangement();
            p
        }
        "showcase" => {
            // As `examples/render.rs` makes docs/screens/product/showcase.wav:
            // the MIDI tune plus our own `hey.wav` as a DPCM sample.
            let song = ui::midi::parse(&std::fs::read("docs/songs/showcase.mid").unwrap()).unwrap();
            let imp = ui::midi::arrange(&song, true);
            let mut p = ui::file::Project::new_legacy();
            p.arrangement = imp.arrangement;
            p.bpm = imp.bpm.unwrap_or(p.bpm);
            p.loop_end_ticks = imp.loop_end;
            let (pcm, sr) = ui::dpcm::read_wav(&std::fs::read("docs/songs/hey.wav").unwrap()).unwrap();
            let slot = ui::dpcm::free_slot(&p.arrangement.samples).unwrap();
            let (s, _) = ui::dpcm::convert("hey", &pcm, sr, ui::dpcm::free_bytes(&p.arrangement.samples)).unwrap();
            p.arrangement.samples.resize(chip::samples::USER_SLOTS, None);
            p.arrangement.samples[slot] = Some(s);
            p
        }
        _ => panic!("unknown song {name} (demo | showcase)"),
    }
}

/// Our driver's writes per frame tick, and the app's own render of the same
/// song (export's path: from the start to the loop end plus a 2 s tail).
fn log_song(name: &str) -> (Log, Vec<f32>, FilterMode) {
    let p = project(name);
    assert!(p.arrangement.lanes.iter().all(|l| l.lock.is_some()), "{name}: only console (locked) tracks can be logged");
    let heard = ui::tracks::audible(&p.arrangement, &p.mute, &p.solo);
    let (mut m, snap) = ui::tracks::offline_mixdown(&p, SR as u32, &heard, true);
    let rom = m.player.synth.rom.to_vec();
    let vrc6 = m.player.synth.vrc6_on();
    let seconds = p.loop_end_ticks as f64 / ui::state::PPQ as f64 * 60.0 / p.bpm as f64;
    let total = ((seconds + 2.0) * SR) as usize;
    let song = (seconds * SR) as usize;
    m.player.set_loop_end(1.0e12);
    m.player.play();
    let mut out = vec![0f32; total];
    let mut frames = Vec::new();
    let mut ticks = m.player.driver.ticks;
    for i in 0..total {
        if i >= song && m.player.playing() {
            m.player.stop();
        }
        m.render(&snap, &mut out[i..i + 1]);
        if m.player.driver.ticks != ticks {
            assert_eq!(m.player.driver.ticks, ticks + 1);
            ticks = m.player.driver.ticks;
            frames.push(m.player.tick_writes().to_vec());
        }
    }
    (Log { vrc6, rom, frames }, out, p.filter)
}

/// Our emulator replaying `log` on the player's frame schedule. `nsf_init`:
/// first the writes an NSF player makes before INIT (the NSF page on the
/// NESdev wiki: $4000-$4013 = 0, $4015 = $0F, $4017 = $40).
fn replay(log: &Log, filter: FilterMode, nsf_init: bool, tail_s: f64) -> Vec<f32> {
    let mut s = Box::new(Synth::new(SR));
    s.rom.copy_from_slice(&log.rom);
    s.set_vrc6(log.vrc6);
    s.set_filter(filter);
    if nsf_init {
        for a in 0x4000..=0x4013u16 {
            s.write(a, 0);
        }
        s.write(0x4015, 0x0F);
        s.write(0x4017, 0x40);
    }
    let secs = log.frames.len() as f64 * FRAME_X2 as f64 / 2.0 / CPU_HZ_NTSC + tail_s;
    let mut out = vec![0f32; (secs * SR) as usize];
    let mut next_x2 = 0u64;
    let mut k = 0usize;
    for chunk in out.chunks_mut(chip::blep::MAX_FRAMES) {
        let end = s.cycle_for_samples(chunk.len());
        while next_x2 / 2 < end && k < log.frames.len() {
            s.run_until(next_x2 / 2);
            for &(a, v) in &log.frames[k] {
                s.write(a, v);
            }
            k += 1;
            next_x2 += FRAME_X2;
        }
        s.render(chunk);
    }
    out
}

fn filter_of(s: &str) -> FilterMode {
    match s {
        "off" => FilterMode::Off,
        "nes" => FilterMode::Nes,
        "440" => FilterMode::NesWith440,
        "fc" => FilterMode::Famicom,
        _ => panic!("filter: off | nes | 440 | fc"),
    }
}

fn write_wav_f32(p: &Path, x: &[f32]) {
    let mut b = Vec::with_capacity(44 + x.len() * 4);
    let n = (x.len() * 4) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + n).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&(SR as u32).to_le_bytes());
    b.extend_from_slice(&(SR as u32 * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&n.to_le_bytes());
    for v in x {
        b.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(p, b).expect("write wav");
}

/// Register groups, for one channel's writes.
fn group_of(a: u16) -> &'static str {
    match a {
        0x4000..=0x4003 => "p1",
        0x4004..=0x4007 => "p2",
        0x4008..=0x400B => "tri",
        0x400C..=0x400F => "noise",
        0x4010..=0x4013 => "dmc",
        0x9000..=0x9003 => "v1",
        0xA000..=0xA002 => "v2",
        0xB000..=0xB002 => "saw",
        _ => "ctl",
    }
}

// ---------------------------------------------------------------- the NSF

/// A minimal 6502 assembler: bytes with labels and fix-ups.
struct Asm {
    org: u16,
    b: Vec<u8>,
    labels: Vec<(&'static str, u16)>,
    fix: Vec<(usize, &'static str, bool)>,
}

impl Asm {
    fn pc(&self) -> u16 {
        self.org + self.b.len() as u16
    }
    fn label(&mut self, n: &'static str) {
        let pc = self.pc();
        self.labels.push((n, pc));
    }
    fn op(&mut self, bytes: &[u8]) {
        self.b.extend_from_slice(bytes);
    }
    /// An opcode with a 16-bit address operand that is a label.
    fn abs(&mut self, opc: u8, l: &'static str) {
        self.b.push(opc);
        self.fix.push((self.b.len(), l, false));
        self.b.extend_from_slice(&[0, 0]);
    }
    /// A branch to a label.
    fn br(&mut self, opc: u8, l: &'static str) {
        self.b.push(opc);
        self.fix.push((self.b.len(), l, true));
        self.b.push(0);
    }
    fn finish(mut self) -> (Vec<u8>, Vec<(&'static str, u16)>) {
        for &(at, l, rel) in &self.fix {
            let t = self.labels.iter().find(|x| x.0 == l).unwrap_or_else(|| panic!("label {l}")).1;
            if rel {
                let d = t as i32 - (self.org as i32 + at as i32 + 1);
                assert!((-128..=127).contains(&d), "branch to {l} out of range");
                self.b[at] = d as i8 as u8;
            } else {
                self.b[at..at + 2].copy_from_slice(&t.to_le_bytes());
            }
        }
        (self.b, self.labels)
    }
}

/// Register index -> address (the stream stores one byte per register).
const REGS: [u16; 34] = [
    0x4000, 0x4001, 0x4002, 0x4003, 0x4004, 0x4005, 0x4006, 0x4007, 0x4008, 0x4009, 0x400A, 0x400B, 0x400C, 0x400D,
    0x400E, 0x400F, 0x4010, 0x4011, 0x4012, 0x4013, 0x4014, 0x4015, 0x4016, 0x4017, 0x9000, 0x9001, 0x9002, 0x9003,
    0xA000, 0xA001, 0xA002, 0xB000, 0xB001, 0xB002,
];

/// The NSF: bank 0 our player at $8000; banks 1-4 the sample ROM at
/// $C000-$FFFF (where the DMC reads); banks 5.. the register stream, read
/// through a 4 KiB window at $9000 switched with $5FF9 (the NSF page's
/// bankswitching). Per frame: (register index, value) pairs, then $FF; the
/// end is $FE. PLAY runs at the header's NTSC rate (16639 µs = 60.0988 Hz).
fn build_nsf(log: &Log, title: &str) -> Vec<u8> {
    let mut a = Asm { org: 0x8000, b: Vec::new(), labels: Vec::new(), fix: Vec::new() };
    // Zero page: $00/$01 stream pointer, $02 stream bank, $03 ended, $04/$05 target.
    a.label("init");
    a.op(&[0xA9, 0x00, 0x85, 0x00]); // LDA #0; STA $00
    a.op(&[0xA9, 0x90, 0x85, 0x01]); // LDA #$90; STA $01
    a.op(&[0xA9, 0x05, 0x85, 0x02, 0x8D, 0xF9, 0x5F]); // LDA #5; STA $02; STA $5FF9
    a.op(&[0xA9, 0x00, 0x85, 0x03]); // LDA #0; STA $03
    a.op(&[0x60]); // RTS
    a.label("play");
    a.op(&[0xA5, 0x03]); // LDA $03
    a.br(0xD0, "done"); // BNE done
    a.label("loop");
    a.abs(0x20, "get"); // JSR get
    a.op(&[0xC9, 0xFF]); // CMP #$FF
    a.br(0xF0, "done"); // BEQ done
    a.op(&[0xC9, 0xFE]); // CMP #$FE
    a.br(0xF0, "end"); // BEQ end
    a.op(&[0xAA]); // TAX
    a.abs(0xBD, "lo"); // LDA lo,X
    a.op(&[0x85, 0x04]); // STA $04
    a.abs(0xBD, "hi"); // LDA hi,X
    a.op(&[0x85, 0x05]); // STA $05
    a.abs(0x20, "get"); // JSR get
    a.op(&[0xA0, 0x00, 0x91, 0x04]); // LDY #0; STA ($04),Y
    a.abs(0x4C, "loop"); // JMP loop
    a.label("end");
    a.op(&[0xA9, 0x01, 0x85, 0x03]); // LDA #1; STA $03
    a.label("done");
    a.op(&[0x60]); // RTS
    // get: A = the next stream byte; the pointer advances (and the bank at $A000).
    a.label("get");
    a.op(&[0xA0, 0x00, 0xB1, 0x00]); // LDY #0; LDA ($00),Y
    a.op(&[0xE6, 0x00]); // INC $00
    a.br(0xD0, "ret"); // BNE ret
    a.op(&[0xE6, 0x01, 0x48, 0xA5, 0x01, 0xC9, 0xA0]); // INC $01; PHA; LDA $01; CMP #$A0
    a.br(0xD0, "pla"); // BNE pla
    a.op(&[0xA9, 0x90, 0x85, 0x01, 0xE6, 0x02, 0xA5, 0x02, 0x8D, 0xF9, 0x5F]); // LDA #$90; STA $01; INC $02; LDA $02; STA $5FF9
    a.label("pla");
    a.op(&[0x68]); // PLA
    a.label("ret");
    a.op(&[0x60]); // RTS
    a.label("lo");
    for r in REGS {
        a.op(&[(r & 0xFF) as u8]);
    }
    a.label("hi");
    for r in REGS {
        a.op(&[(r >> 8) as u8]);
    }
    let (mut code, labels) = a.finish();
    let at = |n: &str| labels.iter().find(|x| x.0 == n).unwrap().1;
    code.resize(0x1000, 0);
    let mut stream = Vec::new();
    for f in &log.frames {
        for &(addr, v) in f {
            let i = REGS.iter().position(|r| *r == addr).unwrap_or_else(|| panic!("register {addr:04x} not in the player's table"));
            stream.push(i as u8);
            stream.push(v);
        }
        stream.push(0xFF);
    }
    stream.push(0xFE);
    stream.resize(stream.len().div_ceil(0x1000) * 0x1000, 0xFE);
    let mut h = vec![0u8; 0x80];
    h[0..5].copy_from_slice(b"NESM\x1A");
    h[5] = 1; // version
    h[6] = 1; // songs
    h[7] = 1; // first song
    h[8..10].copy_from_slice(&0x8000u16.to_le_bytes());
    h[10..12].copy_from_slice(&at("init").to_le_bytes());
    h[12..14].copy_from_slice(&at("play").to_le_bytes());
    let put = |h: &mut Vec<u8>, off: usize, s: &str| {
        for (i, c) in s.bytes().take(31).enumerate() {
            h[off + i] = c;
        }
    };
    put(&mut h, 0x0E, title);
    put(&mut h, 0x2E, "Glintworks (test material)");
    put(&mut h, 0x4E, "2026 Glintworks, MIT OR Apache-2.0");
    h[0x6E..0x70].copy_from_slice(&16639u16.to_le_bytes());
    h[0x70..0x78].copy_from_slice(&[0, 5, 0, 0, 1, 2, 3, 4]);
    h[0x78..0x7A].copy_from_slice(&19997u16.to_le_bytes());
    h[0x7A] = 0; // NTSC
    h[0x7B] = log.vrc6 as u8; // expansion: VRC6
    let mut out = h;
    out.extend_from_slice(&code);
    assert_eq!(log.rom.len(), 0x4000);
    out.extend_from_slice(&log.rom);
    out.extend_from_slice(&stream);
    out
}

// ------------------------------------------------------------ the patterns

/// A test pattern: frames of writes plus labelled segments (first frame,
/// frames, kind, label) that the comparison measures one by one.
struct Pat {
    log: Log,
    segs: Vec<(usize, usize, &'static str, String)>,
}

impl Pat {
    fn new(vrc6: bool) -> Self {
        let mut rom = vec![0u8; chip::dmc::ROM_SIZE];
        let mut r = [0u8; chip::dmc::ROM_SIZE];
        chip::samples::build(&mut r);
        rom.copy_from_slice(&r);
        Pat { log: Log { vrc6, rom, frames: Vec::new() }, segs: Vec::new() }
    }
    fn now(&self) -> usize {
        self.log.frames.len()
    }
    /// Writes at the next frame.
    fn w(&mut self, ws: &[(u16, u8)]) {
        self.log.frames.push(ws.to_vec());
    }
    fn wait(&mut self, n: usize) {
        for _ in 0..n {
            self.log.frames.push(Vec::new());
        }
    }
    /// `ws` at the next frame, held `n` frames as a measured segment.
    fn seg(&mut self, kind: &'static str, label: String, ws: &[(u16, u8)], n: usize) {
        let at = self.now();
        self.w(ws);
        self.wait(n - 1);
        self.segs.push((at, n, kind, label));
    }
    fn save(&self, dir: &Path, name: &str) {
        self.log.save(&dir.join(format!("{name}.log")));
        let mut s = String::new();
        for (a, n, k, l) in &self.segs {
            let _ = writeln!(s, "{a}\t{n}\t{k}\t{l}");
        }
        std::fs::write(dir.join(format!("{name}.seg")), s).unwrap();
    }
}

fn pulse_t(f: f64) -> u16 {
    (CPU_HZ_NTSC / (16.0 * f) - 1.0).round() as u16
}
fn tri_t(f: f64) -> u16 {
    (CPU_HZ_NTSC / (32.0 * f) - 1.0).round() as u16
}
fn vrc6_t(f: f64) -> u16 {
    (CPU_HZ_NTSC / (16.0 * f) - 1.0).round() as u16
}
fn saw_t(f: f64) -> u16 {
    (CPU_HZ_NTSC / (14.0 * f) - 1.0).round() as u16
}
fn midi_hz(n: i32) -> f64 {
    440.0 * 2f64.powf((n - 69) as f64 / 12.0)
}
/// A 2A03 pulse note: (reg base, duty, volume, period) with a length-halted
/// constant volume; writing the high byte restarts the phase.
fn pulse(base: u16, duty: u8, vol: u8, t: u16) -> Vec<(u16, u8)> {
    vec![(base, duty << 6 | 0x30 | vol), (base + 1, 0x08), (base + 2, (t & 0xFF) as u8), (base + 3, (t >> 8) as u8 | 0xF8)]
}

fn patterns(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let off = |base: u16| vec![(base, 0x30u8)];
    // Pulse 1: the four duties at A4, then pitches A2..A6, volume steps.
    let mut p = Pat::new(false);
    p.w(&[(0x4015, 0x01)]);
    for d in 0..4u8 {
        p.seg("tone", format!("p1 duty {d} A4"), &pulse(0x4000, d, 15, pulse_t(440.0)), 60);
        p.seg("gap", "silence".into(), &off(0x4000), 15);
    }
    for n in [45, 52, 57, 64, 69, 76, 81, 88, 93] {
        p.seg("tone", format!("p1 duty 2 midi {n}"), &pulse(0x4000, 2, 15, pulse_t(midi_hz(n))), 30);
    }
    p.seg("gap", "silence".into(), &off(0x4000), 15);
    for v in (0..16u8).rev() {
        p.seg("level", format!("p1 vol {v}"), &pulse(0x4000, 2, v, pulse_t(440.0)), 20);
    }
    p.save(dir, "pulse1");
    // Pulse 2: the same duties (the two pulses mix through one DAC).
    let mut p = Pat::new(false);
    p.w(&[(0x4015, 0x02)]);
    for d in 0..4u8 {
        p.seg("tone", format!("p2 duty {d} A4"), &pulse(0x4004, d, 15, pulse_t(440.0)), 60);
        p.seg("gap", "silence".into(), &off(0x4004), 15);
    }
    p.save(dir, "pulse2");
    // Envelopes: decay at four rates and a looped one; the length counter.
    let mut p = Pat::new(false);
    p.w(&[(0x4015, 0x01)]);
    for v in [0u8, 3, 7, 15] {
        let t = pulse_t(440.0);
        p.seg("decay", format!("p1 decay period {v}"), &[(0x4000, 0x80 | 0x20 | v), (0x4001, 0x08), (0x4002, (t & 0xFF) as u8), (0x4003, (t >> 8) as u8 | 0xF8)], 90);
    }
    let t = pulse_t(440.0);
    p.seg("tone", "p1 looped decay period 2".into(), &[(0x4000, 0x80 | 0x20 | 2), (0x4002, (t & 0xFF) as u8), (0x4003, (t >> 8) as u8 | 0xF8)], 90);
    for li in [0x01u8, 0x03, 0x05, 0x09] {
        // Length counter on (no halt), constant volume: the note stops by itself.
        p.seg("length", format!("p1 length index {li}"), &[(0x4000, 0x80 | 0x10 | 15), (0x4002, (t & 0xFF) as u8), (0x4003, (t >> 8) as u8 | li << 3)], 60);
    }
    p.save(dir, "envelope");
    // Sweeps: up and down on both pulses (pulse 1 negates with ones' complement).
    let mut p = Pat::new(false);
    p.w(&[(0x4015, 0x03)]);
    for (base, name) in [(0x4000u16, "p1"), (0x4004, "p2")] {
        for (sw, lab) in [(0x80 | 0x20 | 0x08 | 0x02, "down shift 2 period 2"), (0x80 | 0x40 | 0x02, "up shift 2 period 4"), (0x80 | 0x70 | 0x01, "up shift 1 period 7")] {
            let t = if sw & 0x08 != 0 { pulse_t(220.0) } else { pulse_t(880.0) };
            p.seg("sweep", format!("{name} sweep {lab}"), &[(base, 0x80 | 0x30 | 15), (base + 1, sw), (base + 2, (t & 0xFF) as u8), (base + 3, (t >> 8) as u8 | 0xF8)], 90);
            p.seg("gap", "silence".into(), &[(base, 0x30), (base + 1, 0x08)], 15);
        }
    }
    p.save(dir, "sweep");
    // Triangle: pitches over its range, then the linear counter cutting a note.
    let mut p = Pat::new(false);
    p.w(&[(0x4015, 0x04)]);
    for n in [28, 36, 40, 45, 48, 52, 57, 64, 69, 76, 81] {
        let t = tri_t(midi_hz(n));
        p.seg("tone", format!("tri midi {n}"), &[(0x4008, 0xFF), (0x400A, (t & 0xFF) as u8), (0x400B, (t >> 8) as u8 | 0xF8)], 30);
    }
    p.seg("gap", "tri halt".into(), &[(0x4008, 0x80)], 15);
    let t = tri_t(220.0);
    p.seg("length", "tri linear counter 40".into(), &[(0x4008, 40), (0x400A, (t & 0xFF) as u8), (0x400B, (t >> 8) as u8 | 0xF8)], 30);
    p.save(dir, "triangle");
    // Noise: every period, long and short mode, then a decay.
    for (mode, name) in [(0u8, "noise_long"), (0x80, "noise_short")] {
        let mut p = Pat::new(false);
        p.w(&[(0x4015, 0x08)]);
        for per in 0..16u8 {
            p.seg("noise", format!("noise mode {} period {per}", mode >> 7), &[(0x400C, 0x30 | 15), (0x400E, mode | per), (0x400F, 0xF8)], 30);
        }
        p.seg("decay", "noise decay period 4".into(), &[(0x400C, 0x20 | 4), (0x400E, mode | 4), (0x400F, 0xF8)], 60);
        p.save(dir, name);
    }
    // DPCM: direct-load levels (steps up and down), then the built-in samples
    // at three rates.
    let mut p = Pat::new(false);
    p.w(&[(0x4015, 0x00), (0x4011, 0)]);
    p.wait(20);
    for lv in [16u8, 32, 64, 96, 127, 96, 64, 32, 0] {
        p.seg("step", format!("dmc $4011 = {lv}"), &[(0x4011, lv)], 20);
    }
    let mut r = [0u8; chip::dmc::ROM_SIZE];
    let refs = chip::samples::build(&mut r);
    for (i, s) in refs.iter().take(chip::samples::BUILTIN_COUNT).enumerate() {
        for rate in [0x0Fu8, 0x0C, 0x08] {
            p.seg("dmc", format!("dmc sample {i} rate {rate}"), &[(0x4015, 0x00), (0x4011, 64), (0x4010, rate), (0x4012, s.addr_reg), (0x4013, s.len_reg), (0x4015, 0x10)], 45);
        }
    }
    p.save(dir, "dpcm");
    // The 2A03 mixer: pulse pairs at several volumes (each alone, then both),
    // a pulse with the triangle, and the triangle and noise under DMC levels
    // (the DMC's level changes their share in the non-linear mixer).
    let mut p = Pat::new(false);
    p.w(&[(0x4015, 0x0F)]);
    let (t1, t2) = (pulse_t(440.0), pulse_t(330.0));
    for (v1, v2) in [(15u8, 15u8), (15, 8), (8, 8), (4, 4), (15, 2)] {
        let mut a = pulse(0x4000, 2, v1, t1);
        a.extend(pulse(0x4004, 2, 0, t2));
        p.seg("level", format!("mix p1 {v1} alone"), &a, 20);
        let mut b = pulse(0x4000, 2, 0, t1);
        b.extend(pulse(0x4004, 2, v2, t2));
        p.seg("level", format!("mix p2 {v2} alone"), &b, 20);
        let mut c = pulse(0x4000, 2, v1, t1);
        c.extend(pulse(0x4004, 2, v2, t2));
        p.seg("level", format!("mix p1 {v1} + p2 {v2}"), &c, 20);
    }
    let mut z = pulse(0x4000, 2, 0, t1);
    z.extend(pulse(0x4004, 2, 0, t2));
    p.seg("gap", "silence".into(), &z, 15);
    let tt = tri_t(110.0);
    p.seg("level", "mix tri alone".into(), &[(0x4008, 0xFF), (0x400A, (tt & 0xFF) as u8), (0x400B, (tt >> 8) as u8 | 0xF8)], 20);
    let mut a = pulse(0x4000, 2, 15, t1);
    a.push((0x4008, 0x80));
    p.seg("level", "mix p1 15 alone (2)".into(), &a, 20);
    let mut a = pulse(0x4000, 2, 15, t1);
    a.extend([(0x4008, 0xFF), (0x400A, (tt & 0xFF) as u8), (0x400B, (tt >> 8) as u8 | 0xF8)]);
    p.seg("level", "mix p1 15 + tri".into(), &a, 20);
    p.w(&[(0x4000, 0x30)]);
    for lv in [0u8, 64, 127] {
        p.seg("level", format!("mix tri + noise under dmc {lv}"), &[(0x4011, lv), (0x400C, 0x30 | 15), (0x400E, 0x04), (0x400F, 0xF8), (0x4008, 0xFF), (0x400A, (tt & 0xFF) as u8), (0x400B, (tt >> 8) as u8 | 0xF8)], 30);
    }
    p.save(dir, "mixer");
    // VRC6: both pulses' eight duties and the digitized mode, volume steps,
    // the saw at several rates (42 is its full-scale rate, above it wraps),
    // and a 2A03 pulse against a VRC6 pulse (their relative level).
    let mut p = Pat::new(true);
    p.w(&[(0x4015, 0x00)]);
    let tv = vrc6_t(440.0);
    for (base, name) in [(0x9000u16, "v1"), (0xA000, "v2")] {
        for d in 0..8u8 {
            p.seg("tone", format!("{name} duty {d}"), &[(base, d << 4 | 15), (base + 1, (tv & 0xFF) as u8), (base + 2, 0x80 | (tv >> 8) as u8)], 45);
        }
        p.seg("level", format!("{name} digitized vol 15"), &[(base, 0x80 | 15)], 20);
        p.seg("gap", "silence".into(), &[(base, 0), (base + 2, 0)], 10);
    }
    for v in (0..16u8).rev() {
        p.seg("level", format!("v1 vol {v}"), &[(0x9000, 7 << 4 | v), (0x9001, (tv & 0xFF) as u8), (0x9002, 0x80 | (tv >> 8) as u8)], 15);
    }
    p.w(&[(0x9000, 0), (0x9002, 0)]);
    let ts = saw_t(220.0);
    for rate in [8u8, 16, 24, 32, 42, 50, 63] {
        p.seg("tone", format!("saw rate {rate} A3"), &[(0xB000, rate), (0xB001, (ts & 0xFF) as u8), (0xB002, 0x80 | (ts >> 8) as u8)], 45);
    }
    for n in [33, 45, 57, 69, 81] {
        let t = saw_t(midi_hz(n));
        p.seg("tone", format!("saw rate 42 midi {n}"), &[(0xB000, 42), (0xB001, (t & 0xFF) as u8), (0xB002, 0x80 | (t >> 8) as u8)], 30);
    }
    p.w(&[(0xB002, 0)]);
    p.w(&[(0x4015, 0x01)]);
    let t1 = pulse_t(440.0);
    p.seg("level", "rel 2A03 p1 duty 2 vol 15 alone".into(), &pulse(0x4000, 2, 15, t1), 30);
    p.seg("level", "rel VRC6 v1 duty 7 vol 15 alone".into(), &[(0x4000, 0x30), (0x9000, 7 << 4 | 15), (0x9001, (tv & 0xFF) as u8), (0x9002, 0x80 | (tv >> 8) as u8)], 30);
    p.seg("level", "rel VRC6 saw rate 42 alone".into(), &[(0x9002, 0), (0xB000, 42), (0xB001, (ts & 0xFF) as u8), (0xB002, 0x80 | (ts >> 8) as u8)], 30);
    let mut both = pulse(0x4000, 2, 15, t1);
    both.extend([(0xB002, 0), (0x9000, 7 << 4 | 15), (0x9001, (tv & 0xFF) as u8), (0x9002, 0x80 | (tv >> 8) as u8)]);
    p.seg("level", "rel 2A03 p1 + VRC6 v1".into(), &both, 30);
    p.save(dir, "vrc6");
    println!("patterns written to {}", dir.display());
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| a.get(i).cloned().unwrap_or_else(|| panic!("missing argument {i}"));
    match a.first().map(|s| s.as_str()) {
        Some("log") => {
            let (log, _, _) = log_song(&arg(1));
            log.save(Path::new(&arg(2)));
            let writes: usize = log.frames.iter().map(|f| f.len()).sum();
            println!("{}: {} frame ticks, {writes} writes, vrc6 {}", arg(1), log.frames.len(), log.vrc6);
        }
        Some("check") => {
            // Proof the log is complete: replaying it into a fresh emulator
            // gives the app's own render of the song, sample for sample.
            let (log, app, filter) = log_song(&arg(1));
            let ours = replay(&log, filter, false, 0.0);
            let n = ours.len().min(app.len());
            let same = ours[..n].iter().zip(&app[..n]).filter(|(x, y)| x.to_bits() == y.to_bits()).count();
            let maxd = ours[..n].iter().zip(&app[..n]).fold(0f32, |m, (x, y)| m.max((x - y).abs()));
            println!("{}: {n} samples compared, {same} bit-identical, max |diff| {maxd:e} (filter {filter:?})", arg(1));
            assert_eq!(same, n, "the replay must equal the app's render");
        }
        Some("patterns") => patterns(Path::new(&arg(1))),
        Some("replay") => {
            let log = Log::load(Path::new(&arg(1)));
            let f = a.get(3).map_or(FilterMode::Off, |s| filter_of(s));
            let init = a.get(4).is_some_and(|s| s == "nsfinit");
            let x = replay(&log, f, init, 1.0);
            write_wav_f32(Path::new(&arg(2)), &x);
            println!("{} samples -> {}", x.len(), arg(2));
        }
        Some("solo") => {
            let mut log = Log::load(Path::new(&arg(1)));
            let g = arg(3);
            for f in &mut log.frames {
                f.retain(|(a, _)| group_of(*a) == g || group_of(*a) == "ctl");
            }
            log.save(Path::new(&arg(2)));
        }
        Some("nsf") => {
            let log = Log::load(Path::new(&arg(1)));
            let title = Path::new(&arg(1)).file_stem().unwrap().to_string_lossy().to_string();
            let b = build_nsf(&log, &title);
            std::fs::write(arg(2), &b).unwrap();
            println!("{} bytes ({} frames) -> {}", b.len(), log.frames.len(), arg(2));
        }
        _ => eprintln!("usage: ab log|check|patterns|replay|solo|nsf ... (see the file's header)"),
    }
}
