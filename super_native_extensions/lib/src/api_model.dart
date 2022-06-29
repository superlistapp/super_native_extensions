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

  final int width;
  final int height;
  final int bytesPerRow;
  final Uint8List data;
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
        devicePixelRatio: devicePixelRatio);
  }

  dynamic serialize() => {
        'width': width,
        'height': height,
        'bytesPerRow': bytesPerRow,
        'data': data,
        'devicePixelRatio': devicePixelRatio,
      };
}

extension RectExt on Rect {
  Map serialize() => {
        'x': left,
        'y': top,
        'width': width,
        'height': height,
      };
  static Rect deserialize(dynamic rect) {
    final map = rect as Map;
    return Rect.fromLTWH(map['x'], map['y'], map['width'], map['height']);
  }
}

extension OffsetExt on Offset {
  Map serialize() => {
        'x': dx,
        'y': dy,
      };
}
