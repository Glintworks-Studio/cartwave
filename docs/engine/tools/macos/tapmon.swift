// Ground truth for the macOS engine tests: captures what the system plays to
// an output device through a CoreAudio process tap (macOS 14.2+), outside the
// app under test, and detects
//   - dropouts of the engine's 19 kHz pilot tone (Goertzel over N-frame blocks,
//     a block below 30% of the running level is a dropout), and
//   - onsets of the A/V-sync test click (|x| > 0.1 after >= 50 ms of quiet).
//
//   swiftc -O tapmon.swift -o tapmon
//   tapmon [--secs 60] [--device <name|uid>] [--pilot 19000] [--block 128]
//          [--events events.csv] [--wav capture.wav] [--quiet]
//
// Why not cpal's loopback: cpal only taps when the device has no inputs. On an
// interface with inputs (the MOTU M2) `build_input_stream` on the output
// device records the physical inputs instead (ENGINE-macos.md §3).
//
// Output: one status line per second on stderr, and in --events a CSV of
// `kind,host_ns` (kind = dropout | click | start | stop), host_ns being
// mach_absolute_time in ns: the same clock as Rust's Instant on macOS.
// The summary line at exit: blocks analysed, dropouts, lowest block level
// relative to the running level in the clean part (the detection margin).
//
// Needs the "System Audio Recording" permission for the terminal running it.

import AudioToolbox
import CoreAudio
import Foundation

setvbuf(stdout, nil, _IOLBF, 0)
setvbuf(stderr, nil, _IOLBF, 0)

func arg(_ k: String) -> String? {
    let a = CommandLine.arguments
    guard let i = a.firstIndex(of: k), i + 1 < a.count else { return nil }
    return a[i + 1]
}
let secs = Double(arg("--secs") ?? "60") ?? 60
let pilotHz = Double(arg("--pilot") ?? "19000") ?? 19000
let blockLen = Int(arg("--block") ?? "128") ?? 128
let eventsPath = arg("--events")
let wavPath = arg("--wav")
let quiet = CommandLine.arguments.contains("--quiet")

func addr(_ s: AudioObjectPropertySelector, _ sc: AudioObjectPropertyScope = kAudioObjectPropertyScopeGlobal) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress(mSelector: s, mScope: sc, mElement: kAudioObjectPropertyElementMain)
}
func str(_ o: AudioObjectID, _ s: AudioObjectPropertySelector) -> String {
    var a = addr(s); var v: Unmanaged<CFString>?; var n = UInt32(MemoryLayout<Unmanaged<CFString>?>.size)
    guard AudioObjectGetPropertyData(o, &a, 0, nil, &n, &v) == noErr, let s = v else { return "" }
    return s.takeRetainedValue() as String
}
func fail(_ m: String, _ st: OSStatus = 0) -> Never {
    FileHandle.standardError.write("tapmon: \(m) (\(st))\n".data(using: .utf8)!)
    exit(1)
}

// --- target device
var dev = AudioObjectID(0)
do {
    var a = addr(kAudioHardwarePropertyDefaultOutputDevice); var n = UInt32(4)
    AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &a, 0, nil, &n, &dev)
    if let want = arg("--device") {
        var da = addr(kAudioHardwarePropertyDevices); var sz: UInt32 = 0
        AudioObjectGetPropertyDataSize(AudioObjectID(kAudioObjectSystemObject), &da, 0, nil, &sz)
        var ids = [AudioObjectID](repeating: 0, count: Int(sz) / 4)
        AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &da, 0, nil, &sz, &ids)
        guard let d = ids.first(where: { str($0, kAudioObjectPropertyName) == want || str($0, kAudioDevicePropertyDeviceUID) == want })
        else { fail("no device \(want)") }
        dev = d
    }
}
let devName = str(dev, kAudioObjectPropertyName)
let devUID = str(dev, kAudioDevicePropertyDeviceUID)

