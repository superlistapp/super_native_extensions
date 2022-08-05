import 'package:collection/collection.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import '../api_model.dart';
import '../drop.dart';
import '../reader.dart';
import '../util.dart';
import 'api_model.dart';
import 'context.dart';
import 'reader_manager.dart';

final _channel =
    NativeMethodChannel('DropManager', context: superNativeExtensionsContext);

class Session {
  DataReader? reader;
}

extension ItemPreviewExt on ItemPreview {
  dynamic serialize() => {
        'destinationRect': destinationRect.serialize(),
        'destinationImage': destinationImage?.serialize(),
        'fadeOutDelay': fadeOutDelay?.inSecondsDouble,
        'fadeOutDuration': fadeOutDuration?.inSecondsDouble,
      };
}

extension BaseDropEventExt on BaseDropEvent {
  static BaseDropEvent deserialize(dynamic event) {
    final map = event as Map;
    return BaseDropEvent(sessionId: map['sessionId']);
  }
}

extension DropItemExt on DropItem {
  static DropItem deserialize(dynamic item, DataReaderItem? readerItem) {
    final map = item as Map;
    return DropItem(
      itemId: map['itemId'],
      formats: (map['formats'] as List).cast<String>(),
      localData: map['localData'],
      readerItem: readerItem,
    );
  }
}

typedef ReaderProvider = DataReader? Function(int sessionId);

class DropEventImpl extends DropEvent {
  DropEventImpl({
    required super.sessionId,
    required super.locationInView,
    required super.allowedOperations,
    required super.items,
    super.acceptedOperation,
    this.reader,
  });

  // readerProvider is to ensure that reader is only deserialized once and
  // same instance is used subsequently.
  static Future<DropEventImpl> deserialize(
      dynamic event, ReaderProvider readerProvider) async {
    final map = event as Map;
    final acceptedOperation = map['acceptedOperation'];
    final sessionId = map['sessionId'] as int;
    DataReader? getReader() {
      final reader = map['reader'];
      return reader != null
          ? DataReader(handle: DataReaderHandleImpl.deserialize(reader))
          : null;
    }

    final reader = readerProvider(sessionId) ?? getReader();
    final items = await reader?.getItems();

    DropItem deserializeItem(int index, dynamic item) {
      final readerItem =
          (items != null && index < items.length) ? items[index] : null;
      return DropItemExt.deserialize(item, readerItem);
    }

    return DropEventImpl(
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

  final DataReader? reader;
}

class DropContextImpl extends DropContext {
  DropContextImpl();

  static final _sessions = <int, Session>{};

  @override
  Future<void> initialize() async {
    WidgetsFlutterBinding.ensureInitialized();
    final view = await getFlutterView();
    _channel.setMethodCallHandler(_handleMethodCall);
    await _channel.invokeMethod("newContext", {'viewHandle': view});
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
          await DropEventImpl.deserialize(call.arguments, _getReaderForSession);
      _sessionForId(event.sessionId).reader = event.reader;
      final operation = await delegate?.onDropUpdate(event);
      return (operation ?? DropOperation.none).name;
    } else if (call.method == 'onPerformDrop') {
      final event =
          await DropEventImpl.deserialize(call.arguments, _getReaderForSession);
      _sessionForId(event.sessionId).reader = event.reader;
      return await delegate?.onPerformDrop(event);
    } else if (call.method == 'onDropLeave') {
      final event = BaseDropEventExt.deserialize(call.arguments);
      return await delegate?.onDropLeave(event);
    } else if (call.method == 'onDropEnded') {
      final event = BaseDropEventExt.deserialize(call.arguments);
      final session = _sessions.remove(event.sessionId);
      session?.reader?.dispose();
      return await delegate?.onDropEnded(event);
    } else if (call.method == 'getPreviewForItem') {
      final request = ItemPreviewRequest.deserialize(call.arguments);
      final preview = await delegate?.onGetItemPreview(request);
      return {
        'preview': preview?.serialize(),
      };
    } else {
      return null;
    }
  }

  @override
  Future<void> registerDropFormats(List<String> formats) {
    return _channel.invokeMethod("registerDropFormats", {'formats': formats});
  }
}
