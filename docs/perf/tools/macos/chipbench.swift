// chipbench: macOS measurement harness for ChipDAW (the counterpart of bench.ps1 + pacing.ps1).
//
// usage: chipbench --app <path> --scenario idle|demo|200|drag [--secs 12] [--warmup 0]
//                  [--env K=V]... [--framelog <csv>] [--sck <txt>] [--drag-hz 125]
//                  [--trace <out.trace> [--template "Time Profiler"]] [--pos X,Y] [--content-height 900] [--keep-open]
//
// Launches the app (with --size 1440x900), focuses it, puts it into the scenario
// (Space = play, F4 = 200-clip set, then a sine drag on a clip for "drag"), waits
// --warmup seconds, then measures for --secs:
//   * process CPU from proc_pid_rusage (user+system, all threads; one core = 100%),
//     the share of it that ran on P-cores, and the kernel's energy estimate
//     (ri_energy_nj; no sudo needed). Cross-checked against `ps -o cputime`.
//   * with --sck <file>: an independent, outside-the-app pacing record. A
//     ScreenCaptureKit stream on the app's window (desktop-independent, so the
//     cursor and other windows don't count) writes the display time of every
//     frame in which the window's content changed (status .complete), one
//     number (seconds, mach-absolute based) per line.
//   * with --trace <out.trace>: records an Instruments "Metal System Trace" attached to
//     the app over the measured period (analyze with displayed_surfaces.py). This is
//     the validated display-side method; it perturbs the app, so measure CPU in
//     separate runs without it.
// --pos moves the window (top-left, global points) before measuring, e.g. to cover
// other animating content so it doesn't add display updates.
// --grab X,Y and --sway AX,AY (MAC_CATCHUP): where the drag presses (content points)
// and how far it sways (default 300,160 and 120,64: the pre-design layout; the Polish 2
// layout's clip grab is 760,172). Scenarios "play-drag" (Space, then the drag: the piano
// roll's note drag) and "200-drag" (= drag) take them; --drag-mods alt|cmd holds a key.
// Prints one JSON line with the results. Needs Accessibility (input) and Screen
// Recording (SCK) permission for the calling terminal.
//
// Build: swiftc -O chipbench.swift -o chipbench

import AppKit
import ApplicationServices
import CoreGraphics
import CoreMedia
import Darwin
import Foundation
import ScreenCaptureKit

// MARK: arguments

var args = Array(CommandLine.arguments.dropFirst())
func opt(_ name: String) -> String? {
    guard let i = args.firstIndex(of: name), i + 1 < args.count else { return nil }
    return args[i + 1]
}
func opts(_ name: String) -> [String] {
    var out: [String] = []
    for (i, a) in args.enumerated() where a == name && i + 1 < args.count { out.append(args[i + 1]) }
    return out
}
guard let appPath = opt("--app"), let scenario = opt("--scenario") else {
    FileHandle.standardError.write("usage: chipbench --app <path> --scenario idle|demo|200|drag [--secs 12] [--warmup 0] [--env K=V]... [--framelog <csv>] [--sck <txt>] [--drag-hz 125]\n".data(using: .utf8)!)
    exit(2)
}
let secs = Double(opt("--secs") ?? "12")!
let warmup = Double(opt("--warmup") ?? "0")!
let dragHz = Double(opt("--drag-hz") ?? "125")!
let contentHeight = Double(opt("--content-height") ?? "900")!
let keepOpen = args.contains("--keep-open")

// MARK: time

var timebase = mach_timebase_info_data_t()
mach_timebase_info(&timebase)
func machToSec(_ t: UInt64) -> Double { Double(t) * Double(timebase.numer) / Double(timebase.denom) / 1e9 }
func pause(_ s: Double) { Thread.sleep(forTimeInterval: s) }

// MARK: launch

let proc = Process()
proc.executableURL = URL(fileURLWithPath: appPath)
proc.arguments = ["--size", "1440x\(Int(contentHeight))"]
var env = ProcessInfo.processInfo.environment
for kv in opts("--env") {
    let parts = kv.split(separator: "=", maxSplits: 1).map(String.init)
    env[parts[0]] = parts.count > 1 ? parts[1] : ""
}
if let fl = opt("--framelog") { env["CHIPDAW_FRAMELOG"] = fl }
proc.environment = env
proc.standardError = FileHandle.standardError
try! proc.run()
let pid = proc.processIdentifier

