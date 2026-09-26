import MapKit
import MobyKit
import SwiftUI

/// The map of nearby bikes above the list of the nearest bikes on foot.
struct NearbyView: View {
  let store: BikeStore
  let location: LocationProvider
  @Environment(\.scenePhase) private var scenePhase
  @State private var camera: MapCameraPosition = .userLocation(fallback: .automatic)
  @State private var selectedID: String?
  @State private var route: MKRoute?

  /// The number of bikes to show on the map.
  private let mapLimit = 60

  var body: some View {
    NavigationStack {
      VStack(spacing: 0) {
        BikeMap(
          bikes: Array(store.bikes.prefix(mapLimit)),
          nearestIDs: Set(store.nearest.map(\.id)),
          route: route,
          camera: $camera,
          selectedID: $selectedID
        )
        .frame(maxHeight: .infinity)
        BikeList(store: store, location: location, selectedID: $selectedID)
          .frame(maxHeight: .infinity)
      }
      .navigationTitle("Nearby bikes")
      .navigationBarTitleDisplayMode(.inline)
      .sheet(item: selectedBike) { bike in
        BikeDetail(
          bike: bike,
          walked: store.nearest.first { $0.id == bike.id },
          origin: location.coordinate
        )
        .presentationDetents([.height(260)])
        .presentationBackgroundInteraction(.enabled)
      }
    }
    .task { await location.run() }
    .task(id: selectedID) {
      route = nil
      if let bike = selectedBike.wrappedValue, let origin = location.coordinate {
        route = await Handoff.route(from: origin, to: bike)
      }
    }
    .task(id: scenePhase == .active && location.coordinate != nil) {
      guard scenePhase == .active else { return }
      await store.run { location.coordinate }
    }
  }

  private var selectedBike: Binding<Bike?> {
    Binding(
      get: { store.bikes.first { $0.id == selectedID } },
      set: { selectedID = $0?.id }
    )
  }
}

struct BikeMap: View {
  let bikes: [Bike]
  let nearestIDs: Set<String>
  let route: MKRoute?
  @Binding var camera: MapCameraPosition
  @Binding var selectedID: String?

  var body: some View {
    Map(position: $camera, selection: $selectedID) {
      UserAnnotation()
      if let route {
        MapPolyline(route)
          .stroke(.blue, style: StrokeStyle(lineWidth: 5, lineCap: .round, dash: [1, 8]))
      }
      ForEach(bikes) { bike in
        Marker("Bike", systemImage: "bicycle", coordinate: bike.coordinate.clLocation)
          .tint(nearestIDs.contains(bike.id) ? .green : .gray)
          .tag(bike.id)
      }
      .annotationTitles(.hidden)
    }
    .mapControls {
      MapUserLocationButton()
      MapCompass()
    }
  }
}

struct BikeList: View {
  let store: BikeStore
  let location: LocationProvider
  @Binding var selectedID: String?

  var body: some View {
    List {
      if location.isDenied {
        Text("Location access is off. Turn it on in Settings to find nearby bikes.")
      } else if location.coordinate == nil {
        Text("Finding your location…")
      }
      if let error = store.lastError {
        Text(error).foregroundStyle(.red)
      }
      ForEach(store.nearest) { walked in
        Button {
          selectedID = walked.id
        } label: {
          BikeRow(walked: walked)
        }
        .tint(.primary)
      }
    }
    .listStyle(.plain)
    .refreshable {
      if let origin = location.coordinate {
        await store.refresh(from: origin)
      }
    }
  }
}
