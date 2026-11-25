"""Writes an original showcase tune (showcase.mid) and a synthesized vocal-ish
"hey" shout sample (hey.wav) for screenshots and demos."""
import math, random, struct, sys, os

out = sys.argv[1]
PPQ = 480

def var(n):
    b = [n & 0x7F]
    n >>= 7
    while n:
        b.insert(0, (n & 0x7F) | 0x80)
        n >>= 7
    return bytes(b)

def track(events, name):
    # events: (tick, bytes)
    ev = [(0, b'\xff\x03' + var(len(name)) + name.encode())] + sorted(events, key=lambda e: (e[0], e[1][0] & 0xF0 != 0x80))
    data, last = b'', 0
    for t, b in ev:
        data += var(t - last) + b
        last = t
    data += b'\x00\xff\x2f\x00'
    return b'MTrk' + struct.pack('>I', len(data)) + data

def note(evs, ch, start, length, pitch, vel=100):
    evs.append((start, bytes([0x90 | ch, pitch, vel])))
    evs.append((start + length, bytes([0x80 | ch, pitch, 0])))

E, Q, BAR = PPQ // 2, PPQ, PPQ * 4
chords = [(57, [0, 3, 7]), (53, [0, 4, 7]), (48, [0, 4, 7]), (55, [0, 4, 7])] * 4  # Am F C G
bars = len(chords)

lead, arp, bass, drums, pad, counter, saw, smp = [], [], [], [], [], [], [], []
motifs = [
    [(0, 2, 76), (2, 1, 74), (3, 1, 72), (4, 2, 74), (6, 2, 72)],
    [(0, 3, 72), (3, 1, 69), (4, 4, 72)],
    [(0, 2, 76), (2, 2, 79), (4, 2, 77), (6, 1, 76), (7, 1, 74)],
    [(0, 6, 74), (6, 2, 71)],
]
for b, (root, tri) in enumerate(chords):
    t0 = b * BAR
    if b >= 4:
        for (s, l, p) in motifs[b % 4]:
            note(lead, 0, t0 + s * E, l * E - 20, p + (12 if b >= 12 else 0), 110)
    for i in range(16):
        p = root + 12 + tri[i % 3] + (12 if i % 6 >= 3 else 0)
        note(arp, 1, t0 + i * (Q // 4), Q // 4 - 10, p, 70 + (i % 4 == 0) * 30)
    for i in range(8):
        p = (root - 24) + (12 if i % 2 else 0) if root - 24 >= 28 else root - 12
        note(bass, 2, t0 + i * E, E - 30, p, 100)
    for i in range(8):
        t = t0 + i * E
        if i in (0, 3, 4) or (b % 4 == 3 and i == 7):
            note(drums, 9, t, 60, 36, 120)
        if i in (2, 6):
            note(drums, 9, t, 60, 38, 110)
        note(drums, 9, t, 40, 46 if i == 7 else 42, 70)
    if b % 4 == 0:
        note(drums, 9, t0, 60, 49, 120)
    # Pad: the chord's third, held for the bar (from bar 3).
    if b >= 2:
        note(pad, 3, t0, BAR - 40, root + 12 + tri[1], 90)
    # Counter-melody answers the lead in bars 9-16.
    if b >= 8:
        for (s, l, d) in [(4, 1, 7), (5, 1, 12), (6, 2, 10)]:
            note(counter, 4, t0 + s * E, l * E - 20, root + 12 + d, 90)
    # Samples: the "hey" shout (User 1 = E2) on the last beat of every 4th bar, sample kicks on fills.
    if b % 4 == 3:
        note(smp, 6, t0 + 6 * E, Q, 40, 110)
    if b % 2 == 1:
        note(smp, 6, t0, E, 36, 100)
    # Saw riff: syncopated root/fifth stabs.
    for (s, d) in [(0, 0), (3, 7), (6, 12)]:
        note(saw, 5, t0 + s * E, E, root - 12 + d, 100)

tempo = [(0, b'\xff\x51\x03' + struct.pack('>I', 60_000_000 // 140)[1:])]
smf = b'MThd' + struct.pack('>IHHH', 6, 1, 9, PPQ)
smf += track(tempo, 'Tempo') + track(lead, 'Lead') + track(arp, 'Arpeggio') + track(pad, 'Strings') + track(counter, 'Counter') + track(saw, 'Saw Riff') + track(bass, 'Bass') + track(drums, 'Drums') + track(smp, 'DPCM Samples')
open(os.path.join(out, 'showcase.mid'), 'wb').write(smf)

# "Hey!" shout: a formant-ish vowel burst with a noisy onset.
sr = 22050
n = int(sr * 0.45)
random.seed(3)
pcm = []
for i in range(n):
    t = i / sr
    env = min(1.0, t / 0.01) * math.exp(-t * 6.0)
    f0 = 220 * (1 + 0.25 * math.exp(-t * 12))
    ph = 2 * math.pi * f0 * t
    voiced = sum(math.sin(ph * k) * a for k, a in [(1, 1.0), (2, 0.6), (3, 0.5), (4, 0.25), (5, 0.3), (8, 0.15)])
    noise = (random.random() * 2 - 1) * math.exp(-t * 40)
    pcm.append(max(-1, min(1, 0.45 * env * voiced + 0.5 * noise)))
data = b''.join(struct.pack('<h', int(x * 32000)) for x in pcm)
wav = b'RIFF' + struct.pack('<I', 36 + len(data)) + b'WAVEfmt ' + struct.pack('<IHHIIHH', 16, 1, 1, sr, sr * 2, 2, 16) + b'data' + struct.pack('<I', len(data)) + data
open(os.path.join(out, 'hey.wav'), 'wb').write(wav)
print('ok', len(smf), len(wav))
