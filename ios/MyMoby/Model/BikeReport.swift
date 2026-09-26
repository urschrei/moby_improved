import Foundation
import MobyKit

/// A report by the rider that a bike in the feed could not be rented.
struct BikeReport: Codable, Identifiable, Equatable {
  enum Kind: String, Codable {
    /// The bike is there, but cannot be rented.
    case needsService
    /// The feed lists the bike, but it is not there.
    case ghost
  }

  /// Why a bike that is there cannot be rented.
  enum Reason: String, Codable, CaseIterable, Identifiable {
    /// The MOBY app shows "Needs servicing, not available".
    case needsServicing
    case wontUnlock
    case damaged
    case batteryFlat
    case other

    var id: Self { self }

    var title: String {
      switch self {
      case .needsServicing: "Needs Servicing, Not Available"
      case .wontUnlock: "Won't Unlock"
      case .damaged: "Damaged"
      case .batteryFlat: "Battery Flat"
      case .other: "Other"
      }
    }
  }

  let id: UUID
  let reportedAt: Date
  let kind: Kind
  let reason: Reason?
  let vehicleID: String
  /// The number at the end of the bike's rental link.
  let bikeNumber: String?
  /// The position of the bike in the feed.
  let bikeLatitude: Double
  let bikeLongitude: Double
  /// The range of the bike in the feed.
  let rangeM: Double
  /// The position of the rider, if it is known.
  let riderLatitude: Double?
  let riderLongitude: Double?
  let riderAccuracyM: Double?
  /// The great-circle distance from the rider to the bike.
  let distanceM: Double?
  /// The time of the feed that listed the bike.
  let feedUpdatedAt: Date?

  var title: String {
    switch kind {
    case .ghost: "Ghost bike"
    case .needsService: reason.map { "Needs service: \($0.title)" } ?? "Needs service"
    }
  }
}

/// A check, after a report, of whether the feed still lists the bike.
struct ListingCheck: Codable, Equatable {
  let reportID: UUID
  let checkedAt: Date
  let isListed: Bool
  /// The time of the feed that the check used.
  let feedUpdatedAt: Date?
}

/// A line in the report log.
enum LogEntry: Codable, Equatable {
  case report(BikeReport)
  case check(ListingCheck)

  private enum CodingKeys: String, CodingKey {
    case entry
    case report
    case check
  }

  private enum Entry: String, Codable {
    case report
    case check
  }

  init(from decoder: any Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    switch try container.decode(Entry.self, forKey: .entry) {
    case .report: self = .report(try container.decode(BikeReport.self, forKey: .report))
    case .check: self = .check(try container.decode(ListingCheck.self, forKey: .check))
    }
  }

  func encode(to encoder: any Encoder) throws {
    var container = encoder.container(keyedBy: CodingKeys.self)
    switch self {
    case .report(let report):
      try container.encode(Entry.report, forKey: .entry)
      try container.encode(report, forKey: .report)
    case .check(let check):
      try container.encode(Entry.check, forKey: .entry)
      try container.encode(check, forKey: .check)
    }
  }
}
