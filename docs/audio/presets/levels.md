# The preset bank: levels

Written by `cargo run --release -p ui --example bank_pack -- --write`. Each preset plays its role's phrase at 150 BPM (48 kHz); authentic presets on a locked track (the console's mixer and filter), the others on a studio track. Loudness is EBU R128 integrated (LUFS), true peak in dBTP, sample peak in dBFS.

**202 presets (141 new, 61 classic); clipping: none; new presets within ±1.5 LU of their category's target: 112 of 141 (largest deviation 6.0 LU); authentic presets identical on a studio voice: 0 of 42.**

Level matching: a category's target is fixed from the first pass at the 35th percentile of its new presets (a level the quieter voices can reach); `presets/levels.json` holds the gains `design_bank.py --levels` folded in. The classic presets (Batch 1) are listed but not changed (RUNLOG PB-D6).

| Category | Target (LUFS) |
|---|---|
| Drums / Hits | -29.5 |
| Drums / Kits | -26.2 |
| Hardware / Famicom + VRC6 / VRC6 Pulse | -26.3 |
| Hardware / Famicom + VRC6 / VRC6 Saw | -22.3 |
| Hardware / NES (2A03) / DPCM | -32.0 |
| Hardware / NES (2A03) / Noise | -31.5 |
| Hardware / NES (2A03) / Pulse | -27.8 |
| Hardware / NES (2A03) / Triangle | -28.0 |
| Sounds / Arps | -25.8 |
| Sounds / Basses | -25.1 |
| Sounds / Bells & Keys | -26.3 |
| Sounds / FX | -26.6 |
| Sounds / Game Sounds | -30.1 |
| Sounds / Leads | -25.1 |
| Sounds / Pads | -26.1 |

