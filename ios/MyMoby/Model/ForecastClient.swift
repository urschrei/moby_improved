import Foundation
import MobyKit

/// The host and the bearer token of the forecast service, from the build
/// settings `MOBY_FORECAST_HOST` and `MOBY_FORECAST_TOKEN`.
struct ForecastConfig: Sendable {
  let host: String
  let token: String

  /// Returns the settings in the Info.plist of `bundle`, or `nil` if the
  /// host or the token is empty.
  static func from(bundle: Bundle = .main) -> ForecastConfig? {
    guard let host = bundle.object(forInfoDictionaryKey: "MobyForecastHost") as? String,
      let token = bundle.object(forInfoDictionaryKey: "MobyForecastToken") as? String,
      !host.isEmpty, !token.isEmpty
    else {
      return nil
    }
    return ForecastConfig(host: host, token: token)
  }
}

/// Why the service gave no forecast.
enum ForecastFailure: Error, Equatable {
  /// The service does not have the place: the app has not sent it.
  case unknownPlace
  /// The place is new, or moved after the last nightly fit. The forecast
  /// starts after the next fit.
  case pending
  /// The parameters or the feed are too old. The count is still correct.
  case paused(ForecastPause)
  /// The service refused the request, or cannot forecast now.
  case unavailable(status: Int)
}

/// Gets forecasts from the forecast service of the collector, and sends it the
/// saved places.
actor ForecastClient {
  private let config: ForecastConfig
  private let session: URLSession

  init(config: ForecastConfig) {
    self.config = config
    let configuration = URLSessionConfiguration.ephemeral
    configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
    configuration.timeoutIntervalForRequest = 15
    configuration.httpAdditionalHeaders = ["User-Agent": "MyMoby/0.1 (personal use)"]
    session = URLSession(configuration: configuration)
  }

  /// Returns the forecast for the place with the service name `place`, at
  /// `targetMs`, in milliseconds since the epoch.
  func forecast(place: String, targetMs: Int64) async throws -> PlaceForecast {
    var components = URLComponents()
    components.scheme = "https"
    components.host = config.host
    components.path = "/v1/forecast"
    components.queryItems = [
      URLQueryItem(name: "place", value: place),
      URLQueryItem(name: "at", value: Self.rfc3339(milliseconds: targetMs)),
    ]
    guard let url = components.url else {
      throw URLError(.badURL)
    }
    var request = URLRequest(url: url)
    request.setValue("Bearer \(config.token)", forHTTPHeaderField: "Authorization")
    let (data, response) = try await session.data(for: request)
    guard let http = response as? HTTPURLResponse else {
      throw URLError(.badServerResponse)
    }
    switch http.statusCode {
    case 200:
      return try parseForecast(body: data)
    case 404:
      throw ForecastFailure.unknownPlace
    case 409:
      throw ForecastFailure.pending
    case 503:
      // A service from before 2026-11-08 gives no count.
      if let pause = try? parsePause(body: data) {
        throw ForecastFailure.paused(pause)
      }
      throw ForecastFailure.unavailable(status: http.statusCode)
    default:
      throw ForecastFailure.unavailable(status: http.statusCode)
    }
  }

  /// Sends all the saved places, by their service names. The service
  /// removes the places that are not in `places`. A place that did not move
  /// keeps its forecast.
  func putPlaces(_ places: [String: Coordinate]) async throws {
    var components = URLComponents()
    components.scheme = "https"
    components.host = config.host
    components.path = "/v1/places"
    guard let url = components.url else {
      throw URLError(.badURL)
    }
    var request = URLRequest(url: url)
    request.httpMethod = "PUT"
    request.setValue("Bearer \(config.token)", forHTTPHeaderField: "Authorization")
    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
    request.httpBody = try JSONEncoder().encode(
      PlacesBody(places: places.mapValues { PlacesBody.Position(lon: $0.lon, lat: $0.lat) }))
    let (_, response) = try await session.data(for: request)
    guard let http = response as? HTTPURLResponse else {
      throw URLError(.badServerResponse)
    }
    guard http.statusCode == 200 else {
      throw ForecastFailure.unavailable(status: http.statusCode)
    }
  }

  /// The body of `PUT /v1/places`.
  private struct PlacesBody: Encodable {
    struct Position: Encodable {
      let lon: Double
      let lat: Double
    }

    let places: [String: Position]
  }

  /// Returns a time in UTC as RFC 3339, to the second.
  private static func rfc3339(milliseconds: Int64) -> String {
    Date(timeIntervalSince1970: Double(milliseconds) / 1000)
      .formatted(.iso8601)
  }
}