// --- tap of everything played to that device (all processes, stream 0)
let desc = CATapDescription(excludingProcesses: [], deviceUID: devUID, stream: 0)
desc.name = "chipdaw tapmon \(getpid())"
desc.isPrivate = true
desc.muteBehavior = .unmuted
var tapID = AudioObjectID(0)
var st = AudioHardwareCreateProcessTap(desc, &tapID)
if st != noErr { fail("AudioHardwareCreateProcessTap failed", st) }
var fmt = AudioStreamBasicDescription()
do {
    var a = addr(kAudioTapPropertyFormat); var n = UInt32(MemoryLayout<AudioStreamBasicDescription>.size)
    st = AudioObjectGetPropertyData(tapID, &a, 0, nil, &n, &fmt)
    if st != noErr { fail("tap format", st) }
}
let sr = fmt.mSampleRate
let ch = Int(fmt.mChannelsPerFrame)
let interleaved = fmt.mFormatFlags & kAudioFormatFlagIsNonInterleaved == 0
let aggUID = "org.chipdaw.tapmon.\(getpid())"
let aggDesc: [String: Any] = [
    kAudioAggregateDeviceNameKey: "chipdaw tapmon",
    kAudioAggregateDeviceUIDKey: aggUID,
    kAudioAggregateDeviceIsPrivateKey: true,
    kAudioAggregateDeviceTapAutoStartKey: true,
    kAudioAggregateDeviceTapListKey: [[kAudioSubTapUIDKey: desc.uuid.uuidString, kAudioSubTapDriftCompensationKey: true]],
]
var agg = AudioObjectID(0)
st = AudioHardwareCreateAggregateDevice(aggDesc as CFDictionary, &agg)
if st != noErr { AudioHardwareDestroyProcessTap(tapID); fail("AudioHardwareCreateAggregateDevice failed", st) }

func cleanup() {
    AudioHardwareDestroyAggregateDevice(agg)
    AudioHardwareDestroyProcessTap(tapID)
}

// --- analysis state (touched only on the IO thread, except the lock-protected outputs)
var tb = mach_timebase_info_data_t(); mach_timebase_info(&tb)
func hostNs(_ t: UInt64) -> UInt64 { UInt64((Double(t) * Double(tb.numer) / Double(tb.denom)).rounded()) }
let k = 2.0 * cos(2.0 * Double.pi * pilotHz / sr)
var s1 = 0.0, s2 = 0.0, n = 0
var blocks = 0, avg = 0.0, low = false
var minRatio = Double.infinity
var quietRun = 0
let quietNeed = Int(0.05 * sr)
var lock = os_unfair_lock()
var events: [(String, UInt64)] = []
var dropouts = 0, clicks = 0, peakAbs: Float = 0
var wav = [Float]()
let keepWav = wavPath != nil
var totalFrames = 0
var lastBlocks = 0

let proc: AudioDeviceIOBlock = { _, inData, inTime, _, _ in
    let list = UnsafeMutableAudioBufferListPointer(UnsafeMutablePointer(mutating: inData))
    guard let b0 = list.first, let p = b0.mData else { return }
    let frames = Int(b0.mDataByteSize) / 4 / (interleaved ? ch : 1)
    let x = p.assumingMemoryBound(to: Float.self)
    let t0 = hostNs(inTime.pointee.mHostTime)
    var newEvents: [(String, UInt64)] = []
    for i in 0..<frames {
        let v = Double(x[interleaved ? i * ch : i])
        let tNs = t0 + UInt64(Double(i) / sr * 1e9)
        if abs(v) > 0.1 {
            if quietRun > quietNeed { newEvents.append(("click", tNs)) }
            quietRun = 0
        } else { quietRun += 1 }
        let s0 = v + k * s1 - s2; s2 = s1; s1 = s0; n += 1
        if n == blockLen {
            let amp = sqrt(max(0, s1 * s1 + s2 * s2 - k * s1 * s2)) * 2 / Double(blockLen)
            blocks += 1
            let warm = blocks > 200
            let isLow = warm && amp < 0.3 * avg
            if warm && avg > 0 && !isLow { minRatio = min(minRatio, amp / avg) }
            if isLow && !low { newEvents.append(("dropout", tNs)) }
            low = isLow
            if !isLow { avg = blocks == 1 ? amp : avg * 0.98 + amp * 0.02 }
            s1 = 0; s2 = 0; n = 0
        }
    }
    os_unfair_lock_lock(&lock)
    totalFrames += frames
    for e in newEvents {
        events.append(e)
        if e.0 == "dropout" { dropouts += 1 } else { clicks += 1 }
    }
    if keepWav { for i in 0..<frames { wav.append(x[interleaved ? i * ch : i]) } }
    for i in 0..<frames { peakAbs = max(peakAbs, abs(x[interleaved ? i * ch : i])) }
    os_unfair_lock_unlock(&lock)
}

