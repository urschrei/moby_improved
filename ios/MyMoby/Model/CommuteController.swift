import Foundation
import MobyKit
import Observation
import UIKit

/// The condition that stops watching.
enum WatchEnd: Equatable {
  /// Watching stops when the rider reaches the chosen bike.
  case reachingBike
  /// Watching stops at the end of the commute window.
  case windowEnd(Date)
  /// Watching stops when the rider stops it.
  case stopped
}

/// Runs the refresh loop, and keeps the app watching in the background during
/// a commute or while the rider walks to a bike.
@MainActor
@Observable
final class CommuteController {
  let store: BikeStore
  let watch: WatchSession
  private let settings: Settings

  /// The wait before the next refresh. Cancelling it starts the next
  /// refresh at once.
  @ObservationIgnored private var sleeper: Task<Void, Never>?

  /// `true` if the rider started watching with the Watch button. Watching
  /// then continues outside commute windows until the rider stops it.
  private(set) var isWatchingManually = false

  /// `true` if the rider stopped watching during the current commute window.
  /// The app then does not start watching again until the window ends.
  private var isStoppedForWindow = false

  /// The last message about the target, and when it was made.
  private var lastMessage: (text: String, date: Date)?
  /// The time for which the Live Activity shows a message.
  private let messageLifetime: TimeInterval = 180

  /// The refresh interval while the rider walks to a bike.
  private let headingInterval: Duration = .seconds(20)

  init(store: BikeStore, settings: Settings, location: LocationProvider) {
    self.store = store
    self.settings = settings
    watch = WatchSession(location: location)
  }

  /// Starts watching if a commute window is open. Call this when the app
  /// comes to the foreground.
  func appDidBecomeActive() {
    if shouldStartForWindow() {
      watch.start(state: activityState(origin: nil))
    }
  }

  /// Chooses a bike, starts watching it, and opens Maps.
  func walk(to bike: Bike) {
    store.setTarget(bike)
    watch.start(state: activityState(origin: nil))
    Task { await Notifier.shared.requestAuthorization() }
    sleeper?.cancel()
    Handoff.walk(to: bike)
  }

  /// Walks to the bike in a link from the Live Activity. If the bike is not
  /// in the feed, opens Maps at the position in the link.
  func walk(to link: WalkLink) {
    if let target = store.target, target.bike.vehicleId == link.vehicleID {
      Handoff.walk(to: target.bike)
    } else if let bike = store.bikes.first(where: { $0.vehicleId == link.vehicleID }) {
      walk(to: bike)
    } else {
      Handoff.walk(to: Coordinate(lat: link.latitude, lon: link.longitude))
    }
  }

  /// Makes `bike` the target, marks it as reserved, and opens it in the
  /// MOBY app so that the rider can reserve it there.
  ///
  /// The bike is marked before the rider reserves it: otherwise a refresh
  /// while the rider is in the MOBY app would report it as rented.
  func reserve(_ bike: Bike) {
    store.setTarget(bike, isReserved: true)
    watch.start(state: activityState(origin: nil))
    sleeper?.cancel()
    Task { await Handoff.openInMoby(bike) }
  }

  /// Stops watching the target, for example because the rider has reached it.
  func arrived() {
    store.clearTarget()
  }

  /// Starts watching outside a commute window. Call this only when the app
  /// is in the foreground.
  func startWatching() {
    isWatchingManually = true
    watch.start(state: activityState(origin: nil))
  }

  func stopWatching() async {
    isWatchingManually = false
    isStoppedForWindow = settings.isCommuting()
    store.clearTarget()
    await watch.stop()
  }

  /// The condition that stops watching, or `nil` if the app does not watch.
  var watchEnd: WatchEnd? {
    guard watch.isRunning else { return nil }
    if store.target != nil {
      return .reachingBike
    }
    if !isWatchingManually, let end = settings.commuteEnd() {
      return .windowEnd(end)
    }
    return .stopped
  }

