import SwiftUI
import UIKit

/// The colours and type of the app and the Live Activity.
enum Theme {
  /// The MOBY green. It has a contrast of at least 4.5:1 with white in the
  /// light appearance, and with black in the dark appearance.
  static let accent = Color(light: 0x00_7F_55, dark: 0x45_C9_8E)

  /// The colour of text and symbols on `accent`.
  static let onAccent = Color(light: 0xFF_FF_FF, dark: 0x00_24_16)

  /// The colour of parking bays.
  static let parking = Color(light: 0x1F_6F_EB, dark: 0x5A_9B_FF)

  /// The colour of a warning, such as a rented bike or old data.
  static let warning = Color.orange

  /// The range of a full battery.
  static let fullRangeM = 50_000.0

  /// Returns the colour for a range: green for a range that is sufficient
  /// for several commutes, amber for one or two, red below that.
  static func rangeColour(_ rangeM: Double) -> Color {
    switch rangeM {
    case 20_000...: accent
    case 10_000..<20_000: Color(light: 0xB2_6B_00, dark: 0xFF_C2_4B)
    default: Color(light: 0xC4_2B_1C, dark: 0xFF_6B_5B)
    }
  }
}

extension Color {
  /// Makes a colour from two RGB values, for the light and dark appearances.
  init(light: UInt32, dark: UInt32) {
    self.init(
      uiColor: UIColor { traits in
        let rgb = traits.userInterfaceStyle == .dark ? dark : light
        return UIColor(
          red: CGFloat((rgb >> 16) & 0xFF) / 255,
          green: CGFloat((rgb >> 8) & 0xFF) / 255,
          blue: CGFloat(rgb & 0xFF) / 255,
          alpha: 1)
      })
  }
}

/// A horizontal bar that shows the range of a bike as a part of a full
/// battery.
struct RangeGauge: View {
  let rangeM: Double
  var width: CGFloat = 28

  var body: some View {
    let fraction = min(max(rangeM / Theme.fullRangeM, 0.05), 1)
    Capsule()
      .fill(.quaternary)
      .frame(width: width, height: 6)
      .overlay(alignment: .leading) {
        Capsule()
          .fill(Theme.rangeColour(rangeM))
          .frame(width: width * fraction)
      }
      .accessibilityHidden(true)
  }
}
