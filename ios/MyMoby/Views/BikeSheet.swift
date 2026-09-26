import MobyKit
import SwiftUI

/// The sheet with the nearest or selected bike, the next bikes, and the state
/// of the feed.
struct BikeSheet: View {
  let controller: CommuteController
  let location: LocationProvider
  let settings: Settings
  let origin: Origin
  let originCoordinate: Coordinate?
  let nearestBay: ParkingBay?
  @Binding var selectedID: String?
  @Binding var isShowingSettings: Bool

  /// The height of the sheet when it shows only the featured bike.
  static let collapsedHeight: CGFloat = 316
  static let collapsed = PresentationDetent.height(collapsedHeight)

  private var store: BikeStore { controller.store }

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 24) {
        VStack(alignment: .leading, spacing: 0) {
          StaleNotice(feed: store.feed)
          mainSection
        }
        if !otherBikes.isEmpty {
          VStack(alignment: .leading, spacing: 0) {
            ForEach(otherBikes) { walked in
              Button {
                selectedID = walked.id
              } label: {
                NextBikeRow(walked: walked)
              }
              .buttonStyle(.plain)
              if walked.id != otherBikes.last?.id {
                Divider()
              }
            }
          }
        }
        HStack {
          if let nearestBay {
            ParkingLine(bay: nearestBay)
          }
          Spacer()
          Button {
            isShowingSettings = true
          } label: {
            Image(systemName: "gearshape.fill")
              .foregroundStyle(.secondary)
              .frame(width: 44, height: 44)
              .background(.quaternary, in: Circle())
          }
          .buttonStyle(.plain)
          .accessibilityLabel("Settings")
        }
        VStack(alignment: .leading, spacing: 12) {
          FeedStatus(
            feed: store.feed, bikeCount: store.bikes.count, minRangeKm: settings.values.minRangeKm,
            isRefreshing: store.isRefreshing)
          Attribution()
        }
      }
      .padding(.horizontal, 20)
      .padding(.top, 12)
      .padding(.bottom, 16)
    }
    .scrollBounceBehavior(.basedOnSize)
    .refreshable {
      if let originCoordinate {
        await store.refresh(from: originCoordinate)
      }
    }
    .sheet(isPresented: $isShowingSettings) {
      SettingsView(settings: settings, location: location.coordinate)
    }
  }

  @ViewBuilder private var mainSection: some View {
    if let target = store.target {
      TargetSection(target: target, origin: location.coordinate, controller: controller)
    } else if let featured {
      FeaturedSection(
        bike: featured.bike, walked: featured.walked, isNearest: featured.isNearest,
        controller: controller, showNearest: { selectedID = nil })
    } else {
      EmptySection(
        location: location, settings: settings, origin: origin, originCoordinate: originCoordinate,
        error: store.lastError, isShowingSettings: $isShowingSettings)
    }
  }

  /// The bike at the top of the sheet: the selected bike, or else the
  /// nearest bike.
  private var featured: (bike: Bike, walked: WalkedBike?, isNearest: Bool)? {
    if let selectedID, let bike = store.bikes.first(where: { $0.id == selectedID }) {
      let walked = store.nearest.first { $0.id == selectedID }
      return (bike, walked, selectedID == store.nearest.first?.id)
    }
    return store.nearest.first.map { ($0.bike, $0, true) }
  }

  /// The ranked bikes other than the featured bike.
  private var otherBikes: [WalkedBike] {
    let featuredID = store.target?.bike.id ?? featured?.bike.id
    return store.nearest.filter { $0.id != featuredID }
  }
}

/// A large number with its unit, such as the walking time.
struct Headline: View {
  let value: String
  let unit: String

  var body: some View {
    HStack(alignment: .firstTextBaseline, spacing: 6) {
      Text(value)
        .font(.system(size: 60, weight: .heavy, design: .rounded))
        .monospacedDigit()
        .contentTransition(.numericText())
      Text(unit)
        .font(.title3.weight(.semibold))
        .foregroundStyle(.secondary)
    }
    .animation(.snappy, value: value)
    .accessibilityElement(children: .combine)
  }
}

/// The range of a bike as a number above a gauge.
struct RangeBlock: View {
  let rangeM: Double

