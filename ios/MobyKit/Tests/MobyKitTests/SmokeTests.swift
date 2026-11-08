import Foundation
import Testing

@testable import MobyKit

/// Reads a fixture from the Rust crate.
func fixture(_ name: String) throws -> Data {
  let root = URL(filePath: #filePath)
    .deletingLastPathComponent()
    .appending(path: "../../../../core/tests/fixtures/\(name).json")
    .standardized
  return try Data(contentsOf: root)
}

let spire = Coordinate(lat: 53.349805, lon: -6.26031)

@Test func manifestGivesTheVehicleStatusURL() throws {
  let url = try vehicleStatusUrl(manifest: fixture("gbfs"))
  #expect(url == "https://moby-move.rideatom.com/gbfs/v3_0/en/vehicle_status?id=2023")
}

@Test func feedParsesAndFilters() throws {
  let feed = try Feed.parse(body: fixture("vehicle_status"))
  let bikes = feed.bikes(origin: spire, minRangeM: 10_000)

  #expect(feed.vehicleCount() > 500)
  #expect(!bikes.isEmpty)
  #expect(bikes.allSatisfy { $0.rangeM >= 10_000 })
  #expect(zip(bikes, bikes.dropFirst()).allSatisfy { $0.straightLineM <= $1.straightLineM })
}

@Test func malformedFeedThrows() {
  #expect(throws: MobyError.self) {
    try Feed.parse(body: Data("<html>".utf8))
  }
}

@Test func walkerCompletesWithAFakeRouter() throws {
  let feed = try Feed.parse(body: fixture("vehicle_status"))
  let walker = Walker()
  walker.start(origin: spire, feed: feed, minRangeM: 10_000, k: 3, maxRequests: 20)

  var requests = 0
  while let bike = walker.nextRequest() {
    requests += 1
    walker.reportRoute(vehicleId: bike.vehicleId, walkingM: bike.straightLineM * 1.2, walkingS: 60)
  }

  #expect(walker.isComplete())
  #expect(walker.results().count == 3)
  #expect(requests >= 3)
  // The search takes only the bikes that it needs from the index.
  #expect(walker.bikesTaken() < feed.bikes(origin: spire, minRangeM: 10_000).count)
  walker.finish()

  // The same search again uses the cached routes.
  walker.start(origin: spire, feed: feed, minRangeM: 10_000, k: 3, maxRequests: 20)
  #expect(walker.nextRequest() == nil)
  #expect(walker.isComplete())
}

@Test func bikeNumberComesFromTheRentalLink() throws {
  let feed = try Feed.parse(body: fixture("vehicle_status"))
  let bikes = feed.bikes(origin: spire, minRangeM: 0)
  #expect(bikes.allSatisfy { $0.number != nil })
  let bike = Bike(
    vehicleId: "1", coordinate: spire, rangeM: 0, straightLineM: 0,
    rentalUri: "https://moby-move.app.link/2025070027")
  #expect(bike.number == "2025070027")
  let noNumber = Bike(
    vehicleId: "2", coordinate: spire, rangeM: 0, straightLineM: 0,
    rentalUri: "https://moby-move.app.link/")
  #expect(noNumber.number == nil)
}

@Test func walkerWithNoBikesToFindIsComplete() throws {
  let feed = try Feed.parse(body: fixture("vehicle_status"))
  let walker = Walker()
  walker.start(origin: spire, feed: feed, minRangeM: 10_000, k: 0, maxRequests: 20)
  #expect(walker.nextRequest() == nil)
  #expect(walker.isComplete())
  #expect(walker.results().isEmpty)
}

@Test func forecastHasATargetForEachTimeOfTheRequest() throws {
  // The fixture has a fourth target, at 6 h. The app asks for three.
  var response = try #require(
    try JSONSerialization.jsonObject(with: fixture("forecast/response_work")) as? [String: Any])
  let targets = try #require(response["targets"] as? [Any])
  response["targets"] = Array(targets.prefix(3))
  let body = try JSONSerialization.data(withJSONObject: response)

  let forecast = try parseForecast(body: body)

  #expect(forecast.place == "work")
  #expect(forecast.targets.map(\.aheadMin) == [30, 60, 120])
  // The third target is in the 6 h bin, which has no skill.
  #expect(forecast.targets.map(\.percent) == [25, 45, nil])
}

@Test func forecastTargetsAreBeforeTheirTimes() throws {
  let now: Int64 = 1_762_594_200_000
  let targets = try forecastTargetsMs(nowMs: now)
  #expect(targets.map { ($0 - now) / 60_000 } == [28, 58, 118])
}
