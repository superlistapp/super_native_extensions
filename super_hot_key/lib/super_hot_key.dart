library super_hot_key;

import 'package:flutter/services.dart';
import 'package:super_keyboard_layout/super_keyboard_layout.dart';
import 'package:super_native_extensions/raw_hot_key.dart' as raw;

class HotKeyDefinition {
  HotKeyDefinition({
    required this.key,
    this.alt = false,
    this.shift = false,
    this.control = false,
    this.meta = false,
  });

  final PhysicalKeyboardKey key;
  final bool alt;
  final bool shift;
  final bool control;
  final bool meta;

  Future<raw.HotKeyDefinition?> toRaw() async {
    final manager = await KeyboardLayoutManager.instance();
    final platformKey = manager.currentLayout.getPlatformKeyCode(key);
    if (platformKey != null) {
      return raw.HotKeyDefinition(
        platformCode: platformKey,
        alt: alt,
        shift: shift,
        control: control,
        meta: meta,
      );
    } else {
      return null;
    }
  }
}

class HotKey {
  final int _handle;
  final HotKeyDefinition definition;
  final VoidCallback? onPressed;
  final VoidCallback? onReleased;

  static Future<HotKey?> create({
    required HotKeyDefinition definition,
    VoidCallback? onPressed,
    VoidCallback? onReleased,
  }) async {
    return _HotKeyManager.instance.createHotKey(
      definition,
      onPressed,
      onReleased,
    );
  }

  Future<void> dispose() async {
    if (!_disposed) {
      _disposed = true;
      await _HotKeyManager.instance.destroyHotKey(this);
    }
  }

  bool _disposed = false;

  HotKey._(this._handle, this.definition, this.onPressed, this.onReleased)
      : assert(_handle != 0);
}

class _HotKeyManager extends raw.HotKeyManagerDelegate {
  _HotKeyManager._() {
    raw.HotKeyManager.instance.delegate = this;
  }

  final _hotKeys = <int, HotKey>{};

  static final instance = _HotKeyManager._();

  Future<HotKey?> createHotKey(
    HotKeyDefinition definition,
    VoidCallback? onPressed,
    VoidCallback? onReleased,
  ) async {
    final rawDefinition = await definition.toRaw();
    if (rawDefinition == null) {
      return null;
    }
    final handle = await raw.HotKeyManager.instance.createHotKey(rawDefinition);
    if (handle != null) {
      final res = HotKey._(
        handle,
        definition,
        onPressed,
        onReleased,
      );
      _hotKeys[handle] = res;
      return res;
    } else {
      return null;
    }
  }

  Future<void> destroyHotKey(HotKey hotKey) async {
    _hotKeys.remove(hotKey);
    await raw.HotKeyManager.instance.destroyHotKey(hotKey._handle);
  }

  @override
  void onHotKeyPressed(int handle) {
    _hotKeys[handle]?.onPressed?.call();
  }

  @override
  void onHotKeyReleased(int handle) {
    _hotKeys[handle]?.onReleased?.call();
  }
}
