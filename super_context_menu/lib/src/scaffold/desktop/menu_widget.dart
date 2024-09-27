import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:pixel_snap/pixel_snap.dart';

import '../../menu_model.dart';
import '../common/deferred_menu_items.dart';
import 'menu_widget_builder.dart';

abstract class MenuWidgetDelegate {
  void pushMenu({
    required Menu parent,
    required Menu menu,
    required BuildContext context,
    required MenuWidgetFocusMode focusMode,
  });
  RenderBox? getMenuRenderBox(Menu menu);
  TextDirection getDirectionalityForMenu(Menu menu);
  void popUntil(Menu? menu);
  void hide({
    required bool itemSelected,
  });
}

enum MenuWidgetFocusMode {
  none,
  menu,
  firstItem,
}

class MenuWidgetItemMetaData {
  MenuWidgetItemMetaData({
    required this.submenuRenderBox,
  });

  final ValueGetter<RenderBox?> submenuRenderBox;
}

class MenuWidget extends StatefulWidget {
  const MenuWidget({
    super.key,
    required this.parentMenu,
    required this.menuWidgetBuilder,
    required this.menu,
    required this.delegate,
    required this.focusMode,
    required this.iconTheme,
    required this.cache,
    required this.tapRegionGroupIds,
    required this.parentFocusNode,
  });

  final DesktopMenuWidgetBuilder menuWidgetBuilder;
  final MenuWidgetFocusMode focusMode;
  final Menu? parentMenu;
  final Menu menu;
  final MenuWidgetDelegate delegate;
  final IconThemeData iconTheme;
  final DeferredMenuElementCache cache;
  final Set<Object> tapRegionGroupIds;
  final FocusNode? parentFocusNode;

  @override
  State<StatefulWidget> createState() => MenuWidgetState();
}

abstract class _ChildEntryDelegate {
  void _didFocusEntry(_ChildEntry entry);
}

class _ChildEntry {
  final _ChildEntryDelegate delegate;
  final MenuElement element;
  final key = GlobalKey();
  final innerKey = GlobalKey();
  final focusNode = FocusNode();

  bool get focusable =>
      element is Menu ||
      (element is MenuAction && !(element as MenuAction).attributes.disabled);

  void dispose() {
    focusNode.dispose();
  }

  _ChildEntry(this.element, this.delegate) {
    focusNode.addListener(() {
      delegate._didFocusEntry(this);
    });
  }
}

