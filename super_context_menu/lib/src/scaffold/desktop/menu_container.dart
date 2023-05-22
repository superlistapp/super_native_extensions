import 'package:collection/collection.dart';
import 'package:flutter/widgets.dart';
import 'package:super_context_menu/src/scaffold/common/deferred_menu_items.dart';

// ignore: implementation_imports
import 'package:super_native_extensions/src/drag_interaction/util.dart';

import '../../menu_model.dart';
import 'menu_layout.dart';
import 'menu_widget.dart';
import 'menu_widget_builder.dart';

abstract class MenuContainerDelegate {
  void hide({
    required bool itemSelected,
  });
}

class MenuContainer extends StatefulWidget {
  const MenuContainer({
    super.key,
    required this.rootMenu,
    required this.rootMenuPosition,
    required this.iconTheme,
    required this.menuWidgetBuilder,
    required this.delegate,
    required this.onInitialPointerUp,
  });

  final Menu rootMenu;
  final Offset rootMenuPosition;
  final IconThemeData iconTheme;
  final DesktopMenuWidgetBuilder menuWidgetBuilder;
  final MenuContainerDelegate delegate;
  final Listenable onInitialPointerUp;

  @override
  State<StatefulWidget> createState() => _MenuContainerState();
}

class _MenuEntry {
  final Menu menu;
  final Offset primaryPosition;
  final MenuLayoutEdge primaryEdge;
  final Offset? secondaryPosition;
  final MenuLayoutEdge? secondaryEdge;
  final MenuWidgetFocusMode focusMode;

  final menuWidgetKey = GlobalKey<MenuWidgetState>();

  _MenuEntry({
    required this.menu,
    required this.primaryPosition,
    required this.primaryEdge,
    required this.focusMode,
    this.secondaryPosition,
    this.secondaryEdge,
  });
}

