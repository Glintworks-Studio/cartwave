// img: small image tool for the MAC_CATCHUP checks (no Python imaging on this Mac).
//   swiftc -O img.swift -o img
//   img size <png>
//   img srgb <png> x y [w h]        mean colour of a box, converted to sRGB through the
//                                   image's embedded profile (screencapture tags captures
//                                   with the display's profile): what Digital Color Meter
//                                   shows in "Display in sRGB" mode. Also prints the raw
//                                   (unconverted) values.
//   img mean <png> x y w h          mean luma (sRGB, 0-255) of a box
//   img crop <in> <out> x y w h
//   img stack h|v <out> <in>...     side by side (h) or above each other (v), 8 px gaps
//   img label <in> <out> <text>     adds a 24 px caption strip above the image
//   img diff <a> <b> [thr=2]        pixels whose max channel difference exceeds thr
//   img runs <png> x y h            banding: run lengths of identical pixels down columns x..x+4
// Coordinates are image pixels, origin top-left.
import AppKit
import CoreGraphics
import Foundation
import ImageIO

let a = Array(CommandLine.arguments.dropFirst())
guard !a.isEmpty else { print("see header"); exit(2) }

func load(_ p: String) -> CGImage {
    guard let src = CGImageSourceCreateWithURL(URL(fileURLWithPath: p) as CFURL, nil),
          let im = CGImageSourceCreateImageAtIndex(src, 0, nil) else { fatalError("can't read \(p)") }
    return im
}
func save(_ im: CGImage, _ p: String) {
    let d = CGImageDestinationCreateWithURL(URL(fileURLWithPath: p) as CFURL, "public.png" as CFString, 1, nil)!
    CGImageDestinationAddImage(d, im, nil); CGImageDestinationFinalize(d)
}
/// RGBA8 pixels of `im` rendered into `space` (colour-matched by CoreGraphics).
func pixels(_ im: CGImage, _ space: CGColorSpace?) -> (UnsafeMutablePointer<UInt8>, Int, Int) {
    let (w, h) = (im.width, im.height)
    let buf = UnsafeMutablePointer<UInt8>.allocate(capacity: w * h * 4)
    let cs = space ?? im.colorSpace ?? CGColorSpace(name: CGColorSpace.sRGB)!
    let ctx = CGContext(data: buf, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: cs,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.interpolationQuality = .none
    ctx.draw(im, in: CGRect(x: 0, y: 0, width: w, height: h))
    return (buf, w, h)
}
func boxMean(_ px: UnsafeMutablePointer<UInt8>, _ w: Int, _ x: Int, _ y: Int, _ bw: Int, _ bh: Int) -> [Double] {
    var s = [0.0, 0.0, 0.0]
    for yy in y..<(y + bh) { for xx in x..<(x + bw) { for c in 0..<3 { s[c] += Double(px[(yy * w + xx) * 4 + c]) } } }
    return s.map { $0 / Double(bw * bh) }
}
func newCtx(_ w: Int, _ h: Int) -> CGContext {
    let c = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    c.setFillColor(CGColor(red: 0.2, green: 0.2, blue: 0.2, alpha: 1)); c.fill(CGRect(x: 0, y: 0, width: w, height: h))
    return c
}
func i(_ k: Int) -> Int { Int(a[k])! }

switch a[0] {
case "size":
    let im = load(a[1]); print("\(im.width) \(im.height) \(im.colorSpace?.name.map { $0 as String } ?? "?")")
case "srgb":
    let im = load(a[1])
    let (bw, bh) = a.count > 5 ? (i(4), i(5)) : (1, 1)
    let (s, w, _) = pixels(im, CGColorSpace(name: CGColorSpace.sRGB))
    let (r, _, _) = pixels(im, im.colorSpace)
    let m = boxMean(s, w, i(2), i(3), bw, bh), rm = boxMean(r, w, i(2), i(3), bw, bh)
    func hex(_ v: [Double]) -> String { String(format: "#%02X%02X%02X", Int(v[0].rounded()), Int(v[1].rounded()), Int(v[2].rounded())) }
    print("sRGB \(hex(m)) (\(m.map { String(format: "%.1f", $0) }.joined(separator: ", ")))  raw \(hex(rm))  profile \(im.colorSpace?.name.map { $0 as String } ?? String(describing: im.colorSpace))")
case "mean":
    let im = load(a[1]); let (s, w, _) = pixels(im, CGColorSpace(name: CGColorSpace.sRGB))
    let m = boxMean(s, w, i(2), i(3), i(4), i(5))
    print(String(format: "%.2f", 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]))
case "crop":
    let im = load(a[1]); save(im.cropping(to: CGRect(x: i(3), y: i(4), width: i(5), height: i(6)))!, a[2])
case "stack":
    let ims = a[3...].map(load); let gap = 8
    let horiz = a[1] == "h"
    let W = horiz ? ims.map(\.width).reduce(0, +) + gap * (ims.count - 1) : ims.map(\.width).max()!
    let H = horiz ? ims.map(\.height).max()! : ims.map(\.height).reduce(0, +) + gap * (ims.count - 1)
    let c = newCtx(W, H); var off = 0
    for im in ims {
        if horiz { c.draw(im, in: CGRect(x: off, y: H - im.height, width: im.width, height: im.height)); off += im.width + gap }
        else { off += im.height; c.draw(im, in: CGRect(x: 0, y: H - off, width: im.width, height: im.height)); off += gap }
    }
    save(c.makeImage()!, a[2])
case "label":
    let im = load(a[1]); let H = im.height + 28
    let c = newCtx(im.width, H)
    c.draw(im, in: CGRect(x: 0, y: 0, width: im.width, height: im.height))
    let ns = NSGraphicsContext(cgContext: c, flipped: false)
    NSGraphicsContext.saveGraphicsState(); NSGraphicsContext.current = ns
    (a[3] as NSString).draw(at: NSPoint(x: 8, y: im.height + 6), withAttributes: [.font: NSFont.systemFont(ofSize: 14, weight: .semibold), .foregroundColor: NSColor.white])
    NSGraphicsContext.restoreGraphicsState()
    save(c.makeImage()!, a[2])
case "diff":
    let (p, w, h) = pixels(load(a[1]), CGColorSpace(name: CGColorSpace.sRGB))
    let (q, w2, h2) = pixels(load(a[2]), CGColorSpace(name: CGColorSpace.sRGB))
    guard w == w2 && h == h2 else { print("size differs \(w)x\(h) vs \(w2)x\(h2)"); exit(1) }
    let thr = a.count > 3 ? i(3) : 2
    var n = 0; var box = (Int.max, Int.max, -1, -1)
    for y in 0..<h { for x in 0..<w {
        let k = (y * w + x) * 4
        if (0..<3).map({ abs(Int(p[k + $0]) - Int(q[k + $0])) }).max()! > thr {
            n += 1; box = (min(box.0, x), min(box.1, y), max(box.2, x), max(box.3, y))
        }
    } }
    print("\(n) px differ (> \(thr))" + (n > 0 ? " in box x \(box.0)…\(box.2) y \(box.1)…\(box.3)" : ""))
case "runs":
    let (p, w, _) = pixels(load(a[1]), CGColorSpace(name: CGColorSpace.sRGB))
    let (x0, y0, h) = (i(2), i(3), i(4))
    var all: [Int] = []; var banded = 0; var rows = 0
    for x in x0..<(x0 + 5) {
        var run = 1
        for y in (y0 + 1)..<(y0 + h) {
            let k = (y * w + x) * 4, k0 = ((y - 1) * w + x) * 4
            if p[k] == p[k0] && p[k + 1] == p[k0 + 1] && p[k + 2] == p[k0 + 2] { run += 1 } else { all.append(run); if run >= 4 { banded += run }; run = 1 }
            rows += 1
        }
        all.append(run); if run >= 4 { banded += run }
    }
    let mean = Double(all.reduce(0, +)) / Double(all.count)
    print(String(format: "longest %d, mean run %.1f px, banded rows %.0f%%", all.max()!, mean, 100 * Double(banded) / Double(rows + 5)))
default: print("unknown \(a[0])")
}
