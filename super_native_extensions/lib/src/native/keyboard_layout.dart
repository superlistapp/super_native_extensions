import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import '../keyboard_layout.dart';
import '../keyboard_layout_model.dart' as model;
import '../mutex.dart';
import '../util.dart';
import 'context.dart';

class KeyboardLayoutManagerImpl extends KeyboardLayoutManager {
  static KeyboardLayoutManagerImpl? _instance;

  static Future<KeyboardLayoutManager> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = KeyboardLayoutManagerImpl();
        await _instance!.initialize();
      }
      return _instance!;
    });
  }

  KeyboardLayoutManagerImpl() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'onLayoutChanged') {
      _update(model.KeyboardLayout.deserialize(call.arguments));
      _onLayoutChanged.notify();
    }
  }

  late KeyboardLayout _currentLayout;

  Future<void> initialize() async {
    final layout = model.KeyboardLayout.deserialize(
        await _channel.invokeMethod('getKeyboardLayout'));
    _update(layout);
  }

  void _update(model.KeyboardLayout layout) {
    final platformToKey = <int, model.KeyboardKey>{};
    final physicalToKey = <int, model.KeyboardKey>{};
    final logicalToKey = <int, model.KeyboardKey>{};

    for (final key in layout.keys) {
      platformToKey[key.platform] = key;
      physicalToKey[key.physical] = key;
      if (key.logicalAltShift != null) {
        logicalToKey[key.logicalAltShift!] = key;
      }
      if (key.logicalAlt != null) {
        logicalToKey[key.logicalAlt!] = key;
      }
      if (key.logicalShift != null) {
        logicalToKey[key.logicalShift!] = key;
      }
      if (key.logicalMeta != null) {
        logicalToKey[key.logicalMeta!] = key;
      }
      if (key.logical != null) {
        logicalToKey[key.logical!] = key;
      }
    }

    _currentLayout = KeyboardLayout(platformToKey, physicalToKey, logicalToKey);
    _supported = layout.keys.isNotEmpty;
  }

  @override
  KeyboardLayout get currentLayout => _currentLayout;

  final _onLayoutChanged = SimpleNotifier();

  @override
  Listenable get onLayoutChanged => _onLayoutChanged;

  bool _supported = false;

  @override
  bool get supported => _supported;

  static final _mutex = Mutex();

  final _channel = NativeMethodChannel('KeyboardLayoutManager',
      context: superNativeExtensionsContext);
}
