import Foundation

/// Describes a failed feed request.
enum FeedFailure {
  /// Returns a message for the rider about `error`.
  static func message(for error: any Error) -> String {
    guard let error = error as? URLError else {
      return "The bike data could not be read."
    }
    switch error.code {
    case .timedOut:
      return "The MOBY server did not respond."
    case .notConnectedToInternet, .dataNotAllowed:
      return "There is no internet connection."
    case .networkConnectionLost:
      return "The connection was lost."
    case .badServerResponse:
      return "The MOBY server returned an error."
    default:
      return "The MOBY server could not be reached."
    }
  }

  /// Returns `true` if `error` is a network failure. Such a failure does not
  /// mean that the feed URL has changed.
  static func isTransient(_ error: any Error) -> Bool {
    guard let error = error as? URLError else {
      return false
    }
    switch error.code {
    case .timedOut, .notConnectedToInternet, .networkConnectionLost, .dataNotAllowed,
      .cannotConnectToHost, .cannotFindHost, .dnsLookupFailed, .internationalRoamingOff:
      return true
    default:
      return false
    }
  }
}
