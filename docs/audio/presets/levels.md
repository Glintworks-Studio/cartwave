# The preset bank: levels

Written by `cargo run --release -p ui --example bank_pack -- --write`. Each preset plays its role's phrase at 150 BPM (48 kHz); authentic presets on a locked track (the console's mixer and filter), the others on a studio track. Loudness is EBU R128 integrated (LUFS), true peak in dBTP, sample peak in dBFS.

**202 presets (141 new, 61 classic); clipping: none; new presets within ±1.5 LU of their category's target: 111 of 141 (largest deviation 6.0 LU); authentic presets identical on a studio voice: 0 of 42.**

Level matching: a category's target is fixed from the first pass at the 35th percentile of its new presets (a level the quieter voices can reach); `presets/levels.json` holds the gains `design_bank.py --levels` folded in. The classic presets (Batch 1) are listed but not changed (RUNLOG PB-D6), except Music Box, Pizzicato and Menu Blip, raised in Batch 5 A2 as far as their voice allows (RUNLOG B5-D4).

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
| Bright Square Lead | Sounds / Leads | new | -25.5 | -0.4 | -13.5 | -13.6 | no | – |
| Late Vibrato Lead | Sounds / Leads | new | -25.0 | +0.1 | -17.2 | -17.2 | no | – |
| Duty Sweep Lead | Sounds / Leads | new | -25.0 | +0.1 | -12.7 | -12.9 | no | – |
| Hollow 12.5% | Sounds / Leads | new | -25.6 | -0.5 | -13.0 | -13.0 | no | – |
| Nasal Reed Lead | Sounds / Leads | new | -25.3 | -0.2 | -13.4 | -13.7 | no | – |
| Glass Pulse Lead | Sounds / Leads | new | -29.6 | -4.5 | -12.2 | -12.4 | no | – |
| Echo Square | Sounds / Leads | new | -27.2 | -2.1 | -17.8 | -17.8 | no | – |
| Echo Saw Lead | Sounds / Leads | new | -25.0 | +0.1 | -12.3 | -12.6 | no | – |
| Supersquare | Sounds / Leads | new | -24.7 | +0.4 | -10.3 | -10.5 | no | – |
| Wide Detuned Lead | Sounds / Leads | new | -24.9 | +0.2 | -10.9 | -11.1 | no | – |
| Detuned Saw Lead | Sounds / Leads | new | -25.0 | +0.1 | -11.0 | -11.0 | no | – |
| Saw Lead | Sounds / Leads | new | -25.4 | -0.3 | -14.2 | -14.3 | no | – |
| Scoop Lead | Sounds / Leads | new | -25.1 | -0.0 | -14.4 | -14.4 | no | – |
| Trill Lead | Sounds / Leads | new | -25.1 | +0.0 | -13.7 | -13.7 | no | – |
| Soft Triangle Lead | Sounds / Leads | new | -23.1 | +2.0 | -17.2 | -17.3 | no | – |
| Round Triangle Bass | Sounds / Basses | new | -24.4 | +0.7 | -16.8 | -16.8 | no | – |
| Kick-Drop Triangle Bass | Sounds / Basses | new | -24.4 | +0.7 | -17.5 | -17.5 | no | – |
| Staccato Triangle Bass | Sounds / Basses | new | -27.5 | -2.4 | -17.3 | -17.3 | no | – |
| Triangle Slide Bass | Sounds / Basses | new | -24.4 | +0.7 | -17.2 | -17.3 | no | – |
| Thick Pulse Bass | Sounds / Basses | new | -25.2 | -0.1 | -16.5 | -16.5 | no | – |
| Pluck Pulse Bass | Sounds / Basses | new | -30.0 | -4.9 | -17.5 | -17.6 | no | – |
| Growl Pulse Bass | Sounds / Basses | new | -27.1 | -2.0 | -13.0 | -13.3 | no | – |
| Octave Bounce Bass | Sounds / Basses | new | -25.1 | +0.0 | -16.5 | -16.5 | no | – |
| Hollow Bass | Sounds / Basses | new | -27.1 | -2.0 | -14.0 | -14.4 | no | – |
| Saw Sub Bass | Sounds / Basses | new | -25.4 | -0.3 | -13.6 | -13.6 | no | – |
| Saw Pluck Bass | Sounds / Basses | new | -25.1 | +0.0 | -14.7 | -14.7 | no | – |
| Detuned Saw Bass | Sounds / Basses | new | -25.2 | -0.1 | -13.1 | -13.1 | no | – |
| Wide Pulse Bass | Sounds / Basses | new | -25.0 | +0.1 | -15.6 | -15.7 | no | – |
| Rubber Bass | Sounds / Basses | new | -27.1 | -2.0 | -14.3 | -14.4 | no | – |
| Fast Major Arp | Sounds / Arps | new | -26.0 | -0.2 | -13.8 | -13.8 | no | – |
| Fast Minor Arp | Sounds / Arps | new | -26.0 | -0.2 | -14.2 | -14.3 | no | – |
| Major Seventh Arp | Sounds / Arps | new | -25.5 | +0.3 | -15.8 | -15.8 | no | – |
| Minor Seventh Arp | Sounds / Arps | new | -26.8 | -1.0 | -11.9 | -11.9 | no | – |
| Dominant Arp | Sounds / Arps | new | -25.8 | -0.0 | -14.6 | -14.6 | no | – |
| Diminished Arp | Sounds / Arps | new | -25.6 | +0.2 | -17.4 | -17.4 | no | – |
| Suspended Shimmer Arp | Sounds / Arps | new | -26.1 | -0.3 | -12.6 | -12.9 | no | – |
| Octave Pump Arp | Sounds / Arps | new | -25.5 | +0.3 | -17.8 | -17.8 | no | – |
| Power Fifth Arp | Sounds / Arps | new | -25.7 | +0.1 | -13.4 | -14.5 | no | – |
| Slow Major Arp | Sounds / Arps | new | -25.8 | -0.0 | -13.7 | -13.8 | no | – |
| Slow Minor Arp | Sounds / Arps | new | -25.3 | +0.5 | -16.5 | -16.5 | no | – |
| Rolling Arp | Sounds / Arps | new | -25.8 | -0.0 | -14.0 | -14.1 | no | – |
| Detuned Chord Arp | Sounds / Arps | new | -25.8 | +0.0 | -13.4 | -13.6 | no | – |
| Triangle Octave Arp | Sounds / Arps | new | -24.1 | +1.7 | -17.1 | -17.1 | no | – |
| Breathing Duty Pad | Sounds / Pads | new | -26.1 | +0.0 | -17.5 | -17.5 | no | – |
| Warm Square Pad | Sounds / Pads | new | -25.7 | +0.4 | -19.7 | -19.9 | no | – |
| Soft Choir Pad | Sounds / Pads | new | -26.1 | +0.0 | -21.5 | -21.6 | no | – |
| Glass Pad | Sounds / Pads | new | -26.7 | -0.6 | -14.1 | -14.1 | no | – |
| Wide Saw Pad | Sounds / Pads | new | -26.0 | +0.1 | -15.6 | -15.6 | no | – |
| Organ Pad | Sounds / Pads | new | -25.7 | +0.4 | -22.0 | -22.1 | no | – |
| Shimmer Pad | Sounds / Pads | new | -26.1 | -0.0 | -19.5 | -19.5 | no | – |
| Clear Bell | Sounds / Bells & Keys | new | -26.1 | +0.2 | -19.2 | -19.2 | no | – |
| Glass Bell | Sounds / Bells & Keys | new | -26.3 | +0.0 | -15.5 | -15.5 | no | – |
| Music Box Tine | Sounds / Bells & Keys | new | -28.4 | -2.1 | -16.2 | -16.2 | no | – |
| Electric Piano | Sounds / Bells & Keys | new | -25.8 | +0.5 | -15.8 | -16.1 | no | – |
| Harpsichord Pluck | Sounds / Bells & Keys | new | -26.9 | -0.6 | -17.4 | -17.5 | no | – |
| Vibraphone | Sounds / Bells & Keys | new | -25.9 | +0.4 | -19.9 | -20.8 | no | – |
| Saw Celesta | Sounds / Bells & Keys | new | -26.3 | -0.0 | -18.4 | -18.5 | no | – |
| Rising Sweep | Sounds / FX | new | -26.7 | -0.1 | -19.9 | -20.0 | no | – |
| Falling Bomb | Sounds / FX | new | -26.8 | -0.2 | -23.1 | -23.2 | no | – |
| Warp Down | Sounds / FX | new | -26.6 | -0.0 | -17.1 | -17.3 | no | – |
| Noise Riser | Sounds / FX | new | -26.1 | +0.5 | -20.2 | -20.4 | no | – |
| Ocean Wash | Sounds / FX | new | -26.8 | -0.2 | -19.4 | -19.4 | no | – |
| Static Crackle | Sounds / FX | new | -26.9 | -0.3 | -17.3 | -17.3 | no | – |
| Alarm | Sounds / FX | new | -26.7 | -0.1 | -19.2 | -19.3 | no | – |
| Teleport | Sounds / FX | new | -30.0 | -3.4 | -16.4 | -16.4 | no | – |
| Ray Gun | Sounds / FX | new | -26.7 | -0.1 | -17.4 | -17.5 | no | – |
| Heartbeat | Sounds / FX | new | -29.5 | -2.9 | -16.7 | -17.0 | no | – |
| Coin Chime | Sounds / Game Sounds | new | -30.4 | -0.3 | -24.3 | -24.4 | no | – |
| Hop | Sounds / Game Sounds | new | -31.0 | -0.9 | -17.6 | -17.7 | no | – |
| Impact Crunch | Sounds / Game Sounds | new | -30.1 | +0.0 | -17.2 | -17.5 | no | – |
| Cursor Blip | Sounds / Game Sounds | new | -35.2 | -5.1 | -16.5 | -16.5 | no | – |
| Rising Power | Sounds / Game Sounds | new | -30.5 | -0.4 | -23.4 | -23.5 | no | – |
| Extra Life | Sounds / Game Sounds | new | -30.0 | +0.1 | -26.9 | -26.9 | no | – |
| Boom | Sounds / Game Sounds | new | -30.0 | +0.1 | -21.9 | -22.2 | no | – |
| Zap Laser | Sounds / Game Sounds | new | -32.4 | -2.3 | -16.3 | -16.4 | no | – |
| Confirm Select | Sounds / Game Sounds | new | -30.1 | -0.0 | -18.6 | -18.7 | no | – |
| Error Buzz | Sounds / Game Sounds | new | -30.7 | -0.6 | -16.3 | -16.4 | no | – |
| Door Slide | Sounds / Game Sounds | new | -30.2 | -0.1 | -24.4 | -24.5 | no | – |
| Item Pickup | Sounds / Game Sounds | new | -29.6 | +0.5 | -17.4 | -17.4 | no | – |
| Punchy Noise Kit | Drums / Kits | new | -26.0 | +0.2 | -14.0 | -14.0 | no | – |
| Tight Noise Kit | Drums / Kits | new | -28.1 | -1.9 | -13.6 | -13.6 | no | – |
| Big Room Noise Kit | Drums / Kits | new | -25.9 | +0.3 | -14.6 | -14.9 | no | – |
| Metal Noise Kit | Drums / Kits | new | -26.4 | -0.2 | -13.6 | -13.6 | no | – |
| Thump Kick | Drums / Hits | new | -31.3 | -1.8 | -16.6 | -16.6 | no | – |
| Crack Snare | Drums / Hits | new | -29.1 | +0.4 | -19.0 | -19.0 | no | – |
| Fat Snare | Drums / Hits | new | -29.5 | +0.0 | -22.6 | -22.6 | no | – |
| Tick Hat | Drums / Hits | new | -30.4 | -0.9 | -15.8 | -16.0 | no | – |
| Sizzle Hat | Drums / Hits | new | -29.2 | +0.3 | -20.8 | -20.9 | no | – |
| Ride Ping | Drums / Hits | new | -29.9 | -0.4 | -20.6 | -20.6 | no | – |
| Splash Cymbal | Drums / Hits | new | -29.2 | +0.3 | -20.7 | -20.9 | no | – |
| Hand Clap | Drums / Hits | new | -29.5 | +0.0 | -21.4 | -21.5 | no | – |
| Shaker | Drums / Hits | new | -29.3 | +0.2 | -17.4 | -17.8 | no | – |
| Drop Kick | Drums / Hits | new | -29.1 | +0.4 | -17.5 | -17.5 | no | – |
| Low Tom | Drums / Hits | new | -27.8 | +1.7 | -16.7 | -16.8 | no | – |
| High Tom | Drums / Hits | new | -27.5 | +2.0 | -16.8 | -17.4 | no | – |
| Sampled Kick | Drums / Hits | new | -26.0 | +3.5 | -10.1 | -10.1 | no | – |
| Sampled Snare | Drums / Hits | new | -34.2 | -4.7 | -17.2 | -17.2 | no | – |
| Sampled Tom | Drums / Hits | new | -23.5 | +6.0 | -9.0 | -9.1 | no | – |
| Sampled Crash | Drums / Hits | new | -34.8 | -5.3 | -21.7 | -21.7 | no | – |
| Duty Lead | Hardware / NES (2A03) / Pulse | new | -27.8 | -0.0 | -17.7 | -17.9 | no | residual -4.6 dB, +0.31 LU |
| Square Melody | Hardware / NES (2A03) / Pulse | new | -27.5 | +0.3 | -21.9 | -22.1 | no | residual -5.7 dB, +0.39 LU |
| Thin Pulse Lead | Hardware / NES (2A03) / Pulse | new | -27.8 | +0.0 | -16.5 | -16.7 | no | residual -1.9 dB, +0.29 LU |
| Duty Swell Lead | Hardware / NES (2A03) / Pulse | new | -27.8 | +0.0 | -19.1 | -19.3 | no | residual -5.0 dB, +0.29 LU |
| Echo Pulse Lead | Hardware / NES (2A03) / Pulse | new | -28.5 | -0.7 | -16.0 | -16.1 | no | residual -7.0 dB, +0.13 LU |
| Pulse Pluck | Hardware / NES (2A03) / Pulse | new | -28.4 | -0.6 | -16.0 | -16.2 | no | residual -3.6 dB, -0.01 LU |
| Firm Pulse Bass | Hardware / NES (2A03) / Pulse | new | -27.9 | -0.1 | -17.7 | -17.8 | no | residual -5.9 dB, +0.20 LU |
| Major Chord Pulse | Hardware / NES (2A03) / Pulse | new | -27.8 | -0.0 | -17.7 | -17.9 | no | residual -4.8 dB, +0.11 LU |
| Minor Chord Pulse | Hardware / NES (2A03) / Pulse | new | -28.0 | -0.2 | -16.5 | -16.7 | no | residual -1.1 dB, +0.10 LU |
| Hardware Sweep Zap | Hardware / NES (2A03) / Pulse | new | -31.7 | -3.9 | -16.0 | -16.2 | no | residual -30.2 dB, -0.00 LU |
| Decay Envelope Pluck | Hardware / NES (2A03) / Pulse | new | -26.5 | +1.3 | -16.0 | -16.1 | no | residual -3.7 dB, +0.01 LU |
| Pulse Bell | Hardware / NES (2A03) / Pulse | new | -27.8 | -0.0 | -19.1 | -19.3 | no | residual -0.1 dB, -0.00 LU |
| Pure Triangle Bass | Hardware / NES (2A03) / Triangle | new | -26.0 | +2.0 | -18.3 | -18.3 | no | residual 3.4 dB, +0.00 LU |
| Triangle Kick Bass | Hardware / NES (2A03) / Triangle | new | -25.7 | +2.3 | -14.1 | -14.1 | no | residual -0.8 dB, -0.01 LU |
| Triangle Staccato | Hardware / NES (2A03) / Triangle | new | -28.3 | -0.3 | -17.0 | -17.0 | no | residual 1.5 dB, +0.00 LU |
| Triangle Flute | Hardware / NES (2A03) / Triangle | new | -23.2 | +4.8 | -14.1 | -14.1 | no | residual 4.3 dB, -0.00 LU |
| Triangle Kick | Hardware / NES (2A03) / Triangle | new | -31.2 | -3.2 | -15.7 | -15.8 | no | residual -16.9 dB, +0.03 LU |
| Triangle Tom | Hardware / NES (2A03) / Triangle | new | -27.7 | +0.3 | -15.8 | -15.9 | no | residual -18.7 dB, -0.01 LU |
| Noise Kit | Hardware / NES (2A03) / Noise | new | -31.0 | +0.5 | -18.2 | -18.5 | no | residual 5.0 dB, +3.53 LU |
| Short-Mode Kit | Hardware / NES (2A03) / Noise | new | -31.5 | -0.0 | -17.5 | -17.5 | no | residual 5.2 dB, +3.47 LU |
| Noise Snare | Hardware / NES (2A03) / Noise | new | -31.1 | +0.4 | -19.9 | -20.0 | no | residual -8.0 dB, +2.79 LU |
| Noise Hat | Hardware / NES (2A03) / Noise | new | -35.1 | -3.6 | -17.1 | -17.4 | no | residual -6.7 dB, +2.65 LU |
| Noise Crash | Hardware / NES (2A03) / Noise | new | -31.5 | -0.0 | -20.4 | -20.7 | no | residual -7.9 dB, +2.80 LU |
| Noise Explosion | Hardware / NES (2A03) / Noise | new | -31.3 | +0.2 | -21.6 | -21.6 | no | residual -8.2 dB, +2.83 LU |
| Noise Wind | Hardware / NES (2A03) / Noise | new | -31.1 | +0.4 | -26.0 | -26.0 | no | residual -8.2 dB, +2.84 LU |
| Hardware Decay Hat | Hardware / NES (2A03) / Noise | new | -32.0 | -0.5 | -17.4 | -17.6 | no | residual -7.6 dB, +2.75 LU |
| DPCM Kick | Hardware / NES (2A03) / DPCM | new | -30.7 | +1.3 | -11.2 | -11.3 | no | residual -8.3 dB, +2.27 LU |
| DPCM Snare | Hardware / NES (2A03) / DPCM | new | -37.2 | -5.2 | -11.2 | -11.3 | no | residual -1.5 dB, +2.12 LU |
| DPCM Low Tom | Hardware / NES (2A03) / DPCM | new | -30.9 | +1.1 | -11.2 | -11.3 | no | residual -9.6 dB, +2.32 LU |
| DPCM Drum Map | Hardware / NES (2A03) / DPCM | new | -32.0 | +0.0 | -11.2 | -11.3 | no | residual -3.1 dB, +2.91 LU |
| VRC6 Reed Lead | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.4 | -0.1 | -16.1 | -16.9 | no | residual 3.0 dB, +0.40 LU |
| VRC6 Hollow Lead | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.5 | -0.2 | -17.5 | -17.7 | no | residual 0.9 dB, +0.31 LU |
| VRC6 Glass Lead | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -30.0 | -3.7 | -15.5 | -15.6 | no | residual 2.6 dB, +0.21 LU |
| VRC6 Width Sweep | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.5 | -0.2 | -16.9 | -17.0 | no | residual 3.0 dB, +0.26 LU |
| VRC6 Chord | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -25.9 | +0.4 | -16.1 | -16.7 | no | residual 3.3 dB, +0.08 LU |
| VRC6 Bell | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.3 | +0.0 | -14.7 | -15.1 | no | residual 3.2 dB, -0.00 LU |
| VRC6 Pad | Hardware / Famicom + VRC6 / VRC6 Pulse | new | -26.1 | +0.2 | -20.5 | -20.6 | no | residual -1.9 dB, +0.01 LU |
| Saw Brass | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.4 | -0.1 | -15.6 | -15.7 | no | residual 3.2 dB, +0.27 LU |
| Fat Saw Bass | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.7 | -0.4 | -11.9 | -11.9 | no | residual 3.7 dB, +0.19 LU |
| Saw Pluck | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.3 | -0.0 | -10.4 | -10.5 | no | residual 1.5 dB, -0.01 LU |
| Saw Power Arp | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.1 | +0.2 | -12.8 | -12.8 | no | residual 1.7 dB, +0.11 LU |
| Saw Strings | Hardware / Famicom + VRC6 / VRC6 Saw | new | -22.1 | +0.2 | -14.7 | -14.8 | no | residual -6.6 dB, +0.01 LU |
| Square Lead | Sounds / Leads | classic | -25.7 | -0.6 | -18.1 | -18.1 | no | – |
| Bright Lead | Sounds / Leads | classic | -27.6 | -2.5 | -14.9 | -15.1 | no | – |
| Thin Lead | Sounds / Leads | classic | -29.1 | -4.0 | -14.9 | -15.0 | no | – |
| Pluck | Sounds / Leads | classic | -31.4 | -6.3 | -19.2 | -19.3 | no | – |
| Echo Lead | Sounds / Leads | classic | -26.6 | -1.5 | -19.4 | -19.9 | no | – |
| Portamento Lead | Sounds / Leads | classic | -24.6 | +0.5 | -16.3 | -16.4 | no | – |
| Hero Lead | Sounds / Leads | classic | -25.4 | -0.3 | -15.1 | -15.2 | no | – |
| Triangle Bass | Sounds / Basses | classic | -24.5 | +0.6 | -17.1 | -17.3 | no | – |
| Punch Bass | Sounds / Basses | classic | -25.8 | -0.7 | -21.3 | -21.4 | no | – |
| Pulse Bass | Sounds / Basses | classic | -28.3 | -3.2 | -19.1 | -19.1 | no | – |
| Saw Bass | Sounds / Basses | classic | -23.1 | +2.0 | -10.4 | -10.6 | no | – |
| Slap Bass | Sounds / Basses | classic | -31.6 | -6.5 | -20.5 | -20.7 | no | – |
| Cello | Sounds / Leads | classic | -26.4 | -1.3 | -18.1 | -18.5 | no | – |
| Violin | Sounds / Leads | classic | -27.9 | -2.8 | -17.7 | -17.7 | no | – |
| Pizzicato | Sounds / Bells & Keys | classic | -31.6 | -5.3 | -17.6 | -17.6 | no | – |
| String Pad | Sounds / Pads | classic | -28.6 | -2.5 | -21.2 | -21.4 | no | – |
| Flute | Sounds / Leads | classic | -25.5 | -0.4 | -21.0 | -21.1 | no | – |
| Clarinet | Sounds / Leads | classic | -26.2 | -1.1 | -20.3 | -20.3 | no | – |
| Oboe | Sounds / Leads | classic | -28.6 | -3.5 | -17.0 | -17.1 | no | – |
| Ocarina | Sounds / Leads | classic | -26.4 | -1.3 | -22.4 | -22.4 | no | – |
| Pan Pipe | Sounds / Leads | classic | -27.1 | -2.0 | -19.4 | -19.4 | no | – |
| Brass | Sounds / Leads | classic | -24.2 | +0.9 | -17.2 | -17.7 | no | – |
| Trumpet | Sounds / Leads | classic | -25.7 | -0.6 | -16.0 | -16.1 | no | – |
| Horn Section | Sounds / Pads | classic | -26.2 | -0.1 | -22.3 | -22.3 | no | – |
| Organ | Sounds / Bells & Keys | classic | -25.4 | +0.9 | -21.7 | -22.0 | no | – |
| Harpsichord | Sounds / Bells & Keys | classic | -31.5 | -5.2 | -18.0 | -18.1 | no | – |
| Bell | Sounds / Bells & Keys | classic | -27.4 | -1.1 | -18.9 | -18.9 | no | – |
| Marimba | Sounds / Bells & Keys | classic | -31.2 | -4.9 | -21.7 | -21.8 | no | – |
| Music Box | Sounds / Bells & Keys | classic | -30.3 | -4.0 | -16.2 | -16.2 | no | – |
| Major Arp | Sounds / Arps | classic | -26.8 | -1.0 | -15.9 | -16.1 | no | – |
| Minor Arp | Sounds / Arps | classic | -26.8 | -1.0 | -16.0 | -16.1 | no | – |
| Octave Arp | Sounds / Arps | classic | -26.5 | -0.7 | -19.0 | -19.2 | no | – |
| Power Arp | Sounds / Arps | classic | -25.7 | +0.1 | -18.6 | -18.8 | no | – |
| Sus Arp | Sounds / Arps | classic | -26.7 | -0.9 | -15.8 | -15.9 | no | – |
| Tri Kick | Drums / Hits | classic | -29.7 | -0.2 | -17.5 | -17.5 | no | – |
| Tri Tom | Drums / Hits | classic | -27.7 | +1.8 | -17.5 | -17.5 | no | – |
| Kick | Drums / Hits | classic | -30.8 | -1.3 | -19.3 | -19.4 | no | – |
| Snare | Drums / Hits | classic | -27.4 | +2.1 | -19.4 | -19.4 | no | – |
| Closed Hat | Drums / Hits | classic | -32.8 | -3.3 | -20.7 | -20.9 | no | – |
| Open Hat | Drums / Hits | classic | -28.8 | +0.7 | -20.7 | -20.9 | no | – |
| Crash | Drums / Hits | classic | -26.0 | +3.5 | -19.1 | -19.2 | no | – |
| Metal Hit | Drums / Hits | classic | -31.0 | -1.5 | -19.1 | -19.2 | no | – |
| Sample Kick | Drums / Hits | classic | -26.0 | +3.5 | -10.1 | -10.1 | no | – |
| Sample Snare | Drums / Hits | classic | -34.2 | -4.7 | -17.2 | -17.2 | no | – |
| Sample Kit | Drums / Hits | classic | -27.0 | +2.5 | -9.5 | -9.5 | no | – |
| Standard Kit | Drums / Kits | classic | -27.5 | -1.3 | -14.6 | -14.6 | no | – |
| Tight Kit | Drums / Kits | classic | -27.8 | -1.6 | -14.6 | -14.6 | no | – |
| Explosion | Sounds / FX | classic | -27.0 | -0.4 | -18.4 | -18.6 | no | – |
| Small Explosion | Sounds / FX | classic | -30.4 | -3.8 | -19.4 | -19.6 | no | – |
| Hit | Sounds / Game Sounds | classic | -32.2 | -2.1 | -19.2 | -19.3 | no | – |
| Thunder | Sounds / FX | classic | -31.6 | -5.0 | -16.9 | -17.4 | no | – |
| Engine | Sounds / FX | classic | -31.4 | -4.8 | -20.0 | -20.1 | no | – |
| Wind | Sounds / FX | classic | -30.2 | -3.6 | -24.6 | -24.7 | no | – |
| Laser | Sounds / FX | classic | -34.2 | -7.6 | -18.1 | -18.1 | no | – |
| Coin | Sounds / Game Sounds | classic | -30.2 | -0.1 | -21.9 | -22.0 | no | – |
| Jump | Sounds / Game Sounds | classic | -32.5 | -2.4 | -19.9 | -20.0 | no | – |
| Power Up | Sounds / Game Sounds | classic | -28.7 | +1.4 | -19.9 | -19.9 | no | – |
| 1-Up | Sounds / Game Sounds | classic | -27.1 | +3.0 | -22.6 | -22.8 | no | – |
| Menu Blip | Sounds / Game Sounds | classic | -33.9 | -3.8 | -17.6 | -17.6 | no | – |
| Siren | Sounds / FX | classic | -26.5 | +0.1 | -23.0 | -23.2 | no | – |
| Fall | Sounds / FX | classic | -30.2 | -3.6 | -23.1 | -23.3 | no | – |
