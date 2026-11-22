import XCTest
@testable import Sketchbook

final class ExportTests: XCTestCase {
    func testEmptyDrawingExports() { XCTAssertNotNil(Export.png(.init())) }
}
