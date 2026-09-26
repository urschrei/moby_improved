import MobyKit
import SwiftUI

struct BikeRow: View {
  let walked: WalkedBike

  var body: some View {
    HStack {
      VStack(alignment: .leading) {
        Text(
          walked.walkDuration,
          format: .units(allowed: [.minutes], width: .abbreviated)
        )
        .font(.headline)
        Text(Units.metres(walked.walkingM))
          .font(.subheadline)
          .foregroundStyle(.secondary)
      }
      if walked.isEstimate {
        Image(systemName: "questionmark.circle")
          .foregroundStyle(.secondary)
          .accessibilityLabel("Estimated walk")
      }
      Spacer()
      RangeLabel(rangeM: walked.bike.rangeM)
    }
    .contentShape(Rectangle())
    .accessibilityElement(children: .combine)
  }
}

struct RangeLabel: View {
  let rangeM: Double

  var body: some View {
    Label(
      Units.kilometres(rangeM),
      systemImage: "battery.75percent"
    )
    .foregroundStyle(.secondary)
    .accessibilityLabel("Range \(Int(rangeM / 1000)) kilometres")
  }
}
