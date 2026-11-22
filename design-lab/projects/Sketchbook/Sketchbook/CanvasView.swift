import SwiftUI
import PencilKit

struct CanvasView: View {
    @Binding var document: SketchDocument
    @State private var tool = PKInkingTool(.pen, color: .black, width: 4)

    var body: some View {
        PencilCanvas(drawing: $document.drawing, tool: tool)
            .toolbar { ToolPicker(tool: $tool) }
    }
}