| Preset | Category | Bank | Loudness (LUFS) | vs target (LU) | True peak | Sample peak | Clips | Authentic on a studio voice |
|---|---|---|---|---|---|---|---|---|
| Bright Square Lead | Sounds / Leads | new | -25.5 | -0.4 | -12.4 | -12.5 | no | – |
| Late Vibrato Lead | Sounds / Leads | new | -25.0 | +0.1 | -14.6 | -14.6 | no | – |
| Duty Sweep Lead | Sounds / Leads | new | -25.0 | +0.1 | -12.5 | -12.8 | no | – |
| Hollow 12.5% | Sounds / Leads | new | -25.6 | -0.5 | -12.8 | -12.8 | no | – |
| Nasal Reed Lead | Sounds / Leads | new | -25.3 | -0.2 | -13.0 | -13.2 | no | – |
| Glass Pulse Lead | Sounds / Leads | new | -29.6 | -4.5 | -11.9 | -12.1 | no | – |
| Echo Square | Sounds / Leads | new | -27.2 | -2.1 | -13.0 | -13.2 | no | – |
| Echo Saw Lead | Sounds / Leads | new | -25.0 | +0.1 | -8.0 | -8.1 | no | – |
| Supersquare | Sounds / Leads | new | -24.7 | +0.4 | -9.2 | -9.4 | no | – |
| Wide Detuned Lead | Sounds / Leads | new | -24.8 | +0.3 | -9.7 | -9.7 | no | – |
| Detuned Saw Lead | Sounds / Leads | new | -25.0 | +0.1 | -8.4 | -8.5 | no | – |
| Saw Lead | Sounds / Leads | new | -25.4 | -0.3 | -11.7 | -12.0 | no | – |
| Scoop Lead | Sounds / Leads | new | -25.1 | -0.0 | -13.3 | -13.3 | no | – |
| Trill Lead | Sounds / Leads | new | -25.1 | +0.0 | -12.6 | -12.8 | no | – |
| Soft Triangle Lead | Sounds / Leads | new | -23.1 | +2.0 | -11.2 | -11.3 | no | – |
| Round Triangle Bass | Sounds / Basses | new | -24.3 | +0.8 | -9.3 | -9.3 | no | – |
| Kick-Drop Triangle Bass | Sounds / Basses | new | -24.4 | +0.7 | -12.2 | -12.3 | no | – |
| Staccato Triangle Bass | Sounds / Basses | new | -27.5 | -2.4 | -12.5 | -12.5 | no | – |
| Triangle Slide Bass | Sounds / Basses | new | -24.3 | +0.8 | -11.7 | -11.8 | no | – |
| Thick Pulse Bass | Sounds / Basses | new | -25.1 | -0.0 | -13.8 | -14.1 | no | – |
| Pluck Pulse Bass | Sounds / Basses | new | -29.9 | -4.8 | -15.5 | -15.6 | no | – |
| Growl Pulse Bass | Sounds / Basses | new | -27.1 | -2.0 | -12.4 | -12.7 | no | – |
| Octave Bounce Bass | Sounds / Basses | new | -25.0 | +0.1 | -13.8 | -13.9 | no | – |
| Hollow Bass | Sounds / Basses | new | -27.1 | -2.0 | -12.9 | -13.0 | no | – |
| Saw Sub Bass | Sounds / Basses | new | -25.4 | -0.3 | -12.6 | -13.2 | no | – |
| Saw Pluck Bass | Sounds / Basses | new | -25.1 | +0.0 | -10.8 | -11.0 | no | – |
| Detuned Saw Bass | Sounds / Basses | new | -25.2 | -0.1 | -10.7 | -10.7 | no | – |
| Wide Pulse Bass | Sounds / Basses | new | -24.9 | +0.2 | -12.8 | -12.8 | no | – |
| Rubber Bass | Sounds / Basses | new | -27.1 | -2.0 | -13.1 | -13.1 | no | – |
| Fast Major Arp | Sounds / Arps | new | -26.0 | -0.2 | -13.0 | -13.0 | no | – |
| Fast Minor Arp | Sounds / Arps | new | -26.0 | -0.2 | -13.0 | -13.1 | no | – |
| Major Seventh Arp | Sounds / Arps | new | -25.5 | +0.3 | -13.7 | -14.0 | no | – |
| Minor Seventh Arp | Sounds / Arps | new | -26.8 | -1.0 | -11.5 | -11.5 | no | – |
| Dominant Arp | Sounds / Arps | new | -25.8 | -0.0 | -11.9 | -11.9 | no | – |
| Diminished Arp | Sounds / Arps | new | -25.6 | +0.2 | -14.6 | -15.0 | no | – |
| Suspended Shimmer Arp | Sounds / Arps | new | -26.1 | -0.3 | -11.9 | -12.2 | no | – |
| Octave Pump Arp | Sounds / Arps | new | -25.5 | +0.3 | -14.9 | -14.9 | no | – |
| Power Fifth Arp | Sounds / Arps | new | -25.7 | +0.1 | -12.1 | -12.1 | no | – |
| Slow Major Arp | Sounds / Arps | new | -25.8 | -0.0 | -12.6 | -13.1 | no | – |
| Slow Minor Arp | Sounds / Arps | new | -25.3 | +0.5 | -14.1 | -14.2 | no | – |
| Rolling Arp | Sounds / Arps | new | -25.8 | -0.0 | -12.9 | -13.1 | no | – |
| Detuned Chord Arp | Sounds / Arps | new | -25.8 | +0.0 | -10.1 | -10.1 | no | – |
| Triangle Octave Arp | Sounds / Arps | new | -24.1 | +1.7 | -11.2 | -11.3 | no | – |
| Breathing Duty Pad | Sounds / Pads | new | -26.1 | +0.0 | -18.1 | -18.2 | no | – |
| Warm Square Pad | Sounds / Pads | new | -25.7 | +0.4 | -19.2 | -19.2 | no | – |
| Soft Choir Pad | Sounds / Pads | new | -26.1 | +0.0 | -20.9 | -21.0 | no | – |
| Glass Pad | Sounds / Pads | new | -26.7 | -0.6 | -14.1 | -14.1 | no | – |
| Wide Saw Pad | Sounds / Pads | new | -26.0 | +0.1 | -15.6 | -15.6 | no | – |
| Organ Pad | Sounds / Pads | new | -25.7 | +0.4 | -18.7 | -19.0 | no | – |
| Shimmer Pad | Sounds / Pads | new | -26.1 | -0.0 | -18.9 | -19.0 | no | – |
| Clear Bell | Sounds / Bells & Keys | new | -26.1 | +0.2 | -17.2 | -17.2 | no | – |
| Glass Bell | Sounds / Bells & Keys | new | -26.3 | +0.0 | -14.6 | -14.9 | no | – |
| Music Box Tine | Sounds / Bells & Keys | new | -28.3 | -2.0 | -15.2 | -15.3 | no | – |
| Electric Piano | Sounds / Bells & Keys | new | -25.8 | +0.5 | -14.0 | -14.0 | no | – |
| Harpsichord Pluck | Sounds / Bells & Keys | new | -26.9 | -0.6 | -15.5 | -15.8 | no | – |
| Vibraphone | Sounds / Bells & Keys | new | -25.8 | +0.5 | -16.1 | -16.2 | no | – |
| Saw Celesta | Sounds / Bells & Keys | new | -26.3 | +0.0 | -14.0 | -14.3 | no | – |
| Rising Sweep | Sounds / FX | new | -26.7 | -0.1 | -17.9 | -18.0 | no | – |
| Falling Bomb | Sounds / FX | new | -26.8 | -0.2 | -18.5 | -18.8 | no | – |
| Warp Down | Sounds / FX | new | -26.5 | +0.1 | -12.5 | -12.5 | no | – |
| Noise Riser | Sounds / FX | new | -26.1 | +0.5 | -20.5 | -20.5 | no | – |
| Ocean Wash | Sounds / FX | new | -26.7 | -0.1 | -19.4 | -19.6 | no | – |
| Static Crackle | Sounds / FX | new | -26.6 | -0.0 | -15.9 | -15.9 | no | – |
| Alarm | Sounds / FX | new | -26.7 | -0.1 | -17.2 | -17.3 | no | – |
| Teleport | Sounds / FX | new | -30.0 | -3.4 | -15.5 | -15.5 | no | – |
| Ray Gun | Sounds / FX | new | -26.7 | -0.1 | -13.4 | -13.4 | no | – |
| Heartbeat | Sounds / FX | new | -29.3 | -2.7 | -12.2 | -12.3 | no | – |
| Coin Chime | Sounds / Game Sounds | new | -30.3 | -0.2 | -19.4 | -19.5 | no | – |
| Hop | Sounds / Game Sounds | new | -31.0 | -0.9 | -15.5 | -15.6 | no | – |
| Impact Crunch | Sounds / Game Sounds | new | -30.0 | +0.1 | -13.7 | -13.8 | no | – |
| Cursor Blip | Sounds / Game Sounds | new | -35.2 | -5.1 | -15.5 | -15.5 | no | – |
| Rising Power | Sounds / Game Sounds | new | -30.5 | -0.4 | -21.4 | -21.7 | no | – |
| Extra Life | Sounds / Game Sounds | new | -30.0 | +0.1 | -22.6 | -22.6 | no | – |
| Boom | Sounds / Game Sounds | new | -30.3 | -0.2 | -18.5 | -19.1 | no | – |
| Zap Laser | Sounds / Game Sounds | new | -32.4 | -2.3 | -15.4 | -15.4 | no | – |
| Confirm Select | Sounds / Game Sounds | new | -30.1 | -0.0 | -16.5 | -16.6 | no | – |
| Error Buzz | Sounds / Game Sounds | new | -30.7 | -0.6 | -15.5 | -15.5 | no | – |
| Door Slide | Sounds / Game Sounds | new | -30.2 | -0.1 | -24.1 | -24.4 | no | – |
| Item Pickup | Sounds / Game Sounds | new | -29.6 | +0.5 | -16.7 | -16.7 | no | – |
| Punchy Noise Kit | Drums / Kits | new | -25.9 | +0.3 | -9.3 | -9.4 | no | – |
| Tight Noise Kit | Drums / Kits | new | -28.0 | -1.8 | -9.4 | -9.6 | no | – |
| Big Room Noise Kit | Drums / Kits | new | -25.8 | +0.4 | -10.4 | -10.4 | no | – |
| Metal Noise Kit | Drums / Kits | new | -26.2 | -0.0 | -9.2 | -9.5 | no | – |
| Thump Kick | Drums / Hits | new | -30.8 | -1.3 | -14.2 | -14.2 | no | – |
| Crack Snare | Drums / Hits | new | -29.1 | +0.4 | -15.4 | -15.7 | no | – |
| Fat Snare | Drums / Hits | new | -29.5 | +0.0 | -19.0 | -19.0 | no | – |
| Tick Hat | Drums / Hits | new | -30.3 | -0.8 | -12.5 | -12.5 | no | – |
| Sizzle Hat | Drums / Hits | new | -29.2 | +0.3 | -17.5 | -17.5 | no | – |
| Ride Ping | Drums / Hits | new | -29.8 | -0.3 | -16.6 | -16.6 | no | – |
| Splash Cymbal | Drums / Hits | new | -29.2 | +0.3 | -17.3 | -18.0 | no | – |
| Hand Clap | Drums / Hits | new | -29.4 | +0.1 | -17.7 | -17.8 | no | – |
| Shaker | Drums / Hits | new | -29.2 | +0.3 | -15.4 | -15.7 | no | – |
| Drop Kick | Drums / Hits | new | -29.1 | +0.4 | -12.3 | -12.3 | no | – |
| Low Tom | Drums / Hits | new | -27.7 | +1.8 | -12.5 | -12.5 | no | – |
| High Tom | Drums / Hits | new | -27.4 | +2.1 | -12.2 | -12.2 | no | – |
| Sampled Kick | Drums / Hits | new | -26.0 | +3.5 | -5.3 | -5.4 | no | – |
| Sampled Snare | Drums / Hits | new | -33.9 | -4.4 | -6.7 | -6.7 | no | – |
| Sampled Tom | Drums / Hits | new | -23.5 | +6.0 | -5.6 | -5.7 | no | – |
| Sampled Crash | Drums / Hits | new | -34.6 | -5.1 | -8.1 | -8.2 | no | – |
| Duty Lead | Hardware / NES (2A03) / Pulse | new | -27.8 | -0.0 | -17.7 | -17.9 | no | residual -4.6 dB, +0.32 LU |
| Square Melody | Hardware / NES (2A03) / Pulse | new | -27.5 | +0.3 | -21.9 | -22.1 | no | residual -5.7 dB, +0.40 LU |
| Thin Pulse Lead | Hardware / NES (2A03) / Pulse | new | -27.8 | +0.0 | -16.5 | -16.7 | no | residual -1.9 dB, +0.29 LU |
| Duty Swell Lead | Hardware / NES (2A03) / Pulse | new | -27.8 | +0.0 | -19.1 | -19.3 | no | residual -5.0 dB, +0.29 LU |
| Echo Pulse Lead | Hardware / NES (2A03) / Pulse | new | -28.5 | -0.7 | -16.0 | -16.1 | no | residual -7.1 dB, +0.14 LU |
| Pulse Pluck | Hardware / NES (2A03) / Pulse | new | -28.4 | -0.6 | -16.0 | -16.2 | no | residual -3.7 dB, +0.00 LU |
| Firm Pulse Bass | Hardware / NES (2A03) / Pulse | new | -27.9 | -0.1 | -17.7 | -17.8 | no | residual -5.9 dB, +0.25 LU |
| Major Chord Pulse | Hardware / NES (2A03) / Pulse | new | -27.8 | -0.0 | -17.7 | -17.9 | no | residual -4.8 dB, +0.11 LU |
| Minor Chord Pulse | Hardware / NES (2A03) / Pulse | new | -28.0 | -0.2 | -16.5 | -16.7 | no | residual -1.1 dB, +0.10 LU |
| Hardware Sweep Zap | Hardware / NES (2A03) / Pulse | new | -31.7 | -3.9 | -16.0 | -16.2 | no | residual -129.6 dB, -0.00 LU |
| Decay Envelope Pluck | Hardware / NES (2A03) / Pulse | new | -26.5 | +1.3 | -16.0 | -16.1 | no | residual -3.7 dB, +0.01 LU |
| Pulse Bell | Hardware / NES (2A03) / Pulse | new | -27.8 | -0.0 | -19.1 | -19.3 | no | residual -0.1 dB, +0.00 LU |
| Pure Triangle Bass | Hardware / NES (2A03) / Triangle | new | -26.0 | +2.0 | -18.3 | -18.3 | no | residual 3.4 dB, +0.01 LU |
| Triangle Kick Bass | Hardware / NES (2A03) / Triangle | new | -25.7 | +2.3 | -14.1 | -14.1 | no | residual -0.8 dB, -0.01 LU |
| Triangle Staccato | Hardware / NES (2A03) / Triangle | new | -28.3 | -0.3 | -17.0 | -17.0 | no | residual 1.5 dB, -0.01 LU |
| Triangle Flute | Hardware / NES (2A03) / Triangle | new | -23.2 | +4.8 | -14.1 | -14.1 | no | residual 4.3 dB, +0.00 LU |
| Triangle Kick | Hardware / NES (2A03) / Triangle | new | -31.2 | -3.2 | -15.7 | -15.8 | no | residual -127.0 dB, +0.00 LU |
| Triangle Tom | Hardware / NES (2A03) / Triangle | new | -27.7 | +0.3 | -15.8 | -15.9 | no | residual -128.3 dB, +0.00 LU |
| Noise Kit | Hardware / NES (2A03) / Noise | new | -31.0 | +0.5 | -18.2 | -18.5 | no | residual 5.0 dB, +3.58 LU |
| Short-Mode Kit | Hardware / NES (2A03) / Noise | new | -31.5 | -0.0 | -17.5 | -17.5 | no | residual 5.3 dB, +3.58 LU |
| Noise Snare | Hardware / NES (2A03) / Noise | new | -31.1 | +0.4 | -19.9 | -20.0 | no | residual -8.3 dB, +2.82 LU |
| Noise Hat | Hardware / NES (2A03) / Noise | new | -35.1 | -3.6 | -17.1 | -17.4 | no | residual -8.5 dB, +2.78 LU |
| Noise Crash | Hardware / NES (2A03) / Noise | new | -31.5 | -0.0 | -20.4 | -20.7 | no | residual -8.3 dB, +2.82 LU |
| Noise Explosion | Hardware / NES (2A03) / Noise | new | -31.3 | +0.2 | -21.6 | -21.6 | no | residual -8.3 dB, +2.48 LU |
| Noise Wind | Hardware / NES (2A03) / Noise | new | -31.1 | +0.4 | -26.0 | -26.0 | no | residual -8.2 dB, +2.85 LU |
| Hardware Decay Hat | Hardware / NES (2A03) / Noise | new | -32.0 | -0.5 | -17.4 | -17.6 | no | residual -8.4 dB, +2.79 LU |
| DPCM Kick | Hardware / NES (2A03) / DPCM | new | -30.7 | +1.3 | -11.2 | -11.3 | no | residual -8.9 dB, +2.32 LU |
| DPCM Snare | Hardware / NES (2A03) / DPCM | new | -37.2 | -5.2 | -11.2 | -11.3 | no | residual -2.3 dB, +2.32 LU |
| DPCM Low Tom | Hardware / NES (2A03) / DPCM | new | -30.9 | +1.1 | -11.2 | -11.3 | no | residual -10.1 dB, +2.36 LU |
| DPCM Drum Map | Hardware / NES (2A03) / DPCM | new | -32.0 | +0.0 | -11.2 | -11.3 | no | residual -2.9 dB, +3.05 LU |
| VRC6 Reed Lead | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.4 | -0.1 | -16.1 | -16.9 | no | residual 3.0 dB, +0.40 LU |
| VRC6 Hollow Lead | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.5 | -0.2 | -17.5 | -17.7 | no | residual 0.9 dB, +0.32 LU |
| VRC6 Glass Lead | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -30.0 | -3.7 | -15.5 | -15.6 | no | residual 2.6 dB, +0.21 LU |
| VRC6 Width Sweep | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.5 | -0.2 | -16.9 | -17.0 | no | residual 3.0 dB, +0.26 LU |
| VRC6 Chord | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -25.9 | +0.4 | -16.1 | -16.7 | no | residual 3.3 dB, +0.08 LU |
| VRC6 Bell | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.3 | +0.0 | -14.7 | -15.1 | no | residual 3.2 dB, -0.00 LU |
| VRC6 Pad | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.1 | +0.2 | -20.5 | -20.6 | no | residual -1.9 dB, +0.01 LU |
| Saw Brass | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.4 | -0.1 | -15.6 | -15.7 | no | residual 3.2 dB, +0.27 LU |
| Fat Saw Bass | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.7 | -0.4 | -11.9 | -11.9 | no | residual 3.7 dB, +0.17 LU |
| Saw Pluck | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.3 | -0.0 | -10.4 | -10.5 | no | residual 1.4 dB, +0.00 LU |
| Saw Power Arp | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.1 | +0.2 | -12.8 | -12.8 | no | residual 1.7 dB, +0.11 LU |
| Saw Strings | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.1 | +0.2 | -14.7 | -14.8 | no | residual -6.6 dB, +0.01 LU |
| Square Lead | Sounds / Leads | classic | -25.7 | -0.6 | -15.3 | -15.3 | no | – |
| Bright Lead | Sounds / Leads | classic | -27.6 | -2.5 | -14.5 | -14.6 | no | – |
| Thin Lead | Sounds / Leads | classic | -29.1 | -4.0 | -14.5 | -14.6 | no | – |
| Pluck | Sounds / Leads | classic | -31.4 | -6.3 | -17.1 | -17.2 | no | – |
| Echo Lead | Sounds / Leads | classic | -26.6 | -1.5 | -17.2 | -17.6 | no | – |
| Portamento Lead | Sounds / Leads | classic | -24.5 | +0.6 | -15.5 | -15.5 | no | – |
| Hero Lead | Sounds / Leads | classic | -25.4 | -0.3 | -14.9 | -15.0 | no | – |
| Triangle Bass | Sounds / Basses | classic | -24.4 | +0.7 | -12.2 | -12.2 | no | – |
| Punch Bass | Sounds / Basses | classic | -25.8 | -0.7 | -17.2 | -17.4 | no | – |
| Pulse Bass | Sounds / Basses | classic | -28.2 | -3.1 | -16.4 | -16.5 | no | – |
| Saw Bass | Sounds / Basses | classic | -23.1 | +2.0 | -8.6 | -8.6 | no | – |
| Slap Bass | Sounds / Basses | classic | -31.5 | -6.4 | -17.1 | -17.3 | no | – |
| Cello | Sounds / Leads | classic | -26.4 | -1.3 | -17.7 | -18.1 | no | – |
| Violin | Sounds / Leads | classic | -27.9 | -2.8 | -17.5 | -17.5 | no | – |
| Pizzicato | Sounds / Bells & Keys | classic | -33.4 | -7.1 | -17.1 | -17.2 | no | – |
| String Pad | Sounds / Pads | classic | -28.5 | -2.4 | -21.1 | -21.3 | no | – |
| Flute | Sounds / Leads | classic | -25.5 | -0.4 | -20.0 | -20.0 | no | – |
| Clarinet | Sounds / Leads | classic | -26.2 | -1.1 | -18.4 | -18.4 | no | – |
| Oboe | Sounds / Leads | classic | -28.6 | -3.5 | -16.7 | -16.8 | no | – |
| Ocarina | Sounds / Leads | classic | -26.4 | -1.3 | -21.2 | -21.2 | no | – |
| Pan Pipe | Sounds / Leads | classic | -27.0 | -1.9 | -16.6 | -16.9 | no | – |
| Brass | Sounds / Leads | classic | -24.2 | +0.9 | -17.3 | -17.9 | no | – |
| Trumpet | Sounds / Leads | classic | -25.7 | -0.6 | -15.1 | -15.3 | no | – |
| Horn Section | Sounds / Pads | classic | -26.2 | -0.1 | -21.8 | -21.9 | no | – |
| Organ | Sounds / Bells & Keys | classic | -25.4 | +0.9 | -17.9 | -18.0 | no | – |
| Harpsichord | Sounds / Bells & Keys | classic | -31.5 | -5.2 | -17.1 | -17.2 | no | – |
| Bell | Sounds / Bells & Keys | classic | -27.4 | -1.1 | -17.2 | -17.3 | no | – |
| Marimba | Sounds / Bells & Keys | classic | -31.1 | -4.8 | -17.1 | -17.2 | no | – |
| Music Box | Sounds / Bells & Keys | classic | -34.7 | -8.4 | -20.0 | -20.1 | no | – |
| Major Arp | Sounds / Arps | classic | -26.8 | -1.0 | -15.0 | -15.1 | no | – |
| Minor Arp | Sounds / Arps | classic | -26.8 | -1.0 | -15.0 | -15.1 | no | – |
| Octave Arp | Sounds / Arps | classic | -26.5 | -0.7 | -16.7 | -16.8 | no | – |
| Power Arp | Sounds / Arps | classic | -25.7 | +0.1 | -16.3 | -16.3 | no | – |
| Sus Arp | Sounds / Arps | classic | -26.7 | -0.9 | -15.0 | -15.2 | no | – |
| Tri Kick | Drums / Hits | classic | -29.7 | -0.2 | -12.3 | -12.3 | no | – |
| Tri Tom | Drums / Hits | classic | -27.6 | +1.9 | -12.3 | -12.3 | no | – |
| Kick | Drums / Hits | classic | -30.6 | -1.1 | -15.9 | -15.9 | no | – |
| Snare | Drums / Hits | classic | -27.4 | +2.1 | -15.7 | -15.7 | no | – |
| Closed Hat | Drums / Hits | classic | -32.7 | -3.2 | -17.3 | -17.3 | no | – |
| Open Hat | Drums / Hits | classic | -28.8 | +0.7 | -17.3 | -17.4 | no | – |
| Crash | Drums / Hits | classic | -26.0 | +3.5 | -15.8 | -15.9 | no | – |
| Metal Hit | Drums / Hits | classic | -30.7 | -1.2 | -15.9 | -15.9 | no | – |
| Sample Kick | Drums / Hits | classic | -26.0 | +3.5 | -5.3 | -5.4 | no | – |
| Sample Snare | Drums / Hits | classic | -33.9 | -4.4 | -6.7 | -6.7 | no | – |
| Sample Kit | Drums / Hits | classic | -26.9 | +2.6 | -5.3 | -5.4 | no | – |
| Standard Kit | Drums / Kits | classic | -27.4 | -1.2 | -10.7 | -10.8 | no | – |
| Tight Kit | Drums / Kits | classic | -27.7 | -1.5 | -10.7 | -10.8 | no | – |
| Explosion | Sounds / FX | classic | -27.0 | -0.4 | -15.0 | -15.1 | no | – |
| Small Explosion | Sounds / FX | classic | -30.4 | -3.8 | -15.8 | -15.9 | no | – |
| Hit | Sounds / Game Sounds | classic | -32.0 | -1.9 | -15.6 | -15.6 | no | – |
| Thunder | Sounds / FX | classic | -31.6 | -5.0 | -17.4 | -17.5 | no | – |
| Engine | Sounds / FX | classic | -31.4 | -4.8 | -19.2 | -19.2 | no | – |
| Wind | Sounds / FX | classic | -30.2 | -3.6 | -23.1 | -23.1 | no | – |
| Laser | Sounds / FX | classic | -34.2 | -7.6 | -17.1 | -17.2 | no | – |
| Coin | Sounds / Game Sounds | classic | -30.2 | -0.1 | -17.2 | -17.3 | no | – |
| Jump | Sounds / Game Sounds | classic | -32.5 | -2.4 | -17.8 | -18.0 | no | – |
| Power Up | Sounds / Game Sounds | classic | -28.7 | +1.4 | -17.9 | -18.0 | no | – |
| 1-Up | Sounds / Game Sounds | classic | -27.1 | +3.0 | -17.8 | -18.0 | no | – |
| Menu Blip | Sounds / Game Sounds | classic | -37.3 | -7.2 | -18.5 | -18.7 | no | – |
| Siren | Sounds / FX | classic | -26.5 | +0.1 | -18.6 | -18.8 | no | – |
| Fall | Sounds / FX | classic | -30.1 | -3.5 | -18.5 | -18.7 | no | – |
