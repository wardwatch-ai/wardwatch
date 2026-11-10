/// How urgently this device needs its system update.
public enum OSStatus: Equatable, Sendable {
    /// Running the newest release we know of.
    case current
    /// Apple has shipped a newer release. Almost every iOS release carries security
    /// fixes, so this gets the amber act-now nudge.
    case updateAvailable(latest: OSVersion)
    /// We don't know the newest release yet, so the app shows how to check.
    case unknown
}

public enum OSNudge {
    /// Compares the running version with the newest release in the same major line,
    /// or any newer major when `latest` is one. `latest` will come from the signed
    /// vulnerability catalog; until then it is nil and the app shows guidance only.
    public static func check(current: OSVersion, latest: OSVersion?) -> OSStatus {
        guard let latest else { return .unknown }
        return current < latest ? .updateAvailable(latest: latest) : .current
    }
}