class _MenuContainerState extends State<MenuContainer>
    implements MenuWidgetDelegate {
  @override
  void initState() {
    super.initState();
    widget.onInitialPointerUp.addListener(_onInitialPointerUp);
  }

  @override
  void dispose() {
    super.dispose();
    widget.onInitialPointerUp.removeListener(_onInitialPointerUp);
  }

  void _onInitialPointerUp() {
    for (final e in _menuEntries) {
      if (e.menuWidgetKey.currentState!.hasFocus()) {
        e.menuWidgetKey.currentState!.onInitialPointerUp();
        return;
      }
    }
    hide(itemSelected: false);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_menuEntries.isEmpty) {
      final renderObject = context.findAncestorRenderObjectOfType<RenderBox>()!;
      final localPosition = renderObject.globalToLocal(widget.rootMenuPosition);
      final directionality = Directionality.of(context);
      _menuEntries.add(_MenuEntry(
        focusMode: MenuWidgetFocusMode.menu,
        menu: widget.rootMenu,
        primaryPosition: localPosition,
        primaryEdge: directionality == TextDirection.ltr
            ? MenuLayoutEdge.left
            : MenuLayoutEdge.right,
        secondaryPosition: localPosition,
        secondaryEdge: directionality == TextDirection.ltr
            ? MenuLayoutEdge.right
            : MenuLayoutEdge.left,
      ));
    }
  }

  final _menuEntries = <_MenuEntry>[];

  Menu? _parentMenu(_MenuEntry entry) {
    final index = _menuEntries.indexOf(entry);
    if (index == 0) {
      return null;
    }
    return _menuEntries[index - 1].menu;
  }

  void _enterMouseRegion() {
    final entries = List.of(_menuEntries);
    for (final e in entries) {
      final state = e.menuWidgetKey.currentState;
      if (state != null) {
        if (state.hasFocus()) {
          state.focusMenu();
        }
      }
    }
  }

  final _deferredMenuElementCache = DeferredMenuElementCache();

  @override
  Widget build(BuildContext context) {
    Widget child = MenuLayout(
      key: _menuLayoutKey,
      padding: const EdgeInsets.all(10),
      children: [
        for (final entry in _menuEntries)
          MenuLayoutData(
            primaryPosition: entry.primaryPosition,
            primaryEdge: entry.primaryEdge,
            secondaryPosition: entry.secondaryPosition,
            secondaryEdge: entry.secondaryEdge,
            child: RepaintBoundary(
              child: MenuWidget(
                key: entry.menuWidgetKey,
                menuWidgetBuilder: widget.menuWidgetBuilder,
                focusMode: entry.focusMode,
                parentMenu: _parentMenu(entry),
                menu: entry.menu,
                iconTheme: widget.iconTheme,
                delegate: this,
                cache: _deferredMenuElementCache,
              ),
            ),
          ),
      ],
    );
    if (_hideFactor != 0) {
      child = Opacity(
        opacity: 1.0 - _hideFactor,
        child: child,
      );
    }
    void hide() {
      this.hide(itemSelected: false);
    }

    return Stack(
      children: [
        Positioned.fill(
          child: GestureDetector(
            onPanDown: (_) => hide(),
            onTapDown: (_) => hide(),
            onSecondaryTapDown: (_) => hide(),
            behavior: HitTestBehavior.translucent,
          ),
        ),
        Positioned.fill(
            child: MouseRegion(
          hitTestBehavior: HitTestBehavior.translucent,
          onEnter: (_) => _enterMouseRegion(),
        )),
        Positioned.fill(
          child: child,
        ),
      ],
    );
  }

  final _menuLayoutKey = GlobalKey();

  double _hideFactor = 0;

  SimpleAnimation? _hideAnimation;

  @override
  void hide({
    required bool itemSelected,
  }) {
    if (_hideAnimation != null) {
      return;
    }
    _hideAnimation =
        SimpleAnimation.animate(const Duration(milliseconds: 200), (value) {
      setState(() {
        _hideFactor = value;
      });
    }, onEnd: () {
      widget.delegate.hide(itemSelected: itemSelected);
    });
  }

  @override
  void popUntil(Menu? menu) {
    if (menu == null) {
      hide(itemSelected: false);
    }
    if (_menuEntries.none((e) => e.menu == menu)) {
      return;
    }
    while (_menuEntries.last.menu != menu) {
      _menuEntries.removeLast();
    }
    setState(() {});
  }

  @override

  /// Returns the directionality of the given menu. This is not necessarily the same
  /// as directionality of menu content. Menu directionality can be flipped in case
  /// the menu submenu can't fit on the screen. The subsequent submenus will then
  /// keep opening in the flipped direction until another edge of screen is reached.
  //
  /// So the directionality of menu is determined as follows:
  ///  - For root menu it is the same as the directionality of the context.
  ///  - For submenu it is determined by the position of the submenu relative to
  ///    to parent menu.
  TextDirection getDirectionalityForMenu(Menu menu) {
    final index = _menuEntries.indexWhere((e) => e.menu == menu);
    if (index == -1) {
      throw StateError('Menu not found');
    } else if (index == 0) {
      return Directionality.of(context);
    } else {
      final renderObject = _menuEntries[index]
          .menuWidgetKey
          .currentContext!
          .findRenderObject() as RenderBox;
      final parentRenderObject = _menuEntries[index - 1]
          .menuWidgetKey
          .currentContext!
          .findRenderObject() as RenderBox;

      final position = renderObject.localToGlobal(Offset.zero);
      final parentPosition = parentRenderObject.localToGlobal(Offset.zero);
      if (position.dx < parentPosition.dx) {
        return TextDirection.rtl;
      } else {
        return TextDirection.ltr;
      }
    }
  }

  @override
  void pushMenu({
    required Menu parent,
    required Menu menu,
    required BuildContext context,
    required MenuWidgetFocusMode focusMode,
  }) {
    if (_menuEntries.last.menu == menu) {
      final entry = _menuEntries.last;
      if (focusMode != MenuWidgetFocusMode.none &&
          !entry.menuWidgetKey.currentState!.hasFocus()) {
        if (focusMode == MenuWidgetFocusMode.menu) {
          entry.menuWidgetKey.currentState!.focusMenu();
        } else {
          entry.menuWidgetKey.currentState!.focusFirstItem();
        }
      }
      return;
    }
    while (_menuEntries.last.menu != parent) {
      _menuEntries.removeLast();
    }
    final renderBox = context.findRenderObject() as RenderBox;
    final directionality = getDirectionalityForMenu(parent);
    final transform = renderBox
        .getTransformTo(_menuLayoutKey.currentContext!.findRenderObject());
    if (directionality == TextDirection.ltr) {
      final primaryPosition = MatrixUtils.transformPoint(
          transform, Offset(renderBox.size.width, 0));
      final secondaryPosition =
          MatrixUtils.transformPoint(transform, Offset.zero);
      _menuEntries.add(_MenuEntry(
        menu: menu,
        primaryPosition: primaryPosition,
        primaryEdge: MenuLayoutEdge.left,
        secondaryPosition: secondaryPosition,
        secondaryEdge: MenuLayoutEdge.right,
        focusMode: focusMode,
      ));
    } else {
      final primaryPosition =
          MatrixUtils.transformPoint(transform, Offset.zero);
      final secondaryPosition = MatrixUtils.transformPoint(
          transform, Offset(renderBox.size.width, 0));
      _menuEntries.add(_MenuEntry(
        menu: menu,
        primaryPosition: primaryPosition,
        primaryEdge: MenuLayoutEdge.right,
        secondaryPosition: secondaryPosition,
        secondaryEdge: MenuLayoutEdge.left,
        focusMode: focusMode,
      ));
    }

    setState(() {});
  }
}
