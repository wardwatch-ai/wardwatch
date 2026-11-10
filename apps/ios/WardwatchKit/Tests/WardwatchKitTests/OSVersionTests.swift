import Testing
@testable import WardwatchKit

@Suite struct OSVersionTests {
    @Test func parses() {
        #expect(OSVersion("18") == OSVersion(18))
        #expect(OSVersion("18.6") == OSVersion(18, 6))
        #expect(OSVersion("26.1.2") == OSVersion(26, 1, 2))
    }

    @Test func rejectsGarbage() {
        for bad in ["", ".", "18.", "a.b", "18.6.2.1", "-1", "18.x", "１８"] {
            #expect(OSVersion(bad) == nil, "\(bad)")
        }
    }

    @Test func comparesNumerically() {
        #expect(OSVersion("18.10")! > OSVersion("18.9")!)
        #expect(OSVersion("26.0")! > OSVersion("18.7.1")!)
        #expect(OSVersion("18.6") == OSVersion("18.6.0"))
    }

    @Test func describesWithoutTrailingZero() {
        #expect(OSVersion(18, 6).description == "18.6")
        #expect(OSVersion(18, 6, 2).description == "18.6.2")
    }
}

@Suite struct OSNudgeTests {
    @Test func unknownWithoutLatest() {
        #expect(OSNudge.check(current: OSVersion(18, 6), latest: nil) == .unknown)
    }

    @Test func currentWhenEqualOrNewer() {
        #expect(OSNudge.check(current: OSVersion(26, 1), latest: OSVersion(26, 1)) == .current)
        #expect(OSNudge.check(current: OSVersion(26, 2), latest: OSVersion(26, 1)) == .current)
    }

    @Test func updateWhenBehind() {
        let latest = OSVersion(26, 1, 1)
        #expect(OSNudge.check(current: OSVersion(26, 1), latest: latest) == .updateAvailable(latest: latest))
    }
}

@Suite struct DeviceModelTests {
    @Test func prefersSimulatorIdentifier() {
        #expect(DeviceModel.identifier(environment: ["SIMULATOR_MODEL_IDENTIFIER": "iPhone17,1"]) == "iPhone17,1")
    }

    @Test func readsHardwareOtherwise() {
        #expect(!DeviceModel.identifier(environment: [:]).isEmpty)
    }
}
