import MobyKit
import SwiftUI

struct BikeListView: View {
  let store: BikeStore
  let location: LocationProvider
  @Environment(\.scenePhase) private var scenePhase

  var body: some View {
    NavigationStack {
      List {
        if location.isDenied {
          Text("Location access is off. Turn it on in Settings to find nearby bikes.")
        } else if location.coordinate == nil {
          Text("Finding your location…")
        }
        if let error = store.lastError {
          Text(error).foregroundStyle(.red)
        }
        ForEach(store.nearest, id: \.bike.vehicleId) { walked in
          BikeRow(walked: walked)
        }
      }
      .navigationTitle("Nearby bikes")
      .refreshable {
        if let origin = location.coordinate {
          await store.refresh(from: origin)
        }
      }
      .task { await location.run() }
      .task(id: scenePhase == .active && location.coordinate != nil) {
        guard scenePhase == .active else { return }
        await store.run { location.coordinate }
      }
    }
  }
}

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
      Label(
        Measurement(value: walked.bike.rangeM / 1000, unit: UnitLength.kilometers)
          .formatted(
            .measurement(
              width: .abbreviated, numberFormatStyle: .number.precision(.fractionLength(0)))),
        systemImage: "battery.75percent"
      )
      .foregroundStyle(.secondary)
    }
    .accessibilityElement(children: .combine)
  }
}
