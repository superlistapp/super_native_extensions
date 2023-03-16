import 'dart:ui';

import 'package:collection/collection.dart';

import 'api_model.dart';
import 'drag.dart';

Future<TargettedImageData> combineDragImage(
    DragConfiguration configuration) async {
  var combinedRect = Rect.zero;
  for (final item in configuration.items) {
    if (combinedRect.isEmpty) {
      combinedRect = item.image.image.rect;
    } else {
      combinedRect = combinedRect.expandToInclude(item.image.image.rect);
    }
  }
  final scale =
      configuration.items.firstOrNull?.image.image.imageData.devicePixelRatio ??
          1.0;
  final offset = combinedRect.topLeft;
  final rect = combinedRect.translate(-offset.dx, -offset.dy);
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder);
  canvas.scale(scale, scale);
  for (final item in configuration.items) {
    final image = item.image.image.imageData.sourceImage;
    final destinationRect =
        item.image.image.rect.translate(-offset.dx, -offset.dy);
    canvas.drawImageRect(
        image,
        Rect.fromLTWH(0, 0, image.width.toDouble(), image.height.toDouble()),
        destinationRect,
        Paint());
  }
  final picture = recorder.endRecording();
  final image = await picture.toImage(
      (rect.width * scale).ceil(), (rect.height * scale).ceil());

  return TargettedImageData(
    imageData: await ImageData.fromImage(image, devicePixelRatio: scale),
    rect: combinedRect,
  );
}
