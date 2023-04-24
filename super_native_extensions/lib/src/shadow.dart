import 'dart:typed_data';

import 'api_model.dart';
import 'web/blur.dart';

extension on ImageData {
  ImageData inflate(int padding) {
    final res = ImageData.allocate(
        width: width + 2 * padding,
        height: height + 2 * padding,
        devicePixelRatio: devicePixelRatio);

    for (var y = 0; y < height; ++y) {
      res.data.setRange(
          (y + padding) * res.bytesPerRow + padding * 4,
          (y + padding) * res.bytesPerRow + (padding + width) * 4,
          data,
          y * bytesPerRow);
    }
    return res;
  }

  void drawShadowOnly(int radius) {
    assert(bytesPerRow == width * 4);
    var shadow = Uint8List(width * height);

    for (var i = 0; i < data.length / 4; ++i) {
      shadow[i] = (data[i * 4 + 3]) ~/ 3;
    }

    blurImageData(shadow, 0, 0, width, height, radius);

    for (var i = 0; i < data.length / 4; ++i) {
      final index = i * 4;
      final a0_ = data[index + 3];
      if (a0_ == 255) {
        continue;
      } else {
        data[index] = 0;
        data[index + 1] = 0;
        data[index + 2] = 0;
        data[index + 3] = shadow[i];
        continue;
      }
    }
  }

  void drawShadow(int radius) {
    assert(bytesPerRow == width * 4);
    var shadow = Uint8List(width * height);

    for (var i = 0; i < data.length / 4; ++i) {
      shadow[i] = (data[i * 4 + 3]) ~/ 3;
    }

    blurImageData(shadow, 0, 0, width, height, radius);

    for (var i = 0; i < data.length / 4; ++i) {
      final index = i * 4;
      final a0_ = data[index + 3];
      if (a0_ == 255) {
        continue;
      } else if (a0_ == 0) {
        data[index] = 0;
        data[index + 1] = 0;
        data[index + 2] = 0;
        data[index + 3] = shadow[i];
        continue;
      }

      final r0 = data[index] / 255;
      final g0 = data[index + 1] / 255;
      final b0 = data[index + 2] / 255;
      final a0 = a0_ / 255;
      final a1 = shadow[i] / 255;

      final a = a0 + a1 * (1 - a0);
      final r = (r0 * a0) / a;
      final g = (g0 * a0) / a;
      final b = (b0 * a0) / a;
      data[index] = (r * 255).toInt();
      data[index + 1] = (g * 255).toInt();
      data[index + 2] = (b * 255).toInt();
      data[index + 3] = (a * 255).toInt();
    }
  }
}

extension ImageDataShadow on TargetedImageData {
  TargetedImageData withShadow(int radius) {
    final adjustedRadius =
        (radius * (imageData.devicePixelRatio ?? 1.0)).round();
    return TargetedImageData(
        imageData: imageData.inflate(adjustedRadius)
          ..drawShadow(adjustedRadius),
        rect: rect.inflate(radius.toDouble()));
  }

  TargetedImageData withShadowOnly(int radius) {
    final adjustedRadius =
        (radius * (imageData.devicePixelRatio ?? 1.0)).round();
    return TargetedImageData(
        imageData: imageData.inflate(adjustedRadius)
          ..drawShadowOnly(adjustedRadius),
        rect: rect.inflate(radius.toDouble()));
  }
}
