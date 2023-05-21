import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import '../../menu_model.dart';
import '../common/deferred_menu_items.dart';
import 'disableable_scroll_controller.dart';
import 'menu_widget_builder.dart';

abstract class MenuWidgetDelegate {
  void pushMenu(BuildContext context, Menu menu);
  void popMenu();
  void hideMenu({required bool itemSelected});
}

class MenuWidget extends StatefulWidget {
  const MenuWidget({
    super.key,
    required this.menuInfo,
    required this.builder,
    required this.delegate,
    required this.canScrollListenable,
    required this.cache,
  });

  final MobileMenuInfo menuInfo;
  final MobileMenuWidgetBuilder builder;
  final MenuWidgetDelegate delegate;
  final ValueListenable<bool> canScrollListenable;
  final DeferredMenuElementCache cache;

  @override
  State<StatefulWidget> createState() => MenuWidgetState();
}

class MenuWidgetState extends State<MenuWidget>
    with DeferredMenuItemsContainer<MenuElement, MenuWidget> {
  late DisableableScrollController scrollController;

  final _keys = <MenuElement, GlobalKey>{};

  @override
  MenuElement newChild(MenuElement e) => e;

  @override
  bool childHasMenuElement(MenuElement element, MenuElement menuElement) =>
      element == menuElement;

  @override
  void initState() {
    super.initState();
    scrollController = DisableableScrollController(widget.canScrollListenable);
    initDeferredElements(widget.menuInfo.menu.children, widget.cache);
  }

  @override
  void deactivate() {
    scrollController.detachListener();
    super.deactivate();
  }

  @override
  void dispose() {
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
        widget.delegate.hideMenu(itemSelected: true);
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
                  MobileMenuButtonState(
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
                            MobileMenuButtonState(
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
