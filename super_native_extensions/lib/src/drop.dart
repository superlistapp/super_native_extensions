import 'package:flutter/widgets.dart';

import 'dart:ui' as ui;

import 'api_model.dart';
import 'mutex.dart';
import 'reader.dart';
import 'util.dart';

import 'native/drop.dart' if (dart.library.js) 'web/drop.dart';

class BaseDropEvent {
  BaseDropEvent({
    required this.sessionId,
  });

  @override
  String toString() => {
        'sessionId': sessionId,
      }.toString();

  final int sessionId;
}

class DropItem {
  DropItem({
    required this.itemId,
    required this.formats,
    this.localData,
    this.readerItem,
  });

  final int itemId;
  final List<String> formats;
  final Object? localData;
  final DataReaderItem? readerItem;

  @override
  String toString() => {
        'itemId': itemId,
        'formats': formats,
        'localData': localData,
      }.toString();
}

class DropEvent extends BaseDropEvent {
  DropEvent({
    required super.sessionId,
    required this.locationInView,
    required this.allowedOperations,
    required this.items,
    this.acceptedOperation,
  });

  final ui.Offset locationInView;
  final List<DropOperation> allowedOperations;
  final List<DropItem> items;
  final DropOperation? acceptedOperation;

  @override
  String toString() => {
        'sessionId': sessionId,
        'locationInView': locationInView.serialize(),
        'items': items.map((e) => e.toString()),
        'allowedOperation':
            allowedOperations.map((e) => e.name).toList(growable: false),
        'acceptedOperation': acceptedOperation?.name,
      }.toString();
}

class ItemPreview {
  ItemPreview({
    required this.destinationRect,
    this.destinationImage,

    /// iOS only
    this.fadeOutDelay,

    /// iOS only
    this.fadeOutDuration,
  });

  /// Destination (in global cooridantes) to where the item should land.
  final ui.Rect destinationRect;

  /// Destination image to which the drag image will morph. If not provided,
  /// drag image will be used.
  final ImageData? destinationImage;

  /// Override fade out delay
  final Duration? fadeOutDelay;

  /// Override fade out duration
  final Duration? fadeOutDuration;
}

class ItemPreviewRequest {
  ItemPreviewRequest({
    required this.sessionId,
    required this.itemId,
    required this.size,
    required this.fadeOutDelay,
    required this.fadeOutDuration,
  });

  static ItemPreviewRequest deserialize(dynamic request) {
    final map = request as Map;
    return ItemPreviewRequest(
      sessionId: map['sessionId'] as int,
      itemId: map['itemId'] as int,
      size: SizeExt.deserialize(map['size']),
      fadeOutDelay: DurationExt.fromSeconds(map['fadeOutDelay'] as double),
      fadeOutDuration:
          DurationExt.fromSeconds(map['fadeOutDuration'] as double),
    );
  }

  final int sessionId;
  final int itemId;
  final ui.Size size;

  /// Default delay before the item preview starts fading out
  final Duration fadeOutDelay;

  /// Default duration of item fade out
  final Duration fadeOutDuration;
}

abstract class DropContextDelegate {
  Future<DropOperation> onDropUpdate(DropEvent event);
  Future<void> onPerformDrop(DropEvent event);
  Future<void> onDropLeave(BaseDropEvent event);
  Future<void> onDropEnded(BaseDropEvent event);

  /// macOS and iOS only.
  Future<ItemPreview?> onGetItemPreview(ItemPreviewRequest request);
}

abstract class DropContext {
  static Future<DropContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = DropContextImpl();
        await _instance!.initialize();
      }
      return _instance!;
    });
  }

  @protected
  Future<void> initialize();

  Future<void> registerDropFormats(List<String> formats);

  DropContextDelegate? delegate;

  static DropContext? _instance;
  static final _mutex = Mutex();
}
