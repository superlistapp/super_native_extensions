import 'package:flutter/widgets.dart';

import '../../menu_model.dart';

class MobileMenuInfo {
  MobileMenuInfo({
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

class MobileMenuButtonState {
  MobileMenuButtonState({
    required this.pressed,
  });

  final bool pressed;
}

abstract class MobileMenuWidgetBuilder {
  const MobileMenuWidgetBuilder();

  /// Builds the outer menu container. This is the decoration (shadow, outline)
  /// Clip rect.
  Widget buildMenuContainer(
    BuildContext context,
    MobileMenuInfo menuInfo,
    Widget child,
  );

  /// Builds the inner menu container (decoratioin within the clip rect)
  Widget buildMenuContainerInner(
    BuildContext context,
    MobileMenuInfo menuInfo,
    Widget child,
  );

  Widget buildMenu(
    BuildContext context,
    MobileMenuInfo menuInfo,
    Widget child,
  );

  /// Builds container for the list view that contains menu children.
  Widget buildMenuItemsContainer(
    BuildContext context,
    MobileMenuInfo menuInfo,
    Widget child,
  );

  /// Builds menu header (used for root menus that have title and child menus).
  Widget buildMenuHeader(
    BuildContext context,
    MobileMenuInfo menuInfo,
    MobileMenuButtonState state,
  );

  /// Builds veil that covers inactive menu (below currently active menu).
  Widget buildInactiveMenuVeil(
    BuildContext context,
    MobileMenuInfo menuInfo,
  );

  /// Builds single menu item.
  Widget buildMenuItem(
    BuildContext context,
    MobileMenuInfo menuInfo,
    MobileMenuButtonState state,
    MenuElement element,
  );

  /// Builds the backgroudn widget for menu overlay.
  Widget buildOverlayBackground(
    BuildContext context,
    double opacity,
  );

  /// Build the container for menu preview image.
  Widget buildMenuPreviewContainer(
    BuildContext context,
    Widget child,
  );
}
