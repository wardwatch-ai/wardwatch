import Testing
@testable import WardwatchKit

@Suite struct HomeCopyTests {
    @Test func onlyAnUpdateIsActNow() throws {
        let latest = try #require(OSVersion("26.1.1"))
        let update = HomeCopy.status(.updateAvailable(latest: latest), systemName: "iOS")
        #expect(update.actNow)
        #expect(update.title == "Update to iOS 26.1.1.")
        #expect(update.steps.first == HomeCopy.softwareUpdatePath)
        #expect(!HomeCopy.status(.unknown, systemName: "iOS").actNow)
        #expect(!HomeCopy.status(.current, systemName: "iOS").actNow)
    }

    @Test func guidanceShowsTheSteps() {
        let copy = HomeCopy.status(.unknown, systemName: "iOS")
        #expect(copy.eyebrow == nil)
        #expect(copy.title == "Check for an iOS update.")
        #expect(copy.steps.count == 3)
    }

    @Test func currentHasNoSteps() {
        #expect(HomeCopy.status(.current, systemName: "iOS").steps.isEmpty)
    }
}
