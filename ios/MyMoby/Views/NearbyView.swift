import MapKit
import MobyKit
import SwiftUI

/// The map of nearby bikes, below the sheet with the nearest bike.
struct NearbyView: View {
  let controller: CommuteController
  let location: LocationProvider
  let settings: Settings
  let parking: ParkingStore
  @Environment(\.scenePhase) private var scenePhase
  @State private var origin: Origin = .here
  @State private var isShowingSettings = false
  @State private var camera: MapCameraPosition = .userLocation(fallback: .automatic)
  @State private var selectedID: String?
  @State private var route: MKRoute?
  @State private var detent: PresentationDetent = BikeSheet.collapsed
  @Namespace private var mapScope

  /// The number of bikes to show on the map.
  private let mapLimit = 60

  private var store: BikeStore { controller.store }

  var body: some View {
    BikeMap(
      bikes: Array(store.bikes.prefix(mapLimit)),
      ranked: store.nearest,
      featuredID: featuredID,
      target: store.target,
      bays: bays,
      route: route,
      place: mapPlace,
      camera: $camera,
      selectedID: $selectedID,
      scope: mapScope,
      report: controller.report
    )
    .safeAreaInset(edge: .top) {
      MapControls(
        origin: $origin, controller: controller, settings: settings,
        location: location.coordinate, mapScope: mapScope)
    }
    .mapScope(mapScope)
    .safeAreaPadding(.bottom, BikeSheet.collapsedHeight)
    .sheet(isPresented: .constant(true)) {
      BikeSheet(
        controller: controller, location: location, settings: settings, origin: origin,
        originCoordinate: originCoordinate, nearestBay: bays.first,
        selectedID: $selectedID, isShowingSettings: $isShowingSettings
      )
      .presentationDetents([BikeSheet.collapsed, .medium, .large], selection: $detent)
      .presentationBackgroundInteraction(.enabled(upThrough: .medium))
      .presentationDragIndicator(.visible)
      .interactiveDismissDisabled()
    }
    .onAppear { location.start() }
    .onOpenURL { url in
      if let link = WalkLink(url: url) {
        controller.walk(to: link)
      }
    }
    .task { await parking.load() }
    .task(id: selectedID) {
      route = nil
      if let bike = selectedBike, let origin = originCoordinate {
        route = await Handoff.route(from: origin, to: bike)
      }
    }
    .task(id: runKey) {
      guard runKey.isActive, runKey.hasOrigin else { return }
      await controller.run { originCoordinate }
    }
    .onChange(of: scenePhase, initial: true) {
      if scenePhase == .active {
        controller.appDidBecomeActive()
      }
    }
    .task(id: originCoordinate) {
      if let originCoordinate {
        await controller.originMoved(to: originCoordinate)
      }
    }
    .onChange(of: fitKey) {
      fitCamera()
    }
    .onChange(of: origin) {
      selectedID = nil
      store.clearBikes()
      if let coordinate = originCoordinate, origin != .here {
        camera = .region(
          MKCoordinateRegion(
            center: coordinate.clLocation, latitudinalMeters: 800, longitudinalMeters: 800))
      } else {
        camera = .userLocation(fallback: .automatic)
      }
    }
  }

  /// The bike with the large marker: the target, or else the nearest bike.
  private var featuredID: String? {
    store.target?.bike.id ?? store.nearest.first?.id
  }

  /// The values that make the map fit the origin and the nearest bike
  /// again. The position of the rider is not one of them, so that the map
  /// does not move while the rider walks or pans it.
  private struct FitKey: Equatable {
    let origin: Origin
    let hasOrigin: Bool
    let nearestID: String?
  }

  private var fitKey: FitKey {
    FitKey(
      origin: origin, hasOrigin: originCoordinate != nil, nearestID: store.nearest.first?.id)
  }

  /// Moves the map to show the origin and the nearest bike.
  private func fitCamera() {
    guard let origin = originCoordinate, let nearest = store.nearest.first else { return }
    withAnimation {
      camera = .rect(Self.rect(containing: origin, and: nearest.bike.coordinate))
    }
  }

  /// The shortest side of the fitted area, in metres. A bike next to the
  /// origin does not make the map zoom in further than this.
  private static let minimumFitM = 400.0

  /// Returns an area that contains `a` and `b`, with a margin for the
  /// markers, which rise above the positions that they mark.
  private static func rect(containing a: Coordinate, and b: Coordinate) -> MKMapRect {
    let pointA = MKMapPoint(a.clLocation)
    let pointB = MKMapPoint(b.clLocation)
    let bounds = MKMapRect(
      x: min(pointA.x, pointB.x), y: min(pointA.y, pointB.y),
      width: abs(pointA.x - pointB.x), height: abs(pointA.y - pointB.y))
    let margin = max(bounds.width, bounds.height) * 0.3
    let minimum = minimumFitM * MKMapPointsPerMeterAtLatitude(a.lat)
    let width = max(bounds.width + 2 * margin, minimum)
    let height = max(bounds.height + 2 * margin, minimum)
    return MKMapRect(
      x: bounds.midX - width / 2, y: bounds.midY - height / 2, width: width, height: height)
  }

