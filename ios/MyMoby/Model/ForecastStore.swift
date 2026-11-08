import Foundation
import MobyKit
import Observation

/// The forecast for the saved place that is the origin.
@MainActor
@Observable
final class ForecastStore {
  /// The state of the forecast.
  enum State: Equatable {
    /// There is no forecast for the origin: the app has no host or token, the
    /// origin is not a saved place, or the service has no reach for it.
    case none
    /// The first request for the place has not finished.
    case loading
    /// The last request gave this forecast.
    case loaded(PlaceForecast)
    /// The last request failed.
    case unavailable
  }

  /// The time between requests. Positions in the feed change in steps of
  /// about 20 minutes.
  static let interval: Duration = .seconds(5 * 60)

  private(set) var place: PlaceKind?
  private(set) var state: State = .none

  private let client: ForecastClient?

  init(client: ForecastClient?) {
    self.client = client
  }

  /// Gets the forecast for `kind` now and after each `interval`, until the
  /// task is cancelled. The service knows a place by the raw value of its
  /// kind.
  func run(for kind: PlaceKind) async {
    guard let client else {
      clear()
      return
    }
    if place != kind {
      place = kind
      state = .loading
    }
    while !Task.isCancelled {
      do {
        let forecast = try await client.forecast(place: kind.rawValue)
        guard !Task.isCancelled else { return }
        state = .loaded(forecast)
      } catch ForecastFailure.unknownPlace {
        state = .none
        return
      } catch {
        guard !Task.isCancelled else { return }
        Log.forecast.error("forecast failed: \(error.localizedDescription)")
        // An old forecast has the wrong times, so the app does not keep it.
        state = .unavailable
      }
      try? await Task.sleep(for: Self.interval)
    }
  }

  /// Removes the forecast, when the origin is not a saved place.
  func clear() {
    place = nil
    state = .none
  }
}
