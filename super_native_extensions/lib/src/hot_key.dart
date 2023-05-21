import 'native/hot_key.dart' if (dart.library.js) 'web/hot_key.dart';

class HotKeyDefinition {
  final int platformCode;
  final bool alt;
  final bool shift;
  final bool meta;
  final bool control;

  HotKeyDefinition({
    required this.platformCode,
    required this.alt,
    required this.shift,
    required this.meta,
    required this.control,
  });

  dynamic serialize() => {
        'platformCode': platformCode,
        'alt': alt,
        'shift': shift,
        'meta': meta,
        'control': control,
      };
}

abstract class HotKeyManagerDelegate {
  /// Invoked when hot key with given handle is pressed.
  void onHotKey(int handle);
}

abstract class HotKeyManager {
  static final _instance = HotKeyManagerImpl();

  static HotKeyManager get instance => _instance;

  /// Creates HotKey for given definition. Returns null if not supported on
  /// this platform.
  Future<int?> createHotKey(HotKeyDefinition definition);

  /// Destroys hot key with given handle;
  Future<void> destroyHotKey(int handle);

  set delegate(HotKeyManagerDelegate? delegate);
}
