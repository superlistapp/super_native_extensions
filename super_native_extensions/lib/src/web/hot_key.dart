import '../hot_key.dart';

class HotKeyManagerImpl extends HotKeyManager {
  @override
  Future<int?> createHotKey(HotKeyDefinition definition) async {
    return null;
  }

  @override
  set delegate(HotKeyManagerDelegate? delegate) {}

  @override
  Future<void> destroyHotKey(int handle) async {}
}
