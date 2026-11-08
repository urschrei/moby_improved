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
  /// The service has no reach for the place.
  case unknownPlace
  /// The service refused the request, or cannot forecast now. The service
  /// gives status 503 if its parameters or the feed are too old.
  case unavailable(status: Int)
}

/// Fetches forecasts from `GET /v1/forecast` of the collector.
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
  /// the times of `forecastTargetsMs` after `now`.
  func forecast(place: String, now: Date = .now) async throws -> PlaceForecast {
    let targets = try forecastTargetsMs(nowMs: now.milliseconds)
    var components = URLComponents()
    components.scheme = "https"
    components.host = config.host
    components.path = "/v1/forecast"
    components.queryItems =
      [URLQueryItem(name: "place", value: place)]
      + targets.map { URLQueryItem(name: "at", value: Self.rfc3339(milliseconds: $0)) }
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
    default:
      throw ForecastFailure.unavailable(status: http.statusCode)
    }
  }

  /// Returns a time in UTC as RFC 3339, to the second.
  private static func rfc3339(milliseconds: Int64) -> String {
    Date(timeIntervalSince1970: Double(milliseconds) / 1000)
      .formatted(.iso8601)
  }
}
