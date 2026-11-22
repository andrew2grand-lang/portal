import SwiftUI

@main
struct SketchbookApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: SketchDocument()) { file in
            CanvasView(document: file.$document)
        }
    }
}
