import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'native/keyboard_layout.dart'
    if (dart.library.js) 'web/keyboard_layout.dart';

import 'keyboard_layout_model.dart' as model;

/// Allows converting between physical and logical keys according to current
/// keyboard layout as well as notification about keyboard layout changes.
abstract class KeyboardLayoutManager {
  /// Returns shared instance of [KeyboardLayoutManager];
  static Future<KeyboardLayoutManager> instance() =>
      KeyboardLayoutManagerImpl.instance();

  /// Whether keyboard layout mapping is supported on this platform.
  /// Currently it is only supported on desktop platforms.
  bool get supported;

  /// Returns mapping for currently active keyboard layout.
  KeyboardLayout get currentLayout;

  /// Event fired when current system keyboard layout changes.
  Listenable get onLayoutChanged;
}

/// Represents a keyboard layout. Allows coverting between platform specific
/// key codes, [PhysicalKeyboardKey]s and [LogicalKeyboardKey]s.
class KeyboardLayout {
  /// Returns the platform specific key code for given [KeyboardKey] for this
  /// keyboard layout or `null` if the code could not have been determined.
  int? getPlatformKeyCode(KeyboardKey key) {
    if (key is PhysicalKeyboardKey) {
      return _physicalToKey[key.usbHidUsage]?.platform;
    } else if (key is LogicalKeyboardKey) {
      return _logicalToKey[key.keyId]?.platform;
    } else {
      return null;
    }
  }

  /// Returns the [PhysicalKeyboardKey] for platform specific key code for this
  /// keyboard layout or `null` if it could not have been determined.
  PhysicalKeyboardKey? getPhysicalKeyForPlatformKeyCode(int code) {
    final key = _platformToKey[code];
    return key != null ? PhysicalKeyboardKey(key.physical) : null;
  }

  /// Returns the [PhysicalKeyboardKey] for given [LogicalKeyboardKey] for
  /// this keyboard layout or `null` if it could not have been determined.
  PhysicalKeyboardKey? getPhysicalKeyForLogicalKey(
      LogicalKeyboardKey logicalKey) {
    final key = _logicalToKey[logicalKey.keyId];
    return key != null ? PhysicalKeyboardKey(key.physical) : null;
  }

  /// Returns the [LogicalKeyboardKey] for given [PhysicalKeyboardKey] and
  /// modifiers for this keyboard layout or `null` if it could not have been
  /// determined.
  LogicalKeyboardKey? getLogicalKeyForPhysicalKey(
    PhysicalKeyboardKey physicalKey, {
    bool shift = false,
    bool alt = false,
    bool meta = false,
  }) {
    final key = _physicalToKey[physicalKey.usbHidUsage];

    if (key == null) {
      return null;
    }

    if (meta && key.logicalMeta != null) {
      return LogicalKeyboardKey(key.logicalMeta!);
    } else if (shift && alt && key.logicalAltShift != null) {
      return LogicalKeyboardKey(key.logicalAltShift!);
    } else if (shift && !alt && key.logicalShift != null) {
      return LogicalKeyboardKey(key.logicalShift!);
    } else if (!shift && alt && key.logicalAlt != null) {
      return LogicalKeyboardKey(key.logicalAlt!);
    } else if (!shift && !alt && key.logical != null) {
      return LogicalKeyboardKey(key.logical!);
    } else {
      return null;
    }
  }

  final Map<int, model.KeyboardKey> _platformToKey;
  final Map<int, model.KeyboardKey> _physicalToKey;
  final Map<int, model.KeyboardKey> _logicalToKey;

  KeyboardLayout(this._platformToKey, this._physicalToKey, this._logicalToKey);
}
