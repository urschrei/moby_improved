import Foundation
import MobyKit
import Observation

/// A saved place.
struct Place: Codable, Equatable {
  var name: String
  var coordinate: Coordinate
}

/// The user's settings, saved on the device.
@MainActor
@Observable
final class Settings {
  struct Values: Codable, Equatable {
    var places: [PlaceKind: Place] = [:]
    var minRangeKm: Double = 10
    var windows: [CommuteWindow] = [
      CommuteWindow(weekdays: 0b001_1111, startMinute: 7 * 60 + 30, endMinute: 9 * 60 + 30),
      CommuteWindow(weekdays: 0b001_1111, startMinute: 16 * 60 + 30, endMinute: 18 * 60 + 30),
    ]
    var commuteIntervalS: Int = 30
    var idleIntervalS: Int = 60
  }

  var values: Values {
    didSet { save() }
  }

  private static let key = "settings"
  private let defaults: UserDefaults

  init(defaults: UserDefaults = .standard) {
    self.defaults = defaults
    values =
      defaults.data(forKey: Self.key)
      .flatMap { try? JSONDecoder().decode(Values.self, from: $0) } ?? Values()
  }

  /// The windows that start before they end. The Rust schedule rejects
  /// the others.
  private var validWindows: [CommuteWindow] {
    values.windows.filter { $0.startMinute < $0.endMinute }
  }

  /// Returns `true` if `date` is in a commute window.
  func isCommuting(at date: Date = .now) -> Bool {
    (try? scheduleIsActive(
      windows: validWindows, nowMs: date.milliseconds, timeZone: TimeZone.current.identifier))
      ?? false
  }

  /// Returns the end of the commute window that contains `date`, or `nil`
  /// if `date` is not in a window.
  func commuteEnd(after date: Date = .now) -> Date? {
    guard isCommuting(at: date),
      let boundary =
        (try? scheduleNextBoundaryMs(
          windows: validWindows, nowMs: date.milliseconds, timeZone: TimeZone.current.identifier))
        ?? nil
    else {
      return nil
    }
    return Date(timeIntervalSince1970: Double(boundary) / 1000)
  }

  /// Returns the time to wait before the next refresh after `date`.
  ///
  /// The wait is shorter in a commute window, and it never goes past the
  /// start or end of a window.
  func refreshInterval(after date: Date = .now) -> Duration {
    let interval = isCommuting(at: date) ? values.commuteIntervalS : values.idleIntervalS
    let boundary = try? scheduleNextBoundaryMs(
      windows: validWindows, nowMs: date.milliseconds, timeZone: TimeZone.current.identifier)
    guard let boundary = boundary ?? nil else {
      return .seconds(interval)
    }
    let untilBoundaryMs = max(boundary - date.milliseconds, 1_000)
    return min(.seconds(interval), .milliseconds(untilBoundaryMs))
  }

  private func save() {
    if let data = try? JSONEncoder().encode(values) {
      defaults.set(data, forKey: Self.key)
    }
  }
}

extension Date {
  var milliseconds: Int64 {
    Int64((timeIntervalSince1970 * 1000).rounded())
  }
}
