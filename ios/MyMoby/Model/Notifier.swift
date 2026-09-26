import Foundation
import MobyKit
import UIKit
import UserNotifications

/// Posts local notifications about the bike the rider is walking to, and
/// opens Maps when the rider taps one.
final class Notifier: NSObject, UNUserNotificationCenterDelegate, Sendable {
  static let shared = Notifier()

  private static let categoryID = "bike-changed"
  private static let walkActionID = "walk"

  func register() {
    let center = UNUserNotificationCenter.current()
    center.delegate = self
    let walk = UNNotificationAction(
      identifier: Self.walkActionID, title: "Walk there", options: [.foreground])
    center.setNotificationCategories([
      UNNotificationCategory(identifier: Self.categoryID, actions: [walk], intentIdentifiers: [])
    ])
  }

  func requestAuthorization() async {
    _ = try? await UNUserNotificationCenter.current().requestAuthorization(options: [
      .alert, .sound,
    ])
  }

  /// Tells the rider that their bike is gone and gives the next one.
  func bikeReplaced(by next: WalkedBike) {
    let minutes = Int((next.walkingS / 60).rounded(.up))
    post(
      title: "Your bike was taken",
      body: "Next nearest: \(minutes) min walk, \(Int(next.bike.rangeM / 1000)) km range.",
      coordinate: next.bike.coordinate)
  }

  /// Tells the rider that their bike moved.
  func bikeMoved(to coordinate: Coordinate) {
    post(title: "Your bike moved", body: "Tap to walk to its new position.", coordinate: coordinate)
  }

  /// Tells the rider that their bike is gone and no other bike is near.
  func bikeLost() {
    post(
      title: "Your bike was taken", body: "No other bike with enough range is nearby.",
      coordinate: nil)
  }

  private func post(title: String, body: String, coordinate: Coordinate?) {
    let content = UNMutableNotificationContent()
    content.title = title
    content.body = body
    content.sound = .default
    content.interruptionLevel = .timeSensitive
    if let coordinate {
      content.categoryIdentifier = Self.categoryID
      content.userInfo = ["lat": coordinate.lat, "lon": coordinate.lon]
    }
    let request = UNNotificationRequest(
      identifier: UUID().uuidString, content: content, trigger: nil)
    UNUserNotificationCenter.current().add(request)
  }

  func userNotificationCenter(
    _ center: UNUserNotificationCenter, willPresent notification: UNNotification
  ) async -> UNNotificationPresentationOptions {
    [.banner, .sound]
  }

  func userNotificationCenter(
    _ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse
  ) async {
    let info = response.notification.request.content.userInfo
    guard let lat = info["lat"] as? Double, let lon = info["lon"] as? Double else { return }
    let coordinate = Coordinate(lat: lat, lon: lon)
    await MainActor.run { Handoff.walk(to: coordinate) }
  }
}
