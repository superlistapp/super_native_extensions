import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import '../api_model.dart';
import '../drop.dart';
import '../reader.dart';

final _channel =
    NativeMethodChannel('DropManager', context: superNativeExtensionsContext);

class Session {
  DataReader? reader;
}

class RawDropContextImpl extends RawDropContext {
  RawDropContextImpl();

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
          await DropEvent.deserialize(call.arguments, _getReaderForSession);
      _sessionForId(event.sessionId).reader = event.reader;
      final operation = await delegate?.onDropUpdate(event);
      return (operation ?? DropOperation.none).name;
    } else if (call.method == 'onPerformDrop') {
      final event =
          await DropEvent.deserialize(call.arguments, _getReaderForSession);
      _sessionForId(event.sessionId).reader = event.reader;
      return await delegate?.onPerformDrop(event);
    } else if (call.method == 'onDropLeave') {
      final event = BaseDropEvent.deserialize(call.arguments);
      return await delegate?.onDropLeave(event);
    } else if (call.method == 'onDropEnded') {
      final event = BaseDropEvent.deserialize(call.arguments);
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
  Future<void> registerDropTypes(List<String> types) {
    return _channel.invokeMethod("registerDropTypes", {'types': types});
  }
}
