import 'dart:typed_data';
import 'dart:ui';

class ImageData {
  ImageData({
    required this.width,
    required this.height,
    required this.bytesPerRow,
    required this.data,
    required this.sourceImage,
    this.devicePixelRatio,
  });

  final int width;
  final int height;
  final int bytesPerRow;
  final Uint8List data;
  final Image sourceImage;
  final double? devicePixelRatio;

  static Future<ImageData> fromImage(
    Image image, {
    double? devicePixelRatio,
  }) async {
    final bytes = await image.toByteData(format: ImageByteFormat.rawRgba);
    return ImageData(
        width: image.width,
        height: image.height,
        bytesPerRow: image.width * 4,
        data: bytes!.buffer.asUint8List(),
        sourceImage: image,
        devicePixelRatio: devicePixelRatio);
  }
}

//
// Drag
//

/// Represents result of a drag & drop operation.
enum DropOperation {
  /// No drop operation performed.
  none,

  /// Drag cancelled by user pressing escape key.
  ///
  /// Supported on: macOS, Windows, Linux.
  userCancelled,

  /// Drag operation is generally supported but forbidden in this instance.
  ///
  /// Supported on: iOS; Maps to none on other platforms.
  forbidden,

  /// Supported on: macOS, Windows, Linux, Android, Web.
  copy,

  /// Supported on: macOS, iOS (only within same app), Windows, Linux.
  move,

  /// Supported on: macOS, Windows, Linux.
  link
}
