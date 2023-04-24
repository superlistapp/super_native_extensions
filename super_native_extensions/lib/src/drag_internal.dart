import 'dart:ui';

import 'package:collection/collection.dart';

import 'image.dart';
import 'api_model.dart';
import 'drag.dart';

Future<TargetedImageData> combineDragImage(
    DragConfiguration configuration) async {
  var combinedRect = Rect.zero;
  for (final item in configuration.items) {
    if (combinedRect.isEmpty) {
      combinedRect = item.image.rect;
    } else {
      combinedRect = combinedRect.expandToInclude(item.image.rect);
    }
  }
  final scale =
      configuration.items.firstOrNull?.image.image.devicePixelRatio ?? 1.0;
  final offset = combinedRect.topLeft;
  final rect = combinedRect.translate(-offset.dx, -offset.dy);
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder);
  canvas.scale(scale, scale);
  for (final item in configuration.items) {
    final image = item.image.image;
    final destinationRect = item.image.rect.translate(-offset.dx, -offset.dy);
    canvas.drawImageRect(
        image,
        Rect.fromLTWH(0, 0, image.width.toDouble(), image.height.toDouble()),
        destinationRect,
        Paint());
  }
  final picture = recorder.endRecording();
  final image = await picture.toImage(
      (rect.width * scale).ceil(), (rect.height * scale).ceil());

  return TargetedImageData(
    imageData: await ImageData.fromImage(image),
    rect: combinedRect,
  );
}
