/// How urgently this device needs its system update.
public enum UpdateStatus: Equatable, Sendable {
    /// Running the newest release we know of.
    case current
    /// Apple has shipped a newer release. Almost every iOS release carries security
    /// fixes, so this gets the amber act-now nudge.
    case updateAvailable(latest: OSVersion)
    /// We don't know the newest release yet, so the app shows how to check.
    case unknown
}

public enum OSNudge {
    /// Compares the running version with `latest`, the newest release this device can
    /// install. The signed catalog must supply the newest release per major line, so a
    /// device that can't run a new major isn't nudged forever. Until the catalog has
    /// it, `latest` is nil and the app shows guidance only.
    public static func check(current: OSVersion, latest: OSVersion?) -> UpdateStatus {
        guard let latest else { return .unknown }
        return current < latest ? .updateAvailable(latest: latest) : .current
    }
}
