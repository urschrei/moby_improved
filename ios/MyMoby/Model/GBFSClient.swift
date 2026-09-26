import Foundation
import MobyKit

/// Fetches the MOBY Dublin feeds.
///
/// The client finds the `vehicle_status` URL from the GBFS manifest. It finds
/// the URL again after the server rejects a request or the response does not
/// parse, but not after a network failure.
actor GBFSClient: FeedSource {
  static let manifestURL = URL(string: "https://moby-move.rideatom.com/gbfs/v3_0/en/gbfs?id=2023")!

  private let session: URLSession
  private var vehicleStatusURL: URL?

  init() {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
    configuration.timeoutIntervalForRequest = 15
    configuration.httpAdditionalHeaders = ["User-Agent": "MyMoby/0.1 (personal use)"]
    session = URLSession(configuration: configuration)
  }

  func vehicleStatus() async throws -> Feed {
    let url = try await resolveVehicleStatusURL()
    do {
      return try Feed.parse(body: try await get(url))
    } catch {
      if !FeedFailure.isTransient(error) {
        vehicleStatusURL = nil
      }
      throw error
    }
  }

  /// Returns the `geofencing_zones` response body.
  func geofencingZones() async throws -> Data {
    let manifest = try await get(Self.manifestURL)
    guard let url = URL(string: try MobyKit.geofencingZonesUrl(manifest: manifest)) else {
      throw URLError(.badURL)
    }
    return try await get(url)
  }

  private func resolveVehicleStatusURL() async throws -> URL {
    if let vehicleStatusURL {
      return vehicleStatusURL
    }
    let manifest = try await get(Self.manifestURL)
    guard let url = URL(string: try MobyKit.vehicleStatusUrl(manifest: manifest)) else {
      throw URLError(.badURL)
    }
    vehicleStatusURL = url
    return url
  }

  private func get(_ url: URL) async throws -> Data {
    let (data, response) = try await session.data(from: url)
    guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
      throw URLError(.badServerResponse)
    }
    return data
  }
}