  var body: some View {
    VStack(alignment: .trailing, spacing: 6) {
      Text("Range")
        .font(.footnote.weight(.medium))
        .foregroundStyle(.secondary)
        .padding(.bottom, -4)
      Text(Units.kilometres(rangeM))
        .font(.title3.weight(.semibold))
        .monospacedDigit()
      RangeGauge(rangeM: rangeM, width: 56)
    }
    .accessibilityElement(children: .ignore)
    .accessibilityLabel("Range \(Int(rangeM / 1000)) kilometres")
  }
}

/// The nearest or selected bike, and its actions.
struct FeaturedSection: View {
  let bike: Bike
  let walked: WalkedBike?
  let isNearest: Bool
  let controller: CommuteController
  let showNearest: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      VStack(alignment: .leading, spacing: 2) {
        HStack(alignment: .lastTextBaseline) {
          if let walked {
            Headline(value: "\(walked.walkMinutes)", unit: "min walk")
          } else {
            Headline(value: "\(Units.roundedMetres(bike.straightLineM))", unit: "m away")
          }
          Spacer()
          RangeBlock(rangeM: bike.rangeM)
        }
        BikePlace(bike: bike, street: controller.streets.street(for: bike))
        HStack {
          if let walked {
            TimelineView(.everyMinute) { context in
              Text(
                "\(Units.metres(walked.walkingM)) on foot, there by \(context.date.addingTimeInterval(walked.walkingS), format: .dateTime.hour().minute())"
              )
            }
          } else {
            Text("In a straight line")
          }
          Spacer()
          if !isNearest {
            Button("Show Nearest", action: showNearest)
              .font(.subheadline.weight(.semibold))
          }
        }
        .font(.subheadline)
        .foregroundStyle(.secondary)
      }
      BikeActions(bike: bike, controller: controller)
    }
    .task(id: bike.id) { await controller.streets.lookUp(bike) }
  }
}

/// The street and number of a bike, for example "Townsend Street, bike
/// 2025070027". The number is the one in the bike's rental link.
struct BikePlace: View {
  let bike: Bike
  let street: String?

  var body: some View {
    let parts = [street, bike.number.map { "bike \($0)" }].compactMap(\.self)
    if !parts.isEmpty {
      Text(parts.joined(separator: ", ").capitalizedFirst)
        .font(.subheadline.weight(.medium))
        .lineLimit(1)
    }
  }
}

extension String {
  /// The string with its first character in upper case.
  var capitalizedFirst: String {
    prefix(1).uppercased() + dropFirst()
  }
}

/// The actions for a bike that the rider has not chosen yet.
struct BikeActions: View {
  let bike: Bike
  let controller: CommuteController

  var body: some View {
    VStack(spacing: 8) {
      Button {
        controller.walk(to: bike)
      } label: {
        Label("Walk", systemImage: "figure.walk")
          .frame(maxWidth: .infinity)
      }
      .buttonStyle(.borderedProminent)
      .foregroundStyle(Theme.onAccent)
      if Handoff.rentalURL(for: bike) != nil {
        HStack(spacing: 8) {
          Button {
            controller.reserve(bike)
          } label: {
            Label("Reserve", systemImage: "clock.badge.checkmark")
              .frame(maxWidth: .infinity)
          }
          Button {
            controller.arrived()
            Task { await Handoff.openInMoby(bike) }
          } label: {
            Label("Unlock", systemImage: "lock.open")
              .frame(maxWidth: .infinity)
          }
        }
        .buttonStyle(.bordered)
      }
    }
    .lineLimit(1)
    .font(.body.weight(.semibold))
    .controlSize(.large)
  }
}

/// The bike the rider is walking to.
struct TargetSection: View {
  let target: Target
  let origin: Coordinate?
  let controller: CommuteController

  var body: some View {
    content
      .task(id: target.bike.id) { await controller.streets.lookUp(target.bike) }
  }

