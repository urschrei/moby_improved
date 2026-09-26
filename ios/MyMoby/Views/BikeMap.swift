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
  @Binding var camera: MapCameraPosition
  @Binding var selectedID: String?

  var body: some View {
    Map(position: $camera, selection: $selectedID) {
      UserAnnotation()

      ForEach(Array(bays.enumerated()), id: \.offset) { _, bay in
        MapPolygon(coordinates: bay.outline.map(\.clLocation))
          .foregroundStyle(Theme.parking.opacity(0.25))
          .stroke(Theme.parking, lineWidth: 2)
      }
      ForEach(Array(labelledBays.enumerated()), id: \.offset) { _, bay in
        Annotation("Parking bay", coordinate: bay.centroid.clLocation) {
          BayMarker()
        }
        .annotationTitles(.hidden)
      }

      if let route {
        MapPolyline(route)
          .stroke(.white, style: StrokeStyle(lineWidth: 8, lineCap: .round, lineJoin: .round))
        MapPolyline(route)
          .stroke(Theme.accent, style: StrokeStyle(lineWidth: 5, lineCap: .round, lineJoin: .round))
      }

      ForEach(others) { bike in
        Annotation(
          "Bike, \(Units.metres(bike.straightLineM))", coordinate: bike.coordinate.clLocation
        ) {
          BikeDot(isSelected: bike.id == selectedID)
        }
        .annotationTitles(.hidden)
        .tag(bike.id)
      }

      ForEach(ranked) { walked in
        Annotation(
          "Bike, \(walked.walkMinutes) minute walk", coordinate: walked.bike.coordinate.clLocation,
          anchor: .bottom
        ) {
          MinuteBadge(
            minutes: walked.walkMinutes,
            isFeatured: walked.id == featuredID,
            isSelected: walked.id == selectedID)
        }
        .annotationTitles(.hidden)
        .tag(walked.id)
      }

      if let target {
        Annotation(
          "Your bike", coordinate: target.bike.coordinate.clLocation, anchor: .bottom
        ) {
          TargetMarker(isReserved: target.isReserved)
        }
        .annotationTitles(.hidden)
        .tag(target.bike.id)
      }
    }
    .mapStyle(.standard(pointsOfInterest: .excludingAll))
    .mapControls {
      MapUserLocationButton()
      MapCompass()
    }
  }

  /// The bays that show a label: the nearest three, without a label within
  /// 40 m of another.
  private var labelledBays: [ParkingBay] {
    bays.prefix(3).reduce(into: []) { labelled, bay in
      if labelled.allSatisfy({ distanceM(a: $0.centroid, b: bay.centroid) > 40 }) {
        labelled.append(bay)
      }
    }
  }

  /// The bikes that are not ranked and are not the target.
  private var others: [Bike] {
    let excluded = Set(ranked.map(\.id)).union([target?.bike.id].compactMap(\.self))
    return bikes.filter { !excluded.contains($0.id) }
  }
}

/// A marker with the walking time to a ranked bike.
struct MinuteBadge: View {
  let minutes: Int
  let isFeatured: Bool
  let isSelected: Bool

  var body: some View {
    VStack(spacing: 0) {
      HStack(alignment: .firstTextBaseline, spacing: 1) {
        Text("\(minutes)")
          .font(.system(size: isFeatured ? 22 : 16, weight: .heavy, design: .rounded))
        Text("min")
          .font(.system(size: isFeatured ? 11 : 9, weight: .bold, design: .rounded))
      }
      .monospacedDigit()
      .foregroundStyle(Theme.onAccent)
      .padding(.horizontal, isFeatured ? 10 : 7)
      .padding(.vertical, isFeatured ? 5 : 3)
      .background(Theme.accent, in: Capsule())
      .overlay {
        Capsule().strokeBorder(
          isSelected ? Color.white : Color(.systemBackground), lineWidth: isSelected ? 3 : 1.5)
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

/// A marker for a bike that is not ranked.
struct BikeDot: View {
  let isSelected: Bool

  var body: some View {
    Circle()
      .fill(isSelected ? Theme.accent : Color(.systemGray))
      .frame(width: isSelected ? 16 : 11, height: isSelected ? 16 : 11)
      .overlay { Circle().strokeBorder(.white, lineWidth: 2) }
      .frame(width: 28, height: 28)
      .contentShape(Circle())
  }
}

/// The marker for the bike the rider is walking to.
struct TargetMarker: View {
  let isReserved: Bool

  var body: some View {
    VStack(spacing: 0) {
      Image(systemName: isReserved ? "clock.badge.checkmark.fill" : "bicycle")
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

/// The marker at the centre of a parking bay, which stays visible at every
/// zoom level.
struct BayMarker: View {
  var body: some View {
    Text("P")
      .font(.system(size: 11, weight: .heavy, design: .rounded))
      .foregroundStyle(.white)
      .frame(width: 18, height: 18)
      .background(Theme.parking, in: RoundedRectangle(cornerRadius: 4))
      .overlay { RoundedRectangle(cornerRadius: 4).strokeBorder(.white, lineWidth: 1.5) }
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