var procID: AudioDeviceIOProcID?
st = AudioDeviceCreateIOProcIDWithBlock(&procID, agg, nil, proc)
if st != noErr { cleanup(); fail("create IOProc", st) }
let startNs = hostNs(mach_absolute_time())
st = AudioDeviceStart(agg, procID)
if st != noErr { cleanup(); fail("AudioDeviceStart", st) }
FileHandle.standardError.write("tapmon: tapping \"\(devName)\" (\(Int(sr)) Hz, \(ch) ch\(interleaved ? "" : ", non-interleaved")), pilot \(Int(pilotHz)) Hz, block \(blockLen) for \(secs) s\n".data(using: .utf8)!)

var stop = false
signal(SIGINT, SIG_IGN); signal(SIGTERM, SIG_IGN)
let sigs = [SIGINT, SIGTERM].map { s -> DispatchSourceSignal in
    let src = DispatchSource.makeSignalSource(signal: s, queue: .main)
    src.setEventHandler { stop = true }
    src.resume()
    return src
}
_ = sigs
let t0 = Date()
while !stop && Date().timeIntervalSince(t0) < secs {
    RunLoop.main.run(until: Date().addingTimeInterval(1.0))
    if !quiet {
        os_unfair_lock_lock(&lock)
        let line = String(format: "tapmon %6.1f s: %d frames, %d blocks, %d dropouts, %d clicks, peak %.4f",
                          Date().timeIntervalSince(t0), totalFrames, blocks, dropouts, clicks, peakAbs)
        os_unfair_lock_unlock(&lock)
        FileHandle.standardError.write((line + "\n").data(using: .utf8)!)
    }
}
AudioDeviceStop(agg, procID)
if let id = procID { AudioDeviceDestroyIOProcID(agg, id) }
let stopNs = hostNs(mach_absolute_time())
cleanup()

os_unfair_lock_lock(&lock)
if let path = eventsPath {
    var s = "kind,host_ns\nstart,\(startNs)\n"
    for e in events { s += "\(e.0),\(e.1)\n" }
    s += "stop,\(stopNs)\n"
    try? s.write(toFile: path, atomically: true, encoding: .utf8)
}
if let path = wavPath {
    var d = Data()
    func u32(_ v: UInt32) { withUnsafeBytes(of: v.littleEndian) { d.append(contentsOf: $0) } }
    func u16(_ v: UInt16) { withUnsafeBytes(of: v.littleEndian) { d.append(contentsOf: $0) } }
    d.append("RIFF".data(using: .ascii)!); u32(UInt32(36 + wav.count * 4)); d.append("WAVEfmt ".data(using: .ascii)!)
    u32(16); u16(3); u16(1); u32(UInt32(sr)); u32(UInt32(sr) * 4); u16(4); u16(32)
    d.append("data".data(using: .ascii)!); u32(UInt32(wav.count * 4))
    wav.withUnsafeBytes { d.append(contentsOf: $0) }
    try? d.write(to: URL(fileURLWithPath: path))
}
let margin = minRatio.isFinite ? String(format: "%.3f", minRatio) : "n/a"
print("tapmon summary: device \"\(devName)\", \(Int(sr)) Hz, \(totalFrames) frames, \(blocks) blocks, \(dropouts) pilot dropouts, \(clicks) clicks, peak \(String(format: "%.4f", peakAbs)), lowest clean block/level \(margin)")
os_unfair_lock_unlock(&lock)