  private var content: some View {
    VStack(alignment: .leading, spacing: 16) {
      VStack(alignment: .leading, spacing: 2) {
        Label(
          target.isReserved ? "Reserved in MOBY" : "Walking to your bike",
          systemImage: target.isReserved ? "clock.badge.checkmark.fill" : "figure.walk"
        )
        .font(.subheadline.weight(.semibold))
        .foregroundStyle(Theme.accent)
        HStack(alignment: .lastTextBaseline) {
          if let origin {
            let metres = Units.roundedMetres(distanceM(a: origin, b: target.bike.coordinate))
            Headline(value: "\(metres)", unit: "m to go")
          } else {
            Headline(value: "–", unit: "m to go")
          }
          Spacer()
          RangeBlock(rangeM: target.bike.rangeM)
        }
        BikePlace(bike: target.bike, street: controller.streets.street(for: target.bike))
      }
      HStack(spacing: 10) {
        if Handoff.rentalURL(for: target.bike) != nil {
          Button {
            controller.arrived()
            Task { await Handoff.openInMoby(target.bike) }
          } label: {
            Label("Unlock", systemImage: "lock.open.fill")
              .frame(maxWidth: .infinity)
          }
          .buttonStyle(.borderedProminent)
          .foregroundStyle(Theme.onAccent)
        }
        Button {
          controller.directions(to: target.bike)
        } label: {
          Label("Walk", systemImage: "figure.walk")
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.bordered)
      }
      .font(.body.weight(.semibold))
      .controlSize(.large)
      HStack {
        if !target.isReserved, Handoff.rentalURL(for: target.bike) != nil {
          Button("Reserve in MOBY") {
            controller.reserve(target.bike)
          }
        }
        Spacer()
        Button("Choose Another Bike") {
          controller.arrived()
        }
      }
      .font(.subheadline.weight(.semibold))
    }
  }
}

/// The state of the sheet when there is no bike to show.
struct EmptySection: View {
  let location: LocationProvider
  let settings: Settings
  let origin: Origin
  let originCoordinate: Coordinate?
  let error: String?
  @Binding var isShowingSettings: Bool

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      if location.isDenied, origin == .here {
        Text("Location access is off")
          .font(.title2.bold())
        Text("Turn on location access for MyMoby in Settings to find bikes near you.")
          .foregroundStyle(.secondary)
      } else if case .place(let kind) = origin, originCoordinate == nil {
        Text("\(settings.title(for: kind)) is not set")
          .font(.title2.bold())
        Button("Set \(settings.title(for: kind))") { isShowingSettings = true }
          .buttonStyle(.borderedProminent)
          .foregroundStyle(Theme.onAccent)
      } else if let error {
        Text("The bikes did not load")
          .font(.title2.bold())
        Text(error)
          .foregroundStyle(.secondary)
        Text("Pull down to try again.")
          .foregroundStyle(.secondary)
      } else {
        HStack(spacing: 12) {
          ProgressView()
          Text(originCoordinate == nil ? "Finding your location" : "Finding the nearest bikes")
            .font(.title3.weight(.semibold))
        }
        .frame(minHeight: 72)
      }
    }
  }
}

/// A ranked bike in the list below the featured bike.
struct NextBikeRow: View {
  let walked: WalkedBike

  var body: some View {
    HStack(alignment: .firstTextBaseline, spacing: 12) {
      HStack(alignment: .firstTextBaseline, spacing: 2) {
        Text("\(walked.walkMinutes)")
          .font(.system(size: 24, weight: .heavy, design: .rounded))
        Text("min")
          .font(.subheadline.weight(.semibold))
          .foregroundStyle(.secondary)
      }
      .monospacedDigit()
      .frame(width: 64, alignment: .leading)
      Text(Units.metres(walked.walkingM))
        .foregroundStyle(.secondary)
      Spacer()
      HStack(spacing: 8) {
        RangeGauge(rangeM: walked.bike.rangeM)
          .alignmentGuide(.firstTextBaseline) { $0[VerticalAlignment.center] + 4 }
        Text(Units.kilometres(walked.bike.rangeM))
          .monospacedDigit()
      }
    }
    .padding(.vertical, 12)
    .contentShape(Rectangle())
    .accessibilityElement(children: .ignore)
    .accessibilityLabel(
      "\(walked.walkMinutes) minute walk, \(Units.metres(walked.walkingM)), range \(Units.kilometres(walked.bike.rangeM))"
    )
  }
}

/// The distance to the nearest parking bay.
struct ParkingLine: View {
  let bay: ParkingBay

  var body: some View {
    Label {
      if bay.distanceM < 1 {
        Text("In a parking bay")
      } else {
        Text("Nearest parking bay: \(Units.metres(bay.distanceM))")
      }
    } icon: {
      Image(systemName: "parkingsign.circle.fill")
        .foregroundStyle(Theme.parking)
    }
    .font(.subheadline)
  }
}
