import 'dart:async';

import 'menu.dart';
import 'menu_model.dart';

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
  Future<MenuResult> showContextMenu(DesktopContextMenuRequest request) async {
    return request.fallback();
  }
}
