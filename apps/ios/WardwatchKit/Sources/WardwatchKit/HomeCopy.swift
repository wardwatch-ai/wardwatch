/// The card's mood: amber with beams on, all clear, or neither.
public enum StatusTone: Equatable, Sendable { case actNow, clear, neutral }

/// The words on the home screen's status card (approved mobile design, 2026-11-10).
/// Kept here, without SwiftUI, so `swift test` checks them on macOS and Linux.
public struct StatusCopy: Equatable, Sendable {
    public let eyebrow: String?
    public let title: String
    public let body: String
    /// Numbered steps to follow in Settings. iOS doesn't let apps start an update.
    public let steps: [String]
    public let tone: StatusTone

    /// Amber card with lighthouse beams on: the device must act now.
    public var actNow: Bool { tone == .actNow }
}

public enum HomeCopy {
    public static let softwareUpdatePath = "Open Settings, then General, then Software Update."

    public static func status(_ status: UpdateStatus, systemName: String) -> StatusCopy {
        switch status {
        case .updateAvailable(let latest):
            return StatusCopy(
                eyebrow: "NEEDS YOU TODAY",
                title: "Update to \(systemName) \(latest.description).",
                body: "It fixes security holes. Install it today.",
                steps: [softwareUpdatePath, "Tap Update Now."],
                tone: .actNow
            )
        case .current:
            return StatusCopy(
                eyebrow: "ALL CLEAR",
                title: "This device is up to date.",
                body: "It runs the newest \(systemName). Open Wardwatch now and then to check again.",
                steps: [],
                tone: .clear
            )
        case .unknown:
            return StatusCopy(
                eyebrow: nil,
                title: "Check for an \(systemName) update.",
                body: "Most \(systemName) updates fix security holes. It takes a minute to check.",
                steps: [softwareUpdatePath, "If an update is waiting, tap Update Now.", "Come back here when it's done."],
                tone: .neutral
            )
        }
    }
}
