import MapKit
import MobyKit
import SwiftUI

/// The actions for one bike.
struct BikeDetail: View {
  let bike: Bike
  let walked: WalkedBike?
  let controller: CommuteController

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      HStack {
        if let walked {
          Text(walked.walkDuration, format: .units(allowed: [.minutes], width: .wide))
            .font(.title2.bold())
            + Text(" walk")
            .font(.title2)
        } else {
          Text(
            Measurement(value: bike.straightLineM, unit: UnitLength.meters),
            format: .measurement(width: .abbreviated, usage: .road)
          )
          .font(.title2.bold())
        }
        Spacer()
        RangeLabel(rangeM: bike.rangeM)
      }
      Button {
        controller.walk(to: bike)
      } label: {
        Label("Walk there", systemImage: "figure.walk")
          .frame(maxWidth: .infinity)
      }
      .buttonStyle(.bordered)
      .controlSize(.large)
      if Handoff.rentalURL(for: bike) != nil {
        Button {
          controller.arrived()
          Task { await Handoff.openInMoby(bike) }
        } label: {
          Label("Open in MOBY", systemImage: "lock.open")
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.large)
      }
    }
    .padding()
  }
}
