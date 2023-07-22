import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/rendering.dart';
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
            child: _MenuSafeTriangleHitTestWidget(
              menuStateProvider: () => entry.menuWidgetKey.currentState!,
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
  RenderBox? getMenuRenderBox(Menu menu) {
    final entry = _menuEntries.firstWhereOrNull((e) => e.menu == menu);
    if (entry != null) {
      final renderObject =
          entry.menuWidgetKey.currentContext?.findRenderObject();
      if (renderObject is RenderBox) {
        return renderObject;
      }
    }
    return null;
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

/// Implements safe triangle for transition between menu and submenu.
/// https://bjk5.com/post/44698559168/breaking-down-amazons-mega-dropdown
/// https://height.app/blog/guide-to-build-context-menus
class _MenuSafeTriangleHitTestWidget extends SingleChildRenderObjectWidget {
  final ValueGetter<MenuWidgetState> menuStateProvider;

  const _MenuSafeTriangleHitTestWidget({
    required this.menuStateProvider,
    required super.child,
  });

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderMenuSafeaTriangleWidget(menuStateProvider);
  }

  @override
  void updateRenderObject(BuildContext context,
      covariant _RenderMenuSafeaTriangleWidget renderObject) {
    renderObject.menuStateProvider = menuStateProvider;
  }
}

class _OpenedSubmenuPosition {
  final Offset position;
  final RenderBox submenuRenderBox;
  final Stopwatch _timestamp;

  Duration get elapsed => _timestamp.elapsed;

  _OpenedSubmenuPosition({
    required this.position,
    required this.submenuRenderBox,
  }) : _timestamp = Stopwatch()..start();
}

class _RenderMenuSafeaTriangleWidget extends RenderProxyBox {
  ValueGetter<MenuWidgetState> menuStateProvider;

  _RenderMenuSafeaTriangleWidget(this.menuStateProvider);

  _OpenedSubmenuPosition? _openedSubmenuPosition;

  static bool _offsetWithinTriangle(
      Offset offset, Offset a, Offset b, Offset c) {
    // barycentric coordinate method
    double denominator =
        ((b.dy - c.dy) * (a.dx - c.dx) + (c.dx - b.dx) * (a.dy - c.dy));
    double bA = ((b.dy - c.dy) * (offset.dx - c.dx) +
            (c.dx - b.dx) * (offset.dy - c.dy)) /
        denominator;
    double bB = ((c.dy - a.dy) * (offset.dx - c.dx) +
            (a.dx - c.dx) * (offset.dy - c.dy)) /
        denominator;
    double bC = 1 - bA - bB;
    return bA >= 0 && bB >= 0 && bC >= 0;
  }

  bool _offsetInSafeArea(Offset offset, Offset a, RenderBox menuBox) {
    // Transform all coordinates to parent space
    final transformOurs = getTransformTo(parent as RenderObject);
    final transformTheirs = menuBox.getTransformTo(parent as RenderObject);
    offset = MatrixUtils.transformPoint(transformOurs, offset);
    a = MatrixUtils.transformPoint(transformOurs, a);
    final menuRect =
        MatrixUtils.transformRect(transformTheirs, Offset.zero & menuBox.size);

    // determine menu edge coordinate
    double menuX;
    if (menuRect.right < a.dx) {
      menuX = menuRect.right;
    } else if (menuRect.left > a.dx) {
      menuX = menuRect.left;
    } else {
      return false; // overlapping?
    }

    final b = Offset(menuX, menuRect.top);
    final c = Offset(menuX, menuRect.bottom);

    return _offsetWithinTriangle(offset, a, b, c);
  }

  Timer? _cleanupTimer;

  void _cleanup() {
    if (_openedSubmenuPosition != null) {
      // TODO(knopp): Uncomment the code below once MouseTracker.updateAllDevices() (without arguments) is available in stable
      // WidgetsBinding.instance.scheduleFrameCallback((timeStamp) {
      //   WidgetsBinding.instance.mouseTracker.updateAllDevices();
      // });
    }
    _openedSubmenuPosition = null;
    _cleanupTimer?.cancel();
    _cleanupTimer = null;
  }

  @override
  bool hitTest(BoxHitTestResult result, {required Offset position}) {
    // Cleanup leftover position if render box was detached in the meanwhile
    if (_openedSubmenuPosition?.submenuRenderBox.attached != true) {
      _openedSubmenuPosition = null;
    }

    RenderBox? getSubmenuRenderBox(HitTestEntry entry) {
      if (entry.target is RenderMetaData) {
        final data = (entry.target as RenderMetaData).metaData;
        if (data is MenuWidgetItemMetaData) {
          return data.submenuRenderBox();
        }
      }
      return null;
    }

    final tempResult = BoxHitTestResult();
    hitTestChildren(tempResult, position: position);
    for (final entry in tempResult.path) {
      RenderBox? renderBox = getSubmenuRenderBox(entry);
      if (renderBox != null) {
        _openedSubmenuPosition = _OpenedSubmenuPosition(
          submenuRenderBox: renderBox,
          position: position,
        );
        _cleanupTimer?.cancel();
        _cleanupTimer = Timer(const Duration(milliseconds: 500), _cleanup);
        break;
      }
    }

    final openedSubmenuPositon = _openedSubmenuPosition;

    if (openedSubmenuPositon != null) {
      // if offset is within safe area reuse last position recorded while over
      // selected item
      if (_offsetInSafeArea(position, openedSubmenuPositon.position,
          openedSubmenuPositon.submenuRenderBox)) {
        position = openedSubmenuPositon.position;
      }
    }

    if (size.contains(position)) {
      final res = hitTestChildren(result, position: position);
      if (result.path.none((e) => getSubmenuRenderBox(e) != null)) {
        _openedSubmenuPosition = null;
      }
      if (res || hitTestSelf(position)) {
        result.add(BoxHitTestEntry(this, position));
        return true;
      }
    }
    return false;
  }
}
