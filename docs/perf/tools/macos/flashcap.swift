// Finds one-frame visual glitches ("flashes") in a window, as composited by
// the window server: ScreenCaptureKit captures the window at full resolution,
// and a frame is flagged when a block of pixels changes and then reverts in the
// next frame (A-B-A). Steady motion (a moving playhead) never reverts, so it
// isn't flagged. For each flagged frame the previous / flagged / next frames
// are saved as PNGs with the changed blocks outlined.
//
//   swiftc -O flashcap.swift -o flashcap
//   flashcap <pid> <secs> <outdir> [--block 16] [--thresh 12] [--max 20]
//
// Prints a per-second count of complete frames and flagged frames, and the
// flagged blocks' positions (window points).

import AppKit
import CoreMedia
import Foundation
import ScreenCaptureKit

setvbuf(stdout, nil, _IOLBF, 0)
let a = CommandLine.arguments
guard a.count >= 4, let pid = pid_t(a[1]), let secs = Double(a[2]) else {
    print("usage: flashcap <pid> <secs> <outdir> [--block 16] [--thresh 12] [--max 20]"); exit(2)
}
let outDir = a[3]
func opt(_ k: String, _ d: Int) -> Int { a.firstIndex(of: k).flatMap { Int(a[$0 + 1]) } ?? d }
let B = opt("--block", 16), thresh = opt("--thresh", 12), maxSave = opt("--max", 20)
try? FileManager.default.createDirectory(atPath: outDir, withIntermediateDirectories: true)

struct Frame { let w: Int; let h: Int; let lum: [UInt8]; let img: CGImage; let t: Double }

final class Sink: NSObject, SCStreamOutput {
    var frames: [Frame] = []   // last 3
    var flagged = 0, complete = 0, saved = 0
    var perSecond: [Int: (Int, Int)] = [:]
    let t0 = Date()
    let ctx = CIContext()

    func lum(_ img: CGImage) -> [UInt8] {
        let w = img.width, h = img.height
        var buf = [UInt8](repeating: 0, count: w * h)
        let cs = CGColorSpaceCreateDeviceGray()
        let c = CGContext(data: &buf, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w, space: cs, bitmapInfo: 0)!
        c.draw(img, in: CGRect(x: 0, y: 0, width: w, height: h))
        return buf
    }

    /// Mean absolute difference per BxB block.
    func blockDiff(_ x: [UInt8], _ y: [UInt8], _ w: Int, _ h: Int) -> [Int] {
        let bw = w / B, bh = h / B
        var out = [Int](repeating: 0, count: bw * bh)
        for by in 0..<bh {
            for bx in 0..<bw {
                var s = 0
                for yy in 0..<B {
                    let row = (by * B + yy) * w + bx * B
                    for xx in 0..<B { s += abs(Int(x[row + xx]) - Int(y[row + xx])) }
                }
                out[by * bw + bx] = s / (B * B)
            }
        }
        return out
    }

    func save(_ f: Frame, _ name: String, _ boxes: [CGRect]) {
        let w = f.w, h = f.h
        let c = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                          space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        c.draw(f.img, in: CGRect(x: 0, y: 0, width: w, height: h))
        c.setStrokeColor(CGColor(red: 1, green: 0, blue: 0, alpha: 1)); c.setLineWidth(1)
        for b in boxes { c.stroke(CGRect(x: b.minX, y: CGFloat(h) - b.maxY, width: b.width, height: b.height)) }
        let url = URL(fileURLWithPath: "\(outDir)/\(name).png")
        let d = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil)!
        CGImageDestinationAddImage(d, c.makeImage()!, nil); CGImageDestinationFinalize(d)
    }

    func stream(_ s: SCStream, didOutputSampleBuffer sb: CMSampleBuffer, of type: SCStreamOutputType) {
        guard let att = CMSampleBufferGetSampleAttachmentsArray(sb, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let info = att.first, let raw = info[.status] as? Int, raw == SCFrameStatus.complete.rawValue,
              let pb = CMSampleBufferGetImageBuffer(sb) else { return }
        let ci = CIImage(cvPixelBuffer: pb)
        guard let img = ctx.createCGImage(ci, from: ci.extent) else { return }
        let t = Date().timeIntervalSince(t0)
        let f = Frame(w: img.width, h: img.height, lum: lum(img), img: img, t: t)
        complete += 1
        let sec = Int(t)
        perSecond[sec, default: (0, 0)].0 += 1
        frames.append(f)
        if frames.count > 3 { frames.removeFirst() }
        guard frames.count == 3, frames[0].w == f.w, frames[1].w == f.w, frames[0].h == f.h, frames[1].h == f.h else { return }
        let (p, m, n) = (frames[0], frames[1], frames[2])
        let dpm = blockDiff(p.lum, m.lum, f.w, f.h)
        let dpn = blockDiff(p.lum, n.lum, f.w, f.h)
        let bw = f.w / B
        var boxes: [CGRect] = []
        for i in 0..<dpm.count where dpm[i] >= thresh && dpn[i] < thresh / 3 {
            boxes.append(CGRect(x: (i % bw) * B, y: (i / bw) * B, width: B, height: B))
        }
        if !boxes.isEmpty {
            flagged += 1
            perSecond[Int(m.t), default: (0, 0)].1 += 1
            let u = boxes.reduce(boxes[0]) { $0.union($1) }
            print(String(format: "flash at %.3f s: %d blocks, union x %.0f-%.0f y %.0f-%.0f (pixels)", m.t, boxes.count, u.minX, u.maxX, u.minY, u.maxY))
            if saved < maxSave {
                saved += 1
                save(p, String(format: "f%02d_a_prev", saved), boxes)
                save(m, String(format: "f%02d_b_flash", saved), boxes)
                save(n, String(format: "f%02d_c_next", saved), boxes)
            }
        }
    }
}

let sink = Sink()
let sem = DispatchSemaphore(value: 0)
Task {
    do {
        let c = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        guard let w = c.windows.first(where: { $0.owningApplication?.processID == pid && $0.frame.width > 400 }) else {
            print("no window for pid \(pid)"); exit(1)
        }
        let scale = NSScreen.screens.first { $0.frame.intersects(w.frame) }?.backingScaleFactor ?? 1
        print("window \(w.windowID) frame \(w.frame), scale \(scale)")
        let cfg = SCStreamConfiguration()
        cfg.width = Int(w.frame.width * scale); cfg.height = Int(w.frame.height * scale)
        cfg.minimumFrameInterval = CMTime(value: 1, timescale: 600)
        cfg.queueDepth = 6; cfg.showsCursor = false
        cfg.pixelFormat = kCVPixelFormatType_32BGRA
        let s = SCStream(filter: SCContentFilter(desktopIndependentWindow: w), configuration: cfg, delegate: nil)
        try s.addStreamOutput(sink, type: .screen, sampleHandlerQueue: DispatchQueue(label: "q"))
        try await s.startCapture()
        try await Task.sleep(nanoseconds: UInt64(secs * 1e9))
        try await s.stopCapture()
    } catch { print("error: \(error)") }
    sem.signal()
}
sem.wait()
print("complete frames \(sink.complete), flagged \(sink.flagged), saved \(sink.saved)")
print("per second (frames, flagged):", sink.perSecond.sorted { $0.key < $1.key }.map { "\($0.key):\($0.value.0)/\($0.value.1)" }.joined(separator: " "))
