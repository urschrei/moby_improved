import Foundation
import MobyKit
import Observation

/// The rider's reports of bikes that could not be rented, and later checks of
/// whether the feed still lists them.
///
/// The log is a JSON Lines file in the app's Documents folder, which the Files
/// app shows and iCloud Backup includes. The app only adds lines to it.
@MainActor
@Observable
final class ReportLog {
  private(set) var reports: [BikeReport] = []
  private(set) var checks: [UUID: [ListingCheck]] = [:]
  /// The last report that the rider made since the app started.
  private(set) var lastReport: BikeReport?
  private(set) var lastError: String?

  /// The time for which the app checks whether the feed still lists a
  /// reported bike.
  static let followTime: TimeInterval = 24 * 60 * 60
  /// The time after which a check that finds the bike still listed is
  /// recorded again.
  static let recheckTime: TimeInterval = 30 * 60

  static var defaultURL: URL {
    URL.documentsDirectory.appending(path: "reports.jsonl")
  }

  private let fileURL: URL

  init(fileURL: URL = ReportLog.defaultURL) {
    self.fileURL = fileURL
    load()
  }

  /// Records a report about `bike`.
  func report(
    _ kind: BikeReport.Kind, reason: BikeReport.Reason? = nil, bike: Bike, rider: Coordinate?,
    riderAccuracyM: Double?, feed: Feed?
  ) {
    let report = BikeReport(
      id: UUID(),
      reportedAt: .now,
      kind: kind,
      reason: reason,
      vehicleID: bike.vehicleId,
      bikeNumber: bike.number,
      bikeLatitude: bike.coordinate.lat,
      bikeLongitude: bike.coordinate.lon,
      rangeM: bike.rangeM,
      riderLatitude: rider?.lat,
      riderLongitude: rider?.lon,
      riderAccuracyM: riderAccuracyM,
      distanceM: rider.map { distanceM(a: $0, b: bike.coordinate) },
      feedUpdatedAt: feed.map(\.updatedAt))
    if append(.report(report)) {
      reports.append(report)
      lastReport = report
      Log.refresh.info("reported \(bike.vehicleId, privacy: .public): \(report.title)")
    }
  }

  /// Records whether `feed` still lists each bike reported in the last
  /// 24 hours. A check is recorded when the listing changes, or when the
  /// bike is still listed 30 minutes after the last check.
  func checkListings(in feed: Feed, now: Date = .now) {
    for report in reports where now.timeIntervalSince(report.reportedAt) < Self.followTime {
      let status = feed.targetStatus(
        vehicleId: report.vehicleID,
        chosenAt: Coordinate(lat: report.bikeLatitude, lon: report.bikeLongitude))
      let isListed =
        switch status {
        case .available, .moved: true
        case .gone: false
        }
      let last = checks[report.id]?.last
      let isDue =
        last.map {
          $0.isListed != isListed
            || (isListed && now.timeIntervalSince($0.checkedAt) >= Self.recheckTime)
        } ?? true
      guard isDue else { continue }
      let check = ListingCheck(
        reportID: report.id, checkedAt: now, isListed: isListed, feedUpdatedAt: feed.updatedAt)
      if append(.check(check)) {
        checks[report.id, default: []].append(check)
      }
    }
  }

  /// Writes the reports as CSV to a temporary file, and returns its URL.
  func exportCSV() throws -> URL {
    let url = URL.temporaryDirectory.appending(path: "MyMoby reports.csv")
    try csv().write(to: url, atomically: true, encoding: .utf8)
    return url
  }

  /// Returns the reports as CSV, one row for each report, with the result of
  /// the listing checks.
  func csv() -> String {
    let header = [
      "reported_at", "kind", "reason", "bike_number", "vehicle_id", "bike_latitude",
      "bike_longitude", "range_m", "rider_latitude", "rider_longitude", "rider_accuracy_m",
      "distance_m", "feed_updated_at", "last_listed_at", "first_unlisted_at",
    ]
    let rows = reports.map(row)
    return ([header] + rows)
      .map { $0.map(Self.csvField).joined(separator: ",") }
      .joined(separator: "\n") + "\n"
  }

  /// Returns the CSV fields for `report`.
  private func row(_ report: BikeReport) -> [String] {
    let checks = checks[report.id] ?? []
    let lastListed: Date? = checks.last(where: \.isListed)?.checkedAt
    let firstUnlisted: Date? = checks.first(where: { !$0.isListed })?.checkedAt
    let bike: [String] = [
      report.bikeNumber ?? "", report.vehicleID, String(report.bikeLatitude),
      String(report.bikeLongitude), String(Int(report.rangeM)),
    ]
    let rider: [String] = [
      Self.optional(report.riderLatitude), Self.optional(report.riderLongitude),
      Self.optional(report.riderAccuracyM.map { Int($0.rounded()) }),
      Self.optional(report.distanceM.map { Int($0.rounded()) }),
    ]
    let times: [String] = [
      report.feedUpdatedAt.map(Self.timestamp) ?? "",
      lastListed.map(Self.timestamp) ?? "",
      firstUnlisted.map(Self.timestamp) ?? "",
    ]
    let kind: [String] = [
      Self.timestamp(report.reportedAt), report.kind.rawValue, report.reason?.rawValue ?? "",
    ]
    return kind + bike + rider + times
  }

  private static func optional(_ value: (some LosslessStringConvertible)?) -> String {
    value.map { String($0) } ?? ""
  }

  private static func timestamp(_ date: Date) -> String {
    date.formatted(.iso8601)
  }

  private static func csvField(_ field: String) -> String {
    guard field.contains(where: { $0 == "," || $0 == "\"" || $0 == "\n" }) else {
      return field
    }
    return "\"" + field.replacingOccurrences(of: "\"", with: "\"\"") + "\""
  }

  private static let encoder: JSONEncoder = {
    let encoder = JSONEncoder()
    encoder.dateEncodingStrategy = .iso8601
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    return encoder
  }()

  private static let decoder: JSONDecoder = {
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .iso8601
    return decoder
  }()

  /// Adds `entry` as a line at the end of the file. Returns `false` if the
  /// write failed.
  private func append(_ entry: LogEntry) -> Bool {
    do {
      var line = try Self.encoder.encode(entry)
      line.append(UInt8(ascii: "\n"))
      if !FileManager.default.fileExists(atPath: fileURL.path) {
        try Data().write(to: fileURL)
      }
      let handle = try FileHandle(forWritingTo: fileURL)
      defer { try? handle.close() }
      try handle.seekToEnd()
      try handle.write(contentsOf: line)
      lastError = nil
      return true
    } catch {
      Log.refresh.error("report log write failed: \(error.localizedDescription)")
      lastError = "The report could not be saved."
      return false
    }
  }

  /// Reads the file. Lines that do not decode are skipped.
  private func load() {
    guard let data = try? Data(contentsOf: fileURL) else { return }
    for line in data.split(separator: UInt8(ascii: "\n")) {
      switch try? Self.decoder.decode(LogEntry.self, from: Data(line)) {
      case .report(let report): reports.append(report)
      case .check(let check): checks[check.reportID, default: []].append(check)
      case nil: continue
      }
    }
  }
}

extension Feed {
  /// The time of the feed.
  var updatedAt: Date {
    Date(timeIntervalSince1970: Double(lastUpdatedMs()) / 1000)
  }
}
