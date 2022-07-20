import 'dart:typed_data';
import 'dart:ui';

import 'data_provider.dart';
import 'util.dart';

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

//
// Drag
//

enum DropOperation { none, userCancelled, forbidden, copy, move, link }

class DragConfiguration {
  DragConfiguration({
    required this.items,
    required this.allowedOperations,
    this.animatesToStartingPositionOnCancelOrFail = true,
  });

  final List<DragItem> items;
  final List<DropOperation> allowedOperations;

  /// macOS specific
  final bool animatesToStartingPositionOnCancelOrFail;

  dynamic serialize() => {
        'items': items.map((e) => e.serialize()),
        'allowedOperations': allowedOperations.map((e) => e.name),
        'animatesToStartingPositionOnCancelOrFail':
            animatesToStartingPositionOnCancelOrFail,
      };
}

class DragItem {
  DragItem({
    required this.dataProvider,
    required this.image,
    this.localData,
  });

  dynamic serialize() => {
        'dataProviderId': dataProvider.id,
        'localData': localData,
        'image': image.serialize(),
      };

  final DataProviderHandle dataProvider;
  final DragImage image;
  final Object? localData;
}

class DragImage {
  DragImage({
    required this.imageData,
    required this.sourceRect,
  });

  dynamic serialize() => {
        'imageData': imageData.serialize(),
        'sourceRect': sourceRect.serialize(),
      };

  final ImageData imageData;
  final Rect sourceRect;
}

class DragRequest {
  DragRequest({
    required this.configuration,
    required this.position,
  });

  final DragConfiguration configuration;
  final Offset position;

  dynamic serialize() => {
        'configuration': configuration.serialize(),
        'position': position.serialize(),
      };
}
