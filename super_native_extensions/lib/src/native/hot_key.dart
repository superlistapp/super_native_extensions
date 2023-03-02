import 'package:flutter/services.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';
import 'package:super_native_extensions/src/hot_key.dart';

import 'context.dart';

class HotKeyManagerImpl extends HotKeyManager {
  HotKeyManagerImpl() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'onHotKey') {
      _delegate?.onHotKey(call.arguments as int);
    }
  }

  @override
  Future<int?> createHotKey(HotKeyDefinition definition) async {
    return _channel.invokeMethod('createHotKey', definition.serialize());
  }

  @override
  Future<void> destroyHotKey(int handle) async {
    await _channel.invokeMethod('destroyHotKey', {'handle': handle});
  }

  @override
  set delegate(HotKeyManagerDelegate? delegate) {
    _delegate = delegate;
  }

  HotKeyManagerDelegate? _delegate;

  final _channel = NativeMethodChannel('HotKeyManager',
      context: superNativeExtensionsContext);
}
