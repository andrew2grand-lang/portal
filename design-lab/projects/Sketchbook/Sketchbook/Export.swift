import AppKit
import PencilKit

enum Export {
    static func png(_ drawing: PKDrawing) -> Data? {
        let image = drawing.image(from: drawing.bounds, scale: 2)
        guard let tiff = image.tiffRepresentation else { return nil }
        return NSBitmapImageRep(data: tiff)?.representation(using: .png, properties: [:])
    }
}
