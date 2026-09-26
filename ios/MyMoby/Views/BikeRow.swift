import MobyKit
import SwiftUI

struct BikeRow: View {
  let walked: WalkedBike

  var body: some View {
    HStack {
      VStack(alignment: .leading) {
        Text(
          Duration.seconds(walked.walkingS),
          format: .units(allowed: [.minutes], width: .abbreviated)
        )
        .font(.headline)
        Text(
          Measurement(value: walked.walkingM, unit: UnitLength.meters),
          format: .measurement(width: .abbreviated, usage: .road)
        )
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
      Measurement(value: rangeM / 1000, unit: UnitLength.kilometers)
        .formatted(
          .measurement(
            width: .abbreviated, numberFormatStyle: .number.precision(.fractionLength(0)))),
      systemImage: "battery.75percent"
    )
    .foregroundStyle(.secondary)
    .accessibilityLabel("Range \(Int(rangeM / 1000)) kilometres")
  }
}
