import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import 'mutex.dart';

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
