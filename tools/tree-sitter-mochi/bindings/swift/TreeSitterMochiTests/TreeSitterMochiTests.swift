import XCTest
import SwiftTreeSitter
import TreeSitterMochi

final class TreeSitterMochiTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_mochi())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Mochi grammar")
    }
}
