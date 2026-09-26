import Foundation
import MobyKit
import Observation

/// The bike the rider is walking to.
struct Target: Equatable {
  var bike: Bike
  /// The position of the bike when the rider chose it.
  var chosenAt: Coordinate
  var chosenOn: Date
  /// `true` if the rider has reserved the bike in the MOBY app. The feed then
  /// shows the bike as reserved, or not at all, so the store does not check it.
  var isReserved = false
}

/// A change to the target after a refresh.
enum TargetEvent {
  /// The target was rented; the rider now walks to `next`.
  case replaced(next: WalkedBike)
  /// The target was rented, and no other bike is near.
  case lost
  /// The target moved to a new position.
  case moved(Coordinate)
}

/// The bikes near the current origin, and the state of the last refresh.
@MainActor
@Observable
final class BikeStore {
  /// The available bikes with enough range, nearest in a straight line first.
  private(set) var bikes: [Bike] = []
  /// The bikes with the shortest walk, shortest first.
  private(set) var nearest: [WalkedBike] = []
  private(set) var feed: Feed?
  private(set) var lastError: String?
  private(set) var isRefreshing = false
  private(set) var target: Target?
  /// The origin of the last ranking.
  private(set) var rankedFrom: Coordinate?

  /// The number of bikes to rank by walking distance.
  let rankedCount: UInt32 = 5
  /// The maximum number of routing requests per refresh.
  let maxRoutingRequests: UInt32 = 12
  /// The time after which a target that the rider has not reached is dropped.
  let targetLifetime: TimeInterval = 30 * 60

  private let source: any FeedSource
  private let settings: Settings
  private let router = WalkingRouter()

  init(source: any FeedSource, settings: Settings) {
    self.source = source
    self.settings = settings
  }

  func setTarget(_ bike: Bike, isReserved: Bool = false) {
    target = Target(
      bike: bike, chosenAt: bike.coordinate, chosenOn: .now, isReserved: isReserved)
  }

  func clearTarget() {
    target = nil
  }

  /// Removes the bikes of the previous origin, so that the rider does not see
  /// them while the store ranks the bikes near a new origin.
  func clearBikes() {
    bikes = []
    nearest = []
    rankedFrom = nil
  }

  /// Fetches the feed. With a target, checks the target and ranks bikes only
  /// if the target is gone. Without a target, ranks the bikes near `origin`.
  @discardableResult
  func refresh(from origin: Coordinate) async -> TargetEvent? {
    isRefreshing = true
    defer { isRefreshing = false }
    do {
      let feed = try await source.vehicleStatus()
      self.feed = feed
      bikes = feed.bikes(origin: origin, minRangeM: settings.values.minRangeKm * 1000)
      lastError = nil
      Log.refresh.info("feed has \(self.bikes.count) bikes with enough range")
    } catch {
      Log.refresh.error("refresh failed: \(error.localizedDescription)")
      lastError = error.localizedDescription
      return nil
    }

    if let target, target.chosenOn.timeIntervalSinceNow < -targetLifetime {
      self.target = nil
    }
    guard let target, let feed else {
      await rank(from: origin)
      return nil
    }
    if target.isReserved {
      return nil
    }
    switch feed.targetStatus(vehicleId: target.bike.vehicleId, chosenAt: target.chosenAt) {
    case .available:
      return nil
    case .moved(let coordinate):
      self.target?.bike.coordinate = coordinate
      self.target?.chosenAt = coordinate
      return .moved(coordinate)
    case .gone:
      await rank(from: origin)
      if let next = nearest.first {
        self.target = Target(bike: next.bike, chosenAt: next.bike.coordinate, chosenOn: .now)
        return .replaced(next: next)
      } else {
        self.target = nil
        return .lost
      }
    }
  }

  private func rank(from origin: Coordinate) async {
    rankedFrom = origin
    nearest = await router.rank(
      origin: origin, bikes: bikes, k: rankedCount, maxRequests: maxRoutingRequests)
  }
}
