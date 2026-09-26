import MapKit
import MobyKit
import SwiftUI

/// The map of nearby bikes above the list of the nearest bikes on foot.
struct NearbyView: View {
  let store: BikeStore
  let location: LocationProvider
  let settings: Settings
  @Environment(\.scenePhase) private var scenePhase
  @State private var origin: Origin = .here
  @State private var isShowingSettings = false
  @State private var camera: MapCameraPosition = .userLocation(fallback: .automatic)
  @State private var selectedID: String?
  @State private var route: MKRoute?

  /// The number of bikes to show on the map.
  private let mapLimit = 60

  var body: some View {
    NavigationStack {
      VStack(spacing: 0) {
        OriginPicker(origin: $origin, settings: settings, location: location.coordinate)
        BikeMap(
          bikes: Array(store.bikes.prefix(mapLimit)),
          nearestIDs: Set(store.nearest.map(\.id)),
          route: route,
          camera: $camera,
          selectedID: $selectedID
        )
        .frame(maxHeight: .infinity)
        FeedStatus(feed: store.feed, bikeCount: store.bikes.count, isRefreshing: store.isRefreshing)
        BikeList(
          store: store, location: location, origin: originCoordinate, selectedID: $selectedID
        )
        .frame(maxHeight: .infinity)
      }
      .navigationTitle("Nearby bikes")
      .navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .topBarTrailing) {
          Button("Settings", systemImage: "gearshape") { isShowingSettings = true }
        }
      }
      .sheet(isPresented: $isShowingSettings) {
        SettingsView(settings: settings, location: location.coordinate)
      }
      .sheet(item: selectedBike) { bike in
        BikeDetail(
          bike: bike,
          walked: store.nearest.first { $0.id == bike.id },
          origin: originCoordinate
        )
        .presentationDetents([.height(260)])
        .presentationBackgroundInteraction(.enabled)
      }
    }
    .task { await location.run() }
    .task(id: selectedID) {
      route = nil
      if let bike = selectedBike.wrappedValue, let origin = originCoordinate {
        route = await Handoff.route(from: origin, to: bike)
      }
    }
    .task(id: runKey) {
      guard runKey.isActive, runKey.hasOrigin else { return }
      await store.run { originCoordinate }
    }
    .onChange(of: origin) {
      if let coordinate = originCoordinate, origin != .here {
        camera = .region(
          MKCoordinateRegion(
            center: coordinate.clLocation, latitudinalMeters: 800, longitudinalMeters: 800))
      } else {
        camera = .userLocation(fallback: .automatic)
      }
    }
  }

  private var originCoordinate: Coordinate? {
    origin.coordinate(location: location.coordinate, settings: settings)
  }

  /// The values that restart the refresh loop when they change.
  private struct RunKey: Equatable {
    let isActive: Bool
    let hasOrigin: Bool
    let origin: Origin
    let settings: Settings.Values
  }

  private var runKey: RunKey {
    RunKey(
      isActive: scenePhase == .active,
      hasOrigin: originCoordinate != nil,
      origin: origin,
      settings: settings.values)
  }

  private var selectedBike: Binding<Bike?> {
    Binding(
      get: { store.bikes.first { $0.id == selectedID } },
      set: { selectedID = $0?.id }
    )
  }
}

/// Chooses where to search from.
struct OriginPicker: View {
  @Binding var origin: Origin
  let settings: Settings
  let location: Coordinate?

  var body: some View {
    HStack {
      Picker("Search from", selection: $origin) {
        ForEach(Origin.allCases) { origin in
          Text(origin.title).tag(origin)
        }
      }
      .pickerStyle(.segmented)
      Button("Nearest saved place", systemImage: "scope") {
        if let nearest = Origin.nearestPlace(to: location, settings: settings) {
          origin = nearest
        }
      }
      .labelStyle(.iconOnly)
      .disabled(settings.values.places.isEmpty || location == nil)
    }
    .padding(.horizontal)
    .padding(.vertical, 8)
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
  let origin: Coordinate?
  @Binding var selectedID: String?

  var body: some View {
    List {
      if location.isDenied {
        Text("Location access is off. Turn it on in Settings to find nearby bikes.")
      } else if origin == nil {
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
      Attribution()
        .listRowSeparator(.hidden)
    }
    .listStyle(.plain)
    .refreshable {
      if let origin {
        await store.refresh(from: origin)
      }
    }
  }
}
