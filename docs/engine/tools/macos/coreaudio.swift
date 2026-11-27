// CoreAudio device tool for the Cartwave engine tests (macOS). No dependencies.
//
//   swiftc -O coreaudio.swift -o coreaudio
//   coreaudio list                  every device: transport, channels, rates, buffer
//                                   frame size and range, latency, safety offset
//   coreaudio default               the default output device (name and UID)
//   coreaudio set-default <name|uid> make a device the default output (and the
//                                   system-sounds output stays as it was)
//   coreaudio watch <secs>          print default-output changes and every
//                                   device's alive / rate changes as they happen
//
// Used by test E (switch the default output and back) instead of the GPL
// SwitchAudioSource. Nothing here is hard-coded to a machine.

import CoreAudio
import Foundation

func addr(_ sel: AudioObjectPropertySelector,
          _ scope: AudioObjectPropertyScope = kAudioObjectPropertyScopeGlobal) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress(mSelector: sel, mScope: scope, mElement: kAudioObjectPropertyElementMain)
}

func getU32(_ obj: AudioObjectID, _ sel: AudioObjectPropertySelector,
            _ scope: AudioObjectPropertyScope = kAudioObjectPropertyScopeGlobal) -> UInt32? {
    var a = addr(sel, scope)
    var v: UInt32 = 0
    var size = UInt32(MemoryLayout<UInt32>.size)
    return AudioObjectGetPropertyData(obj, &a, 0, nil, &size, &v) == noErr ? v : nil
}

func getF64(_ obj: AudioObjectID, _ sel: AudioObjectPropertySelector) -> Double? {
    var a = addr(sel)
    var v: Double = 0
    var size = UInt32(MemoryLayout<Double>.size)
    return AudioObjectGetPropertyData(obj, &a, 0, nil, &size, &v) == noErr ? v : nil
}

func getString(_ obj: AudioObjectID, _ sel: AudioObjectPropertySelector) -> String {
    var a = addr(sel)
    var v: Unmanaged<CFString>?
    var size = UInt32(MemoryLayout<Unmanaged<CFString>?>.size)
    guard AudioObjectGetPropertyData(obj, &a, 0, nil, &size, &v) == noErr, let s = v else { return "?" }
    return s.takeRetainedValue() as String
}

func getArray<T>(_ obj: AudioObjectID, _ sel: AudioObjectPropertySelector,
                 _ scope: AudioObjectPropertyScope = kAudioObjectPropertyScopeGlobal, _ zero: T) -> [T] {
    var a = addr(sel, scope)
    var size: UInt32 = 0
    guard AudioObjectGetPropertyDataSize(obj, &a, 0, nil, &size) == noErr, size > 0 else { return [] }
    var out = [T](repeating: zero, count: Int(size) / MemoryLayout<T>.stride)
    let st = out.withUnsafeMutableBytes { AudioObjectGetPropertyData(obj, &a, 0, nil, &size, $0.baseAddress!) }
    return st == noErr ? out : []
}

func devices() -> [AudioObjectID] {
    getArray(AudioObjectID(kAudioObjectSystemObject), kAudioHardwarePropertyDevices,
             kAudioObjectPropertyScopeGlobal, AudioObjectID(0))
}

func outputChannels(_ d: AudioObjectID) -> Int {
    var a = addr(kAudioDevicePropertyStreamConfiguration, kAudioObjectPropertyScopeOutput)
    var size: UInt32 = 0
    guard AudioObjectGetPropertyDataSize(d, &a, 0, nil, &size) == noErr, size > 0 else { return 0 }
    let raw = UnsafeMutableRawPointer.allocate(byteCount: Int(size), alignment: 16)
    defer { raw.deallocate() }
    guard AudioObjectGetPropertyData(d, &a, 0, nil, &size, raw) == noErr else { return 0 }
    let list = UnsafeMutableAudioBufferListPointer(raw.assumingMemoryBound(to: AudioBufferList.self))
    return list.reduce(0) { $0 + Int($1.mNumberChannels) }
}

func fourcc(_ v: UInt32) -> String {
    let b = [24, 16, 8, 0].map { UInt8((v >> UInt32($0)) & 0xFF) }
    return String(bytes: b, encoding: .ascii) ?? String(v)
}

func defaultOutput() -> AudioObjectID {
    getU32(AudioObjectID(kAudioObjectSystemObject), kAudioHardwarePropertyDefaultOutputDevice) ?? 0
}

func find(_ key: String) -> AudioObjectID? {
    devices().first { getString($0, kAudioObjectPropertyName) == key || getString($0, kAudioDevicePropertyDeviceUID) == key }
        ?? devices().first { getString($0, kAudioObjectPropertyName).localizedCaseInsensitiveContains(key) && outputChannels($0) > 0 }
}

