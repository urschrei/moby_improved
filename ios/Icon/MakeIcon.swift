// Draws the app icon: the green minute badge from the map, with a bicycle
// in it, on a deep green field.
//
// Usage: swift ios/Icon/MakeIcon.swift OUTPUT_DIRECTORY
//
// Writes the three iOS 18 variants: AppIcon.png, AppIcon-Dark.png and
// AppIcon-Tinted.png, each 1024 x 1024 pixels.

import AppKit

let size = 1024.0

/// Makes a colour from an RGB value.
func colour(_ rgb: UInt32, alpha: CGFloat = 1) -> NSColor {
  NSColor(
    srgbRed: CGFloat((rgb >> 16) & 0xFF) / 255,
    green: CGFloat((rgb >> 8) & 0xFF) / 255,
    blue: CGFloat(rgb & 0xFF) / 255,
    alpha: alpha)
}

struct Variant {
  let file: String
  /// The colour of the field, or `nil` for a transparent field.
  let field: NSColor?
  let badge: NSColor
  let bike: NSColor
}

let variants = [
  Variant(file: "AppIcon.png", field: colour(0x00_5C_3D), badge: .white, bike: colour(0x00_7F_55)),
  Variant(
    file: "AppIcon-Dark.png", field: nil, badge: colour(0x45_C9_8E), bike: colour(0x00_24_16)),
  Variant(
    file: "AppIcon-Tinted.png", field: nil, badge: colour(0xFF_FF_FF), bike: colour(0x40_40_40)),
]

/// Returns the outline of the badge: a capsule with a pointer below its
/// centre, as on the map.
func badgePath() -> NSBezierPath {
  let width = 740.0
  let height = 420.0
  let pointerWidth = 150.0
  let pointer = 90.0
  // Centre the badge and its pointer together.
  let bottom = (size - height - pointer) / 2 + pointer
  let capsule = NSRect(x: (size - width) / 2, y: bottom, width: width, height: height)
  let path = NSBezierPath(roundedRect: capsule, xRadius: height / 2, yRadius: height / 2)
  let tip = NSBezierPath()
  tip.move(to: NSPoint(x: size / 2 - pointerWidth / 2, y: bottom + 1))
  tip.line(to: NSPoint(x: size / 2, y: bottom - pointer))
  tip.line(to: NSPoint(x: size / 2 + pointerWidth / 2, y: bottom + 1))
  tip.close()
  tip.lineJoinStyle = .round
  path.append(tip)
  return path
}

/// Returns the bicycle symbol in `colour`, `height` points high.
func bicycle(colour: NSColor, height: CGFloat) -> NSImage {
  let configuration = NSImage.SymbolConfiguration(pointSize: height, weight: .heavy)
    .applying(NSImage.SymbolConfiguration(paletteColors: [colour]))
  guard
    let symbol = NSImage(systemSymbolName: "bicycle", accessibilityDescription: nil)?
      .withSymbolConfiguration(configuration)
  else {
    fatalError("The bicycle symbol is not available.")
  }
  return symbol
}

func draw(_ variant: Variant) -> Data {
  guard
    let bitmap = NSBitmapImageRep(
      bitmapDataPlanes: nil, pixelsWide: Int(size), pixelsHigh: Int(size), bitsPerSample: 8,
      samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
      bytesPerRow: 0, bitsPerPixel: 0)
  else {
    fatalError("Cannot make a bitmap.")
  }
  bitmap.size = NSSize(width: size, height: size)
  NSGraphicsContext.saveGraphicsState()
  NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)

  if let field = variant.field {
    field.setFill()
    NSRect(x: 0, y: 0, width: size, height: size).fill()
  }

  let badge = badgePath()
  variant.badge.setFill()
  badge.fill()

  let symbol = bicycle(colour: variant.bike, height: 300)
  let symbolSize = symbol.size
  let capsule = badge.bounds
  // Centre the symbol in the capsule, above the pointer.
  let capsuleMidY = capsule.maxY - 210
  symbol.draw(
    in: NSRect(
      x: (size - symbolSize.width) / 2, y: capsuleMidY - symbolSize.height / 2,
      width: symbolSize.width, height: symbolSize.height))

  NSGraphicsContext.restoreGraphicsState()
  guard let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("Cannot encode the icon.")
  }
  return png
}

let arguments = CommandLine.arguments
guard arguments.count == 2 else {
  print("Usage: swift MakeIcon.swift OUTPUT_DIRECTORY")
  exit(1)
}
let directory = URL(fileURLWithPath: arguments[1])
for variant in variants {
  try draw(variant).write(to: directory.appendingPathComponent(variant.file))
}