class MenuWidgetState extends State<MenuWidget>
    with DeferredMenuItemsContainer<_ChildEntry, MenuWidget>
    implements _ChildEntryDelegate, _MenuItemWidgetDelegate {
  final _focusScope = FocusScopeNode();
  final _focusNode = FocusNode();

  @override
  newChild(MenuElement e) => _ChildEntry(e, this);

  @override
  bool childHasMenuElement(element, menuElement) =>
      element.element == menuElement;

  bool _pendingFocusApply = true;

  final _scrollController = PixelSnapScrollController();

  @override
  void initState() {
    initDeferredElements(widget.menu.children, widget.cache);
    WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
      if (mounted) {
        _pendingFocusApply = false;
        if (widget.focusMode == MenuWidgetFocusMode.menu) {
          _focusScope.requestFocus();
        } else if (widget.focusMode == MenuWidgetFocusMode.firstItem) {
          resolvedChildren.firstOrNull?.focusNode.requestFocus();
        }
      }
    });
    super.initState();
  }

  @override
  void dispose() {
    _focusScope.dispose();
    _focusNode.dispose();
    for (final item in resolvedChildren) {
      item.dispose();
    }
    _scrollController.dispose();
    super.dispose();
  }

  @override
  void _itemActivated(_ChildEntry entry) {
    final element = entry.element;
    if (element is MenuAction && !element.attributes.disabled) {
      // Make sure the focus is restored to previous node before invoking
      // the callback as the callback may invoke intent on primary focus.
      _focusScope.canRequestFocus = false;
      WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
        element.callback.call();
      });
      widget.delegate.hide(itemSelected: true);
    }
  }

  DesktopMenuInfo _menuInfo() {
    final bool focused;
    // Focus has one frame latency. We know we'll be focused in one frame
    // so set focused to true immediately to prevent flicker.
    if (_pendingFocusApply) {
      focused = widget.focusMode == MenuWidgetFocusMode.firstItem ||
          widget.focusMode == MenuWidgetFocusMode.menu;
    } else {
      focused = _focusScope.hasFocus;
    }
    return DesktopMenuInfo(
      menu: widget.menu,
      parentMenu: widget.parentMenu,
      resolvedChildren:
          resolvedChildren.map((e) => e.element).toList(growable: false),
      iconTheme: widget.iconTheme,
      focused: focused,
    );
  }

  Iterable<_ChildEntry> get _focusableChildEntries =>
      resolvedChildren.where((e) => e.focusable);

  LogicalKeyboardKey getLeadingKey({
    required bool currentItemHasMenu,
  }) {
    final TextDirection directionality;
    if (currentItemHasMenu) {
      // This is a bit confusing behavior, but at least it makes it possible
      // to return from item with submenu in flipped menu.
      directionality = Directionality.of(context);
    } else {
      directionality = widget.delegate.getDirectionalityForMenu(widget.menu);
    }
    if (directionality == TextDirection.ltr) {
      return LogicalKeyboardKey.arrowLeft;
    } else {
      return LogicalKeyboardKey.arrowRight;
    }
  }

  LogicalKeyboardKey getTrailingKey() {
    // When opening submenu always use system directionality so that direction
    // matches the trailing arrow indicator. This is how macOS does it.
    final directionality = Directionality.of(context);
    if (directionality == TextDirection.ltr) {
      return LogicalKeyboardKey.arrowRight;
    } else {
      return LogicalKeyboardKey.arrowLeft;
    }
  }

  void onInitialPointerUp() {
    final selectedEntry = resolvedChildren
        .firstWhereOrNull((element) => element.focusNode.hasFocus);
    if (selectedEntry != null) {
      _itemActivated(selectedEntry);
    } else {
      widget.delegate.hide(itemSelected: false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final menuInfo = _menuInfo();
    Widget child = FocusScope(
      parentNode: widget.parentFocusNode,
      onKeyEvent: (_, e) {
        if (e is! KeyDownEvent && e is! KeyRepeatEvent) {
          return KeyEventResult.handled;
        }
        if (_focusScope.hasPrimaryFocus) {
          if (e.logicalKey == LogicalKeyboardKey.arrowDown ||
              e.logicalKey == LogicalKeyboardKey.arrowLeft ||
              e.logicalKey == LogicalKeyboardKey.arrowRight) {
            _focusableChildEntries.firstOrNull?.focusNode.requestFocus();
            return KeyEventResult.handled;
          } else if (e.logicalKey == LogicalKeyboardKey.arrowUp) {
            _focusableChildEntries.lastOrNull?.focusNode.requestFocus();
            return KeyEventResult.handled;
          }
        }

        final selectedEntry = resolvedChildren
            .firstWhereOrNull((element) => element.focusNode.hasFocus);
        if (selectedEntry != null && selectedEntry.element is Menu) {
          final trailingKey = getTrailingKey();
          if (e.logicalKey == trailingKey ||
              e.logicalKey == LogicalKeyboardKey.enter) {
            widget.delegate.pushMenu(
              parent: widget.menu,
              menu: selectedEntry.element as Menu,
              context: selectedEntry.innerKey.currentContext!,
              focusMode: MenuWidgetFocusMode.firstItem,
            );
            return KeyEventResult.handled;
          }
        }

        final leadingKey =
            getLeadingKey(currentItemHasMenu: selectedEntry?.element is Menu);

        if (e.logicalKey == leadingKey) {
          if (widget.parentMenu != null) {
            widget.delegate.popUntil(widget.parentMenu!);
            return KeyEventResult.handled;
          }
        } else if (e.logicalKey == LogicalKeyboardKey.escape) {
          widget.delegate.hide(itemSelected: false);
          return KeyEventResult.handled;
        }

        if (selectedEntry != null && e.logicalKey == LogicalKeyboardKey.enter) {
          _itemActivated(selectedEntry);
        }

        return KeyEventResult.handled;
      },
      node: _focusScope,
      child: FocusTraversalGroup(
        child: Shortcuts(
          shortcuts: const {
            SingleActivator(LogicalKeyboardKey.arrowUp):
                DirectionalFocusIntent(TraversalDirection.up),
            SingleActivator(LogicalKeyboardKey.arrowDown):
                DirectionalFocusIntent(TraversalDirection.down),
          },
          child: SingleChildScrollView(
            controller: _scrollController,
            child: IntrinsicWidth(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (final item in resolvedChildren)
                    if (item.element is MenuSeparator)
                      _SeparatorWidget(
                        delegate: this,
                        menuInfo: menuInfo,
                        menuWidgetBuilder: widget.menuWidgetBuilder,
                        separator: item.element as MenuSeparator,
                      )
                    else
                      MetaData(
                        metaData: MenuWidgetItemMetaData(
                          submenuRenderBox: () {
                            if (item.element is Menu) {
                              return widget.delegate
                                  .getMenuRenderBox(item.element as Menu);
                            } else {
                              return null;
                            }
                          },
                        ),
                        child: _MenuItemWidget(
                          delegate: this,
                          key: item.key,
                          innerKey: item.innerKey,
                          menuInfo: menuInfo,
                          menuWidgetBuilder: widget.menuWidgetBuilder,
                          isSelected: _isSelected(item),
                          entry: item,
                        ),
                      )
                ],
              ),
            ),
          ),
        ),
      ),
    );
    for (final groupId in widget.tapRegionGroupIds) {
      child = TapRegion(
        groupId: groupId,
        child: child,
      );
    }
    return widget.menuWidgetBuilder.buildMenuContainer(
      context,
      menuInfo,
      child,
    );
  }

  bool _isSelected(_ChildEntry entry) {
    if (_pendingFocusApply) {
      // Prevent flicker - flutter focus has one frame latency.
      return widget.focusMode == MenuWidgetFocusMode.firstItem &&
          entry == resolvedChildren.firstOrNull;
    } else {
      return entry.focusNode.isSelected;
    }
  }

  @override
  void _didFocusEntry(_ChildEntry entry) {
    setState(() {});
    if (entry.element is Menu) {
      widget.delegate.popUntil(entry.element as Menu);
    } else {
      widget.delegate.popUntil(widget.menu);
    }
  }

  bool hasFocus() {
    return _focusScope.hasFocus;
  }

  @override
  void focusMenu() {
    _focusScope.requestFocus();
    if (primaryFocus != _focusScope) {
      primaryFocus?.unfocus();
    }

    widget.delegate.popUntil(widget.menu);
  }

  void focusFirstItem() {
    _focusableChildEntries.firstOrNull?.focusNode.requestFocus();
  }

  @override
  void _didHover(_ChildEntry entry) {
    if (entry.element is Menu) {
      widget.delegate.pushMenu(
        parent: widget.menu,
        menu: entry.element as Menu,
        context: entry.innerKey.currentContext!,
        focusMode: MenuWidgetFocusMode.none,
      );
    }
  }
}

