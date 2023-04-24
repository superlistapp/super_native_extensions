import 'menu.dart';

// Flutter-only menu context implementation, used on web and android.

class _MenuHandle extends MenuHandle {
  @override
  final Menu menu;

  _MenuHandle(this.menu);

  @override
  void dispose() {}
}

class FlutterMenuContext extends MenuContext {
  @override
  Future<MenuHandle> registerMenu(
    Menu menu,
    MenuSerializationOptions options,
  ) async {
    return _MenuHandle(menu);
  }

  @override
  Future<void> initialize() async {}
}
