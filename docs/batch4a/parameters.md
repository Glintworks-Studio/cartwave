# Automatable parameters (Batch 4a §6)

The registry automation is built on (`crates/ui/src/params.rs`, unit-tested): Batch 5's lanes, chooser and engine targets all read it (Batch 5 added `track.active`).

**Rate classes.** *frame-tick*: stepped at the driver's frame tick (60.0988 Hz), as game sound drivers did: on locked tracks the chip registers and the instrument macros (values quantised to the register's real steps, VISION.md "Automation"). A studio track's channel controls and macros are frame-tick too (RUNLOG B4A-D11): a studio voice runs the same driver code, which reads them once per frame tick, so automation there sounds like the chip's own macros. *smooth*: per audio block with de-zippering (a ramp over the block, as the engine does for track gains and the Main gain today): a studio track's volume and pan, and the Main gain.

A locked track has no volume or pan parameters: its fader is pinned at 0 dB and its pan at centre (BATCH4A_PROMPT.md §5.3); its level comes from the chip's Volume register (`chip.volume`) and the volume macro.

### Main track

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `main.gain` | Main gain | dB | −inf … 6 | 0 | smooth |

### Pulse track, studio

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `track.gain` | Volume | dB | −inf … 6 | 0 | smooth |
| `track.pan` | Pan | L/R | -50 … 50 | 0 | smooth |
| `track.active` | Track On | on/off | 0 … 1 | 1 | frame-tick |
| `chip.duty` | Duty | step | 0 … 3 | 2 | frame-tick |
| `chip.volume` | Volume | register (0–15) | 0 … 15 | 12 | frame-tick |
| `chip.envelope` | Envelope | mode | 0 … 2 | 0 | frame-tick |
| `chip.sweep` | Sweep | shift | -7 … 7 | 0 | frame-tick |
| `chip.sweep_rate` | Sweep rate | period | 0 … 7 | 3 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `macro.duty` | Duty macro | step | 0 … 15 | 0 | frame-tick |
| `voice.unison` | Unison | voices | 1 … 4 | 1 | frame-tick |
| `voice.detune` | Detune | cents (spread) | 0 … 50 | 0 | frame-tick |
| `voice.drift` | Drift | cents | 0 … 10 | 0 | frame-tick |

### Pulse track, locked

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `chip.duty` | Duty | step | 0 … 3 | 2 | frame-tick |
| `chip.volume` | Volume | register (0–15) | 0 … 15 | 12 | frame-tick |
| `chip.envelope` | Envelope | mode | 0 … 2 | 0 | frame-tick |
| `chip.sweep` | Sweep | shift | -7 … 7 | 0 | frame-tick |
| `chip.sweep_rate` | Sweep rate | period | 0 … 7 | 3 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `macro.duty` | Duty macro | step | 0 … 15 | 0 | frame-tick |

### Triangle track, studio

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `track.gain` | Volume | dB | −inf … 6 | 0 | smooth |
| `track.pan` | Pan | L/R | -50 … 50 | 0 | smooth |
| `track.active` | Track On | on/off | 0 … 1 | 1 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `macro.gate` | Gate | on/off | 0 … 1 | 1 | frame-tick |
| `voice.unison` | Unison | voices | 1 … 4 | 1 | frame-tick |
| `voice.detune` | Detune | cents (spread) | 0 … 50 | 0 | frame-tick |
| `voice.drift` | Drift | cents | 0 … 10 | 0 | frame-tick |

### Triangle track, locked

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `macro.gate` | Gate | on/off | 0 … 1 | 1 | frame-tick |

### Noise track, studio

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `track.gain` | Volume | dB | −inf … 6 | 0 | smooth |
| `track.pan` | Pan | L/R | -50 … 50 | 0 | smooth |
| `track.active` | Track On | on/off | 0 … 1 | 1 | frame-tick |
| `chip.noise_mode` | Noise mode | mode | 0 … 1 | 0 | frame-tick |
| `chip.noise_period` | Noise period | index | -1 … 15 | -1 | frame-tick |
| `chip.volume` | Volume | register (0–15) | 0 … 15 | 12 | frame-tick |
| `chip.envelope` | Envelope | mode | 0 … 2 | 1 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `macro.duty` | Duty macro | step | 0 … 15 | 0 | frame-tick |
| `voice.unison` | Unison | voices | 1 … 4 | 1 | frame-tick |
| `voice.detune` | Detune | cents (spread) | 0 … 50 | 0 | frame-tick |
| `voice.drift` | Drift | cents | 0 … 10 | 0 | frame-tick |