  /// Refreshes until the task is cancelled.
  func run(origin: @escaping @MainActor () -> Coordinate?) async {
    Log.refresh.info("refresh loop started")
    defer { Log.refresh.info("refresh loop ended") }
    while !Task.isCancelled {
      if let origin = origin() {
        let event = await store.refresh(from: origin)
        if let target = store.target {
          Log.refresh.info(
            "target \(target.bike.vehicleId, privacy: .public), reserved: \(target.isReserved)")
        }
        if let event {
          announce(event)
        }
        if watch.isRunning {
          await watch.update(activityState(origin: origin, event: event), alert: event != nil)
        }
      }
      if watch.isRunning, store.target == nil, !isWatchingManually, !settings.isCommuting() {
        await watch.stop()
      } else if !watch.isRunning, shouldStartForWindow(),
        UIApplication.shared.applicationState == .active
      {
        // A commute window opened while the app is on screen. The session can
        // start only in the foreground, so start it now.
        watch.start(state: activityState(origin: origin()))
      }
      let interval = store.target == nil ? settings.refreshInterval() : headingInterval
      Log.refresh.info("next refresh in \(interval), watching: \(self.watch.isRunning)")
      let sleeper = Task { _ = try? await Task.sleep(for: interval) }
      self.sleeper = sleeper
      await withTaskCancellationHandler {
        await sleeper.value
      } onCancel: {
        sleeper.cancel()
      }
    }
  }

  /// Returns `true` if a commute window is open and the rider has not
  /// stopped watching during it.
  private func shouldStartForWindow() -> Bool {
    let isCommuting = settings.isCommuting()
    if !isCommuting {
      isStoppedForWindow = false
    }
    return isCommuting && !isStoppedForWindow
  }

  private static func message(for event: TargetEvent?) -> String? {
    switch event {
    case .replaced: "Your bike was taken. Walking to the next one."
    case .lost: "Your bike was taken. No other bike is near."
    case .moved: "Your bike moved."
    case nil: nil
    }
  }

  private func announce(_ event: TargetEvent) {
    Log.refresh.info("target event: \(String(describing: event), privacy: .public)")
    UINotificationFeedbackGenerator().notificationOccurred(.warning)
    switch event {
    case .replaced(let next): Notifier.shared.bikeReplaced(by: next)
    case .lost: Notifier.shared.bikeLost()
    case .moved(let coordinate): Notifier.shared.bikeMoved(to: coordinate)
    }
  }

  private func activityState(origin: Coordinate?, event: TargetEvent? = nil)
    -> BikeActivityAttributes.ContentState
  {
    let updated =
      store.feed.map { Date(timeIntervalSince1970: Double($0.lastUpdatedMs()) / 1000) } ?? .now
    if let text = Self.message(for: event) {
      lastMessage = (text, .now)
    }
    let message = lastMessage.flatMap {
      $0.date.timeIntervalSinceNow > -messageLifetime ? $0.text : nil
    }
    if let target = store.target {
      return BikeActivityAttributes.ContentState(
        mode: target.isReserved ? .reserved : .heading,
        walkMinutes: nil,
        distanceM: origin.map { Int(distanceM(a: $0, b: target.bike.coordinate)) },
        rangeKm: Int(target.bike.rangeM / 1000),
        bikeCount: store.bikes.count,
        message: message,
        updated: updated,
        walk: Self.walkLink(target.bike))
    }
    let nearest = store.nearest.first
    return BikeActivityAttributes.ContentState(
      mode: .watching,
      walkMinutes: nearest?.walkMinutes,
      distanceM: nearest.map { Int($0.walkingM) },
      rangeKm: nearest.map { Int($0.bike.rangeM / 1000) },
      bikeCount: store.bikes.count,
      message: message,
      updated: updated,
      walk: nearest.map { Self.walkLink($0.bike) })
  }

  private static func walkLink(_ bike: Bike) -> WalkLink {
    WalkLink(
      vehicleID: bike.vehicleId, latitude: bike.coordinate.lat, longitude: bike.coordinate.lon)
  }
}
