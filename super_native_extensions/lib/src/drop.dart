import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

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

class DropEvent extends BaseDropEvent {
  DropEvent({
    required super.sessionId,
    required this.locationInView,
    required this.localData,
    required this.allowedOperations,
    required this.formats,
    required this.acceptedOperation,
    required this.reader,
  });

  // readerProvider is to ensure that reader is only deserialized once and
  // same instance is used subsequently.
  static DropEvent deserialize(dynamic event, ReaderProvider readerProvider) {
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
    return DropEvent(
      sessionId: sessionId,
      locationInView: OffsetExt.deserialize(map['locationInView']),
      localData: map['localData'],
      allowedOperations: (map['allowedOperations'] as Iterable)
          .map((e) => DropOperation.values.byName(e))
          .toList(growable: false),
      formats: (map['formats'] as List).cast<String>(),
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
        'localData': localData,
        'allowedOperation':
            allowedOperations.map((e) => e.name).toList(growable: false),
        'formats': formats,
        'acceptedOperation': acceptedOperation?.name,
      };

  @override
  String toString() => serialize().toString();

  final Offset locationInView;
  final List<Object?> localData;
  final List<DropOperation> allowedOperations;
  final List<String> formats;
  final DropOperation? acceptedOperation;
  final DataReader? reader;
}

abstract class RawDropContextDelegate {
  Future<DropOperation> onDropUpdate(DropEvent event);
  Future<void> onPerformDrop(DropEvent event);
  Future<void> onDropLeave(BaseDropEvent event);
  Future<void> onDropEnded(BaseDropEvent event);
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
      final event = DropEvent.deserialize(call.arguments, _getReaderForSession);
      _sessionForId(event.sessionId).reader = event.reader;
      final operation = await _delegate?.onDropUpdate(event);
      return (operation ?? DropOperation.none).name;
    } else if (call.method == 'onPerformDrop') {
      final event = DropEvent.deserialize(call.arguments, _getReaderForSession);
      return await _delegate?.onPerformDrop(event);
    } else if (call.method == 'onDropLeave') {
      final event = BaseDropEvent.deserialize(call.arguments);
      return await _delegate?.onDropLeave(event);
    } else if (call.method == 'onDropEnded') {
      final event = BaseDropEvent.deserialize(call.arguments);
      _sessions.remove(event.sessionId);
      return await _delegate?.onDropEnded(event);
    } else {
      return null;
    }
  }

  Future<void> registerDropTypes(List<String> types) {
    return _channel.invokeMethod("registerDropTypes", {'types': types});
  }
}
