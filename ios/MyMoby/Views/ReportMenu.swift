import MobyKit
import SwiftUI

/// The items that report a bike that cannot be rented.
struct ReportMenuItems: View {
  let bike: Bike
  let report: (BikeReport.Kind, BikeReport.Reason?, Bike) -> Void

  var body: some View {
    Section(bike.number.map { "Report bike \($0)" } ?? "Report this bike") {
      Menu("Needs Service", systemImage: "wrench.and.screwdriver") {
        ForEach(BikeReport.Reason.allCases) { reason in
          Button(reason.title) { report(.needsService, reason, bike) }
        }
      }
      Button("Ghost Bike", systemImage: "questionmark.circle") {
        report(.ghost, nil, bike)
      }
    }
  }
}

/// A button that opens the report items.
struct ReportButton: View {
  let bike: Bike
  let controller: CommuteController

  var body: some View {
    Menu {
      ReportMenuItems(bike: bike, report: controller.report)
    } label: {
      Label("Report", systemImage: "flag")
        .font(.subheadline.weight(.semibold))
    }
  }
}

/// A confirmation of the last report, shown for a few seconds after it.
struct ReportConfirmation: View {
  let reports: ReportLog

  /// The time for which the confirmation shows.
  private let lifetime: TimeInterval = 6

  var body: some View {
    TimelineView(.periodic(from: .now, by: 1)) { context in
      if let error = reports.lastError {
        Label(error, systemImage: "exclamationmark.triangle.fill")
          .foregroundStyle(Theme.warning)
          .font(.footnote.weight(.semibold))
          .padding(.bottom, 12)
      } else if let report = reports.lastReport,
        context.date.timeIntervalSince(report.reportedAt) < lifetime
      {
        Label(
          "Reported \(report.bikeNumber.map { "bike \($0)" } ?? "the bike"): \(report.title.lowercasedFirst)",
          systemImage: "checkmark.circle.fill"
        )
        .foregroundStyle(Theme.accent)
        .font(.footnote.weight(.semibold))
        .padding(.bottom, 12)
      }
    }
  }
}

extension String {
  /// The string with its first character in lower case.
  var lowercasedFirst: String {
    prefix(1).lowercased() + dropFirst()
  }
}
