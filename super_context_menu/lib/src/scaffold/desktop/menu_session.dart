import 'package:flutter/widgets.dart';
import 'package:super_context_menu/src/scaffold/desktop/menu_widget_builder.dart';

import '../../menu_model.dart';
import 'menu_container.dart';

class ContextMenuSession implements MenuContainerDelegate {
  ContextMenuSession({
    required BuildContext context,
    required DesktopMenuWidgetBuilder menuWidgetBuilder,
    required Listenable onInitialPointerUp,
    required Menu menu,
    required Offset position,
    required IconThemeData iconTheme,
    required this.onDone,
  }) {
    final overlay = Overlay.of(context, rootOverlay: true);
    _entry = OverlayEntry(
      builder: (context) {
        return MenuContainer(
          rootMenu: menu,
          rootMenuPosition: position,
          delegate: this,
          menuWidgetBuilder: menuWidgetBuilder,
          iconTheme: iconTheme,
          onInitialPointerUp: onInitialPointerUp,
        );
      },
      opaque: false,
    );
    overlay.insert(_entry);
  }

  final ValueSetter<MenuResult> onDone;
  late OverlayEntry _entry;

  @override
  void hide({
    required bool itemSelected,
  }) {
    onDone(MenuResult(itemSelected: itemSelected));
    _entry.remove();
  }
}
