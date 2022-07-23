import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';
import 'package:collection/collection.dart';

import 'api_model.dart';
import 'reader.dart';
import 'reader_manager.dart';
import 'context.dart';
import 'mutex.dart';
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
    DataReader? reader,
  }) : _reader = reader;

  final Offset locationInView;
  final List<DropOperation> allowedOperations;
  final List<DropItem> items;
  final DropOperation? acceptedOperation;
  final DataReader? _reader;

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

    DropItem deserializeItem(int index, dynamic item) =>
        DropItem.deserialize(item, items?[index]);

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
  final Rect destinationRect;

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
  final Size size;

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

final _channel =
    NativeMethodChannel('DropManager', context: superNativeExtensionsContext);

class Session {
  DataReader? reader;
}

class RawDropContext {
  RawDropContext._();

  static RawDropContext? _instance;
  static final _mutex = Mutex();

  static final _sessions = <int, Session>{};

  set delegate(RawDropContextDelegate? delegate) {
    _delegate = delegate;
  }

  RawDropContextDelegate? _delegate;

  Future<void> _initialize() async {
    WidgetsFlutterBinding.ensureInitialized();
    final view = await getFlutterView();
    _channel.setMethodCallHandler(_handleMethodCall);
    await _channel.invokeMethod("newContext", {'viewHandle': view});
  }

  static Future<RawDropContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = RawDropContext._();
        await _instance!._initialize();
      }
      return _instance!;
    });
  }

  Session _sessionForId(int id) {
    return _sessions.putIfAbsent(id, () => Session());
  }

  DataReader? _getReaderForSession(int sessionId) {
    return _sessionForId(sessionId).reader;
  }

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    if (call.method == 'onDropUpdate') {
      final event =
          await DropEvent.deserialize(call.arguments, _getReaderForSession);
      _sessionForId(event.sessionId).reader = event._reader;
      final operation = await _delegate?.onDropUpdate(event);
      return (operation ?? DropOperation.none).name;
    } else if (call.method == 'onPerformDrop') {
      final event =
          await DropEvent.deserialize(call.arguments, _getReaderForSession);
      _sessionForId(event.sessionId).reader = event._reader;
      return await _delegate?.onPerformDrop(event);
    } else if (call.method == 'onDropLeave') {
      final event = BaseDropEvent.deserialize(call.arguments);
      return await _delegate?.onDropLeave(event);
    } else if (call.method == 'onDropEnded') {
      final event = BaseDropEvent.deserialize(call.arguments);
      final session = _sessions.remove(event.sessionId);
      session?.reader?.dispose();
      return await _delegate?.onDropEnded(event);
    } else if (call.method == 'getPreviewForItem') {
      final request = ItemPreviewRequest.deserialize(call.arguments);
      final preview = await _delegate?.onGetItemPreview(request);
      return {
        'preview': preview?.serialize(),
      };
    } else {
      return null;
    }
  }

  Future<void> registerDropTypes(List<String> types) {
    return _channel.invokeMethod("registerDropTypes", {'types': types});
  }
}