func windowInfo() -> (id: CGWindowID, rect: CGRect)? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
    var best: (CGWindowID, CGRect)?
    for w in list where (w[kCGWindowOwnerPID as String] as? Int32) == pid {
        let b = CGRect(dictionaryRepresentation: w[kCGWindowBounds as String] as! CFDictionary)!
        if best == nil || b.width * b.height > best!.1.width * best!.1.height {
            best = (w[kCGWindowNumber as String] as! CGWindowID, b)
        }
    }
    return best
}
var found: (id: CGWindowID, rect: CGRect)?
for _ in 0..<100 { found = windowInfo(); if found != nil { break }; pause(0.1) }
guard found != nil else { proc.terminate(); fatalError("no window") }
pause(1.5)
if let pos = opt("--pos") {
    let xy = pos.split(separator: ",").map { Double($0)! }
    let axApp = AXUIElementCreateApplication(pid)
    var value: CFTypeRef?
    AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &value)
    if let w = (value as? [AXUIElement])?.first {
        var pt = CGPoint(x: xy[0], y: xy[1])
        AXUIElementSetAttributeValue(w, kAXPositionAttribute as CFString, AXValueCreate(.cgPoint, &pt)!)
    }
    pause(0.5)
    found = windowInfo()
}
let win = found!
// Window bounds include the title bar; the content is the bottom `contentHeight` points.
let titleBar = win.rect.height - contentHeight
func screen(_ x: Double, _ y: Double) -> CGPoint { CGPoint(x: win.rect.minX + x, y: win.rect.minY + titleBar + y) }

// MARK: input

@Sendable func post(_ e: CGEvent?) { e?.post(tap: .cghidEventTap) }
func mouse(_ type: CGEventType, _ p: CGPoint) {
    post(CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: p, mouseButton: .left))
}
func key(_ code: CGKeyCode) {
    post(CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true)); pause(0.02)
    post(CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false)); pause(0.06)
}
let kSpace: CGKeyCode = 49, kF4: CGKeyCode = 118

// Focus: activate and click the empty part of the title bar.
NSRunningApplication(processIdentifier: pid)?.activate()
let tb = CGPoint(x: win.rect.midX + 200, y: win.rect.minY + titleBar / 2)
mouse(.mouseMoved, tb); pause(0.05); mouse(.leftMouseDown, tb); pause(0.05); mouse(.leftMouseUp, tb); pause(0.2)
// Park the cursor over the transport bar, as bench.ps1 once did; --park X,Y elsewhere
// (the Polish 2 benches park on empty canvas, 1300,800: the display's hover help at
// 700,40 redraws and costs CPU, DESIGNPASS.md §6).
let park = (opt("--park") ?? "700,40").split(separator: ",").map { Double($0)! }
mouse(.mouseMoved, screen(park[0], park[1])); pause(0.3)

switch scenario {
case "idle": break
case "demo": key(kSpace); pause(2)
case "200", "drag", "200-drag": key(kF4); pause(3)  // F4 also starts playback
case "play-drag": key(kSpace); pause(2)
default: fatalError("unknown scenario \(scenario)")
}

// MARK: drag (runs on a background thread for the measured period)

var dragging = true
var dragThread: Thread?
let grab = (opt("--grab") ?? "300,160").split(separator: ",").map { Double($0)! }
let sway = (opt("--sway") ?? "120,64").split(separator: ",").map { Double($0)! }
let dragFlags: CGEventFlags = opt("--drag-mods") == "alt" ? .maskAlternate : opt("--drag-mods") == "cmd" ? .maskCommand : []
let isDrag = ["drag", "200-drag", "play-drag"].contains(scenario)
if isDrag {
    // Same path as bench.ps1 (client 300,160; sine 120 px / 64 px), parameterized by
    // time so the motion speed matches it (one step per ~38 ms there), but with
    // events at --drag-hz, like a real mouse.
    let p0 = screen(grab[0], grab[1])
    if !dragFlags.isEmpty {
        let code: CGKeyCode = dragFlags == .maskAlternate ? 58 : 55
        let e = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true); e?.flags = dragFlags; post(e); pause(0.05)
    }
    func dm(_ t: CGEventType, _ p: CGPoint) {
        let e = CGEvent(mouseEventSource: nil, mouseType: t, mouseCursorPosition: p, mouseButton: .left); e?.flags = dragFlags; post(e)
    }
    dm(.mouseMoved, p0); pause(0.03); dm(.leftMouseDown, p0); pause(0.04)
    dragThread = Thread {
        let t0 = Date()
        while dragging {
            let i = Date().timeIntervalSince(t0) / 0.038
            dm(.leftMouseDragged, screen(grab[0] + (sway[0] * sin(i / 12)).rounded(), grab[1] + (sway[1] * sin(i / 30)).rounded()))
            pause(1 / dragHz)
        }
    }
    dragThread!.start()
}
pause(warmup)

// MARK: ScreenCaptureKit window-update recorder

