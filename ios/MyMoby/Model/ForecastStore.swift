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
    /// There is no forecast for the origin: the app has no host or token,
    /// the origin is not a saved place, or no commute window contains the
    /// time or starts in the next 2 h.
    case none
    /// The first request for the window has not finished.
    case loading
    /// The last request gave this forecast.
    case loaded(PlaceForecast)
    /// The parameters or the feed of the service are too old. The count is
    /// still correct.
    case paused(ForecastPause)
    /// The place is new or moved, and the forecast starts after the next
    /// nightly fit.
    case pending
    /// The last request failed. `isOffline` is `true` for a network failure.
    /// `last` is the last forecast for the same time, if there is one.
    case failed(isOffline: Bool, last: PlaceForecast?)
  }

  /// The time between requests. Positions in the feed change in steps of
  /// about 20 minutes.
  static let interval: Duration = .seconds(5 * 60)

  /// The time before the next request after a network failure. A request
  /// can fail when iOS suspends the app.
  static let retryInterval: Duration = .seconds(30)

  /// The time before the next check if there are no commute windows.
  static let idleInterval: Duration = .seconds(60 * 60)

  private(set) var place: PlaceKind?
  /// The commute window and the target of the forecast, or `nil` if the
  /// state is `none`.
  private(set) var aim: ForecastAim?
  private(set) var state: State = .none
  /// `true` while a request is in progress.
  private(set) var isRequesting = false

  private let client: ForecastClient?
  /// The saved places of the last call to `send`, or `nil` before the
  /// first call. Without this, a request before the first call would remove
  /// every place at the service.
  private var places: [PlaceKind: Place]?
  /// The last forecast, and the time that it is for.
  private var last: (atMs: Int64, forecast: PlaceForecast)?
  /// The wait before the next request, which `retry` cancels.
  private var wait: Task<Void, Never>?

  init(client: ForecastClient?) {
    self.client = client
  }

  /// Sends all the saved places to the service, by the raw values of their
  /// kinds. The service removes the places that are not in `places`.
  func send(places: [PlaceKind: Place]) async {
    self.places = places
    await resend()
  }

  /// Gets the forecast for `kind` while a commute window of `windows`
  /// contains the time or starts in the next 2 h, now and after each
  /// `interval`, until the task is cancelled, also while the app runs in the
  /// background. After a network failure, the next request is after
  /// `retryInterval`. The service knows a place by the raw value of its
  /// kind.
  func run(for kind: PlaceKind, windows: [CommuteWindow]) async {
    guard let client else {
      clear()
      return
    }
    if place != kind {
      place = kind
      aim = nil
      state = .none
      last = nil
    }
    while !Task.isCancelled {
      let now = Date.now
      let plan = try? forecastPlan(
        windows: windows, nowMs: now.milliseconds, timeZone: TimeZone.current.identifier)
      var delay: Duration
      switch plan {
      case .open(let aim):
        if self.aim?.startMs != aim.startMs {
          self.aim = aim
          state = .loading
        }
        delay = await request(kind, aim: aim, client: client)
        // In a window, the target moves with the time. The state and the
        // aim change together, with no suspension between them.
        self.aim = aim
        // At the end of the window, the forecast closes.
        delay = min(delay, .milliseconds(max(aim.endMs - Date.now.milliseconds, 1_000)))
      case .closed(let opensMs):
        aim = nil
        state = .none
        delay =
          opensMs.map { .milliseconds(max($0 - now.milliseconds, 1_000)) } ?? Self.idleInterval
      case nil:
        aim = nil
        state = .none
        delay = Self.idleInterval
      }
      await sleep(for: delay)
    }
  }

  /// Requests the forecast now, after a failure.
  func retry() {
    wait?.cancel()
  }

  /// Removes the forecast, when the origin is not a saved place.
  func clear() {
    place = nil
    aim = nil
    state = .none
    last = nil
  }

  /// Requests the forecast for `aim` and returns the time to wait before
  /// the next request.
  private func request(_ kind: PlaceKind, aim: ForecastAim, client: ForecastClient) async
    -> Duration
  {
    isRequesting = true
    defer { isRequesting = false }
    do {
      let forecast = try await client.forecast(place: kind.rawValue, targetMs: aim.requestMs)
      guard !Task.isCancelled else { return Self.interval }
      state = .loaded(forecast)
      last = (aim.atMs, forecast)
    } catch ForecastFailure.paused(let pause) {
      state = .paused(pause)
    } catch ForecastFailure.pending {
      state = .pending
    } catch ForecastFailure.unknownPlace {
      // The places did not reach the service, or it lost them.
      state = .pending
      await resend()
    } catch {
      guard !Task.isCancelled else { return Self.interval }
      Log.forecast.error("forecast failed: \(String(describing: error))")
      let isOffline = error is URLError
      state = .failed(isOffline: isOffline, last: last?.atMs == aim.atMs ? last?.forecast : nil)
      if isOffline {
        return Self.retryInterval
      }
    }
    return Self.interval
  }

  /// Waits for `delay`, or until `retry` or the cancellation of the task.
  private func sleep(for delay: Duration) async {
    let wait = Task { _ = try? await Task.sleep(for: delay) }
    self.wait = wait
    await withTaskCancellationHandler {
      await wait.value
    } onCancel: {
      wait.cancel()
    }
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
