import 'dart:async';
import 'dart:ui';
import 'dart:typed_data';

import 'widget_snapshot/widget_snapshot.dart';

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
    Image image,
  ) async {
    final bytes =
        await image.toByteData(format: ImageByteFormat.rawStraightRgba);
    return ImageData(
      width: image.width,
      height: image.height,
      bytesPerRow: image.width * 4,
      data: bytes!.buffer.asUint8List(),
      devicePixelRatio: image.devicePixelRatio,
    );
  }

  Future<Image> toImage() {
    final completer = Completer<Image>();
    decodeImageFromPixels(
      data,
      width,
      height,
      PixelFormat.rgba8888,
      rowBytes: bytesPerRow,
      (result) {
        completer.complete(result);
      },
    );
    return completer.future.then(
      (value) => value..devicePixelRatio = devicePixelRatio,
    );
  }
}

class TargetedImageData {
  TargetedImageData({
    required this.imageData,
    required this.rect,
  });

  final ImageData imageData;
  final Rect rect;
}

extension TargetedImageIntoRaw on TargetedWidgetSnapshot {
  Future<TargetedImageData> intoRaw() async {
    assert(snapshot.isImage);
    return TargetedImageData(
      imageData: await ImageData.fromImage(snapshot.image),
      rect: rect,
    );
  }
}
