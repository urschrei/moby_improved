import Foundation
import MobyKit
import Observation

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

  var minRangeM: Double = 10_000
  var refreshInterval: Duration = .seconds(30)

  /// The number of bikes to rank by walking distance.
  let rankedCount: UInt32 = 5
  /// The maximum number of routing requests per refresh.
  let maxRoutingRequests: UInt32 = 12

  private let source: any FeedSource
  private let router = WalkingRouter()

  init(source: any FeedSource) {
    self.source = source
  }

  /// Refreshes every `refreshInterval` until the task is cancelled.
  func run(origin: @escaping @MainActor () -> Coordinate?) async {
    while !Task.isCancelled {
      if let origin = origin() {
        await refresh(from: origin)
      }
      try? await Task.sleep(for: refreshInterval)
    }
  }

  func refresh(from origin: Coordinate) async {
    isRefreshing = true
    defer { isRefreshing = false }
    do {
      let feed = try await source.vehicleStatus()
      self.feed = feed
      bikes = feed.bikes(origin: origin, minRangeM: minRangeM)
      lastError = nil
    } catch {
      lastError = error.localizedDescription
    }
    nearest = await router.rank(
      origin: origin, bikes: bikes, k: rankedCount, maxRequests: maxRoutingRequests)
  }
}
