import MapKit
import MobyKit
import SwiftUI

/// The map of bikes. The ranked bikes show their walking time; the other
/// bikes are dots.
struct BikeMap: View {
  let bikes: [Bike]
  let ranked: [WalkedBike]
  let featuredID: String?
  let target: Target?
  let bays: [ParkingBay]
  let route: MKRoute?
  /// The saved place that the bikes are ranked from, or `nil` for the
  /// current location.
  let place: MapPlace?
  @Binding var camera: MapCameraPosition
  @Binding var selectedID: String?
  /// The scope that connects the map to its controls in the overlay.
  let scope: Namespace.ID
  /// Records a report about a bike.
  let report: (BikeReport.Kind, BikeReport.Reason?, Bike) -> Void
  /// The distance of the camera from the map, in metres.
  @State private var cameraDistance: Double = .infinity
  /// The length on the ground of one point on the screen, or 0 before the
  /// map reports its region.
  @State private var metresPerPoint: Double = 0
  @State private var mapHeight: Double = 0

  /// The distance in points within which ranked bikes share a badge: about
  /// the width of a badge.
  private let badgeSpacing = 52.0
  /// The distance in points within which other bikes share a dot.
  private let dotSpacing = 16.0

  /// The camera distance below which the map shows parking bays.
  private let bayDistance: Double = 1500

  var body: some View {
    Map(position: $camera, selection: $selectedID, scope: scope) {
      UserAnnotation()

      // Before the bikes, so that a bike at the place stays on top.
      if let place {
        Annotation(place.title, coordinate: place.coordinate.clLocation) {
          PlaceMarker(place: place)
        }
        .annotationTitles(.hidden)
      }

      if showsBays {
        ForEach(Array(bays.enumerated()), id: \.offset) { _, bay in
          MapPolygon(coordinates: bay.outline.map(\.clLocation))
            .foregroundStyle(Theme.parking.opacity(0.12))
            .stroke(Theme.parking.opacity(0.8), lineWidth: 1.5)
        }
      }

      if let route {
        MapPolyline(route)
          .stroke(.white, style: StrokeStyle(lineWidth: 8, lineCap: .round, lineJoin: .round))
        MapPolyline(route)
          .stroke(Theme.accent, style: StrokeStyle(lineWidth: 5, lineCap: .round, lineJoin: .round))
      }

      ForEach(dotGroups) { group in
        Annotation(
          group.count == 1 ? "Bike" : "\(group.count) bikes",
          coordinate: group.leader.coordinate.clLocation
        ) {
          BikeDot(count: group.count, isSelected: group.contains(selectedID))
            .contextMenu {
              if group.count == 1 {
                ReportMenuItems(bike: group.leader, report: report)
              }
            }
            // The map uses the system blue; the menu uses the accent.
            .tint(Theme.accent)
        }
        .annotationTitles(.hidden)
        .tag(group.id)
      }

      ForEach(badgeGroups) { group in
        Annotation(
          group.count == 1
            ? "Bike, \(group.leader.walkMinutes) minute walk"
            : "\(group.count) bikes, \(group.leader.walkMinutes) minute walk",
          coordinate: group.leader.bike.coordinate.clLocation,
          anchor: .bottom
        ) {
          MinuteBadge(
            minutes: group.leader.walkMinutes,
            // A shared badge stands for bikes with different ranges.
            rangeM: group.count == 1 ? group.leader.bike.rangeM : nil,
            count: group.count,
            isFeatured: group.contains(featuredID),
            isSelected: group.contains(selectedID)
          )
          .contextMenu {
            if group.count == 1 {
              ReportMenuItems(bike: group.leader.bike, report: report)
            }
          }
          // The map uses the system blue; the menu uses the accent.
          .tint(Theme.accent)
        }
        .annotationTitles(.hidden)
        .tag(group.id)
      }

      if let target {
        Annotation(
          "Your bike", coordinate: target.bike.coordinate.clLocation, anchor: .bottom
        ) {
          TargetMarker()
            .contextMenu { ReportMenuItems(bike: target.bike, report: report) }
            // The map uses the system blue; the menu uses the accent.
            .tint(Theme.accent)
        }
        .annotationTitles(.hidden)
        .tag(target.bike.id)
      }
    }
    .mapStyle(.standard(pointsOfInterest: .excludingAll))
    // The user location and the map controls use the system blue, so that
    // they do not look like bikes.
    .tint(.blue)
    .onMapCameraChange { context in
      cameraDistance = context.camera.distance
      if mapHeight > 0 {
        metresPerPoint = context.region.span.latitudeDelta * 111_320 / mapHeight
      }
    }
    .onGeometryChange(for: Double.self) {
      $0.size.height
    } action: {
      mapHeight = $0
    }
    // The controls are in the overlay, in line with the other buttons.
    .mapControls {}
  }

