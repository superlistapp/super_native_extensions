import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import '../../cancellation_token.dart';
import '../../menu.dart';
import 'disableable_scroll_controller.dart';
import 'menu_widget_builder.dart';

abstract class MenuWidgetDelegate {
  void pushMenu(BuildContext context, Menu menu);
  void popMenu();
  void hideMenu();
}

class MenuWidget extends StatefulWidget {
  const MenuWidget({
    super.key,
    required this.menuInfo,
    required this.builder,
    required this.delegate,
    required this.canScrollListenable,
  });

  final MenuInfo menuInfo;
  final MenuWidgetBuilder builder;
  final MenuWidgetDelegate delegate;
  final ValueListenable<bool> canScrollListenable;

  @override
  State<StatefulWidget> createState() => MenuWidgetState();
}

class MenuWidgetState extends State<MenuWidget> {
  late DisableableScrollController scrollController;

  late List<MenuElement> resolvedChildren;
  final _keys = <MenuElement, GlobalKey>{};

  @override
  void initState() {
    super.initState();
    scrollController = DisableableScrollController(widget.canScrollListenable);
    resolvedChildren = widget.menuInfo.menu.children;
    _loadDeferred();
  }

  final _inProgressTokens = <SimpleCancellationToken>[];

  void _loadDeferred() {
    for (final element in widget.menuInfo.menu.children) {
      if (element is DeferredMenuElement) {
        _loadDeferredElement(element);
      }
    }
  }

  void _loadDeferredElement(DeferredMenuElement element) {
    final token = SimpleCancellationToken();
    element.provider(token).then((value) {
      if (!token.cancelled) {
        token.dispose();
      }
      _inProgressTokens.remove(token);
      if (mounted) {
        _didLoadItemsForElement(element, value);
      }
    });
    _inProgressTokens.add(token);
  }

  void _didLoadItemsForElement(
      DeferredMenuElement element, List<MenuElement> items) {
    final index = resolvedChildren.indexOf(element);
    if (index != -1) {
      setState(() {
        resolvedChildren.removeAt(index);
        resolvedChildren.insertAll(index, items);
      });
    }
  }

  @override
  void deactivate() {
    scrollController.detachListener();
    super.deactivate();
  }

  @override
  void dispose() {
    for (final token in _inProgressTokens) {
      token.cancel();
    }
    scrollController.dispose();
    super.dispose();
  }

  void _onHeaderTap() {
    if (!widget.menuInfo.isRoot) {
      if (widget.menuInfo.isCollapsed) {
        widget.delegate.pushMenu(context, widget.menuInfo.menu);
      } else {
        widget.delegate.popMenu();
      }
    }
  }

  void _onElementTap(BuildContext context, MenuElement element) {
    if (element is Menu) {
      widget.delegate.pushMenu(context, element);
    } else if (element is MenuAction) {
      if (!element.attributes.disabled) {
        element.callback();
        widget.delegate.hideMenu();
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final child = Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (widget.menuInfo.menu.title?.isNotEmpty == true ||
            !widget.menuInfo.isRoot)
          _MenuButton(
            onTapUp: _onHeaderTap,
            enabled: !widget.menuInfo.isRoot,
            builder: (context, pressed) {
              return widget.builder.buildMenuHeader(
                  context,
                  widget.menuInfo,
                  MenuButtonState(
                    pressed: pressed,
                  ));
            },
          ),
        Flexible(
          child: _ZeroIntrinsicSize(
            child: widget.builder.buildMenuItemsContainer(
              context,
              widget.menuInfo,
              ListView.builder(
                controller: scrollController,
                shrinkWrap: true,
                itemCount: resolvedChildren.length,
                itemBuilder: (context, index) {
                  final element = resolvedChildren[index];
                  final key = _keys.putIfAbsent(element, () => GlobalKey());
                  final bool enabled = element is Menu ||
                      (element is MenuAction) && !element.attributes.disabled;
                  return Builder(
                    key: key,
                    builder: (context) {
                      return _MenuButton(
                        enabled: enabled,
                        onTapUp: () => _onElementTap(context, element),
                        builder: (context, pressed) {
                          return widget.builder.buildMenuItem(
                            context,
                            widget.menuInfo,
                            MenuButtonState(
                              pressed: pressed && enabled,
                            ),
                            element,
                          );
                        },
                      );
                    },
                  );
                },
              ),
            ),
          ),
        ),
      ],
    );
    return ConstrainedBox(
      constraints: const BoxConstraints.tightFor(width: 250),
      child: MediaQuery.removePadding(
        removeTop: true,
        removeBottom: true,
        context: context,
        child: widget.builder.buildMenu(context, widget.menuInfo, child),
      ),
    );
  }
}

class _ZeroIntrinsicSize extends SingleChildRenderObjectWidget {
  const _ZeroIntrinsicSize({
    required super.child,
  });

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _ZeroIntrinsicSizeRenderBox();
  }
}

class _ZeroIntrinsicSizeRenderBox extends RenderProxyBox {
  @override
  double computeMinIntrinsicWidth(double height) => 0.0;

  @override
  double computeMaxIntrinsicWidth(double height) => 0.0;

  @override
  double computeMinIntrinsicHeight(double width) => 0.0;

  @override
  double computeMaxIntrinsicHeight(double width) => 0.0;
}

typedef _MenuButtonBuilder = Widget Function(
    BuildContext context, bool pressed);

class _MenuButton extends StatefulWidget {
  const _MenuButton({
    // ignore: unused_element
    super.key,
    required this.enabled,
    required this.onTapUp,
    required this.builder,
  });

  final bool enabled;
  final VoidCallback onTapUp;
  final _MenuButtonBuilder builder;

  @override
  State<StatefulWidget> createState() => _MenuButtonState();
}

class _MenuButtonState extends State<_MenuButton> {
  bool pressed = false;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTapDown: (details) => setState(() => pressed = true),
      onTapUp: (details) {
        setState(() => pressed = false);
        if (widget.enabled) {
          widget.onTapUp();
        }
      },
      onTapCancel: () => setState(() => pressed = false),
      child: widget.builder(context, widget.enabled && pressed),
    );
  }
}
