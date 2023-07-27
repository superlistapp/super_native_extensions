import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import 'package:super_context_menu/src/menu_internal.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:super_native_extensions/raw_menu.dart' as raw;

import 'scaffold/desktop/menu_session.dart';
import 'scaffold/desktop/menu_widget_builder.dart';
import 'util.dart';

class _ContextMenuDetector extends StatefulWidget {
  const _ContextMenuDetector({
    required this.hitTestBehavior,
    required this.contextMenuIsAllowed,
    required this.onShowContextMenu,
    required this.child,
  });

  final Widget child;
  final HitTestBehavior hitTestBehavior;
  final ContextMenuIsAllowed contextMenuIsAllowed;
  final Function(Offset, Listenable) onShowContextMenu;

  @override
  State<StatefulWidget> createState() => _ContextMenuDetectorState();
}

class _ContextMenuDetectorState extends State<_ContextMenuDetector> {
  int? _pointerDown;
  Stopwatch? _pointerDownStopwatch;

  final _onPointerUp = SimpleNotifier();

  bool _acceptPrimaryButton() {
    final keys = RawKeyboard.instance.keysPressed;
    return defaultTargetPlatform == TargetPlatform.macOS &&
        keys.length == 1 &&
        keys.contains(LogicalKeyboardKey.controlLeft);
  }

  bool _canAcceptEvent(PointerDownEvent event) {
    if (event.kind != PointerDeviceKind.mouse) {
      return false;
    }
    if (event.buttons == kSecondaryButton ||
        event.buttons == kPrimaryButton && _acceptPrimaryButton()) {
      return widget.contextMenuIsAllowed(event.position);
    }

    return false;
  }

  @override
  Widget build(BuildContext context) {
    return Listener(
      behavior: widget.hitTestBehavior,
      onPointerDown: (event) {
        if (_canAcceptEvent(event)) {
          _pointerDown = event.pointer;
          _pointerDownStopwatch = Stopwatch()..start();
          widget.onShowContextMenu(event.position, _onPointerUp);
        }
      },
      onPointerUp: (event) {
        if (_pointerDown == event.pointer) {
          _pointerDown = null;
          // Pointer up would trigger currently selected item. Make sure we don't
          // do this on simple right click.
          if ((_pointerDownStopwatch?.elapsedMilliseconds ?? 0) > 300) {
            _onPointerUp.notify();
          }
          _pointerDownStopwatch = null;
        }
      },
      child: widget.child,
    );
  }
}

class DesktopContextMenuWidget extends StatelessWidget {
  const DesktopContextMenuWidget({
    super.key,
    required this.child,
    required this.hitTestBehavior,
    required this.menuProvider,
    required this.contextMenuIsAllowed,
    required this.menuWidgetBuilder,
    this.iconTheme,
  });

  final HitTestBehavior hitTestBehavior;
  final MenuProvider menuProvider;
  final ContextMenuIsAllowed contextMenuIsAllowed;
  final DesktopMenuWidgetBuilder menuWidgetBuilder;
  final Widget child;

  /// Base icon theme for menu icons. The size will be overridden depending
  /// on platform.
  final IconThemeData? iconTheme;

  @override
  Widget build(BuildContext context) {
    return _ContextMenuDetector(
      hitTestBehavior: hitTestBehavior,
      contextMenuIsAllowed: contextMenuIsAllowed,
      onShowContextMenu: (position, pointerUpListenable) {
        _onContextMenu(
          context,
          position,
          pointerUpListenable,
        );
      },
      // Used on web to determine whether to prevent browser context menu
      child: BaseContextMenuRenderWidget(
        contextMenuIsAllowed: contextMenuIsAllowed,
        getConfiguration: (_) async => null,
        hitTestBehavior: hitTestBehavior,
        child: child,
      ),
    );
  }

  raw.MenuSerializationOptions _serializationOptions(BuildContext context) {
    final mq = MediaQuery.of(context);
    final iconTheme = this.iconTheme ??
        const IconThemeData.fallback().copyWith(
          color: mq.platformBrightness == Brightness.light
              ? const Color(0xFF090909)
              : const Color(0xFFF0F0F0),
        );
    return raw.MenuSerializationOptions(
      iconTheme: iconTheme,
      destructiveIconTheme: iconTheme,
      devicePixelRatio: mq.devicePixelRatio,
    );
  }

  void _onContextMenu(
    BuildContext context,
    Offset globalPosition,
    Listenable onInitialPointerUp,
  ) async {
    final onShowMenu = SimpleNotifier();
    final onHideMenu = ValueNotifier<raw.MenuResult?>(null);
    final onPreviewAction = SimpleNotifier();
    raw.MenuHandle? handle;
    try {
      final request = MenuRequest(
        onShowMenu: onShowMenu,
        onHideMenu: onHideMenu,
        onPreviewAction: onPreviewAction,
        location: globalPosition,
      );
      final menu = await menuProvider(request);
      final menuContext = await raw.MenuContext.instance();
      if (menu != null && context.mounted) {
        final serializationOptions = _serializationOptions(context);
        handle = await menuContext.registerMenu(
          menu,
          serializationOptions,
        );
        // ignore: use_build_context_synchronously
        if (!context.mounted) {
          onHideMenu.value = raw.MenuResult(itemSelected: false);
          return;
        }
        onShowMenu.notify();
        final request = raw.DesktopContextMenuRequest(
            iconTheme: serializationOptions.iconTheme,
            position: globalPosition,
            menu: handle,
            fallback: () {
              final completer = Completer<MenuResult>();
              ContextMenuSession(
                context: context,
                iconTheme: serializationOptions.iconTheme,
                menu: handle!.menu,
                menuWidgetBuilder: menuWidgetBuilder,
                onDone: (value) => completer.complete(value),
                onInitialPointerUp: onInitialPointerUp,
                position: globalPosition,
              );
              return completer.future;
            });
        final res = await menuContext.showContextMenu(request);
        onHideMenu.value = res;
      }
    } finally {
      onShowMenu.dispose();
      onPreviewAction.dispose();
      onHideMenu.dispose();
      handle?.dispose();
    }
  }
}
