import 'package:collection/collection.dart';
import 'package:flutter/widgets.dart';
import 'package:super_native_extensions/raw_drag_drop.dart';

import 'dart:ui' as ui;

import 'api_model.dart';
import 'mutex.dart';
import 'native/drop.dart';
import 'reader.dart';
import 'reader_manager.dart';
import 'util.dart';

class BaseDropEvent {
  BaseDropEvent({
    required this.sessionId,
  });

  static BaseDropEvent deserialize(dynamic event) {
    final map = event as Map;
    return BaseDropEvent(sessionId: map['sessionId']);
  }

  Map serialize() => {
        'sessionId': sessionId,
      };

  @override
  String toString() => serialize().toString();

  final int sessionId;
}

typedef ReaderProvider = DataReader? Function(int sessionId);

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

  static DropItem deserialize(dynamic item, DataReaderItem? readerItem) {
    final map = item as Map;
    return DropItem(
      itemId: map['itemId'],
      formats: (map['formats'] as List).cast<String>(),
      localData: map['localData'],
      readerItem: readerItem,
    );
  }

  dynamic serialize() => {
        'itemId': itemId,
        'formats': formats,
        'localData': localData,
      };
}

class DropEvent extends BaseDropEvent {
  DropEvent({
    required super.sessionId,
    required this.locationInView,
    required this.allowedOperations,
    required this.items,
    this.acceptedOperation,
    this.reader,
  });

  final ui.Offset locationInView;
  final List<DropOperation> allowedOperations;
  final List<DropItem> items;
  final DropOperation? acceptedOperation;
  final DataReader? reader;

  // readerProvider is to ensure that reader is only deserialized once and
  // same instance is used subsequently.
  static Future<DropEvent> deserialize(
      dynamic event, ReaderProvider readerProvider) async {
    final map = event as Map;
    final acceptedOperation = map['acceptedOperation'];
    final sessionId = map['sessionId'] as int;
    DataReader? getReader() {
      final reader = map['reader'];
      return reader != null
          ? DataReader(handle: DataReaderHandle.deserialize(reader))
          : null;
    }

    final reader = readerProvider(sessionId) ?? getReader();
    final items = await reader?.getItems();

    DropItem deserializeItem(int index, dynamic item) {
      final readerItem =
          (items != null && index < items.length) ? items[index] : null;
      return DropItem.deserialize(item, readerItem);
    }

    return DropEvent(
      sessionId: sessionId,
      locationInView: OffsetExt.deserialize(map['locationInView']),
      items: (map['items'] as Iterable)
          .mapIndexed(deserializeItem)
          .toList(growable: false),
      allowedOperations: (map['allowedOperations'] as Iterable)
          .map((e) => DropOperation.values.byName(e))
          .toList(growable: false),
      acceptedOperation: acceptedOperation != null
          ? DropOperation.values.byName(acceptedOperation)
          : null,
      reader: reader,
    );
  }

  @override
  Map serialize() => {
        'sessionId': sessionId,
        'locationInView': locationInView.serialize(),
        'items': items.map((e) => e.serialize()),
        'allowedOperation':
            allowedOperations.map((e) => e.name).toList(growable: false),
        'acceptedOperation': acceptedOperation?.name,
      };

  @override
  String toString() => serialize().toString();
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

  dynamic serialize() => {
        'destinationRect': destinationRect.serialize(),
        'destinationImage': destinationImage?.serialize(),
        'fadeOutDelay': fadeOutDelay?.inSecondsDouble,
        'fadeOutDuration': fadeOutDuration?.inSecondsDouble,
      };
}

class ItemPreviewRequest {
  ItemPreviewRequest({
    required this.sessionId,
    required this.itemId,
    required this.size,
    required this.fadeOutDelay,
    required this.fadeOutDuration,
  });

  static deserialize(dynamic request) {
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

abstract class RawDropContextDelegate {
  Future<DropOperation> onDropUpdate(DropEvent event);
  Future<void> onPerformDrop(DropEvent event);
  Future<void> onDropLeave(BaseDropEvent event);
  Future<void> onDropEnded(BaseDropEvent event);

  /// macOS and iOS only.
  Future<ItemPreview?> onGetItemPreview(ItemPreviewRequest request);
}

abstract class RawDropContext {
  set delegate(RawDropContextDelegate? delegate) {
    _delegate = delegate;
  }

  static Future<RawDropContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = RawDropContextImpl();
        await _instance!.initialize();
      }
      return _instance!;
    });
  }

  @protected
  Future<void> initialize();

  Future<void> registerDropTypes(List<String> types);

  @protected
  RawDropContextDelegate? get delegate => _delegate;

  RawDropContextDelegate? _delegate;

  static RawDropContext? _instance;
  static final _mutex = Mutex();
}