  private var showsBays: Bool {
    cameraDistance < bayDistance
  }

  /// The ranked bikes other than the target, grouped where their badges
  /// would overlap.
  private var badgeGroups: [MarkerGroup<WalkedBike>] {
    MarkerGroup.group(
      ranked.filter { $0.id != target?.bike.id }, radiusM: badgeSpacing * metresPerPoint,
      coordinate: \.bike.coordinate)
  }

  /// The other bikes, grouped where their dots would overlap.
  private var dotGroups: [MarkerGroup<Bike>] {
    MarkerGroup.group(others, radiusM: dotSpacing * metresPerPoint, coordinate: \.coordinate)
  }

  /// The bikes that are not ranked and are not the target.
  private var others: [Bike] {
    let excluded = Set(ranked.map(\.id)).union([target?.bike.id].compactMap(\.self))
    return bikes.filter { !excluded.contains($0.id) }
  }
}

/// A saved place, as the map shows it.
struct MapPlace: Equatable {
  let coordinate: Coordinate
  let title: String
  let systemImage: String
}

/// The marker for the saved place that the bikes are ranked from. It is
/// white, so that it looks different from the bikes, the current location,
/// and the parking bays.
struct PlaceMarker: View {
  let place: MapPlace

  var body: some View {
    Image(systemName: place.systemImage)
      .font(.system(size: 15, weight: .bold))
      .foregroundStyle(.black)
      .frame(width: 34, height: 34)
      .background(.white, in: Circle())
      .overlay { Circle().strokeBorder(Color(.systemBackground), lineWidth: 2) }
      .shadow(color: .black.opacity(0.35), radius: 3, y: 1)
      // The name is below the marker, and does not move its centre from the
      // place.
      .overlay(alignment: .top) {
        Text(place.title)
          .font(.caption2.weight(.bold))
          .foregroundStyle(.white)
          .padding(.horizontal, 6)
          .padding(.vertical, 2)
          .background(.black.opacity(0.6), in: Capsule())
          .fixedSize()
          .offset(y: 38)
      }
      .accessibilityElement(children: .ignore)
      .accessibilityLabel(place.title)
  }
}

/// A marker with the walking time to a ranked bike.
struct MinuteBadge: View {
  let minutes: Int
  /// The range of the bike, or `nil` for a badge that bikes share. The white
  /// part of the outline shows it as a part of a full battery, clockwise
  /// from the top.
  let rangeM: Double?
  /// The number of bikes that share the badge.
  var count = 1
  let isFeatured: Bool
  let isSelected: Bool

