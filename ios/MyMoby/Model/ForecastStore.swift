import Foundation
import MobyKit
import Observation

/// The forecast for the saved place that is the origin, and the saved places
/// that the service knows.
@MainActor
@Observable
final class ForecastStore {
  /// The state of the forecast.
  enum State: Equatable {
    /// There is no forecast for the origin: the app has no host or token, or
    /// the origin is not a saved place.
    case none
    /// The first request for the place has not finished.
    case loading
    /// The last request gave this forecast.
    case loaded(PlaceForecast)
    /// The place is new or moved, and the forecast starts after the next
    /// nightly fit.
    case pending
    /// The last request failed.
    case unavailable
  }

  /// The time between requests. Positions in the feed change in steps of
  /// about 20 minutes.
  static let interval: Duration = .seconds(5 * 60)

  /// The time before the next request after a network failure. A request
  /// can fail when iOS suspends the app.
  static let retryInterval: Duration = .seconds(30)

  private(set) var place: PlaceKind?
  private(set) var state: State = .none

  private let client: ForecastClient?
  /// The saved places of the last call to `send`, or `nil` before the
  /// first call. Without this, a request before the first call would remove
  /// every place at the service.
  private var places: [PlaceKind: Place]?

  init(client: ForecastClient?) {
    self.client = client
  }

  /// Sends all the saved places to the service, by the raw values of their
  /// kinds. The service removes the places that are not in `places`.
  func send(places: [PlaceKind: Place]) async {
    self.places = places
    await resend()
  }

  /// Gets the forecast for `kind` now and after each `interval`, until the
  /// task is cancelled, also while the app runs in the background. After a
  /// network failure, the next request is after `retryInterval`. The service
  /// knows a place by the raw value of its kind.
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
      var wait = Self.interval
      do {
        let forecast = try await client.forecast(place: kind.rawValue)
        guard !Task.isCancelled else { return }
        state = .loaded(forecast)
      } catch ForecastFailure.pending {
        state = .pending
      } catch ForecastFailure.unknownPlace {
        // The places did not reach the service, or it lost them.
        state = .pending
        await resend()
      } catch {
        guard !Task.isCancelled else { return }
        Log.forecast.error("forecast failed: \(String(describing: error))")
        // An old forecast has the wrong times, so the app does not keep it.
        state = .unavailable
        if error is URLError {
          wait = Self.retryInterval
        }
      }
      try? await Task.sleep(for: wait)
    }
  }

  /// Removes the forecast, when the origin is not a saved place.
  func clear() {
    place = nil
    state = .none
  }

  private func resend() async {
    guard let client, let places else { return }
    let named = Dictionary(
      uniqueKeysWithValues: places.map { ($0.key.rawValue, $0.value.coordinate) })
    do {
      try await client.putPlaces(named)
      Log.forecast.info("sent \(named.count) places")
    } catch {
      Log.forecast.error("cannot send the places: \(String(describing: error))")
    }
  }
}
