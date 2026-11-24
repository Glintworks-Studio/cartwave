// Rejected pacing method (PERF-macos.md §4): ScreenCaptureKit window capture.
// Build: swiftc -O sckwin.swift -o sckwin
// usage: sckwin <windowID> <secs>  -> SCK "complete" frame count and gaps for any window
import Foundation
import ScreenCaptureKit
import CoreMedia
import CoreGraphics
final class R: NSObject, SCStreamOutput {
    var t: [UInt64] = []; var other: [Int: Int] = [:]
    func stream(_ s: SCStream, didOutputSampleBuffer sb: CMSampleBuffer, of type: SCStreamOutputType) {
        guard let att = CMSampleBufferGetSampleAttachmentsArray(sb, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let info = att.first, let raw = info[.status] as? Int else { return }
        if raw == SCFrameStatus.complete.rawValue, let d = info[.displayTime] as? UInt64 { t.append(d) } else { other[raw, default: 0] += 1 }
    }
}
let id = UInt32(CommandLine.arguments[1])!; let secs = Double(CommandLine.arguments[2])!
_ = CGMainDisplayID(); let r = R()
let sem = DispatchSemaphore(value: 0)
_ = Task {
    let c = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
    let w = c.windows.first { $0.windowID == id }!
    print("window:", w.owningApplication?.applicationName ?? "?", w.title ?? "")
    let cfg = SCStreamConfiguration(); cfg.width = 256; cfg.height = 160
    cfg.minimumFrameInterval = CMTime(value: 1, timescale: 1000); cfg.queueDepth = 8; cfg.showsCursor = false
    let s = SCStream(filter: SCContentFilter(desktopIndependentWindow: w), configuration: cfg, delegate: nil)
    try s.addStreamOutput(r, type: .screen, sampleHandlerQueue: DispatchQueue(label: "q"))
    try await s.startCapture()
    try await Task.sleep(nanoseconds: UInt64(secs * 1e9))
    try await s.stopCapture()
    sem.signal()
}
sem.wait()
var tb = mach_timebase_info_data_t(); mach_timebase_info(&tb)
let ms = zip(r.t, r.t.dropFirst()).map { Double($1 - $0) * Double(tb.numer) / Double(tb.denom) / 1e6 }
print("complete:", r.t.count, "other statuses:", r.other, "gaps ms (first 20):", ms.prefix(20).map { String(format: "%.1f", $0) })