### Noise track, locked

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `chip.noise_mode` | Noise mode | mode | 0 … 1 | 0 | frame-tick |
| `chip.noise_period` | Noise period | index | -1 … 15 | -1 | frame-tick |
| `chip.volume` | Volume | register (0–15) | 0 … 15 | 12 | frame-tick |
| `chip.envelope` | Envelope | mode | 0 … 2 | 1 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `macro.duty` | Duty macro | step | 0 … 15 | 0 | frame-tick |

### DPCM track, studio

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `track.gain` | Volume | dB | −inf … 6 | 0 | smooth |
| `track.pan` | Pan | L/R | -50 … 50 | 0 | smooth |
| `track.active` | Track On | on/off | 0 … 1 | 1 | frame-tick |
| `chip.sample` | Sample | slot | -1 … 7 | -1 | frame-tick |
| `chip.rate` | Rate | index | 0 … 15 | 15 | frame-tick |
| `chip.loop` | Loop | on/off | 0 … 1 | 0 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `voice.unison` | Unison | voices | 1 … 4 | 1 | frame-tick |
| `voice.detune` | Detune | cents (spread) | 0 … 50 | 0 | frame-tick |
| `voice.drift` | Drift | cents | 0 … 10 | 0 | frame-tick |

### DPCM track, locked

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `chip.sample` | Sample | slot | -1 … 7 | -1 | frame-tick |
| `chip.rate` | Rate | index | 0 … 15 | 15 | frame-tick |
| `chip.loop` | Loop | on/off | 0 … 1 | 0 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |

### VRC6 Pulse track, studio

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `track.gain` | Volume | dB | −inf … 6 | 0 | smooth |
| `track.pan` | Pan | L/R | -50 … 50 | 0 | smooth |
| `track.active` | Track On | on/off | 0 … 1 | 1 | frame-tick |
| `chip.duty` | Duty | step | 0 … 8 | 7 | frame-tick |
| `chip.volume` | Volume | register (0–15) | 0 … 15 | 12 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `macro.duty` | Duty macro | step | 0 … 15 | 0 | frame-tick |
| `voice.unison` | Unison | voices | 1 … 4 | 1 | frame-tick |
| `voice.detune` | Detune | cents (spread) | 0 … 50 | 0 | frame-tick |
| `voice.drift` | Drift | cents | 0 … 10 | 0 | frame-tick |

### VRC6 Pulse track, locked

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `chip.duty` | Duty | step | 0 … 8 | 7 | frame-tick |
| `chip.volume` | Volume | register (0–15) | 0 … 15 | 12 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `macro.duty` | Duty macro | step | 0 … 15 | 0 | frame-tick |

### Saw track, studio

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `track.gain` | Volume | dB | −inf … 6 | 0 | smooth |
| `track.pan` | Pan | L/R | -50 … 50 | 0 | smooth |
| `track.active` | Track On | on/off | 0 … 1 | 1 | frame-tick |
| `chip.rate` | Rate | index | 0 … 63 | 42 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
| `voice.unison` | Unison | voices | 1 … 4 | 1 | frame-tick |
| `voice.detune` | Detune | cents (spread) | 0 … 50 | 0 | frame-tick |
| `voice.drift` | Drift | cents | 0 … 10 | 0 | frame-tick |

### Saw track, locked

| Id | Name | Unit | Range | Default | Rate |
|---|---|---|---|---|---|
| `chip.rate` | Rate | index | 0 … 63 | 42 | frame-tick |
| `macro.volume` | Volume macro | step (0–15) | 0 … 15 | 15 | frame-tick |
| `macro.arpeggio` | Arpeggio | semitones | -12 … 12 | 0 | frame-tick |
| `macro.pitch` | Pitch | cents | -100 … 100 | 0 | frame-tick |
| `macro.vibrato` | Vibrato depth | cents | 0 … 100 | 0 | frame-tick |