final class Recorder: NSObject, SCStreamOutput {
    var times: [UInt64] = []
    var idle = 0
    let lock = NSLock()
    func stream(_ s: SCStream, didOutputSampleBuffer sb: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .screen,
              let att = CMSampleBufferGetSampleAttachmentsArray(sb, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let info = att.first,
              let raw = info[.status] as? Int, let status = SCFrameStatus(rawValue: raw) else { return }
        lock.lock(); defer { lock.unlock() }
        if status == .complete, let t = info[.displayTime] as? UInt64 { times.append(t) } else { idle += 1 }
    }
}
var stream: SCStream?
let recorder = Recorder()
if let sckPath = opt("--sck") {
    let sem = DispatchSemaphore(value: 0)
    _ = Task {
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        guard let w = content.windows.first(where: { $0.windowID == win.id }) else { fatalError("SCK: window not found") }
        let cfg = SCStreamConfiguration()
        // Small output: we only need the per-frame status/timestamps, not pixels.
        cfg.width = 256; cfg.height = 160
        cfg.minimumFrameInterval = CMTime(value: 1, timescale: 1000)  // as fast as the server delivers
        cfg.queueDepth = 8
        cfg.showsCursor = false
        let s = SCStream(filter: SCContentFilter(desktopIndependentWindow: w), configuration: cfg, delegate: nil)
        try s.addStreamOutput(recorder, type: .screen, sampleHandlerQueue: DispatchQueue(label: "sck"))
        try await s.startCapture()
        stream = s
        sem.signal()
    }
    sem.wait()
    _ = sckPath
    pause(0.5)  // let the stream settle; samples before the CPU window are trimmed below
}

// MARK: measure

func rusage() -> rusage_info_v6 {
    var ri = rusage_info_v6()
    let r = withUnsafeMutablePointer(to: &ri) {
        $0.withMemoryRebound(to: rusage_info_t?.self, capacity: 1) { proc_pid_rusage(pid, RUSAGE_INFO_V6, $0) }
    }
    precondition(r == 0, "proc_pid_rusage failed")
    return ri
}
func psCpuSeconds() -> Double {
    let p = Process(); let pipe = Pipe()
    p.executableURL = URL(fileURLWithPath: "/bin/ps"); p.arguments = ["-o", "cputime=", "-p", "\(pid)"]
    p.standardOutput = pipe; try! p.run(); p.waitUntilExit()
    let s = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)!.trimmingCharacters(in: .whitespacesAndNewlines)
    // [[hh:]mm:]ss.cc
    return s.split(separator: ":").reduce(0.0) { $0 * 60 + Double($1)! }
}

var tracer: Process?
if let tracePath = opt("--trace") {
    try? FileManager.default.removeItem(atPath: tracePath)
    let x = Process()
    x.executableURL = URL(fileURLWithPath: "/usr/bin/xcrun")
    x.arguments = ["xctrace", "record", "--template", opt("--template") ?? "Metal System Trace", "--attach", "\(pid)",
                   "--time-limit", "\(Int(secs))s", "--output", tracePath]
    x.standardOutput = FileHandle.nullDevice
    try! x.run()
    tracer = x
    pause(1.5)  // xctrace start-up; the trace covers the measured period that follows
}

let r0 = rusage(); let ps0 = psCpuSeconds(); let t0 = mach_absolute_time()
pause(secs)
let r1 = rusage(); let ps1 = psCpuSeconds(); let t1 = mach_absolute_time()

tracer?.waitUntilExit()
dragging = false
if isDrag {
    pause(0.05); mouse(.leftMouseUp, screen(grab[0], grab[1]))
    if !dragFlags.isEmpty { let e = CGEvent(keyboardEventSource: nil, virtualKey: dragFlags == .maskAlternate ? 58 : 55, keyDown: false); post(e) }
}
if let s = stream {
    let sem = DispatchSemaphore(value: 0)
    Task { try? await s.stopCapture(); sem.signal() }
    sem.wait()
}
if !keepOpen { proc.terminate(); proc.waitUntilExit() }

// rusage times are mach-absolute units on Apple Silicon.
let wall = machToSec(t1 - t0)
let cpu = machToSec((r1.ri_user_time + r1.ri_system_time) - (r0.ri_user_time + r0.ri_system_time))
let pcpu = machToSec((r1.ri_user_ptime + r1.ri_system_ptime) - (r0.ri_user_ptime + r0.ri_system_ptime))
let energyJ = Double(r1.ri_energy_nj - r0.ri_energy_nj) / 1e9
let ginstr = Double(r1.ri_instructions - r0.ri_instructions) / 1e9
let gcycles = Double(r1.ri_cycles - r0.ri_cycles) / 1e9
let wakeups = Double((r1.ri_pkg_idle_wkups + r1.ri_interrupt_wkups) - (r0.ri_pkg_idle_wkups + r0.ri_interrupt_wkups))

var sckInfo = ""
if let sckPath = opt("--sck") {
    recorder.lock.lock()
    let ts = recorder.times.filter { $0 >= t0 && $0 <= t1 }.map(machToSec)
    let idle = recorder.idle
    recorder.lock.unlock()
    try! ts.map { String(format: "%.6f", $0) }.joined(separator: "\n").write(toFile: sckPath, atomically: true, encoding: .utf8)
    sckInfo = String(format: ", \"sck_updates\": %d, \"sck_idle_callbacks\": %d", ts.count, idle)
}
print(String(format: "{\"scenario\": \"%@\", \"secs\": %.2f, \"cpu_pct\": %.2f, \"cpu_pct_ps\": %.2f, \"pcore_share\": %.2f, \"power_mw\": %.1f, \"wakeups_per_s\": %.0f, \"ginstr_per_s\": %.3f, \"gcycles_per_s\": %.3f%@}",
             scenario, wall, cpu / wall * 100, (ps1 - ps0) / wall * 100, cpu > 0 ? pcpu / cpu : 0, energyJ / wall * 1000, wakeups / wall, ginstr / wall, gcycles / wall, sckInfo))
