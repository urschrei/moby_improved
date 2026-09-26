import Foundation

/// Formats distances in metric units, whatever the region settings prefer.
enum Units {
  /// Formats a distance in metres, rounded to 10 m, for example "180 m".
  static func metres(_ metres: Double) -> String {
    Measurement(value: Double(roundedMetres(metres)), unit: UnitLength.meters)
      .formatted(
        .measurement(width: .abbreviated, usage: .asProvided, numberFormatStyle: wholeNumber))
  }

  /// Rounds a distance in metres to 10 m.
  static func roundedMetres(_ metres: Double) -> Int {
    Int((metres / 10).rounded() * 10)
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
