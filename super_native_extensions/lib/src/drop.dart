import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';
import 'package:super_native_extensions/raw_clipboard.dart';
import 'package:super_native_extensions/raw_drag_drop.dart';

import 'context.dart';
import 'mutex.dart';

class DropEvent {
  DropEvent({
    required this.sessionId,
    required this.locationInView,
    required this.localData,
    required this.allowedOperations,
    required this.formats,
    required this.reader,
  });

  final int sessionId;
  final Offset locationInView;
  final Uint8List localData;
  final List<DropOperation> allowedOperations;
  final List<String> formats;
  final DataReader? reader;
}

class DropLeaveEvent {
  DropLeaveEvent({
    required this.sessionId,
  });

  final int sessionId;
}

abstract class RawDropContextDelegate {
  Future<DropOperation> onDropOver(DropEvent event);
  Future<void> onPerformDrop(DropEvent event);
  Future<void> onDropLeave(DropLeaveEvent event);
}

final _channel =
    NativeMethodChannel('DropManager', context: superNativeExtensionsContext);

class RawDropContext {
  RawDropContext._();

  static RawDropContext? _instance;
  static final _mutex = Mutex();

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

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    return null;
  }

  Future<void> registerDropTypes(List<String> types) {
    return _channel.invokeMethod("registerDropTypes", {'types': types});
  }
}
