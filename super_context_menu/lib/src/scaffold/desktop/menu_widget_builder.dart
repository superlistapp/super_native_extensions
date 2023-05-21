import 'package:flutter/widgets.dart';

import '../../menu_model.dart';

class DesktopMenuInfo {
  DesktopMenuInfo({
    required this.menu,
    required this.parentMenu,
    required this.resolvedChildren,
    required this.iconTheme,
    required this.focused,
  });

  bool get isRoot => parentMenu == null;

  final Menu menu;
  final Menu? parentMenu;
  final List<MenuElement> resolvedChildren;
  final IconThemeData iconTheme;
  final bool focused;
}

class DesktopMenuButtonState {
  DesktopMenuButtonState({
    required this.selected,
  });

  final bool selected;
}

abstract class DesktopMenuWidgetBuilder {
  Widget buildMenuContainer(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    Widget child,
  );

  Widget buildSeparator(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    MenuSeparator separator,
  );

  Widget buildMenuItem(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    Key innerKey,
    DesktopMenuButtonState state,
    MenuElement element,
  );
}
