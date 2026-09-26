import Foundation

/// Formats distances in metric units, whatever the region settings prefer.
enum Units {
  /// Formats a distance in metres, rounded to 10 m, for example "180 m".
  static func metres(_ metres: Double) -> String {
    Measurement(value: (metres / 10).rounded() * 10, unit: UnitLength.meters)
      .formatted(
        .measurement(width: .abbreviated, usage: .asProvided, numberFormatStyle: wholeNumber))
  }

  /// Formats a distance given in metres as whole kilometres, for example
  /// "38 km".
  static func kilometres(_ metres: Double) -> String {
    Measurement(value: metres / 1000, unit: UnitLength.kilometers)
      .formatted(
        .measurement(width: .abbreviated, usage: .asProvided, numberFormatStyle: wholeNumber))
  }

  private static let wholeNumber = FloatingPointFormatStyle<Double>.number.precision(
    .fractionLength(0))
}
