import 'package:flutter/widgets.dart';

import '../../menu.dart';

class MenuInfo {
  MenuInfo({
    required this.menu,
    required this.parentMenu,
    required ValueGetter<List<MenuElement>?> resolvedChildren,
    required this.depth,
    required this.isCollapsed,
    required this.transitionDuration,
    required this.iconTheme,
  }) : _resolvedChildren = resolvedChildren;

  bool get isRoot => parentMenu == null;

  final Menu menu;
  final Menu? parentMenu;
  List<MenuElement> get resolvedChildren =>
      _resolvedChildren() ?? menu.children;
  final ValueGetter<List<MenuElement>?> _resolvedChildren;
  final int depth;
  final bool isCollapsed;
  final Duration transitionDuration;
  final IconThemeData iconTheme;
}

class MenuButtonState {
  MenuButtonState({
    required this.pressed,
  });

  final bool pressed;
}

abstract class MenuWidgetBuilder {
  const MenuWidgetBuilder();

  Widget buildMenuContainer(
    BuildContext context,
    MenuInfo menuInfo,
    Widget child,
  );

  Widget buildMenu(
    BuildContext context,
    MenuInfo menuInfo,
    Widget child,
  );

  Widget buildMenuItemsContainer(
    BuildContext context,
    MenuInfo menuInfo,
    Widget child,
  );

  Widget buildInactiveMenuVeil(
    BuildContext context,
    MenuInfo menuInfo,
  );

  Widget buildMenuHeader(
    BuildContext context,
    MenuInfo menuInfo,
    MenuButtonState state,
  );

  Widget buildMenuItem(
    BuildContext context,
    MenuInfo menuInfo,
    MenuButtonState state,
    MenuElement element,
  );

  Widget buildOverlayBackground(
    BuildContext context,
    double opacity,
  );

  Widget buildMenuPreviewContainer(
    BuildContext context,
    Widget child,
  );
}