class _SeparatorWidget extends StatelessWidget {
  final _MenuItemWidgetDelegate delegate;
  final DesktopMenuWidgetBuilder menuWidgetBuilder;
  final DesktopMenuInfo menuInfo;
  final MenuSeparator separator;

  const _SeparatorWidget({
    required this.delegate,
    required this.menuWidgetBuilder,
    required this.menuInfo,
    required this.separator,
  });

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onHover: (_) {
        delegate.focusMenu();
      },
      onEnter: (_) {
        delegate.focusMenu();
      },
      child: menuWidgetBuilder.buildSeparator(
        context,
        menuInfo,
        separator,
      ),
    );
  }
}

abstract class _MenuItemWidgetDelegate {
  void _didHover(_ChildEntry entry);
  void _itemActivated(_ChildEntry entry);
  void focusMenu();
}

class _MenuItemWidget extends StatefulWidget {
  final _ChildEntry entry;
  final _MenuItemWidgetDelegate delegate;
  final DesktopMenuInfo menuInfo;
  final DesktopMenuWidgetBuilder menuWidgetBuilder;
  final bool isSelected;

  final Key innerKey;

  const _MenuItemWidget({
    super.key,
    required this.innerKey,
    required this.delegate,
    required this.entry,
    required this.menuInfo,
    required this.menuWidgetBuilder,
    required this.isSelected,
  });

  @override
  State<_MenuItemWidget> createState() => _MenuItemWidgetState();
}

class _MenuItemWidgetState extends State<_MenuItemWidget> {
  Timer? _hoverTimer;

  @override
  void dispose() {
    _hoverTimer?.cancel();
    super.dispose();
  }

  void _onHover() {
    if (widget.entry.focusable) {
      widget.entry.focusNode.requestFocus();
      _hoverTimer ??= Timer(const Duration(milliseconds: 100), () {
        widget.delegate._didHover(widget.entry);
        _hoverTimer = null;
      });
    } else {
      widget.delegate.focusMenu();
    }
  }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTapDown: (_) {},
      onTapUp: (_) {
        widget.delegate._itemActivated(widget.entry);
      },
      child: Focus(
        focusNode: widget.entry.focusNode,
        canRequestFocus: widget.entry.focusable,
        child: MouseRegion(
          hitTestBehavior: HitTestBehavior.opaque,
          onHover: (_) {
            _onHover();
          },
          onEnter: (_) {
            _onHover();
          },
          onExit: (_) {
            _hoverTimer?.cancel();
            _hoverTimer = null;
          },
          child: widget.menuWidgetBuilder.buildMenuItem(
            context,
            widget.menuInfo,
            widget.innerKey,
            DesktopMenuButtonState(
              selected: widget.isSelected,
            ),
            widget.entry.element,
          ),
        ),
      ),
    );
  }
}

extension on FocusNode {
  /// Focus node is selected when either it has focus, or it would have focus
  /// if the nearest scope was focused.
  bool get isSelected {
    if (hasFocus) {
      return true;
    }
    final focusedChild = nearestScope?.focusedChild;
    return focusedChild != null &&
        (focusedChild == this || focusedChild.ancestors.contains(this));
  }
}
