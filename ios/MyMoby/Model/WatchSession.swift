import ActivityKit
import Foundation
import Observation

/// Keeps the app running in the background, and shows the Live Activity.
@MainActor
@Observable
final class WatchSession {
  typealias State = BikeActivityAttributes.ContentState

  private(set) var isRunning = false
  private let location: LocationProvider
  /// The identifier of the Live Activity. `Activity` is not `Sendable`, so
  /// the nonisolated helpers look it up by this identifier.
  private var activityID: String?

  init(location: LocationProvider) {
    self.location = location
  }

  /// Starts the session. Call this only when the app is in the foreground.
  func start(state: State) {
    guard !isRunning else { return }
    location.setBackgroundUpdates(true)
    if ActivityAuthorizationInfo().areActivitiesEnabled {
      activityID =
        try? Activity.request(attributes: BikeActivityAttributes(), content: Self.content(state)).id
    }
    isRunning = true
  }

  /// Updates the Live Activity. With `alert`, the update also lights the
  /// screen and plays a sound.
  func update(_ state: State, alert isAlert: Bool) async {
    guard let activityID else { return }
    let alert = state.message.flatMap { message in isAlert ? message : nil }.map {
      AlertConfiguration(
        title: "MyMoby", body: LocalizedStringResource(stringLiteral: $0), sound: .default)
    }
    await Self.update(id: activityID, content: Self.content(state), alert: alert)
  }

  func stop() async {
    location.setBackgroundUpdates(false)
    if let activityID {
      await Self.end(id: activityID)
    }
    activityID = nil
    isRunning = false
  }

  private nonisolated static func content(_ state: State) -> ActivityContent<State> {
    // Mark the content stale if no update arrives for three minutes.
    ActivityContent(state: state, staleDate: .now.addingTimeInterval(180))
  }

  private nonisolated static func activity(id: String) -> Activity<BikeActivityAttributes>? {
    Activity<BikeActivityAttributes>.activities.first { $0.id == id }
  }

  private nonisolated static func update(
    id: String, content: ActivityContent<State>, alert: AlertConfiguration?
  ) async {
    await activity(id: id)?.update(content, alertConfiguration: alert)
  }

  private nonisolated static func end(id: String) async {
    await activity(id: id)?.end(nil, dismissalPolicy: .immediate)
  }
}
