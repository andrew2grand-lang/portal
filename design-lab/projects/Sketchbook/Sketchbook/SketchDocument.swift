import SwiftUI
import UniformTypeIdentifiers
import PencilKit

struct SketchDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.sketch] }
    var drawing = PKDrawing()

    init() {}
    init(configuration: ReadConfiguration) throws {
        drawing = try PKDrawing(data: configuration.file.regularFileContents ?? Data())
    }
    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: drawing.dataRepresentation())
    }
}
