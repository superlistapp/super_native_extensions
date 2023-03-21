import 'dart:typed_data';
import 'dart:ui';

class ImageData {
  ImageData({
    required this.width,
    required this.height,
    required this.bytesPerRow,
    required this.data,
    this.devicePixelRatio,
  });

  factory ImageData.allocate({
    required int width,
    required int height,
    double? devicePixelRatio,
  }) {
    return ImageData(
      width: width,
      height: height,
      bytesPerRow: width * 4,
      data: Uint8List(width * height * 4),
      devicePixelRatio: devicePixelRatio,
    );
  }

  final int width;
  final int height;
  final int bytesPerRow;
  final Uint8List data;
  final double? devicePixelRatio;

  static Future<ImageData> fromImage(
    Image image, {
    double? devicePixelRatio,
  }) async {
    final bytes =
        await image.toByteData(format: ImageByteFormat.rawStraightRgba);
    return ImageData(
        width: image.width,
        height: image.height,
        bytesPerRow: image.width * 4,
        data: bytes!.buffer.asUint8List(),
        devicePixelRatio: devicePixelRatio);
  }
}

/// Image representation of part of user interface.
class TargetedImage {
  TargetedImage(this.image, this.rect);

  /// Image to be used as avatar image.
  final Image image;

  /// Initial position of avatar image (in global coordinates).
  final Rect rect;
}

class TargetedImageData {
  TargetedImageData({
    required this.imageData,
    required this.rect,
  });

  final ImageData imageData;
  final Rect rect;
}

extension TargetedImageIntoRaw on TargetedImage {
  Future<TargetedImageData> intoRaw(double devicePixelRatio) async {
    return TargetedImageData(
      imageData:
          await ImageData.fromImage(image, devicePixelRatio: devicePixelRatio),
      rect: rect,
    );
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
  /// Supported on: iOS; Maps to [none] on other platforms.
  forbidden,

  /// Supported on: macOS, iOS, Windows, Linux, Android, Web.
  copy,

  /// Supported on: macOS, iOS (only within same app), Windows, Linux.
  move,

  /// Supported on: macOS, Windows, Linux.
  link
}