  var body: some View {
    VStack(spacing: 0) {
      HStack(alignment: .firstTextBaseline, spacing: 1) {
        Text("\(minutes)")
          .font(.system(size: isFeatured ? 22 : 16, weight: .heavy, design: .rounded))
        Text("min")
          .font(.system(size: isFeatured ? 11 : 9, weight: .bold, design: .rounded))
        if count > 1 {
          Text("×\(count)")
            .font(.system(size: isFeatured ? 11 : 9, weight: .heavy, design: .rounded))
            .padding(.leading, 3)
        }
      }
      .monospacedDigit()
      .foregroundStyle(Theme.onAccent)
      .padding(.horizontal, isFeatured ? 10 : 7)
      .padding(.vertical, isFeatured ? 5 : 3)
      .background(Theme.accent, in: Capsule())
      .overlay {
        let width = isSelected ? 3.5 : 2.0
        ZStack {
          Capsule().strokeBorder(Color(.systemBackground), lineWidth: width)
          if let rangeM {
            CapsuleOutline()
              .trim(from: 0, to: min(max(rangeM / Theme.fullRangeM, 0), 1))
              .stroke(.white, style: StrokeStyle(lineWidth: width, lineCap: .round))
              .padding(width / 2)
          }
        }
      }
      .background {
        // A halo that grows stronger with the number of bikes.
        Capsule()
          .fill(Theme.accent.opacity(Halo.opacity(count)))
          .padding(-Halo.width(count))
      }
      Triangle()
        .fill(Theme.accent)
        .frame(width: 10, height: 6)
    }
    .shadow(color: .black.opacity(0.25), radius: 2, y: 1)
    .scaleEffect(isSelected ? 1.15 : 1, anchor: .bottom)
    .animation(.snappy, value: isSelected)
  }
}

/// A marker for bikes that are not ranked. It grows, and its halo gets
/// stronger, with the number of bikes that share it.
struct BikeDot: View {
  var count = 1
  let isSelected: Bool

  var body: some View {
    let size = (isSelected ? 16.0 : 11.0) + 2 * Double(min(count - 1, 3))
    Circle()
      .fill(isSelected ? Theme.accent : Color(.systemGray))
      .frame(width: size, height: size)
      .overlay { Circle().strokeBorder(.white, lineWidth: 2) }
      .background {
        Circle()
          .fill(Color(.systemGray).opacity(Halo.opacity(count)))
          .padding(-Halo.width(count))
      }
      .frame(width: 28, height: 28)
      .contentShape(Circle())
  }
}

/// The halo around a marker that several bikes share.
enum Halo {
  /// The opacity of the halo: none for one bike, then stronger for each
  /// further bike, up to five bikes.
  static func opacity(_ count: Int) -> Double {
    count > 1 ? 0.4 + 0.15 * Double(min(count, 5) - 2) : 0
  }

  /// The width of the halo in points.
  static func width(_ count: Int) -> Double {
    count > 1 ? 3 + 1.5 * Double(min(count, 5) - 2) : 0
  }
}

/// The marker for the bike the rider is walking to.
struct TargetMarker: View {

  var body: some View {
    VStack(spacing: 0) {
      Image(systemName: "bicycle")
        .font(.system(size: 18, weight: .bold))
        .foregroundStyle(Theme.onAccent)
        .frame(width: 40, height: 40)
        .background(Theme.accent, in: Circle())
        .overlay { Circle().strokeBorder(.white, lineWidth: 3) }
      Triangle()
        .fill(Theme.accent)
        .frame(width: 12, height: 7)
    }
    .shadow(color: .black.opacity(0.3), radius: 3, y: 1)
  }
}

/// The outline of a capsule, from the top centre, clockwise.
struct CapsuleOutline: Shape {
  func path(in rect: CGRect) -> Path {
    let radius = min(rect.width, rect.height) / 2
    return Path { path in
      path.move(to: CGPoint(x: rect.midX, y: rect.minY))
      path.addLine(to: CGPoint(x: rect.maxX - radius, y: rect.minY))
      path.addArc(
        center: CGPoint(x: rect.maxX - radius, y: rect.midY), radius: radius,
        startAngle: .degrees(-90), endAngle: .degrees(90), clockwise: false)
      path.addLine(to: CGPoint(x: rect.minX + radius, y: rect.maxY))
      path.addArc(
        center: CGPoint(x: rect.minX + radius, y: rect.midY), radius: radius,
        startAngle: .degrees(90), endAngle: .degrees(270), clockwise: false)
      path.closeSubpath()
    }
  }
}

/// A triangle that points down.
struct Triangle: Shape {
  func path(in rect: CGRect) -> Path {
    Path { path in
      path.move(to: CGPoint(x: rect.minX, y: rect.minY))
      path.addLine(to: CGPoint(x: rect.maxX, y: rect.minY))
      path.addLine(to: CGPoint(x: rect.midX, y: rect.maxY))
      path.closeSubpath()
    }
  }
}