  /// The saved place that is the origin, for the map.
  private var mapPlace: MapPlace? {
    guard case .place(let kind) = origin, let coordinate = originCoordinate else {
      return nil
    }
    return MapPlace(
      coordinate: coordinate, title: settings.title(for: kind), systemImage: kind.systemImage)
  }

  /// The parking bays nearest to the origin.
  private var bays: [ParkingBay] {
    parking.nearest(to: originCoordinate, count: 8)
  }

  private var originCoordinate: Coordinate? {
    origin.coordinate(location: location.coordinate, settings: settings)
  }

  private var selectedBike: Bike? {
    store.bikes.first { $0.id == selectedID }
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
      // The loop continues in the background while the app watches.
      isActive: scenePhase == .active || controller.watch.isRunning,
      hasOrigin: originCoordinate != nil,
      origin: origin,
      settings: settings.values)
  }
}

/// The controls above the map.
struct MapControls: View {
  @Binding var origin: Origin
  let controller: CommuteController
  let settings: Settings
  let location: Coordinate?
  let mapScope: Namespace.ID

  var body: some View {
    HStack(spacing: 8) {
      OriginMenu(origin: $origin, settings: settings, location: location)
      Spacer()
      WatchControl(controller: controller)
      MapCompass(scope: mapScope)
        .mapControlVisibility(.automatic)
      MapUserLocationButton(scope: mapScope)
        .buttonBorderShape(.circle)
        .tint(.blue)
    }
    .font(.subheadline.weight(.semibold))
    .padding(.horizontal, 16)
    .padding(.top, 4)
  }
}

/// Chooses where to search from.
struct OriginMenu: View {
  @Binding var origin: Origin
  let settings: Settings
  let location: Coordinate?

  var body: some View {
    Menu {
      Picker("Search from", selection: $origin) {
        ForEach(Origin.allCases) { origin in
          Label(origin.title(settings: settings), systemImage: origin.systemImage).tag(origin)
        }
      }
      if let nearest = Origin.nearestPlace(to: location, settings: settings) {
        Button("Nearest Saved Place", systemImage: "scope") {
          origin = nearest
        }
      }
    } label: {
      HStack(spacing: 6) {
        Image(systemName: origin.systemImage)
          .foregroundStyle(Theme.accent)
        Text(origin.title(settings: settings))
          .foregroundStyle(.primary)
        Image(systemName: "chevron.down")
          .font(.caption2.weight(.bold))
          .foregroundStyle(.secondary)
      }
      .padding(.horizontal, 14)
      .frame(height: 44)
      .background(.regularMaterial, in: Capsule())
    }
    .buttonStyle(.plain)
    .accessibilityLabel("Search from \(origin.title(settings: settings))")
  }
}

/// Shows whether the app watches in the background, and starts or stops it.
struct WatchControl: View {
  let controller: CommuteController

  var body: some View {
    if let end = controller.watchEnd {
      Menu {
        Section(Self.description(of: end)) {
          Button("Stop Watching", role: .destructive) {
            Task { await controller.stopWatching() }
          }
        }
      } label: {
        HStack(spacing: 6) {
          Image(systemName: "dot.radiowaves.left.and.right")
            .symbolEffect(.variableColor.iterative, options: .repeat(.continuous))
          Text("Watching")
        }
        .foregroundStyle(Theme.onAccent)
        .padding(.horizontal, 14)
        .frame(height: 44)
        .background(Theme.accent, in: Capsule())
      }
      .buttonStyle(.plain)
      .accessibilityLabel("Watching. \(Self.description(of: end))")
    } else {
      Button {
        controller.startWatching()
      } label: {
        HStack(spacing: 6) {
          Image(systemName: "dot.radiowaves.left.and.right")
            .foregroundStyle(.secondary)
          Text("Watch")
            .foregroundStyle(.primary)
        }
        .padding(.horizontal, 14)
        .frame(height: 44)
        .background(.regularMaterial, in: Capsule())
      }
      .buttonStyle(.plain)
      .accessibilityHint("Keeps the nearest bike up to date on the Lock Screen")
    }
  }

  static func description(of end: WatchEnd) -> String {
    switch end {
    case .reachingBike:
      "The Lock Screen shows your bike until you reach it."
    case .windowEnd(let date):
      "The Lock Screen shows the nearest bike until \(date.formatted(date: .omitted, time: .shortened))."
    case .stopped:
      "The Lock Screen shows the nearest bike until you stop watching."
    }
  }
}