func list() {
    let def = defaultOutput()
    let sys = getU32(AudioObjectID(kAudioObjectSystemObject), kAudioHardwarePropertyDefaultSystemOutputDevice) ?? 0
    for d in devices() {
        let ch = outputChannels(d)
        if ch == 0 { continue }
        let name = getString(d, kAudioObjectPropertyName)
        let uid = getString(d, kAudioDevicePropertyDeviceUID)
        let transport = fourcc(getU32(d, kAudioDevicePropertyTransportType) ?? 0)
        let sr = getF64(d, kAudioDevicePropertyNominalSampleRate) ?? 0
        let rates: [AudioValueRange] = getArray(d, kAudioDevicePropertyAvailableNominalSampleRates,
                                                kAudioObjectPropertyScopeGlobal, AudioValueRange())
        let buf = getU32(d, kAudioDevicePropertyBufferFrameSize) ?? 0
        let ranges: [AudioValueRange] = getArray(d, kAudioDevicePropertyBufferFrameSizeRange,
                                                 kAudioObjectPropertyScopeGlobal, AudioValueRange())
        let lat = getU32(d, kAudioDevicePropertyLatency, kAudioObjectPropertyScopeOutput) ?? 0
        let safety = getU32(d, kAudioDevicePropertySafetyOffset, kAudioObjectPropertyScopeOutput) ?? 0
        let streams: [AudioObjectID] = getArray(d, kAudioDevicePropertyStreams, kAudioObjectPropertyScopeOutput, AudioObjectID(0))
        let slat = streams.map { getU32($0, kAudioStreamPropertyLatency) ?? 0 }
        let alive = getU32(d, kAudioDevicePropertyDeviceIsAlive) ?? 0
        let tags = [d == def ? "DEFAULT OUTPUT" : nil, d == sys ? "system sounds" : nil].compactMap { $0 }
        print("\(name)\(tags.isEmpty ? "" : "  [" + tags.joined(separator: ", ") + "]")")
        print("  id \(d)  uid \(uid)  transport '\(transport)'  out channels \(ch)  alive \(alive)")
        print("  nominal rate \(sr) Hz; available: " + rates.map {
            $0.mMinimum == $0.mMaximum ? String(format: "%.0f", $0.mMinimum) : String(format: "%.0f-%.0f", $0.mMinimum, $0.mMaximum)
        }.joined(separator: ", "))
        print("  buffer \(buf) frames; range " + ranges.map { String(format: "%.0f-%.0f", $0.mMinimum, $0.mMaximum) }.joined(separator: ", "))
        let ms = { (f: UInt32) in sr > 0 ? String(format: "%.2f ms", Double(f) / sr * 1000) : "?" }
        print("  output latency \(lat) frames (\(ms(lat))), safety offset \(safety) frames (\(ms(safety))), stream latency \(slat) frames")
    }
}

func setDefault(_ key: String) -> Int32 {
    guard let d = find(key) else {
        FileHandle.standardError.write("no output device matching \(key)\n".data(using: .utf8)!)
        return 1
    }
    var a = addr(kAudioHardwarePropertyDefaultOutputDevice)
    var id = d
    let st = AudioObjectSetPropertyData(AudioObjectID(kAudioObjectSystemObject), &a, 0, nil,
                                        UInt32(MemoryLayout<AudioObjectID>.size), &id)
    if st != noErr {
        FileHandle.standardError.write("set default failed: \(st)\n".data(using: .utf8)!)
        return 1
    }
    print("default output -> \(getString(d, kAudioObjectPropertyName))")
    return 0
}

func watch(_ secs: Double) {
    let t0 = Date()
    let stamp = { String(format: "%8.3f", Date().timeIntervalSince(t0)) }
    let q = DispatchQueue(label: "watch")
    var a = addr(kAudioHardwarePropertyDefaultOutputDevice)
    AudioObjectAddPropertyListenerBlock(AudioObjectID(kAudioObjectSystemObject), &a, q) { _, _ in
        print("\(stamp()) default output -> \(getString(defaultOutput(), kAudioObjectPropertyName))")
    }
    var da = addr(kAudioHardwarePropertyDevices)
    AudioObjectAddPropertyListenerBlock(AudioObjectID(kAudioObjectSystemObject), &da, q) { _, _ in
        print("\(stamp()) device list changed: \(devices().count) devices")
    }
    for d in devices() where outputChannels(d) > 0 {
        let name = getString(d, kAudioObjectPropertyName)
        for sel in [kAudioDevicePropertyDeviceIsAlive, kAudioDevicePropertyNominalSampleRate] {
            var pa = addr(sel)
            AudioObjectAddPropertyListenerBlock(d, &pa, q) { _, _ in
                print("\(stamp()) \(name): \(sel == kAudioDevicePropertyDeviceIsAlive ? "alive" : "rate") changed")
            }
        }
    }
    setvbuf(stdout, nil, _IOLBF, 0)
    print("\(stamp()) watching (default: \(getString(defaultOutput(), kAudioObjectPropertyName)))")
    RunLoop.main.run(until: Date().addingTimeInterval(secs))
}

let args = CommandLine.arguments
switch args.count > 1 ? args[1] : "list" {
case "list": list()
case "default":
    let d = defaultOutput()
    print("\(getString(d, kAudioObjectPropertyName))\t\(getString(d, kAudioDevicePropertyDeviceUID))")
case "set-default" where args.count > 2: exit(setDefault(args[2]))
case "watch": watch(args.count > 2 ? Double(args[2]) ?? 60 : 60)
default:
    print("usage: coreaudio list | default | set-default <name|uid> | watch <secs>")
    exit(2)
}
